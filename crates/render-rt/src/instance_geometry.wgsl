// One combined R25 attribute table. Each instance has a header, 3 world
// affine rows and 3 inverse-transpose rows. The first normal row's w holds
// the bitcast exclusive end of this instance in the raster triangle stream.
// Each mesh-local triangle has 3 corners, a face normal, 3 normals and 3 UVWs.

fn path_transform(row: u32, p: vec4<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(path_attributes[row], p),
        dot(path_attributes[row + 1u], p),
        dot(path_attributes[row + 2u], p),
    );
}

fn path_normal(row: u32, n: vec3<f32>) -> vec3<f32> {
    let v = vec3<f32>(
        dot(path_attributes[row].xyz, n),
        dot(path_attributes[row + 1u].xyz, n),
        dot(path_attributes[row + 2u].xyz, n),
    );
    if dot(v, v) == 0.0 {
        return vec3<f32>(0.0);
    }
    return normalize(v);
}

fn instance_geometry(p: vec3<f32>, ids: vec2<u32>, bary: vec2<f32>) -> Geom {
    let inst = ids.x * 7u;
    let info = path_attributes[inst];
    let tri = bitcast<u32>(info.x) + ids.y * 10u;
    let w = 1.0 - bary.x - bary.y;
    let local_ng = path_attributes[tri + 3u].xyz;
    var g: Geom;
    g.p = p;
    g.closed = info.w > 0.5;
    g.ng = path_normal(inst + 4u, local_ng);
    g.ns = g.ng;
    let n0 = path_attributes[tri + 4u].xyz;
    let n1 = path_attributes[tri + 5u].xyz;
    let n2 = path_attributes[tri + 6u].xyz;
    let ns = n0 * w + n1 * bary.x + n2 * bary.y;
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
    g.uvw = g.corner_uvw[0] * w + g.corner_uvw[1] * bary.x + g.corner_uvw[2] * bary.y;
    return g;
}
