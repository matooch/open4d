// R24.8: a single attempt from the opaque GGX lobe.
const REFLECTION_SAMPLES: u32 = 8u;

struct ReflectionSample {
    direction: vec3<f32>,
    weight: vec3<f32>,
    accepted: bool,
}

fn reflection_sample(m: Mat, n: vec3<f32>, ng: vec3<f32>, wo: vec3<f32>,
    rng: ptr<function, Rng>) -> ReflectionSample {
    var s: ReflectionSample;
    let nv = dot(n, wo);
    if all(m.spec == vec3<f32>(0.0)) || nv <= 0.0 {
        return s;
    }

    let u = rng_f2(rng);
    let a2 = m.alpha * m.alpha;
    let complement = 1.0 - u.y;
    let denom = complement + a2 * u.y;
    let sin_t = sqrt(a2 * u.y / denom);
    let cos_t = sqrt(complement / denom);
    let phi = 2.0 * PI_F * u.x;
    let h = to_world(n, vec3<f32>(sin_t * cos(phi), sin_t * sin(phi), cos_t));
    let vh = dot(wo, h);
    let wi = normalize(2.0 * vh * h - wo);
    let nl = dot(n, wi);
    if nl <= 0.0 || dot(ng, wi) <= 0.0 {
        return s;
    }

    // In the common hemisphere h is the reflection half-vector. Cancel D
    // analytically instead of dividing two very large narrow-lobe values.
    let nh = dot(n, h);
    if nh <= 0.0 {
        return s;
    }
    let g1_over_nv = 2.0 / (nv + sqrt(a2 + (1.0 - a2) * nv * nv));
    s.direction = wi;
    s.weight = m.spec * opaque_part(m) *
        (g1(m, nl) * g1_over_nv * max(vh, 1e-9) / (nh * f32(REFLECTION_SAMPLES)));
    s.accepted = true;
    return s;
}
