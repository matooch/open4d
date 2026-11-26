// R24.8 lobe and environment specialization for shared bounce transport.
const BOUNCE_SAMPLES: u32 = REFLECTION_SAMPLES;
@group(0) @binding(3) var reflection_environment: texture_2d<f32>;

fn bounce_sample(m: Mat, n: vec3<f32>, ng: vec3<f32>, wo: vec3<f32>,
    rng: ptr<function, Rng>) -> ReflectionSample {
    return reflection_sample(m, n, ng, wo, rng);
}

fn bounce_miss(d: vec3<f32>) -> vec3<f32> {
    let y = clamp(d.y, -1.0, 1.0);
    let intensity = bounce_frame.lighting.w;
    switch bounce_frame.lighting_kind {
        case 1u: {
            return bounce_frame.lighting.xyz * intensity;
        }
        case 2u: {
            let end = select(bounce_frame.ground.xyz, bounce_frame.zenith_rotation.xyz, y >= 0.0);
            let t = abs(y);
            return mix(bounce_frame.lighting.xyz, end, t * t * (3.0 - 2.0 * t)) * intensity;
        }
        case 3u: {
            var azimuth = 0.0;
            if d.x != 0.0 || d.z != 0.0 {
                azimuth = atan2(d.x, d.z);
            }
            let u = fract((azimuth - bounce_frame.zenith_rotation.w) / (2.0 * PI_F));
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

@compute @workgroup_size(8, 8)
fn reflection_signal(@builtin(global_invocation_id) id: vec3<u32>) {
    bounce_signal(id.xy);
}
