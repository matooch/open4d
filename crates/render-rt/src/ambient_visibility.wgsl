// R24.6: the input G-buffer is cleared outside the crop. Dispatch enough
// invocations for ceil(width/2) × ceil(height/2) absolute quads.
struct AmbientFrame {
    seed: vec2<u32>,
    history_frame: u32,
    radius: f32,
    epsilon: f32,
}

struct Representative {
    position: vec4<f32>,
    normal: vec3<f32>,
    open: f32,
}

@group(0) @binding(0) var<uniform> ao_frame: AmbientFrame;
@group(0) @binding(1) var ao_positions: texture_2d<f32>;
@group(0) @binding(2) var ao_normals: texture_2d<f32>;
@group(1) @binding(0) var<storage, read_write> ao_visibility: array<f32>;

fn compatible(p: vec4<f32>, n: vec3<f32>, other: Representative, limit: f32) -> bool {
    let delta = p.xyz - other.position.xyz;
    return p.w == other.position.w && dot(n, other.normal) >= 0.95
        && dot(delta, delta) <= limit * limit
        && abs(dot(delta, n)) <= limit * 0.1
        && abs(dot(delta, other.normal)) <= limit * 0.1;
}

@compute @workgroup_size(8, 8)
fn ambient_visibility(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(ao_positions);
    let phase = ao_frame.history_frame % 4u;
    let limit = ao_frame.radius * 0.15;
    let epsilon = min(ao_frame.epsilon, ao_frame.radius * 0.01);
    var reps: array<Representative, 4>;
    var rep_count = 0u;
    for (var order = 0u; order < 4u; order += 1u) {
        let lane = (phase + order) % 4u;
        let pixel = 2u * id.xy + vec2<u32>(lane % 2u, lane / 2u);
        if any(pixel >= size) {
            continue;
        }
        let index = pixel.y * size.x + pixel.x;
        let p = textureLoad(ao_positions, pixel, 0);
        let raw_n = textureLoad(ao_normals, pixel, 0).xyz;
        let length2 = dot(raw_n, raw_n);
        if p.w < 0.5 || !(length2 > 0.0) || ao_frame.radius <= 0.0 {
            ao_visibility[index] = 1.0;
            continue;
        }
        let n = raw_n * inverseSqrt(length2);
        var open = 1.0;
        var reused = false;
        for (var r = 0u; r < rep_count; r += 1u) {
            if compatible(p, n, reps[r], limit) {
                open = reps[r].open;
                reused = true;
                break;
            }
        }
        if !reused {
            let seed = ao_frame.seed ^ vec2<u32>(ao_frame.history_frame, 0u);
            var rng = rng_new(seed, vec2<u32>(index, 0u));
            let direction = to_world(n, cosine_hemisphere(rng_f2(&rng)));
            let hit = rt_closest(p.xyz + n * epsilon, direction, ao_frame.radius);
            open = select(0.0, 1.0, hit.ids.x == RT_MISS);
            reps[rep_count] = Representative(p, n, open);
            rep_count += 1u;
        }
        ao_visibility[index] = open;
    }
}
