// Viewport shaders (docs/specs/VIEWPORT.md §6).

struct Uniforms {
    view_proj: mat4x4<f32>,
    base: vec4<f32>,
    selection: vec4<f32>,
    wire: vec4<f32>,
    // xyz: unit direction towards the light (headlight = -forward)
    light: vec4<f32>,
    // x: 1 = smooth normals, y: depth bias in NDC (toward the camera)
    params: vec4<f32>,
    // x: half point size in pixels, y/z: NDC units per pixel (2/width, 2/height)
    point: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct TriIn {
    @location(0) pos: vec3<f32>,
    @location(1) flat_n: vec3<f32>,
    @location(2) smooth_n: vec3<f32>,
    @location(3) selected: f32,
};

struct TriOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) n: vec3<f32>,
    @location(1) @interpolate(flat) selected: f32,
};

@vertex
fn vs_tri(v: TriIn) -> TriOut {
    var o: TriOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.n = select(v.flat_n, v.smooth_n, u.params.x > 0.5);
    o.selected = v.selected;
    return o;
}

@fragment
fn fs_tri(i: TriOut) -> @location(0) vec4<f32> {
    let len = length(i.n);
    let n = select(vec3<f32>(0.0), i.n / len, len > 1e-8);
    let k = 0.25 + 0.75 * abs(dot(n, u.light.xyz));
    let base = select(u.base.rgb, u.selection.rgb, i.selected > 0.5);
    return vec4<f32>(base * k, 1.0);
}

struct EdgeIn {
    @location(0) pos: vec3<f32>,
    @location(1) selected: f32,
};

struct EdgeOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) @interpolate(flat) selected: f32,
};

@vertex
fn vs_edge(v: EdgeIn) -> EdgeOut {
    var o: EdgeOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.clip.z = o.clip.z - u.params.y * o.clip.w;
    o.selected = v.selected;
    return o;
}

@fragment
fn fs_edge(i: EdgeOut) -> @location(0) vec4<f32> {
    return vec4<f32>(select(u.wire.rgb, u.selection.rgb, i.selected > 0.5), 1.0);
}

struct PointIn {
    @location(0) pos: vec3<f32>,
    @location(1) selected: f32,
    @location(2) corner: vec2<f32>,
};

// A screen-space square of `u.point.x * 2` pixels round the point; drawn with fs_edge.
@vertex
fn vs_point(v: PointIn) -> EdgeOut {
    var o: EdgeOut;
    var c = u.view_proj * vec4<f32>(v.pos, 1.0);
    c.z = c.z - u.params.y * c.w;
    c.x = c.x + v.corner.x * u.point.x * u.point.y * c.w;
    c.y = c.y + v.corner.y * u.point.x * u.point.z * c.w;
    o.clip = c;
    o.selected = v.selected;
    return o;
}

struct ColorIn {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct ColorOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_color(v: ColorIn) -> ColorOut {
    var o: ColorOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.color = v.color;
    return o;
}

@fragment
fn fs_color(i: ColorOut) -> @location(0) vec4<f32> {
    return vec4<f32>(i.color, 1.0);
}
