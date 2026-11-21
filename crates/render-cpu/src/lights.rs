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

/// Importance sampling of a lat-long environment image (RENDER.md R16): a piecewise constant
/// density over the pixels proportional to `luminance * sin(theta)`, sampled through the marginal
/// distribution of the rows and the conditional distribution inside a row.
#[derive(Debug, Clone)]
pub struct EnvDist {
    width: usize,
    height: usize,
    rotation: f64,
    /// Cumulative row weights, `height + 1` entries from 0 to 1.
    marginal: Vec<f64>,
    /// Cumulative pixel weights of each row, `height * (width + 1)` entries, each row 0 to 1.
    conditional: Vec<f64>,
    /// Probability of each pixel (`weight / total`).
    prob: Vec<f64>,
}

impl EnvDist {
    /// `None` unless the lighting is an image with some light in it.
    pub fn build(l: &Lighting) -> Option<EnvDist> {
        let Lighting::Image {
            image, rotation, ..
        } = l
        else {
            return None;
        };
        let (w, h) = (image.width as usize, image.height as usize);
        let mut weight = vec![0.0f64; w * h];
        for y in 0..h {
            let sin_t = (PI * (y as f64 + 0.5) / h as f64).sin();
            for x in 0..w {
                let p = image.pixel(x as u32, y as u32);
                let lum = 0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64;
                weight[y * w + x] = if lum.is_finite() {
                    lum.max(0.0) * sin_t
                } else {
                    0.0
                };
            }
        }
        let total: f64 = weight.iter().sum();
        if total.is_nan() || total <= 0.0 {
            return None;
        }
        let mut marginal = vec![0.0; h + 1];
        let mut conditional = vec![0.0; h * (w + 1)];
        for y in 0..h {
            let row: f64 = weight[y * w..(y + 1) * w].iter().sum();
            marginal[y + 1] = marginal[y] + row / total;
            let mut acc = 0.0;
            for x in 0..w {
                acc += if row > 0.0 {
                    weight[y * w + x] / row
                } else {
                    0.0
                };
                conditional[y * (w + 1) + x + 1] = acc;
            }
            // exact end points against rounding
            conditional[y * (w + 1) + w] = 1.0;
        }
        marginal[h] = 1.0;
        Some(EnvDist {
            width: w,
            height: h,
            rotation: *rotation,
            marginal,
            conditional,
            prob: weight.iter().map(|x| x / total).collect(),
        })
    }

    fn find(cdf: &[f64], u: f64) -> usize {
        // largest i with cdf[i] <= u, within 0..cdf.len() - 1
        let i = cdf.partition_point(|c| *c <= u).saturating_sub(1);
        i.min(cdf.len() - 2)
    }

    /// A direction and its solid angle density for the uniform numbers `u`.
    pub fn sample(&self, u: (f64, f64)) -> (DVec3, f64) {
        let y = Self::find(&self.marginal, u.0);
        let row = &self.conditional[y * (self.width + 1)..(y + 1) * (self.width + 1)];
        let x = Self::find(row, u.1);
        // jitter inside the pixel with the remainders of the uniform numbers
        let (fy, fx) = (
            ((u.0 - self.marginal[y]) / (self.marginal[y + 1] - self.marginal[y]).max(1e-300))
                .clamp(0.0, 1.0),
            ((u.1 - row[x]) / (row[x + 1] - row[x]).max(1e-300)).clamp(0.0, 1.0),
        );
        let (uu, vv) = (
            (x as f64 + fx) / self.width as f64,
            (y as f64 + fy) / self.height as f64,
        );
        let theta = PI * vv;
        let az = uu * 2.0 * PI + self.rotation;
        let st = theta.sin();
        let d = DVec3::new(st * az.sin(), theta.cos(), st * az.cos());
        (d, self.pdf(d))
    }

    /// Solid angle density of a direction.
    pub fn pdf(&self, d: DVec3) -> f64 {
        let phi = d.x.atan2(d.z) - self.rotation;
        let u = (phi / (2.0 * PI)).rem_euclid(1.0);
        let y = d.y.clamp(-1.0, 1.0);
        let v = y.acos() / PI;
        let x = ((u * self.width as f64) as usize).min(self.width - 1);
        let r = ((v * self.height as f64) as usize).min(self.height - 1);
        let sin_t = (1.0 - y * y).sqrt();
        if sin_t < 1e-9 {
            return 0.0;
        }
        self.prob[r * self.width + x] * (self.width * self.height) as f64 / (2.0 * PI * PI * sin_t)
    }
}

/// Solid angle density with which [`sample_env`] picks `d`.
pub fn env_pdf(dist: Option<&EnvDist>, d: DVec3) -> f64 {
    dist.map_or(INV_4PI, |e| e.pdf(d))
}

pub fn sample_env(l: &Lighting, dist: Option<&EnvDist>, rng: &mut Rng) -> LightSample {
    if let Some(e) = dist {
        let (wi, pdf) = e.sample(rng.f2());
        if pdf > 0.0 {
            return LightSample {
                wi,
                dist: f64::INFINITY,
                weight: env_radiance(l, wi) / pdf,
                pdf,
            };
        }
    }
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
