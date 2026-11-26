// R23: threaded TLAS and BLAS traversal. Node layout follows R20.1; links.z
// links to the next subtree. Instance and triangle IDs stay mesh/snapshot-local.

struct Node {
    lo: vec4<f32>,
    hi: vec4<f32>,
    links: vec4<u32>,
}
struct Instance {
    inverse: mat4x4<f32>,
    ids: vec4<u32>,
}
struct Triangle {
    p0: vec4<f32>,
    e1: vec4<f32>,
    e2: vec4<f32>,
    ids: vec4<u32>,
}
fn box_hit(node: Node, o: vec3<f32>, d: vec3<f32>, t_max: f32) -> bool {
    var near = 0.0;
    var far = t_max;
    for (var axis = 0u; axis < 3u; axis++) {
        let p = o[axis];
        let lo = node.lo[axis];
        let hi = node.hi[axis];
        let direction = d[axis];
        // A parallel ray can lie exactly on a slab; don't multiply zero by infinity.
        if direction == 0.0 {
            if p < lo || p > hi {
                return false;
            }
        } else {
            let a = (lo - p) / direction;
            let b = (hi - p) / direction;
            near = max(near, min(a, b));
            far = min(far, max(a, b));
        }
    }
    return near <= far;
}

fn triangle_hit(tri: Triangle, o: vec3<f32>, d: vec3<f32>, t_max: f32) -> vec3<f32> {
    let pv = cross(d, tri.e2.xyz);
    let det = dot(tri.e1.xyz, pv);
    if abs(det) < 1e-18 {
        return vec3<f32>(t_max, 0.0, 0.0);
    }
    let tv = o - tri.p0.xyz;
    let u = dot(tv, pv) / det;
    let qv = cross(tv, tri.e1.xyz);
    let v = dot(d, qv) / det;
    let t = dot(tri.e2.xyz, qv) / det;
    if u >= 0.0 && v >= 0.0 && u + v <= 1.0 && t > 1e-12 && t < t_max {
        return vec3<f32>(t, u, v);
    }
    return vec3<f32>(t_max, 0.0, 0.0);
}

fn visit_mesh(inst: Instance, origin: vec3<f32>, direction: vec3<f32>, previous: RtHit) -> RtHit {
    let o = (inst.inverse * vec4<f32>(origin, 1.0)).xyz;
    let d = (inst.inverse * vec4<f32>(direction, 0.0)).xyz;
    var best = previous;
    var cursor = inst.ids.x;
    while cursor != RT_MISS {
        let node = blas[cursor];
        cursor = node.links.z;
        if box_hit(node, o, d, best.tuv.x) {
            if node.links.y == 0u {
                cursor = node.links.x;
            } else {
                for (var i = 0u; i < node.links.y; i++) {
                    let tri = triangles[node.links.x + i];
                    let tuv = triangle_hit(tri, o, d, best.tuv.x);
                    if tuv.x < best.tuv.x {
                        best = RtHit(vec4<f32>(tuv, 0.0), vec4<u32>(inst.ids.y, tri.ids.x, 0u, 0u));
                    }
                }
            }
        }
    }
    return best;
}

fn rt_closest(origin: vec3<f32>, direction: vec3<f32>, t_max: f32) -> RtHit {
    var best = rt_miss(t_max);
    var cursor = 0u;
    while cursor != RT_MISS {
        let node = tlas[cursor];
        cursor = node.links.z;
        if box_hit(node, origin, direction, best.tuv.x) {
            if node.links.y == 0u {
                cursor = node.links.x;
            } else {
                for (var i = 0u; i < node.links.y; i++) {
                    best = visit_mesh(instances[node.links.x + i], origin, direction, best);
                }
            }
        }
    }
    return best;
}
