const RT_MISS: u32 = 0xffffffffu;

struct RtHit {
    tuv: vec4<f32>, // t in the caller's ray parameterization, barycentric u and v
    ids: vec4<u32>, // snapshot instance and mesh-local primitive
}

fn rt_miss(t_max: f32) -> RtHit {
    return RtHit(vec4<f32>(t_max, 0.0, 0.0, 0.0), vec4<u32>(RT_MISS, 0u, 0u, 0u));
}
