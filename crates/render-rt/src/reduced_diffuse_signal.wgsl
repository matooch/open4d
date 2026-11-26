// R24.10: one attempt per compatible representative in an absolute quad.
const BOUNCE_SAMPLES: u32 = 1u;

struct DiffuseRepresentative {
    position: vec4<f32>,
    normal: vec3<f32>,
    key: u32,
    rgb: vec3<f32>,
}

fn diffuse_compatible(p: vec4<f32>, n: vec3<f32>, key: u32, rep: DiffuseRepresentative,
    limit: f32) -> bool {
    let delta = p.xyz - rep.position.xyz;
    return key == rep.key && p.w == rep.position.w && dot(n, rep.normal) >= 0.95
        && dot(delta, delta) <= limit * limit
        && abs(dot(delta, n)) <= limit * 0.1
        && abs(dot(delta, rep.normal)) <= limit * 0.1;
}

@compute @workgroup_size(8, 8)
fn reduced_diffuse_signal(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(bounce_positions);
    let phase = bounce_frame.history_frame % 4u;
    let limit = 0.015 * bounce_frame.scale;
    var reps: array<DiffuseRepresentative, 4>;
    var count = 0u;
    for (var order = 0u; order < 4u; order += 1u) {
        let lane = (phase + order) % 4u;
        let pixel = id.xy * 2u + vec2<u32>(lane % 2u, lane / 2u);
        if any(pixel >= size) {
            continue;
        }
        let index = pixel.y * size.x + pixel.x;
        let p = textureLoad(bounce_positions, pixel, 0);
        if p.w < 0.5 {
            bounce_output[index] = vec4<f32>(0.0);
            continue;
        }
        let raw = textureLoad(bounce_normals, pixel, 0);
        let key = u32(raw.w);
        let length2 = dot(raw.xyz, raw.xyz);
        var n = vec3<f32>(0.0);
        if length2 > 0.0 {
            n = raw.xyz * inverseSqrt(length2);
        }
        var representative = 0u;
        while representative < count && !diffuse_compatible(p, n, key, reps[representative], limit) {
            representative += 1u;
        }
        if representative == count {
            let ids = receiver_triangle(key, bounce_frame.instance_count);
            if ids.x >= bounce_frame.instance_count {
                bounce_output[index] = vec4<f32>(0.0);
                continue;
            }
            reps[count] = DiffuseRepresentative(p, n, key, bounce_estimate(pixel, index, p.xyz, ids));
            count += 1u;
        }
        bounce_output[index] = vec4<f32>(reps[representative].rgb, 0.0);
    }
}
