//! Viewport performance benchmarks (docs/specs/VIEWPORT.md §10.10):
//! `cargo bench -p forge-viewport`. CPU benches (instance layout, draw-list building, content
//! revision, instance picking) always run; the upload and frame benches need a GPU adapter
//! and are skipped without one (lavapipe counts: numbers on it measure the CPU side of an
//! upload and the software rasteriser for frames, not a real GPU).

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use forge_geometry::PolyMesh;
use forge_geometry::selection::Selection;
use forge_viewport::cache::{instances_revision, mesh_revision};
use forge_viewport::camera::Camera;
use forge_viewport::cull::Frustum;
use forge_viewport::instancing::{INSTANCE_CHUNK, InstanceInput, build_layout, visible_ranges};
use forge_viewport::picking::pick_instances;
use forge_viewport::render::{
    DrawInstanced, DrawMesh, Frame, Gpu, InstanceSet, MeshBuffers, Settings, render_to_rgba,
};
use glam::{DMat4, DQuat, DVec3};

struct Clones {
    matrices: Vec<DMat4>,
    colors: Vec<[f64; 3]>,
    visible: Vec<bool>,
    child: Vec<u32>,
}

impl Clones {
    /// `n` clones on a square grid in the XZ plane, spacing 3, each turned a little.
    fn new(n: usize) -> Clones {
        let side = (n as f64).sqrt().ceil() as usize;
        let mut c = Clones {
            matrices: Vec::with_capacity(n),
            colors: Vec::with_capacity(n),
            visible: vec![true; n],
            child: vec![0; n],
        };
        for k in 0..n {
            let (i, j) = (k / side, k % side);
            c.matrices.push(DMat4::from_scale_rotation_translation(
                DVec3::splat(0.8),
                DQuat::from_rotation_y(k as f64 * 0.1),
                DVec3::new(i as f64 * 3.0, 0.0, j as f64 * 3.0),
            ));
            c.colors.push([1.0, (k % 7) as f64 / 7.0, 0.5]);
        }
        c
    }

    fn input(&self) -> InstanceInput<'_> {
        InstanceInput {
            matrices: &self.matrices,
            colors: &self.colors,
            visible: &self.visible,
            child: &self.child,
        }
    }

    fn extent(&self) -> f64 {
        (self.matrices.len() as f64).sqrt() * 3.0
    }
}

fn cube() -> PolyMesh {
    let p = |x: f64, y: f64, z: f64| DVec3::new(x, y, z);
    PolyMesh {
        points: vec![
            p(-1.0, -1.0, -1.0),
            p(1.0, -1.0, -1.0),
            p(-1.0, 1.0, -1.0),
            p(1.0, 1.0, -1.0),
            p(-1.0, -1.0, 1.0),
            p(1.0, -1.0, 1.0),
            p(-1.0, 1.0, 1.0),
            p(1.0, 1.0, 1.0),
        ],
        polys: vec![
            vec![0, 1, 5, 4],
            vec![2, 6, 7, 3],
            vec![0, 2, 3, 1],
            vec![4, 5, 7, 6],
            vec![0, 4, 6, 2],
            vec![1, 3, 7, 5],
        ],
    }
}

/// A bumpy sheet of `n x n` quads (2 n^2 triangles).
fn sheet(n: u32) -> PolyMesh {
    let mut points = Vec::with_capacity(((n + 1) * (n + 1)) as usize);
    for i in 0..=n {
        for j in 0..=n {
            let (x, z) = (f64::from(i) * 0.01, f64::from(j) * 0.01);
            points.push(DVec3::new(x, (x * 3.0).sin() * (z * 2.0).cos() * 0.3, z));
        }
    }
    let mut polys = Vec::with_capacity((n * n) as usize);
    for i in 0..n {
        for j in 0..n {
            let a = i * (n + 1) + j;
            polys.push(vec![a, a + 1, a + n + 2, a + n + 1]);
        }
    }
    PolyMesh { points, polys }
}

fn camera_over(c: &Clones) -> Camera {
    let e = c.extent();
    Camera {
        target: DVec3::new(e / 2.0, 0.0, e / 2.0),
        distance: e * 1.2,
        pitch: 0.9,
        yaw: 0.5,
        ..Camera::default()
    }
}

fn cpu(c: &mut Criterion) {
    let unit = Some((DVec3::splat(-1.0), DVec3::splat(1.0)));
    let protos = [cube()];
    for n in [10_000usize, 100_000] {
        let clones = Clones::new(n);
        let mut g = c.benchmark_group("instances");
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::new("build_layout", n), &clones, |b, cl| {
            b.iter(|| {
                black_box(build_layout(
                    &[unit],
                    &cl.input(),
                    &DMat4::IDENTITY,
                    INSTANCE_CHUNK,
                ))
            })
        });
        g.bench_with_input(BenchmarkId::new("revision", n), &clones, |b, cl| {
            b.iter(|| black_box(instances_revision(&[1], &cl.input(), &DMat4::IDENTITY)))
        });
        let layout = build_layout(&[unit], &clones.input(), &DMat4::IDENTITY, INSTANCE_CHUNK);
        let cam = camera_over(&clones);
        // a whole-field view and a close-up that culls most chunks
        let close = Camera {
            distance: clones.extent() * 0.15,
            target: DVec3::new(clones.extent() * 0.3, 0.0, clones.extent() * 0.3),
            ..cam
        };
        for (name, cam) in [("draw_list_all", cam), ("draw_list_close", close)] {
            let f = Frustum::from_view_proj(&cam.view_proj(4.0 / 3.0));
            g.bench_with_input(BenchmarkId::new(name, n), &layout, |b, l| {
                b.iter(|| black_box(visible_ranges(&l.chunks, &f, None)))
            });
        }
        let ray = camera_over(&clones).ray(400.0, 300.0, 800.0, 600.0);
        g.bench_with_input(BenchmarkId::new("pick", n), &clones, |b, cl| {
            b.iter(|| black_box(pick_instances(&protos, &cl.input(), &DMat4::IDENTITY, &ray)))
        });
        g.finish();
    }
    // the content hash a cache lookup costs for a 1M-triangle mesh
    let big = sheet(708);
    let mut g = c.benchmark_group("mesh");
    g.sample_size(20);
    g.throughput(Throughput::Elements(2 * 708 * 708));
    g.bench_function("revision_1M_tris", |b| {
        b.iter(|| black_box(mesh_revision(&big)))
    });
    g.finish();
}

fn gpu_benches(c: &mut Criterion) {
    let gpu = match Gpu::new_headless() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("skipping GPU benchmarks: {e}");
            return;
        }
    };
    eprintln!(
        "adapter: {} ({:?})",
        gpu.adapter_info.name, gpu.adapter_info.device_type
    );
    let protos = [cube()];
    let mut g = c.benchmark_group("upload");
    g.sample_size(10);
    for n in [10_000usize, 100_000] {
        let clones = Clones::new(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::new("instance_set", n), &clones, |b, cl| {
            b.iter(|| {
                black_box(InstanceSet::new(
                    &gpu.device,
                    &protos,
                    Some(std::f64::consts::PI),
                    &cl.input(),
                    &DMat4::IDENTITY,
                ))
            })
        });
    }
    let big = sheet(708);
    g.throughput(Throughput::Elements(2 * 708 * 708));
    g.bench_function("mesh_1M_tris", |b| {
        b.iter(|| {
            black_box(MeshBuffers::with_device_no_points(
                &gpu.device,
                &big,
                &Selection::default(),
                Some(std::f64::consts::PI),
            ))
        })
    });
    g.finish();

    // frames at 320 x 240 on whatever adapter this is (software on CI)
    let mut g = c.benchmark_group("frame_320x240");
    g.sample_size(10);
    for n in [10_000usize, 100_000] {
        let clones = Clones::new(n);
        let set = InstanceSet::new(
            &gpu.device,
            &protos,
            Some(std::f64::consts::PI),
            &clones.input(),
            &DMat4::IDENTITY,
        );
        let cam = camera_over(&clones);
        g.bench_with_input(BenchmarkId::new("instanced", n), &set, |b, set| {
            b.iter(|| {
                let frame = Frame {
                    meshes: &[],
                    instanced: &[DrawInstanced {
                        set,
                        color: [0.7; 3],
                    }],
                    grid: None,
                    gizmo: None,
                    settings: Settings::default(),
                };
                black_box(render_to_rgba(&gpu, 320, 240, &cam, &frame))
            })
        });
    }
    // the same 10k clones as one expanded mesh, to show what instancing saves
    let clones = Clones::new(10_000);
    let mut flat = PolyMesh::default();
    for m in &clones.matrices {
        let base = flat.points.len() as u32;
        flat.points
            .extend(protos[0].points.iter().map(|p| m.transform_point3(*p)));
        flat.polys.extend(
            protos[0]
                .polys
                .iter()
                .map(|f| f.iter().map(|v| v + base).collect::<Vec<u32>>()),
        );
    }
    let mesh = MeshBuffers::with_device_no_points(
        &gpu.device,
        &flat,
        &Selection::default(),
        Some(std::f64::consts::PI),
    );
    let cam = camera_over(&clones);
    g.bench_function("expanded_10000", |b| {
        b.iter(|| {
            let frame = Frame {
                meshes: &[DrawMesh {
                    mesh: &mesh,
                    color: [0.7; 3],
                }],
                instanced: &[],
                grid: None,
                gizmo: None,
                settings: Settings::default(),
            };
            black_box(render_to_rgba(&gpu, 320, 240, &cam, &frame))
        })
    });
    g.bench_function("expanded_10000_upload", |b| {
        b.iter(|| {
            black_box(MeshBuffers::with_device_no_points(
                &gpu.device,
                &flat,
                &Selection::default(),
                Some(std::f64::consts::PI),
            ))
        })
    });
    g.finish();
}

criterion_group!(benches, cpu, gpu_benches);
criterion_main!(benches);
