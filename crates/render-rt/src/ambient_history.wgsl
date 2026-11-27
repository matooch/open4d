// R24.13: only the padded lighting output is strengthened.
struct Frame {
    width: u32,
    height: u32,
    stride: u32, // padded row length in texels
    channels: u32,
    previous_frames: u32,
    weight: f32,
    strength: f32,
}

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read> filtered: array<f32>;
@group(0) @binding(2) var<storage, read> previous: array<f32>;
@group(0) @binding(3) var<storage, read_write> mean: array<f32>;
@group(0) @binding(4) var<storage, read_write> lighting: array<f32>;

@compute @workgroup_size(8, 8)
fn ambient_history(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= frame.stride || id.y >= frame.height {
        return;
    }
    var visibility = 0.0;
    if id.x < frame.width {
        let i = id.y * frame.width + id.x;
        var sample = filtered[i];
        if frame.previous_frames != 0u {
            let old = previous[i];
            sample = old + (sample - old) * frame.weight;
        }
        mean[i] = sample;
        visibility = 1.0 - (1.0 - sample) * frame.strength;
    }
    let dst = (id.y * frame.stride + id.x) * frame.channels;
    lighting[dst] = visibility;
    if frame.channels == 4u {
        lighting[dst + 1u] = 0.0;
        lighting[dst + 2u] = 0.0;
        lighting[dst + 3u] = 0.0;
    }
}
