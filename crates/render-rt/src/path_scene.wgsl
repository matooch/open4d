// R25: one combined table keeps the compute variant at eight storage bindings.
// Seven rows per snapshot instance: header, then affine rows and normal rows.
// Ten rows per mesh-local triangle: corners, face normal, corner normals, UVW.
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

fn path_transform(row: u32, p: vec4<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(path_attributes[row], p),
        dot(path_attributes[row + 1u], p),
        dot(path_attributes[row + 2u], p),
    );
}

fn path_normal(row: u32, n: vec3<f32>) -> vec3<f32> {
    let v = path_transform(row, vec4<f32>(n, 0.0));
    if dot(v, v) == 0.0 {
        return vec3<f32>(0.0);
    }
    return normalize(v);
}

fn scene_geometry(o: vec3<f32>, d: vec3<f32>, h: Hit) -> Geom {
    let inst = h.instance * 7u;
    let info = path_attributes[inst];
    let tri = bitcast<u32>(info.x) + h.tri * 10u;
    let w = 1.0 - h.u - h.v;
    let local_ng = path_attributes[tri + 3u].xyz;
    var g: Geom;
    g.p = o + d * h.t;
    g.closed = info.w > 0.5;
    g.ng = path_normal(inst + 4u, local_ng);
    g.ns = g.ng;
    let n0 = path_attributes[tri + 4u].xyz;
    let n1 = path_attributes[tri + 5u].xyz;
    let n2 = path_attributes[tri + 6u].xyz;
    let ns = n0 * w + n1 * h.u + n2 * h.v;
    if dot(ns, ns) > 0.0 {
        g.ns = path_normal(inst + 4u, ns);
        if dot(g.ns, g.ng) < 0.0 {
            g.ns = -g.ns;
        }
    }
    for (var i = 0u; i < 3u; i += 1u) {
        g.corners[i] = path_transform(inst + 1u, path_attributes[tri + i]);
        g.corner_uvw[i] = path_attributes[tri + 7u + i].xyz;
    }
    g.uvw = g.corner_uvw[0] * w + g.corner_uvw[1] * h.u + g.corner_uvw[2] * h.v;
    return g;
}
