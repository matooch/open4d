// R24.7: evaluate raw direct shadows for the covered raster receivers.
struct DirectShadowFrame {
    camera: mat3x3<f32>,
    film: vec3<f32>,
    scale: f32,
    jitter: vec2<f32>,
    seed: vec2<u32>,
    history_frame: u32,
    instance_count: u32,
    light_count: u32,
    epsilon: f32,
}

struct DirectSignal {
    diffuse_unoccluded: vec4<f32>,
    specular_occluded: vec4<f32>,
}

@group(0) @binding(0) var<uniform> shadow_frame: DirectShadowFrame;
@group(0) @binding(1) var shadow_positions: texture_2d<f32>;
@group(0) @binding(2) var shadow_normals: texture_2d<f32>;
@group(0) @binding(3) var<storage, read> shadow_lights: array<GpuLight>;
@group(0) @binding(4) var<storage, read_write> shadow_signal: array<DirectSignal>;
@group(2) @binding(4) var<storage, read> path_attributes: array<vec4<f32>>;

const SHADOW_SAMPLES: u32 = 8u;

fn direct_signal(p: vec3<f32>, origin: vec3<f32>, ng: vec3<f32>, n: vec3<f32>,
    wo: vec3<f32>, m: Mat, index: u32) -> DirectSignal {
    var signal: DirectSignal;
    let seed = shadow_frame.seed ^ vec2<u32>(shadow_frame.history_frame, 0u);
    let stream_base = u64_mul(vec2<u32>(index, 0u), vec2<u32>(shadow_frame.light_count, 0u));
    for (var i = 0u; i < shadow_frame.light_count; i += 1u) {
        let l = shadow_lights[i];
        let density = l.params.w;
        let tint = vec3<f32>(1.0 - density) + l.shadow_color.xyz * density;
        var rng = rng_new(seed, u64_add(stream_base, vec2<u32>(i, 0u)));
        for (var j = 0u; j < SHADOW_SAMPLES; j += 1u) {
            let s = light_sample(l, p, &rng);
            let cos_i = dot(n, s.wi);
            if !s.ok || cos_i <= 0.0 || dot(ng, s.wi) <= 0.0 {
                continue;
            }
            let f = mat_eval(m, n, wo, s.wi);
            let fd = select(vec3<f32>(0.0), f.fd, l.flags.z > 0.5);
            let fs = select(vec3<f32>(0.0), f.fs, l.flags.w > 0.5);
            if all(fd == vec3<f32>(0.0)) && all(fs == vec3<f32>(0.0)) {
                continue;
            }
            var mis = 1.0;
            if s.pdf > 0.0 {
                mis = power_heuristic(s.pdf, mat_pdf(m, n, wo, s.wi));
            }
            let weight = s.weight * (cos_i * mis / f32(SHADOW_SAMPLES));
            let diffuse = fd * weight;
            let specular = fs * weight;
            let total = diffuse + specular;
            let t_max = max(s.dist * (1.0 - 1e-6) - shadow_frame.epsilon, 0.0);
            let blocked = rt_closest(origin, s.wi, t_max).ids.x != RT_MISS;
            let factor = select(vec3<f32>(1.0), tint, blocked);
            signal.diffuse_unoccluded += vec4<f32>(diffuse * factor, lum(total));
            signal.specular_occluded += vec4<f32>(specular * factor, lum(total * factor));
        }
    }
    return signal;
}

@compute @workgroup_size(8, 8)
fn direct_shadow(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(shadow_positions);
    if any(id.xy >= size) {
        return;
    }
    let index = id.y * size.x + id.x;
    let p = textureLoad(shadow_positions, id.xy, 0);
    var signal: DirectSignal;
    if p.w >= 0.5 && shadow_frame.light_count > 0u {
        let key = u32(textureLoad(shadow_normals, id.xy, 0).w);
        let ids = receiver_triangle(key, shadow_frame.instance_count);
        if ids.x < shadow_frame.instance_count {
            let g = receiver_geometry(p.xyz, ids);
            let wo = receiver_view(vec2<f32>(id.xy), shadow_frame.jitter, shadow_frame.film, shadow_frame.camera);
            let mat_id = u32(path_attributes[ids.x * 7u].y);
            let m = surface_at(mat_id, g, wo, shadow_frame.scale);
            let face = select(-1.0, 1.0, dot(g.ng, wo) > 0.0);
            let ng = g.ng * face;
            let ns = m.n * face;
            let n = select(ns, ng, dot(ns, ng) < 0.0);
            let origin = g.p + ng * shadow_frame.epsilon;
            signal = direct_signal(g.p, origin, ng, n, wo, m, index);
        }
    }
    shadow_signal[index] = signal;
}
