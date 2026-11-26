// Shared R24.8/R24.9 single-bounce transport. Only reflection uses lighting.
struct BounceFrame {
    camera: mat3x3<f32>,
    film: vec3<f32>,
    scale: f32,
    jitter: vec2<f32>,
    seed: vec2<u32>,
    history_frame: u32,
    instance_count: u32,
    epsilon: f32,
    lighting_kind: u32,
    // Constant color or sky horizon in xyz; intensity in w.
    lighting: vec4<f32>,
    // Sky zenith in xyz; lat-long rotation in radians in w.
    zenith_rotation: vec4<f32>,
    ground: vec4<f32>,
    light_count: u32,
}

@group(0) @binding(0) var<uniform> bounce_frame: BounceFrame;
@group(0) @binding(1) var bounce_positions: texture_2d<f32>;
@group(0) @binding(2) var bounce_normals: texture_2d<f32>;
@group(0) @binding(4) var<storage, read_write> bounce_output: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> bounce_lights: array<GpuLight>;
@group(2) @binding(4) var<storage, read> path_attributes: array<vec4<f32>>;

struct BounceHit {
    valid: bool,
    p: vec3<f32>,
    ng: vec3<f32>,
    n: vec3<f32>,
    wo: vec3<f32>,
    m: Mat,
    weight: vec3<f32>,
}

fn bounce_hit(hit: RtHit, origin: vec3<f32>, direction: vec3<f32>, weight: vec3<f32>) -> BounceHit {
    let g = instance_geometry(origin + direction * hit.tuv.x, hit.ids.xy, hit.tuv.yz);
    let material = u32(path_attributes[7u * hit.ids.x].y);
    let wo = -direction;
    let m = surface_at(material, g, wo, bounce_frame.scale);
    let face = select(-1.0, 1.0, dot(g.ng, wo) > 0.0);
    let ng = g.ng * face;
    let ns = m.n * face;
    return BounceHit(true, g.p, ng, select(ns, ng, dot(ns, ng) < 0.0), wo, m, weight);
}

fn bounce_direct(h: BounceHit, l: GpuLight, seed: vec2<u32>, stream: vec2<u32>) -> vec3<f32> {
    var rng = rng_new(seed, stream);
    let s = light_sample(l, h.p, &rng);
    let cos_i = dot(h.n, s.wi);
    if !s.ok || cos_i <= 0.0 || dot(h.ng, s.wi) <= 0.0 {
        return vec3<f32>(0.0);
    }
    let f = mat_eval(h.m, h.n, h.wo, s.wi);
    let d = select(vec3<f32>(0.0), f.fd, l.flags.z > 0.5);
    let r = select(vec3<f32>(0.0), f.fs, l.flags.w > 0.5);
    let value = (d + r) * (cos_i * s.weight) * h.weight;
    if l.params.z > 0.5 && any(value != vec3<f32>(0.0)) {
        let epsilon = bounce_frame.epsilon;
        let t_max = max(s.dist * (1.0 - 1e-6) - epsilon, 0.0);
        if rt_closest(h.p + h.ng * epsilon, s.wi, t_max).ids.x != RT_MISS {
            let density = l.params.w;
            return value * (vec3<f32>(1.0 - density) + l.shadow_color.xyz * density);
        }
    }
    return value;
}

fn bounce_estimate(pixel: vec2<u32>, index: u32, p: vec3<f32>, ids: vec2<u32>) -> vec3<f32> {
    let g = receiver_geometry(p, ids);
    let wo = receiver_view(vec2<f32>(pixel), bounce_frame.jitter,
        bounce_frame.film, bounce_frame.camera);
    let material = u32(path_attributes[7u * ids.x].y);
    let m = surface_at(material, g, wo, bounce_frame.scale);
    let face = select(-1.0, 1.0, dot(g.ng, wo) > 0.0);
    let ng = g.ng * face;
    let ns = m.n * face;
    let n = select(ns, ng, dot(ns, ng) < 0.0);
    let origin = g.p + ng * bounce_frame.epsilon;
    let seed = bounce_frame.seed ^ vec2<u32>(bounce_frame.history_frame, 0u);
    var rng = rng_new(seed, vec2<u32>(index, 0u));
    var secondary: array<BounceHit, BOUNCE_SAMPLES>;
    var rgb = vec3<f32>(0.0);
    for (var attempt = 0u; attempt < BOUNCE_SAMPLES; attempt += 1u) {
        let s = bounce_sample(m, n, ng, wo, &rng);
        if s.accepted {
            let hit = rt_closest(origin, s.direction, 1e30);
            if hit.ids.x == RT_MISS {
                rgb += s.weight * bounce_miss(s.direction);
            } else {
                let h = bounce_hit(hit, origin, s.direction, s.weight);
                rgb += s.weight * h.m.emission;
                secondary[attempt] = h;
            }
        }
    }
    // Keep the absolute attempt IDs independent of the per-receiver budget.
    let stream_base = u64_mul(vec2<u32>(index, 0u), vec2<u32>(8u, 0u));
    let direct_seed = seed ^ vec2<u32>(0x79fa645bu, 0xed20e2d0u);
    for (var light = 0u; light < bounce_frame.light_count; light += 1u) {
        let l = bounce_lights[light];
        let light_seed = direct_seed ^ vec2<u32>(light, 0u);
        for (var attempt = 0u; attempt < BOUNCE_SAMPLES; attempt += 1u) {
            let h = secondary[attempt];
            if h.valid {
                let stream = u64_add(stream_base, vec2<u32>(attempt, 0u));
                rgb += bounce_direct(h, l, light_seed, stream);
            }
        }
    }
    return rgb;
}

fn bounce_signal(pixel: vec2<u32>) {
    let size = textureDimensions(bounce_positions);
    if any(pixel >= size) {
        return;
    }
    let index = pixel.y * size.x + pixel.x;
    let p = textureLoad(bounce_positions, pixel, 0);
    var rgb = vec3<f32>(0.0);
    if p.w >= 0.5 {
        let key = u32(textureLoad(bounce_normals, pixel, 0).w);
        let ids = receiver_triangle(key, bounce_frame.instance_count);
        if ids.x < bounce_frame.instance_count {
            rgb = bounce_estimate(pixel, index, p.xyz, ids);
        }
    }
    bounce_output[index] = vec4<f32>(rgb, 0.0);
}
