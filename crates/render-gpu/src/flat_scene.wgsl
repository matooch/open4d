// R20 flattened scene implementation for the shared path integrator.
struct Tri {
    p0: vec4<f32>,
    e1: vec4<f32>,
    e2: vec4<f32>,
    ng: vec4<f32>,
    n0: vec4<f32>,
    n1: vec4<f32>,
    n2: vec4<f32>,
    u0: vec4<f32>,
    u1: vec4<f32>,
    u2: vec4<f32>,
    ids: vec4<f32>,
};

struct Node {
    bounds_min: vec4<f32>,
    bounds_max: vec4<f32>,
    link: vec4<u32>,
};

@group(0) @binding(1) var<storage, read> forge_tris: array<Tri>;
@group(0) @binding(2) var<storage, read> forge_nodes: array<Node>;
@group(0) @binding(3) var<storage, read> forge_prims: array<u32>;

fn box_hit(bmin: vec3<f32>, bmax: vec3<f32>, o: vec3<f32>, inv: vec3<f32>, tmax: f32) -> bool {
    let t0 = (bmin - o) * inv;
    let t1 = (bmax - o) * inv;
    let a = min(t0, t1);
    let b = max(t0, t1);
    let tn = max(max(a.x, max(a.y, a.z)), 0.0);
    let tf = min(min(b.x, min(b.y, b.z)), tmax);
    return tn <= tf;
}

fn tri_hit(idx: u32, o: vec3<f32>, d: vec3<f32>, tmax: f32) -> Hit {
    var h = empty_hit();
    h.tri = idx;
    let tri = forge_tris[idx];
    let e1 = tri.e1.xyz;
    let e2 = tri.e2.xyz;
    let pv = cross(d, e2);
    let det = dot(e1, pv);
    if (abs(det) < 1e-18) {
        return h;
    }
    let inv = 1.0 / det;
    let tv = o - tri.p0.xyz;
    let u = dot(tv, pv) * inv;
    if (u < 0.0 || u > 1.0) {
        return h;
    }
    let qv = cross(tv, e1);
    let v = dot(d, qv) * inv;
    if (v < 0.0 || u + v > 1.0) {
        return h;
    }
    let t = dot(e2, qv) * inv;
    if (t > 1e-12 && t < tmax) {
        h.found = true;
        h.t = t;
        h.u = u;
        h.v = v;
    }
    return h;
}

fn flat_intersect(o: vec3<f32>, d: vec3<f32>, tmax_in: f32, any: bool) -> Hit {
    var best = empty_hit();
    var tmax = tmax_in;
    best.t = tmax_in;
    if (arrayLength(&forge_nodes) == 0u) {
        return best;
    }
    let inv = vec3<f32>(1.0) / d;
    var stack: array<u32, 128>;
    var sp = 0u;
    var cur = 0u;
    loop {
        let n = forge_nodes[cur];
        if (box_hit(n.bounds_min.xyz, n.bounds_max.xyz, o, inv, tmax)) {
            if (n.link.y > 0u) {
                for (var k = n.link.x; k < n.link.x + n.link.y; k = k + 1u) {
                    let h = tri_hit(forge_prims[k], o, d, tmax);
                    if (h.found) {
                        tmax = h.t;
                        best = h;
                        if (any) {
                            return best;
                        }
                    }
                }
            } else {
                let l = n.link.x;
                let lb = forge_nodes[l];
                let rb = forge_nodes[l + 1u];
                let dl = dot(lb.bounds_min.xyz + lb.bounds_max.xyz - o * 2.0, lb.bounds_min.xyz + lb.bounds_max.xyz - o * 2.0);
                let dr = dot(rb.bounds_min.xyz + rb.bounds_max.xyz - o * 2.0, rb.bounds_min.xyz + rb.bounds_max.xyz - o * 2.0);
                var near = l;
                var far = l + 1u;
                if (dr < dl) {
                    near = l + 1u;
                    far = l;
                }
                if (sp < 128u) {
                    stack[sp] = far;
                    sp = sp + 1u;
                }
                cur = near;
                continue;
            }
        }
        if (sp == 0u) {
            return best;
        }
        sp = sp - 1u;
        cur = stack[sp];
    }
    return best;
}

fn scene_intersect(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> Hit {
    return flat_intersect(o, d, tmax, false);
}

fn scene_occluded(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> bool {
    return flat_intersect(o, d, tmax, true).found;
}

fn scene_ids(h: Hit) -> vec2<f32> {
    return forge_tris[h.tri].ids.xy;
}

fn scene_geometry(o: vec3<f32>, d: vec3<f32>, h: Hit) -> Geom {
    let tri = forge_tris[h.tri];
    let w = 1.0 - h.u - h.v;
    var g: Geom;
    g.p = o + d * h.t;
    g.ng = tri.ng.xyz;
    if (tri.n0.w == 0.0) {
        g.ns = g.ng;
    } else {
        var ns = tri.n0.xyz * w + tri.n1.xyz * h.u + tri.n2.xyz * h.v;
        if (dot(ns, ns) <= 0.0) {
            ns = g.ng;
        }
        g.ns = normalize(ns);
        if (dot(g.ns, g.ng) < 0.0) {
            g.ns = -g.ns;
        }
    }
    g.corner_uvw[0] = tri.u0.xyz;
    g.corner_uvw[1] = tri.u1.xyz;
    g.corner_uvw[2] = tri.u2.xyz;
    g.uvw = tri.u0.xyz * w + tri.u1.xyz * h.u + tri.u2.xyz * h.v;
    g.closed = tri.ids.w > 0.5;
    g.corners[0] = tri.p0.xyz;
    g.corners[1] = tri.p0.xyz + tri.e1.xyz;
    g.corners[2] = tri.p0.xyz + tri.e2.xyz;
    return g;
}
