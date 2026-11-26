//! Host dispatch for the raw R24.8/R24.9 bounce signals.

use bytemuck::{Pod, Zeroable};
use forge_render_api::{Lighting, RenderError, RenderSettings, Snapshot};
use forge_render_cpu::lights::PLight;

use super::direct_shadow::{RawSignalBindings, RawSignalProgram, SignalFrameLayout, upload_lights};
use super::{Bounce, PrimaryInput};
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

pub(super) struct ReflectionProgram(RawSignalProgram, Bounce);

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
    pub fn new(
        device: &wgpu::Device,
        kind: BackendKind,
        source: String,
        bitmap_count: usize,
    ) -> Self {
        Self::for_bounce(device, kind, source, bitmap_count, Bounce::Reflection)
    }

    #[cfg(test)]
    pub fn new_diffuse(
        device: &wgpu::Device,
        kind: BackendKind,
        source: String,
        bitmap_count: usize,
    ) -> Self {
        Self::for_bounce(device, kind, source, bitmap_count, Bounce::Diffuse)
    }

    fn for_bounce(
        device: &wgpu::Device,
        kind: BackendKind,
        source: String,
        bitmap_count: usize,
        bounce: Bounce,
    ) -> Self {
        let (entry, layout) = match bounce {
            Bounce::Reflection => ("reflection_signal", SignalFrameLayout::Reflection),
            Bounce::Diffuse => ("diffuse_signal", SignalFrameLayout::Diffuse),
        };
        Self(
            RawSignalProgram::new(device, kind, source, bitmap_count, entry, layout),
            bounce,
        )
    }

    #[cfg(test)]
    pub fn pipeline(&self) -> wgpu::ComputePipeline {
        self.0.pipeline()
    }

    pub fn read(
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
            Bounce::Reflection => Some(reflection_environment(
                device,
                queue,
                &snapshot.environment.lighting,
                &mut frame,
            )?),
            Bounce::Diffuse => None,
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
