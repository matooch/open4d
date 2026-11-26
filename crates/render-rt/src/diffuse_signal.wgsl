// R24.9 lobe and black misses for diffuse single-bounce GI.
const BOUNCE_SAMPLES: u32 = 8u;

struct DiffuseSample {
    direction: vec3<f32>,
    weight: vec3<f32>,
    accepted: bool,
}

fn bounce_sample(m: Mat, n: vec3<f32>, ng: vec3<f32>, wo: vec3<f32>,
    rng: ptr<function, Rng>) -> DiffuseSample {
    var s: DiffuseSample;
    let weight = m.albedo * opaque_part(m);
    if dot(n, wo) <= 0.0 || all(weight == vec3<f32>(0.0)) {
        return s;
    }
    let wi = to_world(n, cosine_hemisphere(rng_f2(rng)));
    if dot(ng, wi) <= 0.0 {
        return s;
    }
    return DiffuseSample(wi, weight / f32(BOUNCE_SAMPLES), true);
}

fn bounce_miss(d: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(0.0);
}

@compute @workgroup_size(8, 8)
fn diffuse_signal(@builtin(global_invocation_id) id: vec3<u32>) {
    bounce_signal(id.xy);
}
