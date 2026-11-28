// R7 material evaluation and opaque BSDF shared by path transport and direct shadows.
// Uses the compiled material dispatcher, the shared geometry and the RNG math helpers.

struct Mat {
    albedo: vec3<f32>,
    spec: vec3<f32>,
    alpha: f32,
    emission: vec3<f32>,
    trans: vec3<f32>,
    ior: f32,
    scattering_distance: f32,
    n: vec3<f32>,
};

fn bump_height(mat: u32, ctx: ShadeCtx) -> f32 {
    let s = forge_shade(mat, ctx);
    return (s.bump.x + s.bump.y + s.bump.z) / 3.0;
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

fn surface_at(mat: u32, g: Geom, wo: vec3<f32>, scale: f32) -> Mat {
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
    m.scattering_distance = s.diffusion_present.y;
    m.n = g.ns;
    if (s.bump_present.x > 0.5) {
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
    m.n = forge_tangent_normal(s, m.n, g.corners[1]-g.corners[0], g.corners[2]-g.corners[0],
        (g.corner_uvw[1]-g.corner_uvw[0]).xy, (g.corner_uvw[2]-g.corner_uvw[0]).xy);
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
