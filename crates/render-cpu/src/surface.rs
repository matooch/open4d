//! Surfaces from the shading IR: Lambert, GGX highlight, dielectric (RENDER.md R7).

use std::f64::consts::PI;

use forge_material::{DiffuseModel, ShadeCtx, ShadingGraph, TextureSampler};
use glam::DVec3;

use crate::sampling::{Rng, cosine_hemisphere, lum, to_world};
use crate::scene::HitGeom;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lobe {
    Diffuse,
    Specular,
}

/// What the material is at one point.
#[derive(Debug, Clone)]
pub struct Surface {
    pub albedo: DVec3,
    pub spec: DVec3,
    pub alpha: f64,
    pub emission: DVec3,
    /// Transmission colour (zero = opaque).
    pub trans: DVec3,
    pub ior: f64,
    /// Shading normal (after bump), on the outward side.
    pub n: DVec3,
}

fn v3(c: [f64; 3]) -> DVec3 {
    DVec3::from(c)
}

/// Mean of the texture at a context (the bump height).
fn height(g: &ShadingGraph, s: &dyn TextureSampler, ctx: &ShadeCtx, ix: usize) -> f64 {
    let c = g.eval(ix, s, ctx);
    (c[0] + c[1] + c[2]) / 3.0
}

/// Barycentric interpolation of the corner UVWs at a world point of the triangle plane.
fn uvw_at(geom: &HitGeom, p: DVec3) -> DVec3 {
    let [a, b, c] = geom.corners;
    let (e1, e2) = (b - a, c - a);
    let w = p - a;
    let (d11, d12, d22) = (e1.dot(e1), e1.dot(e2), e2.dot(e2));
    let (w1, w2) = (w.dot(e1), w.dot(e2));
    let det = d11 * d22 - d12 * d12;
    if det.abs() < 1e-30 {
        return geom.uvw;
    }
    let u = (w1 * d22 - w2 * d12) / det;
    let v = (w2 * d11 - w1 * d12) / det;
    geom.corner_uvw[0] * (1.0 - u - v) + geom.corner_uvw[1] * u + geom.corner_uvw[2] * v
}

impl Surface {
    pub fn at(
        g: &ShadingGraph,
        sampler: &dyn TextureSampler,
        geom: &HitGeom,
        wo: DVec3,
        scale: f64,
    ) -> Surface {
        let ctx = ShadeCtx {
            uvw: geom.uvw.to_array(),
            position: geom.p.to_array(),
            normal: geom.ns.to_array(),
            view: wo.to_array(),
            ..ShadeCtx::default()
        };
        let o = &g.outputs;
        let mut albedo = DVec3::ZERO;
        if let Some(d) = &o.diffuse {
            let _ = d.model == DiffuseModel::Lambertian;
            albedo = v3(g.eval(d.color, sampler, &ctx)) * d.level;
        }
        let mut spec = DVec3::ZERO;
        let mut alpha = 0.02f64.powi(2);
        if let Some(s) = &o.specular {
            spec = v3(g.eval(s.color, sampler, &ctx)) * s.strength;
            let width = o
                .roughness_input
                .map_or(s.width, |i| g.eval(i, sampler, &ctx)[0]);
            alpha = width.clamp(0.02, 1.0).powi(2);
            let metal = o
                .metallic_input
                .map_or(s.metal, |i| g.eval(i, sampler, &ctx)[0] > 0.5);
            if metal {
                albedo *= (1.0 - s.strength).max(0.0);
            }
        }
        let emission = o
            .emission
            .map_or(DVec3::ZERO, |e| v3(g.eval(e, sampler, &ctx)));
        let (mut trans, mut ior) = (DVec3::ZERO, 1.0);
        if let Some(t) = &o.transmission {
            trans = v3(g.eval(t.color, sampler, &ctx)).clamp(DVec3::ZERO, DVec3::ONE);
            ior = t.ior.max(1e-3);
        }
        let mut n = geom.ns;
        if let Some(b) = &o.bump {
            let eps = 1e-3 * scale;
            let (t1, t2) = crate::sampling::basis(geom.ng);
            let h0 = height(g, sampler, &ctx, b.texture);
            let sample = |dp: DVec3| {
                let c = ShadeCtx {
                    position: (geom.p + dp).to_array(),
                    uvw: uvw_at(geom, geom.p + dp).to_array(),
                    ..ctx
                };
                height(g, sampler, &c, b.texture)
            };
            let d1 = (sample(t1 * eps) - h0) / eps;
            let d2 = (sample(t2 * eps) - h0) / eps;
            n = (n - (t1 * d1 + t2 * d2) * (b.strength * scale)).normalize_or_zero();
            if n == DVec3::ZERO {
                n = geom.ns;
            }
        }
        if let Some(map) = &o.normal
            && map.method == "tangent"
        {
            let [a, b, c] = geom.corners;
            let [u, v, w] = geom.corner_uvw;
            let (uv1, uv2) = (v - u, w - u);
            let det = uv1.x * uv2.y - uv1.y * uv2.x;
            if det.abs() > 1e-12 {
                let du = ((b - a) * uv2.y - (c - a) * uv1.y) / det;
                let dv = ((c - a) * uv1.x - (b - a) * uv2.x) / det;
                if let Some(t) = (du - n * n.dot(du)).try_normalize() {
                    let b = n.cross(t) * n.cross(t).dot(dv).signum();
                    let mut local = v3(g.eval(map.texture, sampler, &ctx)) * 2.0 - DVec3::ONE;
                    for (i, flip) in map.flip.iter().enumerate() {
                        if *flip {
                            local[i] = -local[i];
                        }
                    }
                    if map.swap_yz {
                        local = DVec3::new(local.x, local.z, local.y);
                    }
                    n = n
                        .lerp(t * local.x + b * local.y + n * local.z, map.strength)
                        .try_normalize()
                        .unwrap_or(n);
                }
            }
        }
        Surface {
            albedo,
            spec,
            alpha,
            emission,
            trans,
            ior,
            n,
        }
    }

    pub fn is_transmissive(&self) -> bool {
        self.trans.max_element() > 0.0
    }

    /// Opaque part weights `(1 - T)`.
    fn opaque(&self) -> DVec3 {
        DVec3::ONE - self.trans
    }

    pub fn lobe_probability(&self) -> f64 {
        let (ld, ls) = (self.albedo.element_sum(), self.spec.element_sum());
        if ld + ls <= 0.0 { 0.0 } else { ls / (ld + ls) }
    }

    fn ggx_d(&self, nh: f64) -> f64 {
        let a2 = self.alpha * self.alpha;
        let d = nh * nh * (a2 - 1.0) + 1.0;
        a2 / (PI * d * d)
    }

    fn g1(&self, nv: f64) -> f64 {
        let a2 = self.alpha * self.alpha;
        2.0 * nv / (nv + (a2 + (1.0 - a2) * nv * nv).sqrt())
    }

    /// Diffuse and specular BSDF values (without the cosine) for `wo`, `wi` on the side of `n`.
    pub fn eval(&self, n: DVec3, wo: DVec3, wi: DVec3) -> (DVec3, DVec3) {
        let (nl, nv) = (n.dot(wi), n.dot(wo));
        if nl <= 0.0 || nv <= 0.0 {
            return (DVec3::ZERO, DVec3::ZERO);
        }
        let op = self.opaque();
        let fd = self.albedo * op / PI;
        let h = (wo + wi).normalize_or_zero();
        let nh = n.dot(h).max(0.0);
        let fs = if self.spec == DVec3::ZERO {
            DVec3::ZERO
        } else {
            self.spec * op * (self.ggx_d(nh) * self.g1(nl) * self.g1(nv) / (4.0 * nl * nv))
        };
        (fd, fs)
    }

    /// Solid-angle pdf of `sample_opaque` for `wi`.
    pub fn pdf(&self, n: DVec3, wo: DVec3, wi: DVec3) -> f64 {
        let (nl, nv) = (n.dot(wi), n.dot(wo));
        if nl <= 0.0 || nv <= 0.0 {
            return 0.0;
        }
        let ps = self.lobe_probability();
        (1.0 - ps) * nl / PI + ps * self.specular_pdf(n, wo, wi)
    }

    fn specular_pdf(&self, n: DVec3, wo: DVec3, wi: DVec3) -> f64 {
        let h = (wo + wi).normalize_or_zero();
        let nh = n.dot(h).max(0.0);
        let vh = wo.dot(h).max(1e-9);
        self.ggx_d(nh) * nh / (4.0 * vh)
    }

    fn reflect_ggx(&self, n: DVec3, wo: DVec3, (u1, u2): (f64, f64)) -> DVec3 {
        let a2 = self.alpha * self.alpha;
        let cos_t = ((1.0 - u2) / (1.0 + (a2 - 1.0) * u2)).sqrt();
        let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
        let phi = 2.0 * PI * u1;
        let h = to_world(n, DVec3::new(sin_t * phi.cos(), sin_t * phi.sin(), cos_t));
        (2.0 * wo.dot(h) * h - wo).normalize_or_zero()
    }

    /// Sample just the opaque specular lobe: direction and `f_s cos / pdf_s`.
    pub fn sample_specular(&self, n: DVec3, wo: DVec3, rng: &mut Rng) -> Option<(DVec3, DVec3)> {
        if self.spec == DVec3::ZERO || n.dot(wo) <= 0.0 {
            return None;
        }
        let wi = self.reflect_ggx(n, wo, rng.f2());
        let nl = n.dot(wi);
        if nl <= 0.0 {
            return None;
        }
        let pdf = self.specular_pdf(n, wo, wi);
        let fs = self.eval(n, wo, wi).1;
        (pdf > 0.0).then(|| (wi, fs * (nl / pdf)))
    }

    /// Samples the opaque part: direction, weight `f cos / pdf` per channel, pdf and lobe.
    pub fn sample_opaque(
        &self,
        n: DVec3,
        wo: DVec3,
        rng: &mut Rng,
    ) -> Option<(DVec3, DVec3, f64, Lobe)> {
        let ps = self.lobe_probability();
        let lobe = if rng.f() < ps {
            Lobe::Specular
        } else {
            Lobe::Diffuse
        };
        let wi = match lobe {
            Lobe::Diffuse => to_world(n, cosine_hemisphere(rng.f2())),
            Lobe::Specular => self.reflect_ggx(n, wo, rng.f2()),
        };
        let nl = n.dot(wi);
        if nl <= 0.0 || wi == DVec3::ZERO {
            return None;
        }
        let pdf = self.pdf(n, wo, wi);
        if pdf <= 0.0 {
            return None;
        }
        let (fd, fs) = self.eval(n, wo, wi);
        Some((wi, (fd + fs) * nl / pdf, pdf, lobe))
    }

    /// Probability of taking the dielectric branch.
    pub fn p_trans(&self) -> f64 {
        lum(self.trans).clamp(0.0, 0.95)
    }

    /// Weight of the dielectric branch per channel, `T / p`.
    pub fn trans_weight(&self) -> DVec3 {
        self.trans / self.p_trans().max(1e-9)
    }

    /// Weight of the opaque branch per channel, `(1 - T) / (1 - p)`.
    pub fn opaque_weight(&self) -> DVec3 {
        self.opaque() / (1.0 - self.p_trans())
    }
}

/// Unpolarised Fresnel reflectance for a dielectric.
pub fn fresnel(cos_i: f64, eta: f64) -> f64 {
    // eta = n_t / n_i
    let sin2_t = (1.0 - cos_i * cos_i) / (eta * eta);
    if sin2_t >= 1.0 {
        return 1.0;
    }
    let cos_t = (1.0 - sin2_t).sqrt();
    let rs = (cos_i - eta * cos_t) / (cos_i + eta * cos_t);
    let rp = (eta * cos_i - cos_t) / (eta * cos_i + cos_t);
    0.5 * (rs * rs + rp * rp)
}

/// Refracts `wo` (pointing away from the surface) about `n` (on the `wo` side);
/// `eta = n_i / n_t` of the side of `wo` over the other side. `None` on total internal reflection.
pub fn refract(wo: DVec3, n: DVec3, eta: f64) -> Option<DVec3> {
    let c = n.dot(wo);
    let k = 1.0 - eta * eta * (1.0 - c * c);
    if k < 0.0 {
        return None;
    }
    Some(-wo * eta + n * (eta * c - k.sqrt()))
}
