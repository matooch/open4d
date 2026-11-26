// R24.12: update one full-film signal, leaving the accepted history untouched.
struct HistoryParams {
    film: vec2<u32>,
    stride: u32,
    frames: u32,
    weight: f32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> params: HistoryParams;
@group(0) @binding(1) var<storage, read> filtered: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> previous: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> next: array<vec4<f32>>;

@compute @workgroup_size(8, 8)
fn diffuse_history(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.stride || id.y >= params.film.y {
        return;
    }
    let history_index = id.y * params.stride + id.x;
    if id.x >= params.film.x {
        next[history_index] = vec4<f32>(0.0);
        return;
    }
    let sample = filtered[id.y * params.film.x + id.x];
    if params.frames == 0u {
        next[history_index] = sample;
    } else {
        let mean = previous[history_index];
        next[history_index] = mean + (sample - mean) * params.weight;
    }
}
