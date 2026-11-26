// Packed light and emitter sampling shared by GPU transport (R20) and hybrid shadows (R24.7).
// Depends only on the shared RNG and geometric sampling helpers.

struct GpuLight {
    pos_radius: vec4<f32>,
    axis_kind: vec4<f32>,
    color: vec4<f32>,
    tangent_shape: vec4<f32>,
    bitangent: vec4<f32>,
    params: vec4<f32>,
    flags: vec4<f32>,
    shadow_color: vec4<f32>,
    spot: vec4<f32>,
    shape: vec4<f32>,
};

struct LightSample {
    ok: bool,
    wi: vec3<f32>,
    dist: f32,
    weight: vec3<f32>,
    pdf: f32,
};

fn no_light_sample() -> LightSample {
    var s: LightSample;
    s.ok = false;
    s.wi = vec3<f32>(0.0);
    s.dist = 0.0;
    s.weight = vec3<f32>(0.0);
    s.pdf = 0.0;
    return s;
}

fn light_falloff(l: GpuLight, d: f32) -> f32 {
    let r = max(l.params.y, 1e-12);
    let k = u32(l.params.x + 0.5);
    if (k == 0u) {
        return 1.0;
    }
    if (k == 1u) {
        return max(1.0 - d / r, 0.0);
    }
    if (k == 2u) {
        return min(r / max(d, 1e-12), 1.0);
    }
    let q = r / max(d, 1e-12);
    return min(q * q, 1.0);
}

fn area_sample(
    l: GpuLight,
    p: vec3<f32>,
    q: vec3<f32>,
    n: vec3<f32>,
    area: f32,
    two_sided: bool,
) -> LightSample {
    var s = no_light_sample();
    let v = q - p;
    let d2 = dot(v, v);
    if (d2 <= 1e-18) {
        return s;
    }
    let dist = sqrt(d2);
    let wi = v / dist;
    var cos_l = dot(-wi, n);
    if (two_sided) {
        cos_l = abs(cos_l);
    }
    if (cos_l <= 0.0) {
        return s;
    }
    let pdf = d2 / (cos_l * area);
    s.ok = true;
    s.wi = wi;
    s.dist = dist;
    s.weight = l.color.xyz * (light_falloff(l, dist) / pdf);
    s.pdf = pdf;
    return s;
}

fn light_sample(l: GpuLight, p: vec3<f32>, rng: ptr<function, Rng>) -> LightSample {
    var s = no_light_sample();
    let kind = u32(l.axis_kind.w + 0.5);
    let soft = l.flags.x > 0.5;
    let two_sided = l.bitangent.w > 0.5;
    if (kind == 0u || kind == 1u) {
        var q = l.pos_radius.xyz;
        if (soft && l.pos_radius.w > 0.0) {
            q = q + uniform_ball(rng) * l.pos_radius.w;
        }
        let v = q - p;
        let dist = length(v);
        if (dist <= 1e-12) {
            return s;
        }
        let wi = v / dist;
        var w = light_falloff(l, dist);
        if (kind == 1u) {
            let cos_a = clamp(dot(-wi, l.axis_kind.xyz), -1.0, 1.0);
            let a = acos(cos_a);
            let hi = l.spot.x;
            let ho = l.spot.y;
            if (a >= ho) {
                w = 0.0;
            } else if (a > hi) {
                let t = (ho - a) / (ho - hi);
                w = w * t * t * (3.0 - 2.0 * t);
            }
        }
        if (!(w > 0.0)) {
            return s;
        }
        s.ok = true;
        s.wi = wi;
        s.dist = dist;
        s.weight = l.color.xyz * (PI_F * w);
        s.pdf = 0.0;
        return s;
    }
    if (kind == 2u) {
        var wi = -l.axis_kind.xyz;
        if (l.spot.x > 0.0 && soft) {
            wi = uniform_cone(-l.axis_kind.xyz, l.spot.x * 0.5, rng_f2(rng));
        }
        s.ok = true;
        s.wi = wi;
        s.dist = 1e30;
        s.weight = l.color.xyz * PI_F;
        s.pdf = 0.0;
        return s;
    }
    let shape_kind = u32(l.tangent_shape.w + 0.5);
    if (shape_kind == 0u) {
        let uv = rng_f2(rng);
        let q = l.pos_radius.xyz
            + l.tangent_shape.xyz * ((uv.x - 0.5) * l.shape.x)
            + l.bitangent.xyz * ((uv.y - 0.5) * l.shape.y);
        return area_sample(l, p, q, l.axis_kind.xyz, l.shape.x * l.shape.y * l.shape.w, two_sided);
    }
    if (shape_kind == 1u) {
        let uv = rng_f2(rng);
        let r = l.shape.z * sqrt(uv.x);
        let a = TAU_F * uv.y;
        let q = l.pos_radius.xyz
            + normalize(l.tangent_shape.xyz) * (r * cos(a))
            + normalize(l.bitangent.xyz) * (r * sin(a));
        return area_sample(l, p, q, l.axis_kind.xyz, PI_F * l.shape.z * l.shape.z, two_sided);
    }
    let v = l.pos_radius.xyz - p;
    let d = length(v);
    if (d <= l.shape.z * 1.0001) {
        return s;
    }
    let half = asin(l.shape.z / d);
    let wi = uniform_cone(v / d, half, rng_f2(rng));
    let b = dot(wi, v);
    let disc = b * b - (dot(v, v) - l.shape.z * l.shape.z);
    if (disc < 0.0) {
        return s;
    }
    let t = b - sqrt(disc);
    let omega = 2.0 * PI_F * (1.0 - cos(half));
    let w = light_falloff(l, t);
    s.ok = true;
    s.wi = wi;
    s.dist = t;
    s.weight = l.color.xyz * (w * omega);
    s.pdf = 1.0 / omega;
    return s;
}
