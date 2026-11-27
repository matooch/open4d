// Selection outline (docs/specs/VIEWPORT.md §16.2): dilates the mask of the highlighted objects
// (R: coverage of every highlighted object, G: coverage of the active one) and draws the ring
// outside it over the image.

struct Params {
    // rgb: outline colour of a selected object, of the active object
    selected: vec4<f32>,
    stronger: vec4<f32>,
    // x: width of the selected outline in pixels, y: width of the active outline
    widths: vec4<f32>,
};

@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var mask: texture_2d<f32>;

@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    // one triangle covering the target
    let x = f32(i32(i & 1u) * 4 - 1);
    let y = f32(i32(i >> 1u) * 4 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_dilate(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let c = vec2<i32>(pos.xy);
    let hi = vec2<i32>(textureDimensions(mask)) - vec2<i32>(1, 1);
    let here = textureLoad(mask, c, 0).r;
    if here >= 0.999 {
        discard;
    }
    let reach = i32(ceil(max(p.widths.x, p.widths.y)));
    var any_cov = 0.0;
    var active_cov = 0.0;
    for (var dy = -reach; dy <= reach; dy++) {
        for (var dx = -reach; dx <= reach; dx++) {
            let d = length(vec2<f32>(f32(dx), f32(dy)));
            let m = textureLoad(mask, clamp(c + vec2<i32>(dx, dy), vec2<i32>(0, 0), hi), 0).rg;
            if d <= p.widths.x + 0.5 {
                any_cov = max(any_cov, m.r);
            }
            if d <= p.widths.y + 0.5 {
                active_cov = max(active_cov, m.g);
            }
        }
    }
    let alpha = max(any_cov, active_cov) * (1.0 - here);
    if alpha <= 0.0 {
        discard;
    }
    let rgb = select(p.selected.rgb, p.stronger.rgb, active_cov > 0.0);
    return vec4<f32>(rgb, alpha);
}
