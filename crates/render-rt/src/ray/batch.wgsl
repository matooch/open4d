struct Query {
    origin: vec4<f32>, // w = t_max
    direction: vec4<f32>,
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&queries) {
        return;
    }
    let ray = queries[id.x];
    hits[id.x] = rt_closest(ray.origin.xyz, ray.direction.xyz, ray.origin.w);
}
