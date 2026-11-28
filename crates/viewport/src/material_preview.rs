//! Bounded UV-space bake of the viewport subset of the material IR (VIEWPORT.md §15).

use forge_material::{IrNode, ShadeCtx, ShadingGraph, TextureSampler};
use wgpu::util::DeviceExt;

const TEXTURE_SIZE: u32 = 128;

pub struct PreviewMaterial {
    pub(crate) view: wgpu::TextureView,
    pub(crate) params: [f32; 4],
}

fn textures(graph: &ShadingGraph, ix: usize, present: &impl Fn(&str) -> bool) -> bool {
    match &graph.nodes[ix] {
        IrNode::Color(_) => false,
        IrNode::Fresnel {
            ior,
            falloff,
            center_darkening,
            center_hold,
        } => [ior, falloff, center_darkening, center_hold]
            .into_iter()
            .any(|i| textures(graph, *i, present)),
        IrNode::Luminance(input) | IrNode::Component { input, .. } => {
            textures(graph, *input, present)
        }
        IrNode::Ramp {
            a,
            b,
            position,
            start,
            end,
            midpoint,
        } => [a, b, position, start, end, midpoint]
            .into_iter()
            .any(|i| textures(graph, *i, present)),
        IrNode::Texture(name) => present(name),
        IrNode::Scale { input, .. } => textures(graph, *input, present),
        IrNode::Mix { base, top, .. } => {
            textures(graph, *base, present) || textures(graph, *top, present)
        }
    }
}

pub(crate) fn texture(size: u32) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("viewport preview maps"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 2,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    }
}

impl PreviewMaterial {
    /// `present` identifies the texture names that successfully built in `sampler`.
    pub fn bake(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        graph: &ShadingGraph,
        sampler: &dyn TextureSampler,
        present: impl Fn(&str) -> bool,
    ) -> Self {
        let d = graph.outputs.diffuse.as_ref();
        let s = graph.outputs.specular.as_ref();
        let roughness = s.and(graph.outputs.roughness_input);
        let base_roughness = s.map_or(0.5, |s| s.width);
        let metal = s.is_some_and(|s| s.metal);
        let color = d
            .map(|d| d.color)
            .or_else(|| s.filter(|_| metal).map(|s| s.color));
        let normal = graph.outputs.normal.as_ref().filter(|n| {
            n.method == "tangent" && n.strength > 0.0 && textures(graph, n.texture, &present)
        });
        let size = if normal.is_some()
            || [color, roughness]
                .into_iter()
                .flatten()
                .any(|i| textures(graph, i, &present))
        {
            TEXTURE_SIZE
        } else {
            1
        };
        let mut bytes = Vec::with_capacity((size * size * 8) as usize);
        for layer in 0..2 {
            for y in 0..size {
                for x in 0..size {
                    let ctx = ShadeCtx {
                        uvw: [
                            (f64::from(x) + 0.5) / f64::from(size),
                            (f64::from(y) + 0.5) / f64::from(size),
                            0.0,
                        ],
                        ..ShadeCtx::default()
                    };
                    let rgb = if layer == 0 {
                        color.map_or([0.0; 3], |ix| {
                            graph
                                .eval(ix, sampler, &ctx)
                                .map(|c| c * d.map_or(1.0, |d| d.level))
                        })
                    } else {
                        normal.map_or([0.5, 0.5, 1.0], |n| {
                            let mut map = graph.eval(n.texture, sampler, &ctx);
                            if n.swap_yz {
                                map.swap(1, 2);
                            }
                            for (v, flip) in map.iter_mut().zip(n.flip) {
                                if flip {
                                    *v = 1.0 - *v;
                                }
                            }
                            map
                        })
                    };
                    bytes.extend(rgb.map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8));
                    let alpha = if layer == 0 {
                        roughness.map_or(base_roughness, |ix| graph.eval(ix, sampler, &ctx)[0])
                    } else {
                        1.0
                    };
                    bytes.push((alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
                }
            }
        }
        let tex = device.create_texture_with_data(
            queue,
            &texture(size),
            wgpu::util::TextureDataOrder::LayerMajor,
            &bytes,
        );
        Self {
            view: tex.create_view(&wgpu::TextureViewDescriptor::default()),
            params: [
                1.0,
                0.0,
                if metal { 1.0 } else { 0.0 },
                normal.map_or(0.0, |n| n.strength) as f32,
            ],
        }
    }
}
