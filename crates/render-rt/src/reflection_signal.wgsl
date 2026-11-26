// R24.8 standalone single-bounce reflection transport.
struct ReflectionFrame {
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

@group(0) @binding(0) var<uniform> reflection_frame: ReflectionFrame;
@group(0) @binding(1) var reflection_positions: texture_2d<f32>;
@group(0) @binding(2) var reflection_normals: texture_2d<f32>;
@group(0) @binding(3) var reflection_environment: texture_2d<f32>;
@group(0) @binding(4) var<storage, read_write> reflections: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> reflection_lights: array<GpuLight>;
@group(2) @binding(4) var<storage, read> path_attributes: array<vec4<f32>>;

fn reflection_miss(d: vec3<f32>) -> vec3<f32> {
    let y = clamp(d.y, -1.0, 1.0);
    let intensity = reflection_frame.lighting.w;
    switch reflection_frame.lighting_kind {
        case 1u: {
            return reflection_frame.lighting.xyz * intensity;
        }
        case 2u: {
            let end = select(reflection_frame.ground.xyz, reflection_frame.zenith_rotation.xyz, y >= 0.0);
            let t = abs(y);
            return mix(reflection_frame.lighting.xyz, end, t * t * (3.0 - 2.0 * t)) * intensity;
        }
        case 3u: {
            var azimuth = 0.0;
            if d.x != 0.0 || d.z != 0.0 {
                azimuth = atan2(d.x, d.z);
            }
            let u = fract((azimuth - reflection_frame.zenith_rotation.w) / (2.0 * PI_F));
            let v = acos(y) / PI_F;
            let size = textureDimensions(reflection_environment);
            let texel = min(vec2<u32>(vec2<f32>(size) * vec2<f32>(u, v)), size - vec2<u32>(1u));
            return textureLoad(reflection_environment, texel, 0).rgb * intensity;
        }
        default: {
            return vec3<f32>(0.0);
        }
    }
}

struct ReflectionHit {
    valid: bool,
    p: vec3<f32>,
    ng: vec3<f32>,
    n: vec3<f32>,
    wo: vec3<f32>,
    m: Mat,
    weight: vec3<f32>,
}

fn reflection_hit(hit: RtHit, origin: vec3<f32>, direction: vec3<f32>, weight: vec3<f32>) -> ReflectionHit {
    let g = instance_geometry(origin + direction * hit.tuv.x, hit.ids.xy, hit.tuv.yz);
    let material = u32(path_attributes[7u * hit.ids.x].y);
    let wo = -direction;
    let m = surface_at(material, g, wo, reflection_frame.scale);
    let face = select(-1.0, 1.0, dot(g.ng, wo) > 0.0);
    let ng = g.ng * face;
    let ns = m.n * face;
    return ReflectionHit(true, g.p, ng, select(ns, ng, dot(ns, ng) < 0.0), wo, m, weight);
}

fn reflection_direct(h: ReflectionHit, l: GpuLight, seed: vec2<u32>, stream: vec2<u32>) -> vec3<f32> {
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
        let epsilon = reflection_frame.epsilon;
        let t_max = max(s.dist * (1.0 - 1e-6) - epsilon, 0.0);
        if rt_closest(h.p + h.ng * epsilon, s.wi, t_max).ids.x != RT_MISS {
            let density = l.params.w;
            return value * (vec3<f32>(1.0 - density) + l.shadow_color.xyz * density);
        }
    }
    return value;
}

@compute @workgroup_size(8, 8)
fn reflection_signal(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(reflection_positions);
    if any(id.xy >= size) {
        return;
    }
    let index = id.y * size.x + id.x;
    let p = textureLoad(reflection_positions, id.xy, 0);
    var rgb = vec3<f32>(0.0);
    if p.w >= 0.5 {
        let key = u32(textureLoad(reflection_normals, id.xy, 0).w);
        let ids = receiver_triangle(key, reflection_frame.instance_count);
        if ids.x < reflection_frame.instance_count {
            let g = receiver_geometry(p.xyz, ids);
            let wo = receiver_view(vec2<f32>(id.xy), reflection_frame.jitter,
                reflection_frame.film, reflection_frame.camera);
            let material = u32(path_attributes[7u * ids.x].y);
            let m = surface_at(material, g, wo, reflection_frame.scale);
            let face = select(-1.0, 1.0, dot(g.ng, wo) > 0.0);
            let ng = g.ng * face;
            let ns = m.n * face;
            let n = select(ns, ng, dot(ns, ng) < 0.0);
            let origin = g.p + ng * reflection_frame.epsilon;
            let seed = reflection_frame.seed ^ vec2<u32>(reflection_frame.history_frame, 0u);
            var rng = rng_new(seed, vec2<u32>(index, 0u));
            var secondary: array<ReflectionHit, REFLECTION_SAMPLES>;
            for (var attempt = 0u; attempt < REFLECTION_SAMPLES; attempt += 1u) {
                let s = reflection_sample(m, n, ng, wo, &rng);
                if s.accepted {
                    let hit = rt_closest(origin, s.direction, 1e30);
                    if hit.ids.x == RT_MISS {
                        rgb += s.weight * reflection_miss(s.direction);
                    } else {
                        let h = reflection_hit(hit, origin, s.direction, s.weight);
                        rgb += s.weight * h.m.emission;
                        secondary[attempt] = h;
                    }
                }
            }
            let stream_base = u64_mul(vec2<u32>(index, 0u), vec2<u32>(REFLECTION_SAMPLES, 0u));
            let direct_seed = seed ^ vec2<u32>(0x79fa645bu, 0xed20e2d0u);
            for (var light = 0u; light < reflection_frame.light_count; light += 1u) {
                let l = reflection_lights[light];
                let light_seed = direct_seed ^ vec2<u32>(light, 0u);
                for (var attempt = 0u; attempt < REFLECTION_SAMPLES; attempt += 1u) {
                    let h = secondary[attempt];
                    if h.valid {
                        let stream = u64_add(stream_base, vec2<u32>(attempt, 0u));
                        rgb += reflection_direct(h, l, light_seed, stream);
                    }
                }
            }
        }
    }
    reflections[index] = vec4<f32>(rgb, 0.0);
}
