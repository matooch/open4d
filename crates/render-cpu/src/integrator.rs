//! The path tracer: NEE + BSDF sampling with MIS, AOV bookkeeping (RENDER.md R3, R9).

use forge_render_api::{Background, RenderSettings, Snapshot};
use glam::DVec3;

use crate::lights::{EnvDist, PLight, env_pdf, env_radiance, has_env, sample_env};
use crate::sampling::{Rng, lum, power_heuristic};
use crate::scene::Scene;
use crate::surface::{Lobe, Surface, fresnel, refract};

pub struct Ctx<'a> {
    pub snap: &'a Snapshot,
    pub scene: &'a Scene,
    pub lights: &'a [PLight],
    pub settings: &'a RenderSettings,
    /// Importance sampling of an environment image, if the lighting is one.
    pub env: Option<EnvDist>,
}

/// First-hit data and the light split of one camera sample.
#[derive(Debug, Clone, Default)]
pub struct Sample {
    pub rgb: DVec3,
    pub covered: bool,
    pub depth: f64,
    pub normal: DVec3,
    pub albedo: DVec3,
    pub position: DVec3,
    pub object_id: f64,
    pub material_id: f64,
    pub shadow_total: f64,
    pub shadow_visible: f64,
    pub diffuse: DVec3,
    pub specular: DVec3,
    pub emission: DVec3,
}

#[derive(Clone, Copy)]
enum Target {
    Emission,
    Diffuse,
    Specular,
}

fn target_of(cat: Option<Lobe>) -> Target {
    match cat {
        None => Target::Emission,
        Some(Lobe::Diffuse) => Target::Diffuse,
        Some(Lobe::Specular) => Target::Specular,
    }
}

impl Sample {
    fn add(&mut self, c: DVec3, depth: u32, t: Target, clamp: f64) {
        let mut c = c;
        if clamp > 0.0 && depth > 0 {
            let m = c.max_element();
            if m > clamp {
                c *= clamp / m;
            }
        }
        self.rgb += c;
        match t {
            Target::Emission => self.emission += c,
            Target::Diffuse => self.diffuse += c,
            Target::Specular => self.specular += c,
        }
    }
}

const FAR: f64 = f64::INFINITY;

pub fn trace_path(cx: &Ctx, mut o: DVec3, mut d: DVec3, rng: &mut Rng) -> Sample {
    let snap = cx.snap;
    let scene = cx.scene;
    let clamp = cx.settings.clamp;
    let eps = 1e-6 * scene.scale;
    let env = &snap.environment.lighting;
    let mut s = Sample::default();
    let mut throughput = DVec3::ONE;
    let mut prev_pdf = 0.0;
    let mut prev_delta = true;
    let mut cat: Option<Lobe> = None;

    for depth in 0..cx.settings.max_depth.max(1) {
        let hit = scene.intersect(o, d, FAR);
        let tmax = hit.as_ref().map_or(FAR, |h| h.t);
        // Emitting area lights.
        let mut lh: Option<(f64, DVec3, f64)> = None;
        for l in cx.lights {
            if !l.is_area() || (depth == 0 && !l.light.visible) {
                continue;
            }
            let lim = lh.map_or(tmax, |x| x.0);
            if let Some(x) = l.hit(o, d, lim) {
                lh = Some(x);
            }
        }
        if let Some((_, le, pdf)) = lh {
            let w = if prev_delta {
                1.0
            } else {
                power_heuristic(prev_pdf, pdf)
            };
            s.add(throughput * le * w, depth, target_of(cat), clamp);
            if depth == 0 {
                s.covered = true;
            }
            break;
        }
        let Some(hit) = hit else {
            if depth == 0 {
                if let Background::Color(c) = snap.environment.background {
                    s.rgb += DVec3::from(c);
                } else {
                    s.rgb += env_radiance(env, d);
                }
            } else if has_env(env) {
                let le = env_radiance(env, d);
                let w = if prev_delta {
                    1.0
                } else {
                    power_heuristic(prev_pdf, env_pdf(cx.env.as_ref(), d))
                };
                s.add(throughput * le * w, depth, target_of(cat), clamp);
            }
            break;
        };
        let geom = scene.geometry(snap, o, d, &hit);
        let inst = &scene.insts[hit.inst];
        let mat = &snap.materials[inst.material];
        let wo = -d;
        let front = geom.ng.dot(wo) > 0.0;
        let ngv = if front { geom.ng } else { -geom.ng };
        let surf = Surface::at(&mat.graph, &*mat.sampler, &geom, wo, scene.scale);
        // Shading normal on the viewer side, never below the geometric surface.
        let nv = if front { surf.n } else { -surf.n };
        let n = if nv.dot(ngv) < 0.0 { ngv } else { nv };
        if depth == 0 {
            s.covered = true;
            s.depth = hit.t;
            s.normal = n;
            s.albedo = surf.albedo;
            s.position = geom.p;
            s.object_id = inst.object_id as f64 + 1.0;
            s.material_id = inst.material as f64 + 1.0;
        }
        if surf.emission != DVec3::ZERO {
            s.add(throughput * surf.emission, depth, target_of(cat), clamp);
        }
        let origin = geom.p + ngv * eps;

        // Next-event estimation.
        let nee = |ls: crate::lights::LightSample,
                   mode: (bool, bool, crate::lights::ShadowInfo),
                   s: &mut Sample,
                   rng: &mut Rng| {
            let _ = rng;
            let cos = n.dot(ls.wi);
            if cos <= 0.0 || ngv.dot(ls.wi) <= 0.0 {
                return;
            }
            let (mut fd, mut fs) = surf.eval(n, wo, ls.wi);
            if !mode.0 {
                fd = DVec3::ZERO;
            }
            if !mode.1 {
                fs = DVec3::ZERO;
            }
            if fd == DVec3::ZERO && fs == DVec3::ZERO {
                return;
            }
            let w = if ls.pdf > 0.0 {
                power_heuristic(ls.pdf, surf.pdf(n, wo, ls.wi))
            } else {
                1.0
            };
            let shadow = mode.2;
            let visible = !shadow.cast
                || !scene.occluded(
                    origin,
                    ls.wi,
                    if ls.dist.is_finite() {
                        (ls.dist * (1.0 - 1e-6) - eps).max(0.0)
                    } else {
                        FAR
                    },
                );
            let factor = if visible {
                DVec3::ONE
            } else {
                DVec3::splat(1.0 - shadow.density) + DVec3::from(shadow.color) * shadow.density
            };
            let un = (fd + fs) * cos * ls.weight;
            if depth == 0 {
                s.shadow_total += lum(un);
                s.shadow_visible += lum(un * factor);
            }
            let (cd, cs) = (
                fd * cos * ls.weight * w * factor,
                fs * cos * ls.weight * w * factor,
            );
            let (td, ts) = match cat {
                None => (Target::Diffuse, Target::Specular),
                c => (target_of(c), target_of(c)),
            };
            s.add(throughput * cd, depth, td, clamp);
            s.add(throughput * cs, depth, ts, clamp);
        };
        for l in cx.lights {
            if let Some(ls) = l.sample(geom.p, rng) {
                let info = crate::lights::ShadowInfo {
                    cast: l.casts_shadow(),
                    density: l.light.shadow_density,
                    color: l.light.shadow_color,
                };
                nee(ls, (l.light.diffuse, l.light.specular, info), &mut s, rng);
            }
        }
        if has_env(env) {
            let ls = sample_env(env, cx.env.as_ref(), rng);
            let info = crate::lights::ShadowInfo {
                cast: true,
                density: 1.0,
                color: [0.0; 3],
            };
            nee(ls, (true, true, info), &mut s, rng);
        }

        if depth + 1 >= cx.settings.max_depth {
            break;
        }
        // Scatter.
        let p_t = if surf.is_transmissive() {
            surf.p_trans()
        } else {
            0.0
        };
        if p_t > 0.0 && rng.f() < p_t {
            throughput *= surf.trans_weight();
            let (nn, entering) = (ngv, front);
            if geom.closed {
                let (n1, n2) = if entering {
                    (1.0, surf.ior)
                } else {
                    (surf.ior, 1.0)
                };
                let cos_i = nn.dot(wo).clamp(0.0, 1.0);
                let f = fresnel(cos_i, n2 / n1);
                let dir = if rng.f() < f {
                    None
                } else {
                    refract(wo, nn, n1 / n2)
                };
                match dir {
                    Some(t) => {
                        d = t.normalize();
                        o = geom.p - nn * eps;
                    }
                    None => {
                        d = (2.0 * nn.dot(wo) * nn - wo).normalize();
                        o = geom.p + nn * eps;
                    }
                }
            } else {
                // Thin sheet: straight through.
                o = geom.p - nn * eps;
            }
            prev_delta = true;
            cat.get_or_insert(Lobe::Specular);
        } else {
            let Some((wi, w, pdf, lobe)) = surf.sample_opaque(n, wo, rng) else {
                break;
            };
            if ngv.dot(wi) <= 0.0 {
                break;
            }
            throughput *= w * if p_t > 0.0 {
                surf.opaque_weight()
            } else {
                DVec3::ONE
            };
            d = wi;
            o = origin;
            prev_pdf = pdf;
            prev_delta = false;
            cat.get_or_insert(lobe);
        }
        if depth >= 3 {
            let q = throughput.max_element().clamp(0.05, 1.0);
            if rng.f() > q {
                break;
            }
            throughput /= q;
        }
        if !throughput.is_finite() {
            break;
        }
    }
    s
}
