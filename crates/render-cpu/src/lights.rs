//! Prepared lights: sampling, shape intersection, environment (RENDER.md R8).

use std::f64::consts::PI;

use forge_render_api::{
    AreaShape, Falloff, Light, LightKind, Lighting, RenderError, ShadowMode, Snapshot,
};
use glam::{DMat4, DVec3};

use crate::sampling::{INV_4PI, Rng, uniform_ball, uniform_cone, uniform_sphere};

/// Shadow behaviour of one light at a shading point.
#[derive(Debug, Clone, Copy)]
pub struct ShadowInfo {
    pub cast: bool,
    pub density: f64,
    pub color: [f64; 3],
}

#[derive(Debug, Clone)]
pub struct PLight {
    pub light: Light,
    pub pos: DVec3,
    /// Direction the light shines along (local +Z in world).
    pub axis: DVec3,
    pub color: DVec3,
    pub tangent: DVec3,
    pub bitangent: DVec3,
}

/// One light sample toward a shading point.
#[derive(Debug, Clone, Copy)]
pub struct LightSample {
    pub wi: DVec3,
    /// Distance to the sampled point (infinite for parallel lights and the environment).
    pub dist: f64,
    /// Contribution weight: multiply with `f * cos` (already divided by the pdf).
    pub weight: DVec3,
    /// Solid-angle pdf for MIS (0 for delta lights).
    pub pdf: f64,
}

fn falloff(l: &Light, d: f64) -> f64 {
    let r = l.falloff_radius.max(1e-12);
    match l.falloff {
        Falloff::None => 1.0,
        Falloff::Linear => (1.0 - d / r).max(0.0),
        Falloff::InverseLinear => (r / d.max(1e-12)).min(1.0),
        Falloff::InverseSquare => {
            let q = r / d.max(1e-12);
            (q * q).min(1.0)
        }
        Falloff::Steps => 1.0,
    }
}

impl PLight {
    pub fn new(light: Light) -> Result<PLight, RenderError> {
        if light.falloff == Falloff::Steps {
            return Err(RenderError::Unsupported("light falloff Steps".into()));
        }
        if let LightKind::Area {
            shape: AreaShape::Object,
            ..
        } = light.kind
        {
            return Err(RenderError::Unsupported(
                "area light with an object shape".into(),
            ));
        }
        let m: DMat4 = light.matrix();
        let axis = m.transform_vector3(DVec3::Z).normalize_or_zero();
        Ok(PLight {
            pos: m.w_axis.truncate(),
            axis,
            color: DVec3::from(light.color) * light.intensity,
            tangent: m.transform_vector3(DVec3::X),
            bitangent: m.transform_vector3(DVec3::Y),
            light,
        })
    }

    pub fn is_area(&self) -> bool {
        matches!(self.light.kind, LightKind::Area { .. })
    }

    pub fn casts_shadow(&self) -> bool {
        self.light.shadow != ShadowMode::None
    }

    /// Samples the light from point `p`.
    pub fn sample(&self, p: DVec3, rng: &mut Rng) -> Option<LightSample> {
        let l = &self.light;
        match l.kind {
            LightKind::Omni | LightKind::Spot { .. } => {
                let r = if l.shadow == ShadowMode::Soft {
                    l.radius
                } else {
                    0.0
                };
                let q = if r > 0.0 {
                    self.pos + uniform_ball(rng) * r
                } else {
                    self.pos
                };
                let v = q - p;
                let dist = v.length();
                if dist <= 1e-12 {
                    return None;
                }
                let wi = v / dist;
                let mut w = falloff(l, dist);
                if let LightKind::Spot { inner, outer } = l.kind {
                    let cos_a = (-wi).dot(self.axis).clamp(-1.0, 1.0);
                    let a = cos_a.acos();
                    let (hi, ho) = (inner * 0.5, outer * 0.5);
                    w *= if a <= hi {
                        1.0
                    } else if a >= ho {
                        0.0
                    } else {
                        let t = (ho - a) / (ho - hi);
                        t * t * (3.0 - 2.0 * t)
                    };
                }
                (w > 0.0).then_some(LightSample {
                    wi,
                    dist,
                    weight: self.color * (PI * w),
                    pdf: 0.0,
                })
            }
            LightKind::Infinite { angle } => {
                let to_light = -self.axis;
                let wi = if angle > 0.0 && l.shadow == ShadowMode::Soft {
                    uniform_cone(to_light, angle * 0.5, rng.f2())
                } else {
                    to_light
                };
                Some(LightSample {
                    wi,
                    dist: f64::INFINITY,
                    weight: self.color * PI,
                    pdf: 0.0,
                })
            }
            LightKind::Area { shape, two_sided } => match shape {
                AreaShape::Rectangle { width, height } => {
                    let (u, v) = rng.f2();
                    let q = self.pos
                        + self.tangent * ((u - 0.5) * width)
                        + self.bitangent * ((v - 0.5) * height);
                    self.area_sample(
                        p,
                        q,
                        self.axis,
                        width * height * self.area_scale(),
                        two_sided,
                    )
                }
                AreaShape::Disc { radius } => {
                    let (u, v) = rng.f2();
                    let (r, a) = (radius * u.sqrt(), 2.0 * PI * v);
                    let q = self.pos
                        + self.tangent.normalize() * (r * a.cos())
                        + self.bitangent.normalize() * (r * a.sin());
                    self.area_sample(p, q, self.axis, PI * radius * radius, two_sided)
                }
                AreaShape::Sphere { radius } => {
                    let c = self.pos;
                    let v = c - p;
                    let d = v.length();
                    if d <= radius * 1.0001 {
                        return None;
                    }
                    let half = (radius / d).asin();
                    let axis = v / d;
                    let wi = uniform_cone(axis, half, rng.f2());
                    // Nearest intersection with the sphere along wi.
                    let b = wi.dot(v);
                    let disc = b * b - (v.length_squared() - radius * radius);
                    if disc < 0.0 {
                        return None;
                    }
                    let t = b - disc.sqrt();
                    let omega = 2.0 * PI * (1.0 - half.cos());
                    let w = falloff(l, t);
                    Some(LightSample {
                        wi,
                        dist: t,
                        weight: self.color * (w * omega),
                        pdf: 1.0 / omega,
                    })
                }
                AreaShape::Object => None,
            },
        }
    }

    fn area_scale(&self) -> f64 {
        // The transform's scale is part of the shape: a scaled rectangle light is larger.
        self.tangent.length() * self.bitangent.length()
    }

    fn area_sample(
        &self,
        p: DVec3,
        q: DVec3,
        n: DVec3,
        area: f64,
        two_sided: bool,
    ) -> Option<LightSample> {
        let v = q - p;
        let d2 = v.length_squared();
        if d2 <= 1e-18 {
            return None;
        }
        let dist = d2.sqrt();
        let wi = v / dist;
        let mut cos_l = (-wi).dot(n);
        if two_sided {
            cos_l = cos_l.abs();
        }
        if cos_l <= 0.0 {
            return None;
        }
        let pdf = d2 / (cos_l * area);
        Some(LightSample {
            wi,
            dist,
            weight: self.color * (falloff(&self.light, dist) / pdf),
            pdf,
        })
    }

    /// Intersection of a ray with the emitting surface: `(t, emitted radiance, pdf)`.
    pub fn hit(&self, o: DVec3, d: DVec3, tmax: f64) -> Option<(f64, DVec3, f64)> {
        let LightKind::Area { shape, two_sided } = self.light.kind else {
            return None;
        };
        match shape {
            AreaShape::Rectangle { width, height } => {
                let denom = d.dot(self.axis);
                if denom.abs() < 1e-12 {
                    return None;
                }
                let t = (self.pos - o).dot(self.axis) / denom;
                if t <= 1e-9 || t >= tmax {
                    return None;
                }
                let q = o + d * t - self.pos;
                let (tl, bl) = (
                    self.tangent.length_squared(),
                    self.bitangent.length_squared(),
                );
                let (u, v) = (q.dot(self.tangent) / tl, q.dot(self.bitangent) / bl);
                if u.abs() > width * 0.5 || v.abs() > height * 0.5 {
                    return None;
                }
                self.finish_area_hit(
                    o,
                    d,
                    t,
                    self.axis,
                    width * height * self.area_scale(),
                    two_sided,
                )
            }
            AreaShape::Disc { radius } => {
                let denom = d.dot(self.axis);
                if denom.abs() < 1e-12 {
                    return None;
                }
                let t = (self.pos - o).dot(self.axis) / denom;
                if t <= 1e-9 || t >= tmax {
                    return None;
                }
                if (o + d * t - self.pos).length_squared() > radius * radius {
                    return None;
                }
                self.finish_area_hit(o, d, t, self.axis, PI * radius * radius, two_sided)
            }
            AreaShape::Sphere { radius } => {
                let v = self.pos - o;
                let b = d.dot(v);
                let disc = b * b - (v.length_squared() - radius * radius);
                if disc < 0.0 {
                    return None;
                }
                let t = b - disc.sqrt();
                if t <= 1e-9 || t >= tmax {
                    return None;
                }
                let dist = v.length();
                let pdf = if dist > radius {
                    1.0 / (2.0 * PI * (1.0 - (radius / dist).asin().cos()))
                } else {
                    0.0
                };
                Some((t, self.color * falloff(&self.light, t), pdf))
            }
            AreaShape::Object => None,
        }
    }

    fn finish_area_hit(
        &self,
        _o: DVec3,
        d: DVec3,
        t: f64,
        n: DVec3,
        area: f64,
        two_sided: bool,
    ) -> Option<(f64, DVec3, f64)> {
        let mut cos_l = (-d).dot(n);
        if two_sided {
            cos_l = cos_l.abs();
        }
        if cos_l <= 0.0 {
            return None;
        }
        Some((
            t,
            self.color * falloff(&self.light, t),
            t * t / (cos_l * area),
        ))
    }
}

/// Environment radiance in direction `d` (the direction a ray travels toward the sky).
pub fn env_radiance(l: &Lighting, d: DVec3) -> DVec3 {
    match l {
        Lighting::None => DVec3::ZERO,
        Lighting::Constant { color, intensity } => DVec3::from(*color) * *intensity,
        Lighting::Sky {
            zenith,
            horizon,
            ground,
            intensity,
        } => {
            let y = d.y.clamp(-1.0, 1.0);
            let (z, h, g) = (
                DVec3::from(*zenith),
                DVec3::from(*horizon),
                DVec3::from(*ground),
            );
            let s = |t: f64| t * t * (3.0 - 2.0 * t);
            let c = if y >= 0.0 {
                h.lerp(z, s(y))
            } else {
                h.lerp(g, s(-y))
            };
            c * *intensity
        }
        Lighting::Image {
            image,
            rotation,
            intensity,
        } => {
            let phi = d.x.atan2(d.z) - rotation;
            let u = (phi / (2.0 * PI)).rem_euclid(1.0);
            let v = d.y.clamp(-1.0, 1.0).acos() / PI;
            let x = ((u * image.width as f64) as u32).min(image.width - 1);
            let y = ((v * image.height as f64) as u32).min(image.height - 1);
            let p = image.pixel(x, y);
            DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64) * *intensity
        }
    }
}

pub fn has_env(l: &Lighting) -> bool {
    !matches!(l, Lighting::None)
}

pub fn sample_env(l: &Lighting, rng: &mut Rng) -> LightSample {
    let wi = uniform_sphere(rng.f2());
    LightSample {
        wi,
        dist: f64::INFINITY,
        // Radiance / pdf; the caller multiplies by f * cos.
        weight: env_radiance(l, wi) / INV_4PI,
        pdf: INV_4PI,
    }
}

pub fn prepare(snap: &Snapshot, default_light: bool) -> Result<Vec<PLight>, RenderError> {
    let mut v = snap
        .lights
        .iter()
        .map(|l| PLight::new(*l))
        .collect::<Result<Vec<_>, _>>()?;
    if v.is_empty() && default_light && !has_env(&snap.environment.lighting) {
        let l = Light::new(LightKind::Omni, snap.camera.matrix());
        v.push(PLight::new(l)?);
    }
    Ok(v)
}
