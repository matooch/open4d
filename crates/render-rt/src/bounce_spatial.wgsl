// R24.11: filter already evaluated bounce and shadow signals without mapping them.
struct SpatialLimits {
    distance_squared: f32,
    depth: f32,
    lanes: u32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> spatial_limits: SpatialLimits;
@group(0) @binding(1) var spatial_positions: texture_2d<f32>;
@group(0) @binding(2) var spatial_normals: texture_2d<f32>;
@group(0) @binding(3) var<storage, read> spatial_raw: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> spatial_filtered: array<vec4<f32>>;

@compute @workgroup_size(8, 8)
fn bounce_spatial(@builtin(global_invocation_id) id: vec3<u32>) {
    let film = textureDimensions(spatial_positions);
    if any(id.xy >= film) { return; }
    let center = vec2<i32>(id.xy);
    let lanes = spatial_limits.lanes;
    let dual = lanes == 2u;
    let i = (id.y * film.x + id.x) * lanes;
    let value = spatial_raw[i];
    var second = vec4<f32>(0.0);
    if dual { second = spatial_raw[i + 1u]; }
    let p = textureLoad(spatial_positions, center, 0);
    if p.w < 0.5 {
        spatial_filtered[i] = value;
        if dual { spatial_filtered[i + 1u] = second; }
        return;
    }

    let n = textureLoad(spatial_normals, center, 0).xyz;
    let lo = max(center - vec2<i32>(1), vec2<i32>(0));
    let hi = min(center + vec2<i32>(1), vec2<i32>(film) - vec2<i32>(1));
    var sum = value * 2.0;
    var second_sum = second * 2.0;
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
            let j = (u32(y) * film.x + u32(x)) * lanes;
            sum += spatial_raw[j];
            if dual { second_sum += spatial_raw[j + 1u]; }
            weight += 1.0;
        }
    }
    let inv_weight = 1.0 / weight;
    spatial_filtered[i] = sum * inv_weight;
    if dual { spatial_filtered[i + 1u] = second_sum * inv_weight; }
}
