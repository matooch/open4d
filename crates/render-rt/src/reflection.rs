//! Host dispatch for the raw R24.8/R24.9 bounce signals.

use bytemuck::{Pod, Zeroable};
use forge_render_api::{Lighting, RenderError, RenderSettings, Snapshot};
use forge_render_cpu::lights::PLight;

use super::direct_shadow::{RawSignalBindings, RawSignalProgram, SignalFrameLayout, upload_lights};
use super::{Bounce, PrimaryInput, bounce_spatial::BounceSpatialPass};
use crate::ray::{BackendKind, InlineScene};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ReflectionFrame {
    camera: [[f32; 4]; 3],
    film_scale: [f32; 4],
    jitter: [f32; 2],
    seed: [u32; 2],
    history: u32,
    instance_count: u32,
    epsilon: f32,
    lighting_kind: u32,
    lighting: [f32; 4],
    zenith_rotation: [f32; 4],
    ground: [f32; 4],
    light_count: u32,
    _padding: [u32; 3],
}

pub(super) struct ReflectionProgram(RawSignalProgram, BounceEntry);

#[derive(Clone, Copy)]
pub(super) enum BounceEntry {
    Reflection,
    Diffuse { reduced: bool },
}

impl BounceEntry {
    pub fn live(bounce: Bounce) -> Self {
        match bounce {
            Bounce::Reflection => Self::Reflection,
            Bounce::Diffuse => Self::Diffuse { reduced: false },
        }
    }

    pub fn source(self, backend: BackendKind, materials: &str) -> String {
        match self {
            Self::Reflection => crate::shader::reflection_source(backend, materials),
            Self::Diffuse { reduced: false } => crate::shader::diffuse_source(backend, materials),
            Self::Diffuse { reduced: true } => {
                crate::shader::reduced_diffuse_source(backend, materials)
            }
        }
    }

    pub(super) fn dispatch(self) -> (&'static str, SignalFrameLayout) {
        match self {
            Self::Reflection => ("reflection_signal", SignalFrameLayout::Reflection),
            Self::Diffuse { reduced: false } => ("diffuse_signal", SignalFrameLayout::Diffuse),
            Self::Diffuse { reduced: true } => {
                ("reduced_diffuse_signal", SignalFrameLayout::ReducedDiffuse)
            }
        }
    }
}

pub(super) struct ReflectionBindings<'a> {
    pub scene: InlineScene<'a>,
    pub primary: [&'a wgpu::TextureView; 2],
    pub attributes: &'a wgpu::Buffer,
    pub bitmaps: &'a [wgpu::TextureView],
    pub lights: &'a [PLight],
}

#[derive(Clone, Copy)]
pub(super) struct ReflectionInput<'a> {
    pub snapshot: &'a Snapshot,
    pub settings: &'a RenderSettings,
    pub input: PrimaryInput<'a>,
    pub scale: f64,
    pub history: u32,
}

impl ReflectionProgram {
    #[cfg(test)]
    pub fn new(
        device: &wgpu::Device,
        kind: BackendKind,
        source: String,
        bitmap_count: usize,
    ) -> Self {
        Self::for_entry(device, kind, source, bitmap_count, BounceEntry::Reflection)
    }

    #[cfg(test)]
    pub fn new_diffuse(
        device: &wgpu::Device,
        kind: BackendKind,
        source: String,
        bitmap_count: usize,
    ) -> Self {
        Self::for_entry(
            device,
            kind,
            source,
            bitmap_count,
            BounceEntry::Diffuse { reduced: false },
        )
    }

    #[cfg(test)]
    pub fn new_reduced_diffuse(
        device: &wgpu::Device,
        kind: BackendKind,
        source: String,
        bitmap_count: usize,
    ) -> Self {
        Self::for_entry(
            device,
            kind,
            source,
            bitmap_count,
            BounceEntry::Diffuse { reduced: true },
        )
    }

    pub fn for_entry(
        device: &wgpu::Device,
        kind: BackendKind,
        source: String,
        bitmap_count: usize,
        entry: BounceEntry,
    ) -> Self {
        let (symbol, layout) = entry.dispatch();
        Self(
            RawSignalProgram::new(device, kind, source, bitmap_count, symbol, layout),
            entry,
        )
    }

    #[cfg(test)]
    pub fn pipeline(&self) -> wgpu::ComputePipeline {
        self.0.pipeline()
    }

    pub fn read(
        &self,
        gpu: (&wgpu::Device, &wgpu::Queue),
        bindings: ReflectionBindings<'_>,
        input: ReflectionInput<'_>,
    ) -> Result<Vec<[f32; 4]>, RenderError> {
        self.read_signal(gpu, bindings, input, None)
    }

    pub fn read_spatial(
        &self,
        gpu: (&wgpu::Device, &wgpu::Queue),
        bindings: ReflectionBindings<'_>,
        input: ReflectionInput<'_>,
        spatial: BounceSpatialPass<'_>,
    ) -> Result<Vec<[f32; 4]>, RenderError> {
        self.read_signal(gpu, bindings, input, Some(spatial))
    }

    fn read_signal(
        &self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        bindings: ReflectionBindings<'_>,
        ReflectionInput {
            snapshot,
            settings,
            input,
            scale,
            history,
        }: ReflectionInput<'_>,
        spatial: Option<BounceSpatialPass<'_>>,
    ) -> Result<Vec<[f32; 4]>, RenderError> {
        let (f, [x, y]) = snapshot
            .camera
            .pixel_frame(settings.width, settings.height)
            .unwrap_or_default();
        let [u, v, w, _] = snapshot.camera.matrix().as_mat4().to_cols_array_2d();
        let lights = upload_lights(device, bindings.lights)?;
        let mut frame = ReflectionFrame {
            camera: [u, v, w],
            film_scale: [f as f32, x as f32, y as f32, scale as f32],
            jitter: input.jitter,
            seed: [settings.seed as u32, (settings.seed >> 32) as u32],
            history,
            instance_count: snapshot.instances.len() as u32,
            epsilon: input.epsilon as f32,
            light_count: bindings.lights.len() as u32,
            ..ReflectionFrame::zeroed()
        };
        let environment = match self.1 {
            BounceEntry::Reflection => Some(reflection_environment(
                device,
                queue,
                &snapshot.environment.lighting,
                &mut frame,
            )?),
            BounceEntry::Diffuse { .. } => None,
        };
        self.0.read(
            (device, queue),
            RawSignalBindings {
                scene: bindings.scene,
                primary: bindings.primary,
                attributes: bindings.attributes,
                bitmaps: bindings.bitmaps,
                lights: &lights,
                environment: environment.as_ref(),
                spatial,
            },
            settings,
            &frame,
        )
    }
}

fn reflection_environment(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    lighting: &Lighting,
    frame: &mut ReflectionFrame,
) -> Result<wgpu::TextureView, RenderError> {
    let pack = |[r, g, b]: [f64; 3], w: f64| [r as f32, g as f32, b as f32, w as f32];
    let dummy = [0.; 4];
    let (width, height, texels) = match lighting {
        Lighting::None => (1, 1, &dummy[..]),
        Lighting::Constant { color, intensity } => {
            frame.lighting_kind = 1;
            frame.lighting = pack(*color, *intensity);
            (1, 1, &dummy[..])
        }
        Lighting::Sky {
            zenith,
            horizon,
            ground,
            intensity,
        } => {
            frame.lighting_kind = 2;
            frame.lighting = pack(*horizon, *intensity);
            frame.zenith_rotation = pack(*zenith, 0.);
            frame.ground = pack(*ground, 0.);
            (1, 1, &dummy[..])
        }
        Lighting::Image {
            image,
            rotation,
            intensity,
        } => {
            frame.lighting_kind = 3;
            frame.lighting[3] = *intensity as f32;
            frame.zenith_rotation[3] = rotation.rem_euclid(std::f64::consts::TAU) as f32;
            (image.width, image.height, image.data.as_slice())
        }
    };
    let max_dim = device.limits().max_texture_dimension_2d;
    if width == 0 || height == 0 || width > max_dim || height > max_dim {
        return Err(RenderError::Unsupported(
            "raw reflection environment dimensions exceed device limits".into(),
        ));
    }
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let environment = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("raw reflection environment"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        environment.as_image_copy(),
        bytemuck::cast_slice(texels),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 16),
            rows_per_image: Some(height),
        },
        size,
    );
    Ok(environment.create_view(&Default::default()))
}
