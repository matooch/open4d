mod support;

use forge_render_api::{Control, RenderOutput, Snapshot};
use forge_render_gpu::pack::Tri;
use forge_render_rt::RtEngine;
use forge_render_rt::ray::{BackendKind, preferred_backend};
use std::collections::BTreeMap;
use std::error::Error;
use std::hint::black_box;
use std::time::Instant;
use support::{Identity, Run, SAMPLES, WARMUPS, fixture, regression_check, settings};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let mut args = std::env::args()
        .skip(1)
        .filter(|arg| !matches!(arg.as_str(), "--bench" | "--test"));
    let Some(machine) = args.next() else {
        eprintln!(
            "Usage: cargo bench -p forge-render-rt --bench s10 -- MACHINE tier1|tier2|compute CURRENT.json [BASELINE.json]"
        );
        return Ok(());
    };
    let mode = args.next().ok_or("missing tier: tier1, tier2 or compute")?;
    let current_path = args.next().ok_or("missing output file")?;
    let baseline: Option<Run> = args
        .next()
        .map(|path| -> Result<Run> { Ok(serde_json::from_reader(std::fs::File::open(path)?)?) })
        .transpose()?;
    if args.next().is_some() || machine.trim().is_empty() {
        return Err("expected a machine label, tier, output and optional baseline".into());
    }
    let hybrid = match mode.as_str() {
        "tier1" => false,
        "tier2" | "compute" => true,
        _ => return Err("expected tier1, tier2 or compute".into()),
    };
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))?;
    let info = adapter.get_info();
    let advertised = adapter.limits();
    let expected_backend = match mode.as_str() {
        "tier1" => None,
        "compute" => Some(BackendKind::Compute),
        _ => {
            let kind = preferred_backend(info.backend, adapter.features());
            if kind == BackendKind::Compute {
                return Err(format!(
                    "hardware Tier 2 unavailable on {:?}: {}",
                    info.backend, info.name
                )
                .into());
            }
            Some(kind)
        }
    };

    let snapshot = fixture();
    let triangles: usize = snapshot
        .instances
        .iter()
        .map(|inst| snapshot.meshes[inst.mesh].triangles.len())
        .sum();
    let bytes = (triangles * size_of::<Tri>()) as u64;
    if bytes > advertised.max_storage_buffer_binding_size || bytes > advertised.max_buffer_size {
        return Err(format!(
            "S10 needs a {bytes}-byte raster storage binding; {} supports a {}-byte binding and a {}-byte buffer",
            info.name, advertised.max_storage_buffer_binding_size, advertised.max_buffer_size
        )
        .into());
    }
    let mut desc = wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits {
            max_storage_buffer_binding_size: bytes,
            max_buffer_size: bytes,
            ..Default::default()
        },
        ..Default::default()
    };
    if matches!(
        expected_backend,
        Some(BackendKind::Metal | BackendKind::Vulkan)
    ) {
        desc.required_limits = desc
            .required_limits
            .using_acceleration_structure_values(advertised);
        desc.required_features = wgpu::Features::EXPERIMENTAL_RAY_QUERY;
        // SAFETY: the adapter advertised ray queries and Forge uses wgpu's validated API.
        desc.experimental_features = unsafe { wgpu::ExperimentalFeatures::enabled() };
    }
    let (device, queue) = pollster::block_on(adapter.request_device(&desc))?;
    let identity = Identity::new(
        &machine,
        &format!(
            "{} ({:04x}:{:04x}) {} {}",
            info.name, info.vendor, info.device, info.driver, info.driver_info
        ),
        &format!("{:?}", info.backend),
        expected_backend,
        hybrid,
    );
    let survey = Measurement {
        snapshot: &snapshot,
        expected_backend,
        triangles,
        hybrid,
    };
    let mut timings =
        survey.measure(&RtEngine::with_device(device.clone(), queue.clone()), false)?;
    let profiled_engine = RtEngine::with_device(device, queue);
    for (name, samples) in survey.measure(&profiled_engine, true)? {
        timings.insert(name, samples);
    }
    let current = Run {
        identity,
        median_ns: timings
            .into_iter()
            .map(|(name, mut samples)| {
                samples.sort_unstable();
                (name, samples[samples.len() / 2])
            })
            .collect(),
    };
    serde_json::to_writer_pretty(std::fs::File::create(current_path)?, &current)?;
    for (name, nanos) in &current.median_ns {
        println!("{name:>18}: {:9.3} ms", *nanos as f64 / 1e6);
    }
    let fps = 1e9 / current.median_ns["frame"] as f64;
    let target = if hybrid { 30.0 } else { 60.0 };
    println!(
        "{mode} on {} ({:?}/{expected_backend:?}): {fps:.3} fps; {target:.0} fps target {}",
        info.name,
        info.backend,
        if fps >= target { "met" } else { "missed" }
    );
    if std::env::var("FORGE_S10_CAMERA_SETUP").as_deref() == Ok("1") {
        let setup = survey.measure_camera_setup(&profiled_engine)?;
        println!("camera-only setup: {:9.3} ms", setup as f64 / 1e6);
    }
    if let Some(baseline) = baseline {
        regression_check(&baseline, &current)?;
        println!("Same-machine baseline: no timing regressed by more than 10%");
    }
    Ok(())
}

struct Measurement<'a> {
    snapshot: &'a Snapshot,
    expected_backend: Option<BackendKind>,
    triangles: usize,
    hybrid: bool,
}

impl Measurement<'_> {
    fn measure(&self, engine: &RtEngine, profiled: bool) -> Result<BTreeMap<String, Vec<u64>>> {
        let settings = settings(self.hybrid);
        let control = Control::default();
        let mut timings = BTreeMap::<String, Vec<u64>>::new();
        let mut expected_passes = vec!["setup", "shadow-atlas", "g-buffer"];
        if self.hybrid {
            expected_passes.extend([
                "hybrid-primary",
                "rt-ao",
                "rt-shadows",
                "rt-reflections",
                "rt-gi",
                "hybrid-upload",
            ]);
        }
        expected_passes.extend(["lighting", "output"]);
        for i in 0..WARMUPS + SAMPLES {
            let (output, backend) = if profiled {
                let (output, backend, passes) =
                    engine.render_profiled(self.snapshot, &settings, &control)?;
                if !passes
                    .iter()
                    .map(|pass| pass.name)
                    .eq(expected_passes.iter().copied())
                {
                    return Err(format!("unexpected passes for {backend:?}: {passes:?}").into());
                }
                if i >= WARMUPS {
                    for pass in passes {
                        timings
                            .entry(pass.name.into())
                            .or_default()
                            .push(pass.elapsed.as_nanos().try_into()?);
                    }
                }
                (output, backend)
            } else {
                let start = Instant::now();
                let result = engine.render_with_backend(self.snapshot, &settings, &control)?;
                let elapsed = start.elapsed();
                if i >= WARMUPS {
                    timings
                        .entry("frame".into())
                        .or_default()
                        .push(elapsed.as_nanos().try_into()?);
                }
                result
            };
            self.check_frame(&output, backend, settings.width, settings.height)?;
            black_box(output);
        }
        Ok(timings)
    }

    fn measure_camera_setup(&self, engine: &RtEngine) -> Result<u64> {
        let settings = settings(self.hybrid);
        let control = Control::default();
        let mut moved = self.snapshot.clone();
        moved.camera.transform[12] += 10.0;
        let mut setup = Vec::new();
        // Keep this series separate from the unchanged-scene run and its JSON baseline.
        for i in 0..WARMUPS + SAMPLES {
            let snapshot = if i % 2 == 0 { &moved } else { self.snapshot };
            let (output, backend, passes) =
                engine.render_profiled(snapshot, &settings, &control)?;
            self.check_frame(&output, backend, settings.width, settings.height)?;
            if i >= WARMUPS {
                let timing = passes
                    .iter()
                    .find(|pass| pass.name == "setup")
                    .ok_or("missing camera-only setup timing")?;
                setup.push(timing.elapsed.as_nanos().try_into()?);
            }
            black_box(output);
        }
        setup.sort_unstable();
        Ok(setup[setup.len() / 2])
    }

    fn check_frame(
        &self,
        output: &RenderOutput,
        backend: Option<BackendKind>,
        w: u32,
        h: u32,
    ) -> Result<()> {
        if backend != self.expected_backend {
            return Err(format!(
                "expected {:?}, actual ray backend {backend:?}",
                self.expected_backend
            )
            .into());
        }
        if (
            output.width,
            output.height,
            output.stats.triangles,
            output.stats.instances,
        ) != (w, h, self.triangles, self.snapshot.instances.len())
        {
            return Err(format!(
                "incomplete S10 frame: {}×{}, {:?}",
                output.width, output.height, output.stats
            )
            .into());
        }
        Ok(())
    }
}
