// R24.11: filter an already evaluated diffuse signal without mapping it.
struct SpatialLimits {
    distance_squared: f32,
    depth: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> spatial_limits: SpatialLimits;
@group(0) @binding(1) var spatial_positions: texture_2d<f32>;
@group(0) @binding(2) var spatial_normals: texture_2d<f32>;
@group(0) @binding(3) var<storage, read> spatial_raw: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> spatial_filtered: array<vec4<f32>>;

@compute @workgroup_size(8, 8)
fn diffuse_spatial(@builtin(global_invocation_id) id: vec3<u32>) {
    let film = textureDimensions(spatial_positions);
    if any(id.xy >= film) { return; }
    let center = vec2<i32>(id.xy);
    let i = id.y * film.x + id.x;
    let value = spatial_raw[i];
    let p = textureLoad(spatial_positions, center, 0);
    if p.w < 0.5 {
        spatial_filtered[i] = value;
        return;
    }

    let n = textureLoad(spatial_normals, center, 0).xyz;
    let lo = max(center - vec2<i32>(1), vec2<i32>(0));
    let hi = min(center + vec2<i32>(1), vec2<i32>(film) - vec2<i32>(1));
    var sum = value * 2.0;
    var weight = 2.0;
    for (var y = lo.y; y <= hi.y; y += 1) {
        for (var x = lo.x; x <= hi.x; x += 1) {
            let neighbour = vec2<i32>(x, y);
            if all(neighbour == center) { continue; }
            let q = textureLoad(spatial_positions, neighbour, 0);
            if q.w != p.w { continue; }
            let delta = q.xyz - p.xyz;
            if dot(n, textureLoad(spatial_normals, neighbour, 0).xyz) < 0.95 ||
                dot(delta, delta) > spatial_limits.distance_squared ||
                abs(dot(delta, n)) > spatial_limits.depth {
                continue;
            }
            sum += spatial_raw[u32(y) * film.x + u32(x)];
            weight += 1.0;
        }
    }
    spatial_filtered[i] = sum * (1.0 / weight);
}
