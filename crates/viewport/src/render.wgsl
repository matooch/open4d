// Viewport shaders (docs/specs/VIEWPORT.md §6).

struct Uniforms {
    view_proj: mat4x4<f32>,
    base: vec4<f32>,
    selection: vec4<f32>,
    wire: vec4<f32>,
    // unused (the lights are in group 1)
    light: vec4<f32>,
    // x: 1 = smooth normals, y: depth bias in NDC (toward the camera), z: 1 = unlit,
    // w: 1 = instanced draws multiply the base colour by the clone colour
    params: vec4<f32>,
    // x: half point size in pixels, y/z: NDC units per pixel (2/width, 2/height)
    point: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

// Lights (docs/specs/VIEWPORT.md §8.1-§8.2)
struct Light {
    // directional: xyz unit vector towards the light; point/spot: position; w: kind 0/1/2
    pos_kind: vec4<f32>,
    // rgb: colour * intensity; w: shadow layer, -1 = none
    color: vec4<f32>,
    // spot: xyz axis (travel direction), w: cos(outer)
    axis_cos: vec4<f32>,
    // x: cos(inner), y: range (0 = none), z: shadow bias in NDC depth
    params: vec4<f32>,
    shadow_vp: mat4x4<f32>,
};

struct Lights {
    lights: array<Light, 8>,
    // x: light count, y: ambient, z: 1 / shadow map size
    info: vec4<f32>,
};

@group(1) @binding(0) var<uniform> lit: Lights;
@group(1) @binding(1) var shadow_map: texture_depth_2d_array;
@group(1) @binding(2) var shadow_samp: sampler_comparison;

// Fraction of the 3x3 taps around the fragment that the light reaches.
fn shadow_factor(light: Light, wpos: vec3<f32>) -> f32 {
    let layer = i32(light.color.w);
    if layer < 0 {
        return 1.0;
    }
    let lp = light.shadow_vp * vec4<f32>(wpos, 1.0);
    if lp.w <= 0.0 {
        return 1.0;
    }
    let ndc = lp.xyz / lp.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || ndc.z > 1.0 || ndc.z < 0.0 {
        return 1.0;
    }
    let reference = ndc.z - light.params.z;
    var sum = 0.0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let o = vec2<f32>(f32(dx), f32(dy)) * lit.info.z;
            sum += textureSampleCompareLevel(shadow_map, shadow_samp, uv + o, layer, reference);
        }
    }
    return sum / 9.0;
}

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
    @location(2) wpos: vec3<f32>,
    @location(3) @interpolate(flat) tint: vec3<f32>,
};

// Per-instance data of clones (docs/specs/VIEWPORT.md §10.1): rows of the affine matrix
// (xyz linear part, w translation) and the clone colour.
struct InstIn {
    @location(4) r0: vec4<f32>,
    @location(5) r1: vec4<f32>,
    @location(6) r2: vec4<f32>,
    @location(7) color: vec4<f32>,
};

fn inst_point(i: InstIn, p: vec3<f32>) -> vec3<f32> {
    let h = vec4<f32>(p, 1.0);
    return vec3<f32>(dot(i.r0, h), dot(i.r1, h), dot(i.r2, h));
}

// Cofactor matrix of the linear part times n: the inverse transpose up to scale and sign.
fn inst_normal(i: InstIn, n: vec3<f32>) -> vec3<f32> {
    let c0 = vec3<f32>(i.r0.x, i.r1.x, i.r2.x);
    let c1 = vec3<f32>(i.r0.y, i.r1.y, i.r2.y);
    let c2 = vec3<f32>(i.r0.z, i.r1.z, i.r2.z);
    return cross(c1, c2) * n.x + cross(c2, c0) * n.y + cross(c0, c1) * n.z;
}

@vertex
fn vs_shadow(v: TriIn) -> @builtin(position) vec4<f32> {
    return u.view_proj * vec4<f32>(v.pos, 1.0);
}

@vertex
fn vs_shadow_inst(v: TriIn, i: InstIn) -> @builtin(position) vec4<f32> {
    return u.view_proj * vec4<f32>(inst_point(i, v.pos), 1.0);
}

@vertex
fn vs_tri(v: TriIn) -> TriOut {
    var o: TriOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.wpos = v.pos;
    o.n = select(v.flat_n, v.smooth_n, u.params.x > 0.5);
    o.selected = v.selected;
    o.tint = vec3<f32>(1.0);
    return o;
}

@vertex
fn vs_tri_inst(v: TriIn, i: InstIn) -> TriOut {
    var o: TriOut;
    let wpos = inst_point(i, v.pos);
    o.clip = u.view_proj * vec4<f32>(wpos, 1.0);
    o.wpos = wpos;
    o.n = inst_normal(i, select(v.flat_n, v.smooth_n, u.params.x > 0.5));
    o.selected = v.selected;
    o.tint = mix(vec3<f32>(1.0), i.color.rgb, u.params.w);
    return o;
}

@fragment
fn fs_tri(i: TriOut) -> @location(0) vec4<f32> {
    let len = length(i.n);
    let n = select(vec3<f32>(0.0), i.n / len, len > 1e-8);
    let base = select(u.base.rgb, u.selection.rgb, i.selected > 0.5) * i.tint;
    if u.params.z > 0.5 {
        return vec4<f32>(base, 1.0);
    }
    var sum = vec3<f32>(0.0);
    let count = i32(lit.info.x);
    for (var k = 0; k < count; k++) {
        let li = lit.lights[k];
        var l = li.pos_kind.xyz;
        var att = 1.0;
        if li.pos_kind.w > 0.5 {
            let d = li.pos_kind.xyz - i.wpos;
            let dist = length(d);
            l = d / max(dist, 1e-9);
            if li.params.y > 0.0 {
                let t = 1.0 - min(dist / li.params.y, 1.0);
                att = t * t;
            }
            if li.pos_kind.w > 1.5 {
                att = att * smoothstep(li.axis_cos.w, li.params.x, dot(-l, li.axis_cos.xyz));
            }
        }
        sum += li.color.rgb * (att * abs(dot(n, l)) * shadow_factor(li, i.wpos));
    }
    return vec4<f32>(base * (vec3<f32>(lit.info.y) + sum), 1.0);
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

@vertex
fn vs_edge_inst(v: EdgeIn, i: InstIn) -> EdgeOut {
    var o: EdgeOut;
    o.clip = u.view_proj * vec4<f32>(inst_point(i, v.pos), 1.0);
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
