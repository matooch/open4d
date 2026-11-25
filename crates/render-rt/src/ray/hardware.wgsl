// R23.1: closest opaque triangle in wgpu's hardware acceleration structures.
enable wgpu_ray_query;

struct Query {
    origin: vec4<f32>, // w = t_max
    direction: vec4<f32>,
}
struct Hit {
    tuv: vec4<f32>,
    ids: vec4<u32>,
}

@group(0) @binding(0) var scene: acceleration_structure;
@group(0) @binding(1) var<storage, read> queries: array<Query>;
@group(0) @binding(2) var<storage, read_write> hits: array<Hit>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= arrayLength(&queries) {
        return;
    }
    hits[i] = Hit(vec4<f32>(0.0), vec4<u32>(0xffffffffu, 0u, 0u, 0u));
    let ray = queries[i];
    let speed = length(ray.direction.xyz);
    let t_max = ray.origin.w;
    if !(speed > 0.0 && t_max > 1e-12) {
        return;
    }
    var query: ray_query;
    rayQueryInitialize(&query, scene, RayDesc(
        RAY_FLAG_FORCE_OPAQUE, 0xffu, 1e-12 * speed, t_max * speed,
        ray.origin.xyz, ray.direction.xyz / speed,
    ));
    while rayQueryProceed(&query) {}
    let hit = rayQueryGetCommittedIntersection(&query);
    let t = hit.t / speed;
    if hit.kind == RAY_QUERY_INTERSECTION_TRIANGLE && t > 1e-12 && t < t_max {
        hits[i] = Hit(vec4<f32>(t, hit.barycentrics, 0.0),
            vec4<u32>(hit.instance_custom_data, hit.primitive_index, 0u, 0u));
    }
}
