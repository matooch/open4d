// R23: threaded TLAS and BLAS traversal. Node layout follows R20.1; meta.z
// links to the next subtree. Instance and triangle IDs stay mesh/snapshot-local.
const END: u32 = 0xffffffffu;

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
struct Query {
    origin: vec4<f32>, // w = t_max
    direction: vec4<f32>,
}
struct Hit {
    tuv: vec4<f32>,
    ids: vec4<u32>,
}

@group(0) @binding(0) var<storage, read> tlas: array<Node>;
@group(0) @binding(1) var<storage, read> instances: array<Instance>;
@group(0) @binding(2) var<storage, read> blas: array<Node>;
@group(0) @binding(3) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(4) var<storage, read> queries: array<Query>;
@group(0) @binding(5) var<storage, read_write> hits: array<Hit>;

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

fn visit_mesh(inst: Instance, q: Query, previous: Hit) -> Hit {
    let o = (inst.inverse * vec4<f32>(q.origin.xyz, 1.0)).xyz;
    let d = (inst.inverse * vec4<f32>(q.direction.xyz, 0.0)).xyz;
    var best = previous;
    var cursor = inst.ids.x;
    while cursor != END {
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
                        best = Hit(vec4<f32>(tuv, 0.0), vec4<u32>(inst.ids.y, tri.ids.x, 0u, 0u));
                    }
                }
            }
        }
    }
    return best;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&queries) {
        return;
    }
    let q = queries[id.x];
    var best = Hit(vec4<f32>(q.origin.w, 0.0, 0.0, 0.0), vec4<u32>(END));
    var cursor = 0u;
    while cursor != END {
        let node = tlas[cursor];
        cursor = node.links.z;
        if box_hit(node, q.origin.xyz, q.direction.xyz, best.tuv.x) {
            if node.links.y == 0u {
                cursor = node.links.x;
            } else {
                for (var i = 0u; i < node.links.y; i++) {
                    best = visit_mesh(instances[node.links.x + i], q, best);
                }
            }
        }
    }
    hits[id.x] = best;
}
