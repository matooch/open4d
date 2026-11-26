// R23.1: closest opaque triangle in wgpu's hardware acceleration structures.
fn rt_closest(origin: vec3<f32>, direction: vec3<f32>, t_max: f32) -> RtHit {
    let speed = length(direction);
    if !(speed > 0.0 && t_max > 1e-12) {
        return rt_miss(t_max);
    }
    var query: ray_query;
    rayQueryInitialize(&query, scene, RayDesc(
        RAY_FLAG_FORCE_OPAQUE, 0xffu, 1e-12 * speed, t_max * speed,
        origin, direction / speed,
    ));
    while rayQueryProceed(&query) {}
    let hit = rayQueryGetCommittedIntersection(&query);
    let t = hit.t / speed;
    if hit.kind == RAY_QUERY_INTERSECTION_TRIANGLE && t > 1e-12 && t < t_max {
        return RtHit(vec4<f32>(t, hit.barycentrics, 0.0),
            vec4<u32>(hit.instance_custom_data, hit.primitive_index, 0u, 0u));
    }
    return rt_miss(t_max);
}
