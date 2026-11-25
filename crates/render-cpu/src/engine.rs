//! The CPU engine: tiles, pixels, filters, outputs (RENDER.md R2-R4, R9).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use forge_image::{AlphaMode, Image};
use forge_render_api::{
    Aov, Control, PixelFilter, RenderEngine, RenderError, RenderEvent, RenderOutput,
    RenderSettings, RenderStats, Snapshot,
};
use glam::DVec3;
use rayon::prelude::*;

use crate::integrator::{Ctx, Sample, trace_path};
use crate::lights::prepare;
use crate::sampling::{Rng, splitmix};
use crate::scene::Scene;

#[derive(Debug, Default, Clone, Copy)]
pub struct CpuEngine;

impl CpuEngine {
    pub fn new() -> CpuEngine {
        CpuEngine
    }
}

/// Offset from the pixel centre for a uniform pair under the pixel filter (support `w`).
fn filter_offset(f: PixelFilter, w: f64, u: (f64, f64), rng: &mut Rng) -> (f64, f64) {
    let tri = |u: f64| {
        if u < 0.5 {
            (2.0 * u).sqrt() - 1.0
        } else {
            1.0 - (2.0 * (1.0 - u)).sqrt()
        }
    };
    match f {
        PixelFilter::Box => ((u.0 - 0.5) * w, (u.1 - 0.5) * w),
        PixelFilter::Triangle => (tri(u.0) * w * 0.5, tri(u.1) * w * 0.5),
        PixelFilter::Gaussian => {
            let r = (-2.0 * u.0.max(1e-12).ln()).sqrt() * (w * 0.25);
            let a = std::f64::consts::TAU * u.1;
            (
                (r * a.cos()).clamp(-w * 0.5, w * 0.5),
                (r * a.sin()).clamp(-w * 0.5, w * 0.5),
            )
        }
        // Four uniforms sum to the cubic B-spline density on [-2, 2].
        PixelFilter::Cubic => {
            let (a, b) = rng.f2();
            let (c, d) = rng.f2();
            (
                (u.0 + a + b + c - 2.0) * w * 0.25,
                (u.1 + d + rng.f() + rng.f() - 2.0) * w * 0.25,
            )
        }
    }
}

pub fn ignored_channels(snap: &Snapshot) -> Vec<String> {
    let mut v = Vec::new();
    for m in &snap.materials {
        let o = &m.graph.outputs;
        for (name, on) in [
            ("normal", o.normal.is_some()),
            ("alpha", o.alpha.is_some()),
            ("displacement", o.displacement.is_some()),
            ("fog", o.fog.is_some()),
            ("environment", o.environment.is_some()),
            ("diffusion", o.diffusion.is_some()),
        ] {
            if on && !v.iter().any(|x| x == name) {
                v.push(name.to_string());
            }
        }
    }
    v
}

struct TileOut {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    beauty: Vec<[f32; 4]>,
    aovs: Vec<Vec<[f32; 4]>>,
}

fn v4(c: DVec3, a: f64) -> [f32; 4] {
    [c.x as f32, c.y as f32, c.z as f32, a as f32]
}

fn render_pixel(cx: &Ctx, x: u32, y: u32, wanted: &[Aov]) -> ([f32; 4], Vec<[f32; 4]>) {
    let st = cx.settings;
    let n = st.samples;
    let pix = splitmix(st.seed ^ splitmix(((x as u64) << 32) | y as u64));
    let mut rng = Rng::new(pix, pix >> 17);
    let (sx, sy) = rng.f2();
    let mut acc = Sample::default();
    let mut hits = 0u32;
    let (mut vis, mut tot) = (0.0, 0.0);
    let mut first_hit: Option<Sample> = None;
    let mut cover = 0u32;
    let (mut depth, mut normal, mut albedo, mut pos) = (0.0, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO);
    for i in 0..n {
        // R2 sequence with a per-pixel shift.
        let u = (
            (sx + 0.5 + i as f64 * 0.754_877_666_246_692_7).fract(),
            (sy + 0.5 + i as f64 * 0.569_840_290_998_053_2).fract(),
        );
        let (ox, oy) = filter_offset(st.filter, st.filter_width, u, &mut rng);
        let (px, py) = (x as f64 + 0.5 + ox, y as f64 + 0.5 + oy);
        let (o, d) = if st.depth_of_field && cx.snap.camera.aperture.is_some() {
            // only a lens render draws from the stream here, so pinhole renders are unchanged
            let lens = rng.f2();
            cx.snap.camera.ray_lens(px, py, st.width, st.height, lens)
        } else {
            cx.snap.camera.ray(px, py, st.width, st.height)
        };
        let s = trace_path(cx, o, d, &mut rng);
        if !s.rgb.is_finite() {
            continue;
        }
        acc.rgb += s.rgb;
        acc.diffuse += s.diffuse;
        acc.specular += s.specular;
        acc.emission += s.emission;
        tot += s.shadow_total;
        vis += s.shadow_visible;
        if s.covered {
            cover += 1;
        }
        if s.covered && s.depth > 0.0 {
            hits += 1;
            depth += s.depth;
            normal += s.normal;
            albedo += s.albedo;
            pos += s.position;
            if first_hit.is_none() {
                first_hit = Some(s);
            }
        }
    }
    let inv = 1.0 / n as f64;
    let alpha = cover as f64 * inv;
    let beauty = v4(acc.rgb * inv, alpha);
    let hinv = if hits > 0 { 1.0 / hits as f64 } else { 0.0 };
    let aovs = wanted
        .iter()
        .map(|a| match a {
            Aov::Depth => v4(DVec3::splat(depth * hinv), alpha),
            Aov::Normal => v4(normal.normalize_or_zero(), alpha),
            Aov::Albedo => v4(albedo * hinv, alpha),
            Aov::Position => v4(pos * hinv, alpha),
            Aov::ObjectId => v4(
                DVec3::splat(first_hit.as_ref().map_or(0.0, |s| s.object_id)),
                alpha,
            ),
            Aov::MaterialId => v4(
                DVec3::splat(first_hit.as_ref().map_or(0.0, |s| s.material_id)),
                alpha,
            ),
            Aov::Shadow => v4(DVec3::splat(if tot > 0.0 { vis / tot } else { 1.0 }), alpha),
            Aov::Diffuse => v4(acc.diffuse * inv, alpha),
            Aov::Specular => v4(acc.specular * inv, alpha),
            Aov::Emission => v4(acc.emission * inv, alpha),
        })
        .collect();
    (beauty, aovs)
}

impl RenderEngine for CpuEngine {
    fn id(&self) -> &str {
        "forge.render.cpu"
    }

    fn render(
        &self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        control: &Control,
    ) -> Result<RenderOutput, RenderError> {
        settings.validate()?;
        let scene = Scene::build(snapshot)?;
        let lights = prepare(snapshot, settings.default_light)?;
        let cx = Ctx {
            snap: snapshot,
            scene: &scene,
            lights: &lights,
            settings,
            env: crate::lights::EnvDist::build(&snapshot.environment.lighting),
        };
        let (w, h) = (settings.width, settings.height);
        let [rx0, ry0, rx1, ry1] = settings.region.unwrap_or([0, 0, w, h]);
        let mut wanted: Vec<Aov> = settings.aovs.clone();
        wanted.sort();
        wanted.dedup();
        let ts = settings.tile;
        let mut tiles = Vec::new();
        let mut y = ry0;
        while y < ry1 {
            let mut x = rx0;
            while x < rx1 {
                tiles.push((x, y, ts.min(rx1 - x), ts.min(ry1 - y)));
                x += ts;
            }
            y += ts;
        }
        control.emit(RenderEvent::Started {
            width: w,
            height: h,
            tiles: tiles.len(),
        });
        let done = AtomicUsize::new(0);
        let total = tiles.len();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(settings.threads.unwrap_or(0))
            .build()
            .map_err(|e| RenderError::Settings(e.to_string()))?;
        let outs: Vec<Option<TileOut>> = pool.install(|| {
            tiles
                .par_iter()
                .map(|&(tx, ty, tw, th)| {
                    if control.is_cancelled() {
                        return None;
                    }
                    let mut beauty = Vec::with_capacity((tw * th) as usize);
                    let mut aovs = vec![Vec::with_capacity((tw * th) as usize); wanted.len()];
                    for y in ty..ty + th {
                        for x in tx..tx + tw {
                            let (b, a) = render_pixel(&cx, x, y, &wanted);
                            beauty.push(b);
                            for (k, v) in a.into_iter().enumerate() {
                                aovs[k].push(v);
                            }
                        }
                    }
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    control.emit(RenderEvent::Tile {
                        x: tx,
                        y: ty,
                        width: tw,
                        height: th,
                        rgba: beauty.iter().flatten().copied().collect(),
                    });
                    control.emit(RenderEvent::Progress { done: n, total });
                    Some(TileOut {
                        x: tx,
                        y: ty,
                        w: tw,
                        h: th,
                        beauty,
                        aovs,
                    })
                })
                .collect()
        });
        if control.is_cancelled() || outs.iter().any(Option::is_none) {
            return Err(RenderError::Cancelled);
        }
        let mut beauty = Image::new(w, h, AlphaMode::Premultiplied);
        let mut aov_imgs: BTreeMap<Aov, Image> = wanted
            .iter()
            .map(|a| (*a, Image::new(w, h, AlphaMode::Straight)))
            .collect();
        for t in outs.into_iter().flatten() {
            for j in 0..t.h {
                for i in 0..t.w {
                    let k = (j * t.w + i) as usize;
                    beauty.set_pixel(t.x + i, t.y + j, t.beauty[k]);
                    for (a, img) in wanted.iter().zip(t.aovs.iter()) {
                        if let Some(im) = aov_imgs.get_mut(a) {
                            im.set_pixel(t.x + i, t.y + j, img[k]);
                        }
                    }
                }
            }
        }
        control.emit(RenderEvent::Finished);
        Ok(RenderOutput {
            width: w,
            height: h,
            beauty,
            aovs: aov_imgs,
            stats: RenderStats {
                triangles: scene.triangles,
                instances: scene.insts.len(),
                tiles: total,
                samples: (rx1 - rx0) as u64 * (ry1 - ry0) as u64 * settings.samples as u64,
                ignored_channels: ignored_channels(snapshot),
            },
        })
    }
}
