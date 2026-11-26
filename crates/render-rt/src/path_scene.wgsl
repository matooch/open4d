@group(2) @binding(4) var<storage, read> path_attributes: array<vec4<f32>>;

fn scene_intersect(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> Hit {
    let hit = rt_closest(o, d, tmax);
    return Hit(hit.ids.x != RT_MISS, hit.tuv.x, hit.ids.y, hit.ids.x, hit.tuv.y, hit.tuv.z);
}

fn scene_occluded(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> bool {
    return rt_closest(o, d, tmax).ids.x != RT_MISS;
}

fn scene_ids(h: Hit) -> vec2<f32> {
    return path_attributes[h.instance * 7u].yz;
}

fn scene_geometry(o: vec3<f32>, d: vec3<f32>, h: Hit) -> Geom {
    return instance_geometry(o + d * h.t, vec2<u32>(h.instance, h.tri), vec2<f32>(h.u, h.v));
}
