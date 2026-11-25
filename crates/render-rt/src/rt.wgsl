// Real-time raster passes (docs/specs/RENDER.md R21): G-buffer, shadow depth and deferred
// lighting. The surface functions (`forge_shade`) are generated above this file from the same
// shading IR as the CPU tracer (R7) and the compute tracer (R20.1); the BRDF, falloff and
// environment helpers repeat trace.wgsl's formulas (R20.2) so the tiers agree.

struct Tri {
    p0: vec4<f32>,
    e1: vec4<f32>,
    e2: vec4<f32>,
    ng: vec4<f32>,
    n0: vec4<f32>,
    n1: vec4<f32>,
    n2: vec4<f32>,
    u0: vec4<f32>,
    u1: vec4<f32>,
    u2: vec4<f32>,
    ids: vec4<f32>,
};

struct GpuLight {
    pos_radius: vec4<f32>,
    axis_kind: vec4<f32>,
    color: vec4<f32>,
    tangent_shape: vec4<f32>,
    bitangent: vec4<f32>,
    params: vec4<f32>,
    flags: vec4<f32>,
    shadow_color: vec4<f32>,
    spot: vec4<f32>,
    shape: vec4<f32>,
};

struct ShadowMap {
    m: array<mat4x4<f32>, 6>,
    // light index, cube (0/1), tile, soft (0/1)
    params: vec4<u32>,
    // depth bias
    bias: vec4<f32>,
};

@group(0) @binding(0) var<uniform> forge_frame: array<vec4<f32>, 128>;
@group(0) @binding(1) var<storage, read> forge_tris: array<Tri>;
@group(0) @binding(2) var<storage, read> forge_lights: array<GpuLight>;
@group(0) @binding(3) var<storage, read> forge_shadows: array<ShadowMap>;
@group(1) @binding(0) var forge_g0: texture_2d<f32>;
@group(1) @binding(1) var forge_g1: texture_2d<f32>;
@group(1) @binding(2) var forge_shadow_tex: texture_depth_2d;
@group(1) @binding(3) var forge_shadow_sampler: sampler_comparison;
@group(1) @binding(4) var forge_env_tex: texture_2d<f32>;
@group(1) @binding(5) var forge_env_sampler: sampler;
@group(1) @binding(6) var forge_lighting_src: texture_2d_array<f32>;

const PI_F: f32 = 3.141592653589793;

fn fr(i: u32) -> f32 {
    return forge_frame[i].x;
}

fn m4(base: u32) -> mat4x4<f32> {
    return mat4x4<f32>(
        vec4<f32>(fr(base), fr(base + 1u), fr(base + 2u), fr(base + 3u)),
        vec4<f32>(fr(base + 4u), fr(base + 5u), fr(base + 6u), fr(base + 7u)),
        vec4<f32>(fr(base + 8u), fr(base + 9u), fr(base + 10u), fr(base + 11u)),
        vec4<f32>(fr(base + 12u), fr(base + 13u), fr(base + 14u), fr(base + 15u)),
    );
}

// ---------------------------------------------------------------- camera (R21.2)

fn project(p: vec3<f32>) -> vec4<f32> {
    let q = m4(F_VIEW) * vec4<f32>(p, 1.0);
    let near = fr(F_NEAR);
    let far = fr(F_FAR);
    if (fr(F_PROJ_KIND) == 0.0) {
        let k = 2.0 * fr(F_FOCAL) / fr(F_SENSOR);
        let ky = k * fr(F_W) / fr(F_H);
        let a = far / (far - near);
        let b = far * near / (far - near);
        // +Y in camera space is up, and NDC +y is the top of the viewport.
        // the TAA subpixel jitter moves the principal point by whole pixels (R22.5)
        return vec4<f32>(
            2.0 * fr(F_FILM_OFF_X) * q.z + k * q.x + 2.0 * fr(F_JITTER_X) * q.z / fr(F_W),
            2.0 * fr(F_FILM_OFF_Y) * q.z + ky * q.y - 2.0 * fr(F_JITTER_Y) * q.z / fr(F_H),
            a * q.z - b,
            q.z,
        );
    }
    return vec4<f32>(
        2.0 * q.x / fr(F_ORTHO_W) + 2.0 * fr(F_JITTER_X) / fr(F_W),
        2.0 * q.y * fr(F_W) / (fr(F_ORTHO_W) * fr(F_H))
            - 2.0 * fr(F_JITTER_Y) / fr(F_H),
        (q.z - near) / (far - near),
        1.0,
    );
}

fn camera_dir(px: vec2<f32>) -> vec3<f32> {
    let cam = m4(F_CAM);
    if (fr(F_PROJ_KIND) == 0.0) {
        let f = fr(F_W) * fr(F_FOCAL) / fr(F_SENSOR);
        let cx = fr(F_W) * 0.5 + fr(F_FILM_OFF_X) * fr(F_W);
        let cy = fr(F_H) * 0.5 - fr(F_FILM_OFF_Y) * fr(F_H);
        let d = vec3<f32>((px.x - cx) / f, -(px.y - cy) / f, 1.0);
        return normalize((cam * vec4<f32>(d, 0.0)).xyz);
    }
    return normalize((cam * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz);
}

// ---------------------------------------------------------------- G-buffer (R21.3)

struct GbufOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) bary: vec3<f32>,
    @location(1) @interpolate(flat) tri: u32,
};

@vertex
fn vs_gbuffer(@builtin(vertex_index) vi: u32) -> GbufOut {
    let t = forge_tris[vi / 3u];
    let c = vi % 3u;
    var w = t.p0.xyz;
    var b = vec3<f32>(1.0, 0.0, 0.0);
    if (c == 1u) {
        w = w + t.e1.xyz;
        b = vec3<f32>(0.0, 1.0, 0.0);
    } else if (c == 2u) {
        w = w + t.e2.xyz;
        b = vec3<f32>(0.0, 0.0, 1.0);
    }
    var out: GbufOut;
    out.pos = project(w);
    out.bary = b;
    out.tri = vi / 3u;
    return out;
}

struct GbufTargets {
    // position, material index + 1
    @location(0) g0: vec4<f32>,
    // shading normal, triangle index + 1
    @location(1) g1: vec4<f32>,
};

@fragment
fn fs_gbuffer(in: GbufOut) -> GbufTargets {
    let t = forge_tris[in.tri];
    let b = in.bary;
    let p = t.p0.xyz + b.y * t.e1.xyz + b.z * t.e2.xyz;
    let view = normalize(m4(F_CAM)[3].xyz - p);
    var ns = normalize(t.ng.xyz);
    if (t.n0.w > 0.5) {
        ns = normalize(b.x * t.n0.xyz + b.y * t.n1.xyz + b.z * t.n2.xyz);
    }
    if (dot(ns, view) < 0.0) {
        ns = -ns;
    }
    var out: GbufTargets;
    out.g0 = vec4<f32>(p, t.ids.x + 1.0);
    out.g1 = vec4<f32>(ns, f32(in.tri) + 1.0);
    return out;
}

// ---------------------------------------------------------------- shadow depth (R21.4)

@vertex
fn vs_shadow(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    let t = forge_tris[vi / 3u];
    var w = t.p0.xyz;
    if (vi % 3u == 1u) {
        w = w + t.e1.xyz;
    } else if (vi % 3u == 2u) {
        w = w + t.e2.xyz;
    }
    return m4(F_SHADOW_MAT) * vec4<f32>(w, 1.0);
}

// ---------------------------------------------------------------- lighting (R21.5)

fn light_falloff(l: GpuLight, d: f32) -> f32 {
    let r = max(l.params.y, 1e-12);
    let k = u32(l.params.x + 0.5);
    if (k == 0u) {
        return 1.0;
    }
    if (k == 1u) {
        return max(1.0 - d / r, 0.0);
    }
    if (k == 2u) {
        return min(r / max(d, 1e-12), 1.0);
    }
    let q = r / max(d, 1e-12);
    return min(q * q, 1.0);
}

fn has_env() -> bool {
    return u32(fr(F_LIGHTING_KIND) + 0.5) != 0u;
}

fn env_radiance(d_in: vec3<f32>) -> vec3<f32> {
    let kind = u32(fr(F_LIGHTING_KIND) + 0.5);
    if (kind == 0u) {
        return vec3<f32>(0.0);
    }
    let intensity = fr(F_LIGHTING_INTENSITY);
    if (kind == 1u) {
        return vec3<f32>(fr(F_CONST), fr(F_CONST + 1u), fr(F_CONST + 2u)) * intensity;
    }
    let d = normalize(d_in);
    if (kind == 2u) {
        let y = clamp(d.y, -1.0, 1.0);
        let hor = vec3<f32>(fr(F_SKY_HOR), fr(F_SKY_HOR + 1u), fr(F_SKY_HOR + 2u));
        if (y >= 0.0) {
            let s = y * y * (3.0 - 2.0 * y);
            let zen = vec3<f32>(fr(F_SKY_ZEN), fr(F_SKY_ZEN + 1u), fr(F_SKY_ZEN + 2u));
            return mix(hor, zen, s) * intensity;
        }
        let s = (-y) * (-y) * (3.0 - 2.0 * (-y));
        let gnd = vec3<f32>(fr(F_SKY_GND), fr(F_SKY_GND + 1u), fr(F_SKY_GND + 2u));
        return mix(hor, gnd, s) * intensity;
    }
    if (fr(F_ENV_PRESENT) < 0.5) {
        return vec3<f32>(0.0);
    }
    // R16: nearest pixel of the lat-long image.
    let phi = atan2(d.x, d.z) - fr(F_ENV_ROT);
    let u = fract(phi / (2.0 * PI_F));
    let v = acos(clamp(d.y, -1.0, 1.0)) / PI_F;
    let tex = textureSampleLevel(forge_env_tex, forge_env_sampler, vec2<f32>(u, v), 0.0);
    return tex.xyz * intensity;
}

fn background(px: vec2<f32>) -> vec3<f32> {
    if (u32(fr(F_BG_KIND) + 0.5) == 0u) {
        return vec3<f32>(fr(F_BG_X), fr(F_BG_Y), fr(F_BG_Z));
    }
    return env_radiance(camera_dir(px));
}

/// The framebuffer pixel a world point projects to (NDC y up, pixel y down, R21.2).
fn pixel_of(p: vec3<f32>) -> vec2<i32> {
    let c = project(p);
    let uv = c.xy / c.w * 0.5 + vec2<f32>(0.5, 0.5);
    return vec2<i32>(floor(vec2<f32>(uv.x * fr(F_W), (1.0 - uv.y) * fr(F_H))));
}

/// Screen-space ambient occlusion at a covered pixel (R22.2): twelve fixed tangent-disc
/// samples, each read back from the G-buffer. Only the environment terms are scaled by it.
fn ssao_at(p: vec3<f32>, n: vec3<f32>) -> f32 {
    if (fr(F_SSAO_ON) < 0.5) {
        return 1.0;
    }
    let radius = fr(F_SSAO_RADIUS);
    let up = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), abs(n.x) > 0.9);
    let t1 = normalize(cross(n, up));
    let t2 = cross(n, t1);
    let eye = m4(F_CAM)[3].xyz;
    var occluded = 0.0;
    for (var k = 0u; k < 12u; k += 1u) {
        let a = 2.0 * PI_F * f32(k) / 12.0;
        let rk = 0.35 + 0.65 * f32(k % 3u) * 0.5;
        // the hemisphere: 0.4 radii above the tangent plane, then the disc offset (R22.2)
        let s = p + n * (0.4 * radius * rk) + radius * rk * (cos(a) * t1 + sin(a) * t2);
        let c = project(s);
        if (c.w <= 0.0) {
            continue;
        }
        let uv = c.xy / c.w * 0.5 + vec2<f32>(0.5, 0.5);
        if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) {
            continue;
        }
        let spx = pixel_of(s);
        let q0 = textureLoad(forge_g0, spx, 0);
        if (q0.w < 0.5) {
            continue;
        }
        // R22.2: occluded when the stored point is nearer the eye than the sample
        let dv = normalize(s - eye);
        if (length(q0.xyz - p) < radius && dot(q0.xyz - s, dv) < -0.02 * radius) {
            occluded += 1.0;
        }
    }
    return clamp(1.0 - fr(F_SSAO_STRENGTH) * occluded / 12.0, 0.0, 1.0);
}

struct Mat {
    albedo: vec3<f32>,
    spec: vec3<f32>,
    alpha: f32,
};

fn ggx_d(m: Mat, nh: f32) -> f32 {
    let a2 = m.alpha * m.alpha;
    let d = nh * nh * (a2 - 1.0) + 1.0;
    return a2 / (PI_F * d * d);
}

fn g1(m: Mat, nv: f32) -> f32 {
    let a2 = m.alpha * m.alpha;
    return 2.0 * nv / (nv + sqrt(a2 + (1.0 - a2) * nv * nv));
}

struct FdFs {
    fd: vec3<f32>,
    fs: vec3<f32>,
};

fn mat_eval(m: Mat, n: vec3<f32>, wo: vec3<f32>, wi: vec3<f32>) -> FdFs {
    var out: FdFs;
    out.fd = vec3<f32>(0.0);
    out.fs = vec3<f32>(0.0);
    let nl = dot(n, wi);
    let nv = dot(n, wo);
    if (nl <= 0.0 || nv <= 0.0) {
        return out;
    }
    out.fd = m.albedo / PI_F;
    let h = normalize(wo + wi);
    let nh = max(dot(n, h), 0.0);
    if (any(m.spec != vec3<f32>(0.0))) {
        out.fs = m.spec * (ggx_d(m, nh) * g1(m, nl) * g1(m, nv) / (4.0 * nl * nv));
    }
    return out;
}

fn lobe_probability(m: Mat) -> f32 {
    let ld = m.albedo.x + m.albedo.y + m.albedo.z;
    let ls = m.spec.x + m.spec.y + m.spec.z;
    if (ld + ls <= 0.0) {
        return 0.0;
    }
    return ls / (ld + ls);
}

fn mat_pdf(m: Mat, n: vec3<f32>, wo: vec3<f32>, wi: vec3<f32>) -> f32 {
    let nl = dot(n, wi);
    let nv = dot(n, wo);
    if (nl <= 0.0 || nv <= 0.0) {
        return 0.0;
    }
    let ps = lobe_probability(m);
    let h = normalize(wo + wi);
    let nh = max(dot(n, h), 0.0);
    let vh = max(dot(wo, h), 0.0);
    if (vh <= 0.0) {
        return 0.0;
    }
    return (1.0 - ps) * nl / PI_F + ps * ggx_d(m, nh) * nh / (4.0 * vh);
}

fn power_heuristic(a: f32, b: f32) -> f32 {
    let a2 = a * a;
    let s = a2 + b * b;
    if (s <= 0.0) {
        return 0.0;
    }
    return a2 / s;
}

/// How much of the light reaches `p` (R21.4): 1 outside a shadow map's frustum.
fn shadow_visibility(si: i32, p: vec3<f32>) -> f32 {
    let sm = forge_shadows[u32(si)];
    var m = sm.m[0];
    if (sm.params.y == 1u) {
        let d = p - forge_lights[sm.params.x].pos_radius.xyz;
        let ax = abs(d.x);
        let ay = abs(d.y);
        let az = abs(d.z);
        var f = 0u;
        if (ay >= ax && ay >= az) {
            f = select(3u, 2u, d.y > 0.0);
        } else if (az >= ax) {
            f = select(5u, 4u, d.z > 0.0);
        } else {
            f = select(1u, 0u, d.x > 0.0);
        }
        m = sm.m[f];
    }
    let c = m * vec4<f32>(p, 1.0);
    let ndc = c.xyz / c.w;
    let uv = vec2<f32>(0.5 + 0.5 * ndc.x, 0.5 - 0.5 * ndc.y);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || ndc.z > 1.0) {
        return 1.0;
    }
    let tile = sm.params.z;
    let base = vec2<f32>(
        f32(tile % u32(fr(F_ATLAS_X))),
        f32(tile / u32(fr(F_ATLAS_X))),
    );
    let atlas = vec2<f32>(fr(F_ATLAS_X), fr(F_ATLAS_Y));
    let px = fr(F_TILE_PX);
    let zref = ndc.z - sm.bias.x;
    if (sm.params.w == 0u) {
        return textureSampleCompareLevel(
            forge_shadow_tex,
            forge_shadow_sampler,
            (base + uv) / atlas,
            zref,
        );
    }
    let uvc = clamp(uv, vec2<f32>(0.5 / px), vec2<f32>(1.0 - 0.5 / px));
    var s = 0.0;
    for (var j = -1; j <= 1; j += 1) {
        for (var i = -1; i <= 1; i += 1) {
            let t = (base + uvc + vec2<f32>(f32(i), f32(j)) / px) / atlas;
            s += textureSampleCompareLevel(forge_shadow_tex, forge_shadow_sampler, t, zref);
        }
    }
    return s / 9.0;
}

fn shadow_index(li: u32) -> i32 {
    let n = u32(fr(F_SHADOW_COUNT) + 0.5);
    for (var j = 0u; j < n; j += 1u) {
        if (forge_shadows[j].params.x == li) {
            return i32(j);
        }
    }
    return -1;
}

fn lum(c: vec3<f32>) -> f32 {
    return 0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z;
}

struct FullOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) vi: u32) -> FullOut {
    var p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    var out: FullOut;
    out.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

/// The IR surface values of a G-buffer pixel (R21.3).
struct SurfaceVals {
    albedo: vec3<f32>,
    spec: vec3<f32>,
    alpha: f32,
    emission: vec3<f32>,
    // the shading normal after bump, facing the camera
    n: vec3<f32>,
};

/// The UVW at a point of triangle `t`: the corner UVWs blended with the weights solved from the
/// point's offset along the edges (R20.1b `uvw_at`).
fn uvw_at(t: Tri, p: vec3<f32>) -> vec3<f32> {
    let e1 = t.e1.xyz;
    let e2 = t.e2.xyz;
    let w = p - t.p0.xyz;
    let d11 = dot(e1, e1);
    let d12 = dot(e1, e2);
    let d22 = dot(e2, e2);
    let w1 = dot(w, e1);
    let w2 = dot(w, e2);
    let det = d11 * d22 - d12 * d12;
    if (abs(det) < 1e-30) {
        return t.u0.xyz;
    }
    let u = (w1 * d22 - w2 * d12) / det;
    let v = (w2 * d11 - w1 * d12) / det;
    return t.u0.xyz * (1.0 - u - v) + t.u1.xyz * u + t.u2.xyz * v;
}

fn bump_height(mat: u32, ctx: ShadeCtx) -> f32 {
    let s = forge_shade(mat, ctx);
    return (s.bump.x + s.bump.y + s.bump.z) / 3.0;
}

/// A right-handed orthonormal basis whose third axis is `n` (the tracer's `basis`).
fn tangent_basis(n: vec3<f32>) -> mat3x3<f32> {
    let s = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let t = vec3<f32>(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let u = vec3<f32>(b, s + n.y * n.y * a, -n.y);
    return mat3x3<f32>(t, u, n);
}

fn surface_vals(g0: vec4<f32>, g1: vec4<f32>) -> SurfaceVals {
    let p = g0.xyz;
    let tri = forge_tris[u32(g1.w - 1.0)];
    let mat = u32(g0.w - 1.0);
    let ng = tri.ng.xyz;
    // the outward-side shading normal of the tracer's `surface_at` (R21.3)
    let flip = dot(g1.xyz, ng) < 0.0;
    let ns = normalize(select(g1.xyz, -g1.xyz, flip));
    var ctx: ShadeCtx;
    ctx.uvw = uvw_at(tri, p);
    ctx.position = p;
    ctx.normal = ns;
    ctx.view = normalize(m4(F_CAM)[3].xyz - p);
    ctx.time = 0.0;
    ctx.vertex_weight = 0.0;
    ctx.ao = 1.0;
    let s = forge_shade(mat, ctx);
    var out: SurfaceVals;
    out.albedo = s.diffuse.xyz * s.diffuse.w;
    out.spec = vec3<f32>(0.0);
    out.alpha = 1.0;
    if (s.specular_flags.x > 0.5) {
        out.spec = s.specular.xyz * s.specular.w;
        let rough = clamp(s.specular_params.x, 0.02, 1.0);
        out.alpha = rough * rough;
        if (s.specular_params.w > 0.5) {
            out.albedo = out.albedo * max(1.0 - s.specular.w, 0.0);
        }
    }
    out.emission = s.emission.xyz;
    var nb = ns;
    if (s.bump_present.x > 0.5) {
        let scale = fr(F_SCALE);
        let eps = 1e-3 * scale;
        let b = tangent_basis(ng);
        let t1 = b[0];
        let t2 = b[1];
        let h0 = bump_height(mat, ctx);
        var c1 = ctx;
        c1.position = p + t1 * eps;
        c1.uvw = uvw_at(tri, c1.position);
        var c2 = ctx;
        c2.position = p + t2 * eps;
        c2.uvw = uvw_at(tri, c2.position);
        let d1 = (bump_height(mat, c1) - h0) / eps;
        let d2 = (bump_height(mat, c2) - h0) / eps;
        let n = ns - (t1 * d1 + t2 * d2) * (s.bump.w * scale);
        if (dot(n, n) > 0.0) {
            nb = normalize(n);
        }
    }
    out.n = select(nb, -nb, flip);
    return out;
}

/// One pixel's light split: the beauty radiance, its diffuse and specular parts, the R3
/// shadow fraction and the coverage.
struct Shaded {
    beauty: vec3<f32>,
    diffuse: vec3<f32>,
    specular: vec3<f32>,
    shadow: f32,
    coverage: f32,
};

fn shade_pixel(px: vec2<i32>, frag: vec2<f32>) -> Shaded {
    var out: Shaded;
    out.beauty = vec3<f32>(0.0);
    out.diffuse = vec3<f32>(0.0);
    out.specular = vec3<f32>(0.0);
    out.shadow = 1.0;
    out.coverage = 0.0;
    let g0 = textureLoad(forge_g0, px, 0);
    if (g0.w < 0.5) {
        out.beauty = background(frag);
        return out;
    }
    let g1 = textureLoad(forge_g1, px, 0);
    let p = g0.xyz;
    let view = normalize(m4(F_CAM)[3].xyz - p);
    let sv = surface_vals(g0, g1);
    let n = sv.n;
    var m: Mat;
    m.albedo = sv.albedo;
    m.spec = sv.spec;
    m.alpha = sv.alpha;
    var diffuse = vec3<f32>(0.0);
    var specular = vec3<f32>(0.0);
    var visible_sum = 0.0;
    var contribution_sum = 0.0;
    let light_count = u32(fr(F_LIGHT_COUNT) + 0.5);
    for (var i = 0u; i < light_count; i += 1u) {
        let l = forge_lights[i];
        if (fr(F_RT_SHADOWS_ON) > 0.5 && l.params.z > 0.5) {
            continue;
        }
        let kind = u32(l.axis_kind.w + 0.5);
        var wi = vec3<f32>(0.0, 0.0, 0.0);
        var weight = vec3<f32>(0.0);
        var pdf = 0.0;
        var ok = false;
        if (kind == 0u || kind == 1u) {
            let v = l.pos_radius.xyz - p;
            let dist = length(v);
            if (dist > 1e-12) {
                wi = v / dist;
                var w = light_falloff(l, dist);
                if (kind == 1u) {
                    let cos_a = clamp(dot(-wi, l.axis_kind.xyz), -1.0, 1.0);
                    let a = acos(cos_a);
                    let hi = l.spot.x;
                    let ho = l.spot.y;
                    if (a >= ho) {
                        w = 0.0;
                    } else if (a > hi) {
                        let t = (ho - a) / (ho - hi);
                        w = w * t * t * (3.0 - 2.0 * t);
                    }
                }
                if (w > 0.0) {
                    ok = true;
                    weight = l.color.xyz * (PI_F * w);
                }
            }
        } else if (kind == 2u) {
            wi = -l.axis_kind.xyz;
            weight = l.color.xyz * PI_F;
            ok = true;
        } else {
            let shape_kind = u32(l.tangent_shape.w + 0.5);
            let q = l.pos_radius.xyz;
            var area = max(l.shape.w, l.shape.x * l.shape.y);
            if (shape_kind == 1u || shape_kind == 2u) {
                area = PI_F * l.shape.z * l.shape.z;
            }
            let v = q - p;
            let d2 = dot(v, v);
            if (d2 > 1e-18 && area > 0.0) {
                let dist = sqrt(d2);
                wi = v / dist;
                var cos_l = dot(-wi, l.axis_kind.xyz);
                if (l.bitangent.w > 0.5 || shape_kind == 2u) {
                    cos_l = abs(cos_l);
                }
                if (cos_l > 0.0) {
                    pdf = d2 / (cos_l * area);
                    weight = l.color.xyz * (light_falloff(l, dist) / pdf);
                    ok = true;
                }
            }
        }
        if (!ok) {
            continue;
        }
        let cos = dot(n, wi);
        if (cos <= 0.0) {
            continue;
        }
        let f = mat_eval(m, n, view, wi);
        var fd = f.fd;
        var fs = f.fs;
        if (l.flags.z < 0.5) {
            fd = vec3<f32>(0.0);
        }
        if (l.flags.w < 0.5) {
            fs = vec3<f32>(0.0);
        }
        if (all(fd == vec3<f32>(0.0)) && all(fs == vec3<f32>(0.0))) {
            continue;
        }
        var wm = 1.0;
        if (pdf > 0.0) {
            wm = power_heuristic(pdf, mat_pdf(m, n, view, wi));
        }
        var vis = 1.0;
        if (l.params.z > 0.5) {
            let si = shadow_index(i);
            if (si >= 0) {
                vis = shadow_visibility(si, p);
            }
        }
        var factor = vec3<f32>(1.0);
        if (vis < 0.999) {
            factor = mix(
                vec3<f32>(1.0),
                vec3<f32>(1.0 - l.params.w) + l.shadow_color.xyz * l.params.w,
                1.0 - vis,
            );
        }
        let un = (fd + fs) * cos * weight * wm;
        contribution_sum += lum(un);
        visible_sum += lum(un * factor);
        diffuse += fd * cos * weight * wm * factor;
        specular += fs * cos * weight * wm * factor;
    }
    if (fr(F_RT_SHADOWS_ON) > 0.5) {
        let d = textureLoad(forge_lighting_src, px, 0, 0);
        let s = textureLoad(forge_lighting_src, px, 1, 0);
        diffuse += d.xyz;
        specular += s.xyz;
        contribution_sum += d.w;
        visible_sum += s.w;
    }
    if (has_env()) {
        // R22.2: occlusion darkens the ambient term only, never direct light.
        var ao = 1.0;
        if (fr(F_RT_AO_ON) > 0.5) {
            let ao_layer = select(0, 2, fr(F_RT_SHADOWS_ON) > 0.5);
            ao = textureLoad(forge_lighting_src, px, ao_layer, 0).x;
        } else {
            ao = ssao_at(p, n);
        }
        diffuse += sv.albedo * env_radiance(n) * ao;
        let r = reflect(-view, n);
        specular += sv.spec * env_radiance(normalize(r)) * ao;
    }
    if (contribution_sum > 0.0) {
        out.shadow = clamp(visible_sum / contribution_sum, 0.0, 1.0);
    }
    out.diffuse = diffuse;
    out.specular = specular;
    out.beauty = sv.emission + diffuse + specular;
    out.coverage = 1.0;
    return out;
}

struct LightOut {
    @location(0) beauty: vec4<f32>,
    @location(1) shadow: vec4<f32>,
};

@fragment
fn fs_light(in: FullOut) -> LightOut {
    let px = vec2<i32>(floor(in.pos.xy));
    let s = shade_pixel(px, in.pos.xy);
    var out: LightOut;
    out.beauty = vec4<f32>(s.beauty, s.coverage);
    out.shadow = vec4<f32>(s.shadow, 0.0, 0.0, s.coverage);
    return out;
}

/// R22.3: screen-space reflections after the lighting pass. Reads the lit beauty, the
/// G-buffer and the environment; writes the beauty with the environment reflection replaced by
/// the first screen-space hit (weighted by roughness), or a copy when there is no hit.
struct SsrOut {
    @location(0) beauty: vec4<f32>,
};

@fragment
fn fs_ssr(in: FullOut) -> SsrOut {
    let px = vec2<i32>(floor(in.pos.xy));
    let base = textureLoad(forge_lighting_src, px, 0, 0);
    var out: SsrOut;
    out.beauty = base;
    let g0 = textureLoad(forge_g0, px, 0);
    if (g0.w < 0.5 || fr(F_SSR_ON) < 0.5) {
        return out;
    }
    let g1 = textureLoad(forge_g1, px, 0);
    let p = g0.xyz;
    let eye = m4(F_CAM)[3].xyz;
    let view = normalize(eye - p);
    let sv = surface_vals(g0, g1);
    let n = sv.n;
    if (all(sv.spec == vec3<f32>(0.0))) {
        return out;
    }
    let rough = sqrt(clamp(sv.alpha, 0.0, 1.0));
    let fade = 1.0 - smoothstep(0.1, 0.6, rough);
    if (fade <= 0.0) {
        return out;
    }
    let r = normalize(reflect(-view, n));
    let steps = u32(fr(F_SSR_STEPS) + 0.5);
    let step0 = 0.05 * fr(F_SCALE);
    var t = step0;
    var hit = false;
    var colour = vec3<f32>(0.0);
    for (var i = 0u; i < steps; i += 1u) {
        if (i > 0u) {
            t = t * 1.35;
        }
        let s = p + r * t;
        let c = project(s);
        if (c.w <= 0.0) {
            break;
        }
        let uv = c.xy / c.w * 0.5 + vec2<f32>(0.5, 0.5);
        if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) {
            break;
        }
        let spx = pixel_of(s);
        let q0 = textureLoad(forge_g0, spx, 0);
        if (q0.w < 0.5) {
            continue;
        }
        if (length(q0.xyz - s) < 0.05 * t + 0.002 * fr(F_SCALE)) {
            colour = textureLoad(forge_lighting_src, spx, 0, 0).xyz;
            hit = true;
            break;
        }
    }
    if (!hit) {
        return out;
    }
    let delta = sv.spec * fade * (colour - env_radiance(r));
    out.beauty = vec4<f32>(base.xyz + delta, base.w);
    return out;
}

struct SplitOut {
    @location(0) diffuse: vec4<f32>,
    @location(1) specular: vec4<f32>,
};

@fragment
fn fs_split(in: FullOut) -> SplitOut {
    let px = vec2<i32>(floor(in.pos.xy));
    let s = shade_pixel(px, in.pos.xy);
    var out: SplitOut;
    out.diffuse = vec4<f32>(s.diffuse, s.coverage);
    out.specular = vec4<f32>(s.specular, s.coverage);
    return out;
}

struct AlbedoOut {
    @location(0) albedo: vec4<f32>,
};

@fragment
fn fs_albedo(in: FullOut) -> AlbedoOut {
    let px = vec2<i32>(floor(in.pos.xy));
    let g0 = textureLoad(forge_g0, px, 0);
    var out: AlbedoOut;
    // the diffuse colour times its level is the split pass's diffuse over the diffuse BRDF
    let g1 = textureLoad(forge_g1, px, 0);
    if (g0.w < 0.5) {
        out.albedo = vec4<f32>(0.0);
        return out;
    }
    let sv = surface_vals(g0, g1);
    out.albedo = vec4<f32>(sv.albedo, 1.0);
    return out;
}
