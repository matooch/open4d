// The R9 integrator in WGSL (docs/specs/RENDER.md R20). Every rule mirrors
// forge_render_cpu::{scene, lights, surface, integrator, engine} in the same order, so the
// two images agree sample for sample up to f32 rounding.

@group(0) @binding(0) var<uniform> forge_frame: array<vec4<f32>, 96>;
@group(0) @binding(4) var<storage, read> forge_lights: array<GpuLight>;
@group(0) @binding(5) var<storage, read_write> forge_out: array<vec4<f32>>;

fn fr(i: u32) -> f32 {
    return forge_frame[i].x;
}

// ---------------------------------------------------------------- camera (R10)

struct Ray {
    o: vec3<f32>,
    d: vec3<f32>,
};

fn cam_col(i: u32) -> vec4<f32> {
    let b = F_CAMERA + i * 4u;
    return vec4<f32>(fr(b), fr(b + 1u), fr(b + 2u), fr(b + 3u));
}

fn cam_point(p: vec3<f32>) -> vec3<f32> {
    return cam_col(0u).xyz * p.x + cam_col(1u).xyz * p.y + cam_col(2u).xyz * p.z + cam_col(3u).xyz;
}

fn cam_dir(v: vec3<f32>) -> vec3<f32> {
    return cam_col(0u).xyz * v.x + cam_col(1u).xyz * v.y + cam_col(2u).xyz * v.z;
}

fn pixel_frame() -> vec3<f32> {
    let w = fr(F_W);
    let f = w * fr(F_FOCAL) / fr(F_SENSOR);
    return vec3<f32>(f, w * 0.5 + fr(F_FILM_OFF_X) * w, fr(F_H) * 0.5 - fr(F_FILM_OFF_Y) * fr(F_H));
}

fn concentric_disc(u: vec2<f32>) -> vec2<f32> {
    let a = 2.0 * u.x - 1.0;
    let b = 2.0 * u.y - 1.0;
    if (a == 0.0 && b == 0.0) {
        return vec2<f32>(0.0, 0.0);
    }
    var r = a;
    var th = 0.7853981633974483 * (b / a);
    if (abs(a) <= abs(b)) {
        r = b;
        th = 1.5707963267948966 - 0.7853981633974483 * (a / b);
    }
    return vec2<f32>(r * cos(th), r * sin(th));
}

fn camera_ray(px: f32, py: f32) -> Ray {
    var r: Ray;
    if (fr(F_PROJ_KIND) == 0.0) {
        let pf = pixel_frame();
        let d = vec3<f32>((px - pf.y) / pf.x, -(py - pf.z) / pf.x, 1.0);
        r.o = cam_point(vec3<f32>(0.0));
        r.d = normalize(cam_dir(d));
    } else {
        let s = fr(F_ORTHO_WIDTH) / fr(F_W);
        let o = vec3<f32>((px - fr(F_W) * 0.5) * s, -(py - fr(F_H) * 0.5) * s, 0.0);
        r.o = cam_point(o);
        r.d = normalize(cam_dir(vec3<f32>(0.0, 0.0, 1.0)));
    }
    return r;
}

fn camera_ray_lens(px: f32, py: f32, lens: vec2<f32>) -> Ray {
    if (fr(F_APERTURE_PRESENT) == 0.0 || fr(F_PROJ_KIND) != 0.0) {
        return camera_ray(px, py);
    }
    let pf = pixel_frame();
    let d = vec3<f32>((px - pf.y) / pf.x, -(py - pf.z) / pf.x, 1.0);
    let focus = d * fr(F_APERTURE_FOCUS);
    let disc = concentric_disc(lens);
    let origin = vec3<f32>(disc.x * fr(F_APERTURE_RADIUS), disc.y * fr(F_APERTURE_RADIUS), 0.0);
    var r: Ray;
    r.o = cam_point(origin);
    r.d = normalize(cam_dir(focus - origin));
    return r;
}

// ---------------------------------------------------------------- geometry (R6)

struct Hit {
    found: bool,
    t: f32,
    tri: u32,
    instance: u32,
    u: f32,
    v: f32,
};

fn empty_hit() -> Hit {
    var h: Hit;
    h.found = false;
    h.t = 1e30;
    h.tri = 0u;
    h.instance = 0xffffffffu;
    h.u = 0.0;
    h.v = 0.0;
    return h;
}

fn uvw_at(g: Geom, p: vec3<f32>) -> vec3<f32> {
    let a = g.corners[0];
    let e1 = g.corners[1] - a;
    let e2 = g.corners[2] - a;
    let w = p - a;
    let d11 = dot(e1, e1);
    let d12 = dot(e1, e2);
    let d22 = dot(e2, e2);
    let w1 = dot(w, e1);
    let w2 = dot(w, e2);
    let det = d11 * d22 - d12 * d12;
    if (abs(det) < 1e-30) {
        return g.uvw;
    }
    let u = (w1 * d22 - w2 * d12) / det;
    let v = (w2 * d11 - w1 * d12) / det;
    return g.corner_uvw[0] * (1.0 - u - v) + g.corner_uvw[1] * u + g.corner_uvw[2] * v;
}

// ---------------------------------------------------------------- lights (R8)

struct LightHit {
    found: bool,
    t: f32,
    le: vec3<f32>,
    pdf: f32,
};

fn light_hit(l: GpuLight, o: vec3<f32>, d: vec3<f32>, tmax: f32) -> LightHit {
    var h: LightHit;
    h.found = false;
    h.t = 0.0;
    h.le = vec3<f32>(0.0);
    h.pdf = 0.0;
    if (u32(l.axis_kind.w + 0.5) != 3u) {
        return h;
    }
    let two_sided = l.bitangent.w > 0.5;
    let shape_kind = u32(l.tangent_shape.w + 0.5);
    if (shape_kind == 2u) {
        let v = l.pos_radius.xyz - o;
        let b = dot(d, v);
        let disc = b * b - (dot(v, v) - l.shape.z * l.shape.z);
        if (disc < 0.0) {
            return h;
        }
        let t = b - sqrt(disc);
        if (t <= 1e-9 || t >= tmax) {
            return h;
        }
        let dist = length(v);
        var pdf = 0.0;
        if (dist > l.shape.z) {
            pdf = 1.0 / (2.0 * PI_F * (1.0 - cos(asin(l.shape.z / dist))));
        }
        h.found = true;
        h.t = t;
        h.le = l.color.xyz * light_falloff(l, t);
        h.pdf = pdf;
        return h;
    }
    let denom = dot(d, l.axis_kind.xyz);
    if (abs(denom) < 1e-12) {
        return h;
    }
    let t = dot(l.pos_radius.xyz - o, l.axis_kind.xyz) / denom;
    if (t <= 1e-9 || t >= tmax) {
        return h;
    }
    var area = PI_F * l.shape.z * l.shape.z;
    if (shape_kind == 0u) {
        let q = o + d * t - l.pos_radius.xyz;
        let tl = dot(l.tangent_shape.xyz, l.tangent_shape.xyz);
        let bl = dot(l.bitangent.xyz, l.bitangent.xyz);
        let u = dot(q, l.tangent_shape.xyz) / tl;
        let v = dot(q, l.bitangent.xyz) / bl;
        if (abs(u) > l.shape.x * 0.5 || abs(v) > l.shape.y * 0.5) {
            return h;
        }
        area = l.shape.x * l.shape.y * l.shape.w;
    } else {
        if (dot(o + d * t - l.pos_radius.xyz, o + d * t - l.pos_radius.xyz) > l.shape.z * l.shape.z) {
            return h;
        }
    }
    var cos_l = dot(-d, l.axis_kind.xyz);
    if (two_sided) {
        cos_l = abs(cos_l);
    }
    if (cos_l <= 0.0) {
        return h;
    }
    h.found = true;
    h.t = t;
    h.le = l.color.xyz * light_falloff(l, t);
    h.pdf = t * t / (cos_l * area);
    return h;
}

fn env_radiance(d: vec3<f32>) -> vec3<f32> {
    let kind = u32(fr(F_LIGHTING_KIND) + 0.5);
    if (kind == 0u) {
        return vec3<f32>(0.0);
    }
    let intensity = fr(F_LIGHTING_INTENSITY);
    if (kind == 1u) {
        return vec3<f32>(fr(F_CONST_X), fr(F_CONST_Y), fr(F_CONST_Z)) * intensity;
    }
    let y = clamp(d.y, -1.0, 1.0);
    let hor = vec3<f32>(fr(F_SKY_HOR_X), fr(F_SKY_HOR_Y), fr(F_SKY_HOR_Z));
    if (y >= 0.0) {
        let s = y * y * (3.0 - 2.0 * y);
        let zen = vec3<f32>(fr(F_SKY_ZEN_X), fr(F_SKY_ZEN_Y), fr(F_SKY_ZEN_Z));
        return mix(hor, zen, s) * intensity;
    }
    let s = (-y) * (-y) * (3.0 - 2.0 * (-y));
    let gnd = vec3<f32>(fr(F_SKY_GND_X), fr(F_SKY_GND_Y), fr(F_SKY_GND_Z));
    return mix(hor, gnd, s) * intensity;
}

fn has_env() -> bool {
    return u32(fr(F_LIGHTING_KIND) + 0.5) != 0u;
}

fn sample_env(rng: ptr<function, Rng>) -> LightSample {
    var s = no_light_sample();
    let wi = uniform_sphere(rng_f2(rng));
    s.ok = true;
    s.wi = wi;
    s.dist = 1e30;
    s.weight = env_radiance(wi) / INV_4PI_F;
    s.pdf = INV_4PI_F;
    return s;
}

// ---------------------------------------------------------------- surfaces (R7)

struct Mat {
    albedo: vec3<f32>,
    spec: vec3<f32>,
    alpha: f32,
    emission: vec3<f32>,
    trans: vec3<f32>,
    ior: f32,
    n: vec3<f32>,
};

fn bump_height(mat: u32, ctx: ShadeCtx) -> f32 {
    let s = forge_shade(mat, ctx);
    return (s.bump.x + s.bump.y + s.bump.z) / 3.0;
}

fn surface_at(mat: u32, g: Geom, wo: vec3<f32>) -> Mat {
    var ctx: ShadeCtx;
    ctx.uvw = g.uvw;
    ctx.position = g.p;
    ctx.normal = g.ns;
    ctx.view = wo;
    ctx.time = 0.0;
    ctx.vertex_weight = 0.0;
    ctx.ao = 1.0;
    let s = forge_shade(mat, ctx);
    var m: Mat;
    m.albedo = s.diffuse.xyz * s.diffuse.w;
    m.spec = vec3<f32>(0.0);
    m.alpha = 0.0004;
    if (s.specular_flags.x > 0.5) {
        m.spec = s.specular.xyz * s.specular.w;
        m.alpha = clamp(s.specular_params.x, 0.02, 1.0) * clamp(s.specular_params.x, 0.02, 1.0);
        if (s.specular_params.w > 0.5) {
            m.albedo = m.albedo * max(1.0 - s.specular.w, 0.0);
        }
    }
    m.emission = s.emission.xyz;
    m.trans = vec3<f32>(0.0);
    m.ior = 1.0;
    if (s.transmission_present.x > 0.5) {
        m.trans = clamp(s.transmission.xyz, vec3<f32>(0.0), vec3<f32>(1.0));
        m.ior = max(s.transmission.w, 1e-3);
    }
    m.n = g.ns;
    if (s.bump_present.x > 0.5) {
        let scale = fr(F_SCALE);
        let eps = 1e-3 * scale;
        let b = basis(g.ng);
        let t1 = b[0];
        let t2 = b[1];
        let h0 = bump_height(mat, ctx);
        var c1 = ctx;
        c1.position = g.p + t1 * eps;
        c1.uvw = uvw_at(g, c1.position);
        var c2 = ctx;
        c2.position = g.p + t2 * eps;
        c2.uvw = uvw_at(g, c2.position);
        let d1 = (bump_height(mat, c1) - h0) / eps;
        let d2 = (bump_height(mat, c2) - h0) / eps;
        let n = g.ns - (t1 * d1 + t2 * d2) * (s.bump.w * scale);
        if (dot(n, n) > 0.0) {
            m.n = normalize(n);
        }
    }
    return m;
}

fn opaque_part(m: Mat) -> vec3<f32> {
    return vec3<f32>(1.0) - m.trans;
}

fn lobe_probability(m: Mat) -> f32 {
    let ld = m.albedo.x + m.albedo.y + m.albedo.z;
    let ls = m.spec.x + m.spec.y + m.spec.z;
    if (ld + ls <= 0.0) {
        return 0.0;
    }
    return ls / (ld + ls);
}

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
}

fn mat_eval(m: Mat, n: vec3<f32>, wo: vec3<f32>, wi: vec3<f32>) -> FdFs {
    // (diffuse, specular) without the cosine, or zeros when either is behind the surface.
    var out: FdFs;
    out.fd = vec3<f32>(0.0);
    out.fs = vec3<f32>(0.0);
    let nl = dot(n, wi);
    let nv = dot(n, wo);
    if (nl <= 0.0 || nv <= 0.0) {
        return out;
    }
    let op = opaque_part(m);
    let fd = m.albedo * op / PI_F;
    let h = normalize(wo + wi);
    let nh = max(dot(n, h), 0.0);
    out.fd = fd;
    if (any(m.spec != vec3<f32>(0.0))) {
        out.fs = m.spec * op * (ggx_d(m, nh) * g1(m, nl) * g1(m, nv) / (4.0 * nl * nv));
    }
    return out;
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
    let vh = max(dot(wo, h), 1e-9);
    return (1.0 - ps) * nl / PI_F + ps * ggx_d(m, nh) * nh / (4.0 * vh);
}

struct BsdfSample {
    ok: bool,
    wi: vec3<f32>,
    weight: vec3<f32>,
    pdf: f32,
    lobe: u32,
};

fn mat_sample_opaque(m: Mat, n: vec3<f32>, wo: vec3<f32>, rng: ptr<function, Rng>) -> BsdfSample {
    var s: BsdfSample;
    s.ok = false;
    s.wi = vec3<f32>(0.0);
    s.weight = vec3<f32>(0.0);
    s.pdf = 0.0;
    s.lobe = 0u;
    let ps = lobe_probability(m);
    var wi = vec3<f32>(0.0);
    if (rng_f(rng) < ps) {
        s.lobe = 1u;
        let u = rng_f2(rng);
        let a2 = m.alpha * m.alpha;
        let cos_t = sqrt((1.0 - u.y) / (1.0 + (a2 - 1.0) * u.y));
        let sin_t = sqrt(max(1.0 - cos_t * cos_t, 0.0));
        let phi = 2.0 * PI_F * u.x;
        let h = to_world(n, vec3<f32>(sin_t * cos(phi), sin_t * sin(phi), cos_t));
        wi = normalize(2.0 * dot(wo, h) * h - wo);
    } else {
        wi = to_world(n, cosine_hemisphere(rng_f2(rng)));
    }
    let nl = dot(n, wi);
    if (nl <= 0.0 || dot(wi, wi) <= 0.0) {
        return s;
    }
    let pdf = mat_pdf(m, n, wo, wi);
    if (pdf <= 0.0) {
        return s;
    }
    let f = mat_eval(m, n, wo, wi);
    s.ok = true;
    s.wi = wi;
    s.weight = (f.fd + f.fs) * nl / pdf;
    s.pdf = pdf;
    return s;
}

fn fresnel_dielectric(cos_i: f32, eta: f32) -> f32 {
    let sin2_t = (1.0 - cos_i * cos_i) / (eta * eta);
    if (sin2_t >= 1.0) {
        return 1.0;
    }
    let cos_t = sqrt(1.0 - sin2_t);
    let rs = (cos_i - eta * cos_t) / (cos_i + eta * cos_t);
    let rp = (eta * cos_i - cos_t) / (eta * cos_i + cos_t);
    return 0.5 * (rs * rs + rp * rp);
}

fn refract(wo: vec3<f32>, n: vec3<f32>, eta: f32) -> vec3<f32> {
    // Zero when the direction is inside the critical angle; the caller checks the length.
    let c = dot(n, wo);
    let k = 1.0 - eta * eta * (1.0 - c * c);
    if (k < 0.0) {
        return vec3<f32>(0.0);
    }
    return -wo * eta + n * (eta * c - sqrt(k));
}

// ---------------------------------------------------------------- integrator (R9)

struct Acc {
    rgb: vec3<f32>,
    diffuse: vec3<f32>,
    specular: vec3<f32>,
    emission: vec3<f32>,
};

// Takes the whole `TraceOut`, not a pointer to its `acc` field: the SPIR-V writer of naga 30
// panics ("Expression is not cached") on a pointer to a sub-field passed as an argument.
fn acc_add(t: ptr<function, TraceOut>, c: vec3<f32>, depth: u32, lobe_target: u32, clamp: f32) {
    var cc = c;
    if (clamp > 0.0 && depth > 0u) {
        let m = max(max(cc.x, cc.y), cc.z);
        if (m > clamp) {
            cc = cc * (clamp / m);
        }
    }
    (*t).acc.rgb = (*t).acc.rgb + cc;
    if (lobe_target == 0u) {
        (*t).acc.emission = (*t).acc.emission + cc;
    } else if (lobe_target == 1u) {
        (*t).acc.diffuse = (*t).acc.diffuse + cc;
    } else {
        (*t).acc.specular = (*t).acc.specular + cc;
    }
}

struct Sample {
    rgb: vec3<f32>,
    covered: bool,
    depth: f32,
    normal: vec3<f32>,
    albedo: vec3<f32>,
    position: vec3<f32>,
    object_id: f32,
    material_id: f32,
    shadow_total: f32,
    shadow_visible: f32,
};

struct TraceOut {
    sample: Sample,
    acc: Acc,
};

const TARGET_EMISSION: u32 = 0u;
const TARGET_DIFFUSE: u32 = 1u;
const TARGET_SPECULAR: u32 = 2u;

fn trace_path(o_in: vec3<f32>, d_in: vec3<f32>, rng: ptr<function, Rng>) -> TraceOut {
    var out: TraceOut;
    var o = o_in;
    var d = d_in;
    let clamp_value = fr(F_CLAMP);
    let eps = fr(F_EPS);
    let max_depth = max(u32(fr(F_MAX_DEPTH) + 0.5), 1u);
    var throughput = vec3<f32>(1.0);
    var prev_pdf = 0.0;
    var prev_delta = true;
    var cat = LOBE_NONE; // 0 = diffuse, 1 = specular, 2 = none
    let lights = arrayLength(&forge_lights);

    for (var depth = 0u; depth < max_depth; depth = depth + 1u) {
        let hit = scene_intersect(o, d, 1e30);
        var tmax = 1e30;
        if (hit.found) {
            tmax = hit.t;
        }
        var lh = light_hit_empty();
        for (var i = 0u; i < lights; i = i + 1u) {
            let l = forge_lights[i];
            if (u32(l.axis_kind.w + 0.5) != 3u) {
                continue;
            }
            if (depth == 0u && l.flags.y < 0.5) {
                continue;
            }
            let lim = select(tmax, lh.t, lh.found);
            let x = light_hit(l, o, d, lim);
            if (x.found) {
                lh = x;
            }
        }
        if (lh.found) {
            var w = 1.0;
            if (!prev_delta) {
                w = power_heuristic(prev_pdf, lh.pdf);
            }
            acc_add(&out, throughput * lh.le * w, depth, target_of(cat), clamp_value);
            if (depth == 0u) {
                out.sample.covered = true;
            }
            break;
        }
        if (!hit.found) {
            if (depth == 0u) {
                if (fr(F_BG_KIND) == 0.0) {
                    out.sample.rgb = out.sample.rgb + vec3<f32>(fr(F_BG_X), fr(F_BG_Y), fr(F_BG_Z));
                } else {
                    out.sample.rgb = out.sample.rgb + env_radiance(d);
                }
            } else if (has_env()) {
                let le = env_radiance(d);
                var w = 1.0;
                if (!prev_delta) {
                    w = power_heuristic(prev_pdf, INV_4PI_F);
                }
                acc_add(&out, throughput * le * w, depth, target_of(cat), clamp_value);
            }
            break;
        }
        let g = scene_geometry(o, d, hit);
        let ids = scene_ids(hit);
        let mat_ix = u32(ids.x);
        let object_id = ids.y;
        let wo = -d;
        let front = dot(g.ng, wo) > 0.0;
        let ngv = select(-g.ng, g.ng, front);
        let surf = surface_at(mat_ix, g, wo);
        let nv = select(-surf.n, surf.n, front);
        var n = nv;
        if (dot(nv, ngv) < 0.0) {
            n = ngv;
        }
        if (depth == 0u) {
            out.sample.covered = true;
            out.sample.depth = hit.t;
            out.sample.normal = n;
            out.sample.albedo = surf.albedo;
            out.sample.position = g.p;
            out.sample.object_id = object_id + 1.0;
            out.sample.material_id = f32(mat_ix) + 1.0;
        }
        if (any(surf.emission != vec3<f32>(0.0))) {
            acc_add(&out, throughput * surf.emission, depth, target_of(cat), clamp_value);
        }
        let origin = g.p + ngv * eps;

        // Next-event estimation.
        for (var i = 0u; i < lights; i = i + 1u) {
            let l = forge_lights[i];
            let ls = light_sample(l, g.p, rng);
            if (!ls.ok) {
                continue;
            }
            nee(&out, surf, n, ngv, wo, origin, ls, l.flags.z > 0.5, l.flags.w > 0.5, l.params.z > 0.5, l.params.w, l.shadow_color.xyz, throughput, depth, cat, clamp_value);
        }
        if (has_env()) {
            let ls = sample_env(rng);
            nee(&out, surf, n, ngv, wo, origin, ls, true, true, true, 1.0, vec3<f32>(0.0), throughput, depth, cat, clamp_value);
        }

        if (depth + 1u >= max_depth) {
            break;
        }
        // Scatter.
        var p_t = 0.0;
        if (surf.trans.x + surf.trans.y + surf.trans.z > 0.0) {
            p_t = clamp(lum(surf.trans), 0.0, 0.95);
        }
        if (p_t > 0.0 && rng_f(rng) < p_t) {
            throughput = throughput * (surf.trans / max(p_t, 1e-9));
            if (g.closed) {
                var eta = 1.0;
                if (front) {
                    eta = surf.ior;
                }
                let cos_i = clamp(dot(ngv, wo), 0.0, 1.0);
                let f = fresnel_dielectric(cos_i, eta);
                if (rng_f(rng) < f) {
                    d = normalize(2.0 * dot(ngv, wo) * ngv - wo);
                    o = g.p + ngv * eps;
                } else {
                    var n1 = 1.0;
                    var n2 = surf.ior;
                    if (!front) {
                        n1 = surf.ior;
                        n2 = 1.0;
                    }
                    let t = refract(wo, ngv, n1 / n2);
                    if (dot(t, t) <= 0.0) {
                        d = normalize(2.0 * dot(ngv, wo) * ngv - wo);
                        o = g.p + ngv * eps;
                    } else {
                        d = normalize(t);
                        o = g.p - ngv * eps;
                    }
                }
            } else {
                o = g.p - ngv * eps;
            }
            prev_delta = true;
            if (cat == LOBE_NONE) {
                cat = 1u;
            }
        } else {
            let bs = mat_sample_opaque(surf, n, wo, rng);
            if (!bs.ok || dot(ngv, bs.wi) <= 0.0) {
                break;
            }
            throughput = throughput * bs.weight;
            if (p_t > 0.0) {
                throughput = throughput * (opaque_part(surf) / max(1.0 - p_t, 1e-9));
            }
            d = bs.wi;
            o = origin;
            prev_pdf = bs.pdf;
            prev_delta = false;
            if (cat == LOBE_NONE) {
                cat = bs.lobe;
            }
        }
        if (depth >= 3u) {
            let q = clamp(max(max(throughput.x, throughput.y), throughput.z), 0.05, 1.0);
            if (rng_f(rng) > q) {
                break;
            }
            throughput = throughput / q;
        }
        if (!all(throughput == throughput)) {
            break;
        }
    }
    return out;
}

fn light_hit_empty() -> LightHit {
    var h: LightHit;
    h.found = false;
    h.t = 0.0;
    h.le = vec3<f32>(0.0);
    h.pdf = 0.0;
    return h;
}

// `cat` is a lobe: 0 diffuse, 1 specular, 2 none (the CPU's Option<Lobe>).
const LOBE_NONE: u32 = 2u;

fn target_of(lobe: u32) -> u32 {
    if (lobe == 0u) {
        return TARGET_DIFFUSE;
    }
    if (lobe == 1u) {
        return TARGET_SPECULAR;
    }
    return TARGET_EMISSION;
}

fn nee(
    out: ptr<function, TraceOut>,
    surf: Mat,
    n: vec3<f32>,
    ngv: vec3<f32>,
    wo: vec3<f32>,
    origin: vec3<f32>,
    ls: LightSample,
    diffuse_on: bool,
    specular_on: bool,
    shadow_cast: bool,
    shadow_density: f32,
    shadow_color: vec3<f32>,
    throughput: vec3<f32>,
    depth: u32,
    cat: u32,
    clamp_value: f32,
) {
    let cos = dot(n, ls.wi);
    if (cos <= 0.0 || dot(ngv, ls.wi) <= 0.0) {
        return;
    }
    let f = mat_eval(surf, n, wo, ls.wi);
    var fd = f.fd;
    var fs = f.fs;
    if (!diffuse_on) {
        fd = vec3<f32>(0.0);
    }
    if (!specular_on) {
        fs = vec3<f32>(0.0);
    }
    if (all(fd == vec3<f32>(0.0)) && all(fs == vec3<f32>(0.0))) {
        return;
    }
    var w = 1.0;
    if (ls.pdf > 0.0) {
        w = power_heuristic(ls.pdf, mat_pdf(surf, n, wo, ls.wi));
    }
    var visible = true;
    if (shadow_cast && ls.dist < 1e29) {
        visible = !scene_occluded(origin, ls.wi, max(ls.dist * (1.0 - 1e-6) - fr(F_EPS), 0.0));
    }
    var factor = vec3<f32>(1.0);
    if (!visible) {
        factor = vec3<f32>(1.0 - shadow_density) + shadow_color * shadow_density;
    }
    let un = (fd + fs) * cos * ls.weight;
    if (depth == 0u) {
        (*out).sample.shadow_total += lum(un);
        (*out).sample.shadow_visible += lum(un * factor);
    }
    let cd = fd * cos * ls.weight * w * factor;
    let cs = fs * cos * ls.weight * w * factor;
    var td = TARGET_DIFFUSE;
    var ts = TARGET_SPECULAR;
    if (cat != LOBE_NONE) {
        td = target_of(cat);
        ts = target_of(cat);
    }
    acc_add(out, throughput * cd, depth, td, clamp_value);
    acc_add(out, throughput * cs, depth, ts, clamp_value);
}

// ---------------------------------------------------------------- pixel (R2, R4)

fn tri_warp(x: f32) -> f32 {
    if (x < 0.5) {
        return sqrt(2.0 * x) - 1.0;
    }
    return 1.0 - sqrt(2.0 * (1.0 - x));
}

fn filter_offset(f: u32, wd: f32, u: vec2<f32>, rng: ptr<function, Rng>) -> vec2<f32> {
    if (f == 0u) {
        return vec2<f32>((u.x - 0.5) * wd, (u.y - 0.5) * wd);
    }
    if (f == 1u) {
        return vec2<f32>(tri_warp(u.x) * wd * 0.5, tri_warp(u.y) * wd * 0.5);
    }
    if (f == 2u) {
        let r = sqrt(-2.0 * log(max(u.x, 1e-12))) * (wd * 0.25);
        let a = TAU_F * u.y;
        return vec2<f32>(
            clamp(r * cos(a), -wd * 0.5, wd * 0.5),
            clamp(r * sin(a), -wd * 0.5, wd * 0.5),
        );
    }
    let ab = rng_f2(rng);
    let cd = rng_f2(rng);
    return vec2<f32>(
        (u.x + ab.x + ab.y + cd.x - 2.0) * wd * 0.25,
        (u.y + cd.y + rng_f(rng) + rng_f(rng) - 2.0) * wd * 0.25,
    );
}

fn aov_tag(k: u32) -> u32 {
    return bitcast<u32>(fr(F_AOV_TAG + k));
}

// Storage stride: 160 bytes. No bools (not host-shareable) or averaged outputs.
struct PixelState {
    acc: Acc,
    normal_sum: vec3<f32>,
    depth_sum: f32,
    albedo_sum: vec3<f32>,
    vis: f32,
    pos_sum: vec3<f32>,
    tot: f32,
    rng: Rng,
    sxy: vec2<f32>,
    first_ids: vec2<f32>,
    hits: u32,
    cover: u32,
};

@group(0) @binding(6) var<storage, read_write> pixel_states: array<PixelState>;

fn new_pixel(x: u32, y: u32) -> PixelState {
    var p: PixelState;
    let seed_w = vec2<u32>(bitcast<u32>(fr(F_SEED_LO)), bitcast<u32>(fr(F_SEED_HI)));
    let pix = splitmix(u64_xor(seed_w, splitmix(vec2<u32>(y, x))));
    var rng = rng_new(pix, u64_shr(pix, 17u));
    p.sxy = rng_f2(&rng);
    p.rng = rng;
    return p;
}

fn advance_pixel(previous: PixelState, x: u32, y: u32, start: u32, end: u32) -> PixelState {
    var p = previous;
    var rng = p.rng;
    let dof = fr(F_APERTURE_PRESENT) > 0.5;
    for (var i = start; i < end; i = i + 1u) {
        let u = vec2<f32>(
            fract(p.sxy.x + 0.5 + f32(i) * 0.7548776662466927),
            fract(p.sxy.y + 0.5 + f32(i) * 0.5698402909980532),
        );
        let off = filter_offset(u32(fr(F_FILTER) + 0.5), fr(F_FILTER_WIDTH), u, &rng);
        let px = f32(x) + 0.5 + off.x;
        let py = f32(y) + 0.5 + off.y;
        var ray: Ray;
        if (dof) {
            ray = camera_ray_lens(px, py, rng_f2(&rng));
        } else {
            ray = camera_ray(px, py);
        }
        let t = trace_path(ray.o, ray.d, &rng);
        let s = t.sample;
        if (!(all(t.acc.rgb == t.acc.rgb))) {
            continue;
        }
        p.acc.rgb = p.acc.rgb + t.acc.rgb;
        p.acc.diffuse = p.acc.diffuse + t.acc.diffuse;
        p.acc.specular = p.acc.specular + t.acc.specular;
        p.acc.emission = p.acc.emission + t.acc.emission;
        p.tot = p.tot + s.shadow_total;
        p.vis = p.vis + s.shadow_visible;
        if (s.covered) {
            p.cover = p.cover + 1u;
        }
        if (s.covered && s.depth > 0.0) {
            if (p.hits == 0u) {
                p.first_ids = vec2<f32>(s.object_id, s.material_id);
            }
            p.hits = p.hits + 1u;
            p.depth_sum = p.depth_sum + s.depth;
            p.normal_sum = p.normal_sum + s.normal;
            p.albedo_sum = p.albedo_sum + s.albedo;
            p.pos_sum = p.pos_sum + s.position;
        }
    }
    p.rng = rng;
    return p;
}

fn resolve_pixel(p: PixelState, base: u32, samples: u32) {
    let inv = 1.0 / f32(samples);
    let alpha = f32(p.cover) * inv;
    forge_out[base] = vec4<f32>(p.acc.rgb * inv, alpha);
    let hinv = select(0.0, 1.0 / f32(p.hits), p.hits > 0u);
    let aovs = u32(fr(F_AOV_COUNT) + 0.5);
    for (var k = 0u; k < aovs; k = k + 1u) {
        let tag = aov_tag(k);
        var v = vec3<f32>(0.0);
        if (tag == 0u) {
            v = vec3<f32>(p.depth_sum * hinv);
        } else if (tag == 1u) {
            v = p.normal_sum * hinv;
            if (dot(v, v) > 0.0) {
                v = normalize(v);
            }
        } else if (tag == 2u) {
            v = p.albedo_sum * hinv;
        } else if (tag == 3u) {
            v = p.pos_sum * hinv;
        } else if (tag == 4u) {
            v = vec3<f32>(p.first_ids.x);
        } else if (tag == 5u) {
            v = vec3<f32>(p.first_ids.y);
        } else if (tag == 6u) {
            v = vec3<f32>(select(1.0, p.vis / p.tot, p.tot > 0.0));
        } else if (tag == 7u) {
            v = p.acc.diffuse * inv;
        } else if (tag == 8u) {
            v = p.acc.specular * inv;
        } else {
            v = p.acc.emission * inv;
        }
        forge_out[base + 1u + k] = vec4<f32>(v, alpha);
    }
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let tw = u32(fr(F_TILE_W) + 0.5);
    let th = u32(fr(F_TILE_H) + 0.5);
    if (gid.x >= tw || gid.y >= th) {
        return;
    }
    let x = u32(fr(F_TX) + 0.5) + gid.x;
    let y = u32(fr(F_TY) + 0.5) + gid.y;
    let pixel = gid.y * tw + gid.x;
    let start = bitcast<u32>(fr(F_SAMPLE_START));
    let end = bitcast<u32>(fr(F_SAMPLE_END));
    var p: PixelState;
    if (start == 0u) {
        p = new_pixel(x, y);
    } else {
        p = pixel_states[pixel];
    }
    p = advance_pixel(p, x, y, start, end);
    let more = end < bitcast<u32>(fr(F_SAMPLE_TOTAL));
    let preview = fr(F_PREVIEW) != 0.0;
    if (more || preview) {
        pixel_states[pixel] = p;
    }
    if (!more) {
        let stride = u32(fr(F_STRIDE) + 0.5);
        let base = pixel * stride;
        resolve_pixel(p, base, end);
        if (preview) {
            let ray = camera_ray(f32(x) + 0.5, f32(y) + 0.5);
            let hit = scene_intersect(ray.o, ray.d, 1e30);
            var guide = vec4<f32>(0.0, 0.0, 0.0, -1.0);
            if (hit.found && hit.t > 0.0) {
                let g = scene_geometry(ray.o, ray.d, hit);
                guide = vec4<f32>(g.ns, hit.t);
            }
            forge_out[base + stride - 1u] = guide;
        }
    }
}
