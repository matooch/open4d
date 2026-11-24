//! The WGSL noise library (docs/specs/REALTIME.md §2.2): `forge-noise` on `f32` with the
//! identical SplitMix64 hash on `vec2<u32>` (x = low word, y = high word).

/// 64-bit helpers shared by noise and the tile hash.
pub const U64_WGSL: &str = r#"
fn forge_mul32(a: u32, b: u32) -> vec2<u32> {
    let a0 = a & 0xffffu;
    let a1 = a >> 16u;
    let b0 = b & 0xffffu;
    let b1 = b >> 16u;
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = p01 + p10;
    let mid_carry = select(0u, 1u, mid < p01);
    let lo = p00 + (mid << 16u);
    let lo_carry = select(0u, 1u, lo < p00);
    let hi = p11 + (mid >> 16u) + (mid_carry << 16u) + lo_carry;
    return vec2<u32>(lo, hi);
}
fn forge_u64_mul(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let p = forge_mul32(a.x, b.x);
    return vec2<u32>(p.x, p.y + a.x * b.y + a.y * b.x);
}
fn forge_u64_add(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let lo = a.x + b.x;
    return vec2<u32>(lo, a.y + b.y + select(0u, 1u, lo < a.x));
}
// shift right by 0 < n < 32
fn forge_u64_shr(a: vec2<u32>, n: u32) -> vec2<u32> {
    return vec2<u32>((a.x >> n) | (a.y << (32u - n)), a.y >> n);
}
// shift right by 32 <= n < 64
fn forge_u64_shr_hi(a: vec2<u32>, n: u32) -> vec2<u32> {
    return vec2<u32>(a.y >> (n - 32u), 0u);
}
// SplitMix64 finalizer
fn forge_mix64(z0: vec2<u32>) -> vec2<u32> {
    var z = z0;
    z = forge_u64_mul(z ^ forge_u64_shr(z, 30u), vec2<u32>(0x1ce4e5b9u, 0xbf58476du));
    z = forge_u64_mul(z ^ forge_u64_shr(z, 27u), vec2<u32>(0x133111ebu, 0x94d049bbu));
    return z ^ forge_u64_shr(z, 31u);
}
fn forge_sext(c: i32) -> vec2<u32> {
    return vec2<u32>(bitcast<u32>(c), select(0u, 0xffffffffu, c < 0));
}
"#;

/// Noise kinds, fractals, warp (always the 4D variants the shaders sample).
pub const NOISE_WGSL: &str = r#"
fn forge_hash4(seed: u32, c: vec4<i32>) -> vec2<u32> {
    var h = forge_mix64(forge_u64_mul(vec2<u32>(seed, 0u), vec2<u32>(0x7f4a7c15u, 0x9e3779b9u))
        ^ vec2<u32>(0x9abcdef0u, 0x12345678u));
    h = forge_mix64(h ^ forge_sext(c.x));
    h = forge_mix64(h ^ forge_sext(c.y));
    h = forge_mix64(h ^ forge_sext(c.z));
    h = forge_mix64(h ^ forge_sext(c.w));
    return h;
}
// the hash as a value in [0, 1): top 24 bits plus the next 24
fn forge_unit(h: vec2<u32>) -> f32 {
    return f32(h.y >> 8u) * (1.0 / 16777216.0)
        + f32(((h.y & 255u) << 16u) | (h.x >> 16u)) * (1.0 / 281474976710656.0);
}
fn forge_octave_seed(seed: u32, i: u32) -> u32 {
    return forge_mix64(vec2<u32>(seed ^ 0x51ed270bu, i)).y;
}
fn forge_fade(t: f32) -> f32 {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}
fn forge_lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + (b - a) * t;
}
fn forge_grad4(h: vec2<u32>, d: vec4<f32>) -> f32 {
    let g = (h.x >> 8u) & 31u;
    let axis = g >> 3u;
    var bits = g & 7u;
    var s = 0.0;
    for (var i = 0u; i < 4u; i++) {
        if (i == axis) { continue; }
        s += select(d[i], -d[i], (bits & 1u) == 1u);
        bits = bits >> 1u;
    }
    return s;
}
fn forge_perlin4(seed: u32, p: vec4<f32>) -> f32 {
    let fl = floor(p);
    let c = vec4<i32>(fl);
    let f = p - fl;
    let w = vec4<f32>(forge_fade(f.x), forge_fade(f.y), forge_fade(f.z), forge_fade(f.w));
    var vals: array<f32, 16>;
    for (var m = 0u; m < 16u; m++) {
        let bit = vec4<u32>(m & 1u, (m >> 1u) & 1u, (m >> 2u) & 1u, (m >> 3u) & 1u);
        vals[m] = forge_grad4(forge_hash4(seed, c + vec4<i32>(bit)), f - vec4<f32>(bit));
    }
    var n = 16u;
    for (var k = 0u; k < 4u; k++) {
        n = n >> 1u;
        for (var j = 0u; j < n; j++) {
            vals[j] = forge_lerp(vals[2u * j], vals[2u * j + 1u], w[k]);
        }
    }
    return vals[0] * 0.95;
}
fn forge_value4(seed: u32, p: vec4<f32>) -> f32 {
    let fl = floor(p);
    let c = vec4<i32>(fl);
    let f = p - fl;
    var vals: array<f32, 16>;
    for (var m = 0u; m < 16u; m++) {
        let bit = vec4<u32>(m & 1u, (m >> 1u) & 1u, (m >> 2u) & 1u, (m >> 3u) & 1u);
        vals[m] = forge_unit(forge_hash4(seed, c + vec4<i32>(bit))) * 2.0 - 1.0;
    }
    var n = 16u;
    for (var k = 0u; k < 4u; k++) {
        n = n >> 1u;
        let t = forge_fade(f[k]);
        for (var j = 0u; j < n; j++) {
            vals[j] = forge_lerp(vals[2u * j], vals[2u * j + 1u], t);
        }
    }
    return vals[0];
}
fn forge_simplex4(seed: u32, p: vec4<f32>) -> f32 {
    let skew = SIMPLEX_SKEW;
    let unskew = SIMPLEX_UNSKEW;
    let s = (p.x + p.y + p.z + p.w) * skew;
    let cellf = floor(p + vec4<f32>(s));
    var cur = vec4<i32>(cellf);
    let t = (cellf.x + cellf.y + cellf.z + cellf.w) * unskew;
    let d0 = p - (cellf - vec4<f32>(t));
    var order = array<u32, 4>(0u, 1u, 2u, 3u);
    for (var i = 1u; i < 4u; i++) {
        var j = i;
        while (j > 0u && d0[order[j - 1u]] < d0[order[j]]) {
            let tmp = order[j - 1u];
            order[j - 1u] = order[j];
            order[j] = tmp;
            j = j - 1u;
        }
    }
    var total = 0.0;
    for (var k = 0u; k <= 4u; k++) {
        if (k > 0u) {
            cur[order[k - 1u]] += 1;
        }
        let shift = unskew * f32(k);
        var d = vec4<f32>(0.0);
        var r2 = 0.0;
        for (var i = 0u; i < 4u; i++) {
            var stepped = 0.0;
            for (var q = 0u; q < k; q++) {
                if (order[q] == i) { stepped = 1.0; }
            }
            d[i] = d0[i] - stepped + shift;
            r2 += d[i] * d[i];
        }
        let a = 0.6 - r2;
        if (a > 0.0) {
            let a2 = a * a;
            total += a2 * a2 * forge_grad4(forge_hash4(seed, cur), d);
        }
    }
    return total * 27.0;
}
// (f1, f2, id)
fn forge_worley4(seed: u32, p: vec4<f32>) -> vec3<f32> {
    let fl = floor(p);
    let c = vec4<i32>(fl);
    let f = p - fl;
    var f1 = 1e30;
    var f2 = 1e30;
    var id = 0.0;
    for (var m = 0u; m < 81u; m++) {
        var mm = m;
        var off = vec4<i32>(0);
        for (var i = 0u; i < 4u; i++) {
            off[i] = i32(mm % 3u) - 1;
            mm = mm / 3u;
        }
        let h = forge_hash4(seed, c + off);
        var g = h;
        var d2 = 0.0;
        for (var i = 0u; i < 4u; i++) {
            g = forge_mix64(forge_u64_add(g, vec2<u32>(0x7f4a7c15u, 0x9e3779b9u)));
            let q = f32(off[i]) + forge_unit(g) - f[i];
            d2 += q * q;
        }
        if (d2 < f1) {
            f2 = f1;
            f1 = d2;
            id = forge_unit(forge_mix64(h ^ vec2<u32>(0xa5a5a5a5u, 0u))) * 2.0 - 1.0;
        } else if (d2 < f2) {
            f2 = d2;
        }
    }
    return vec3<f32>(sqrt(f1), sqrt(f2), id);
}
fn forge_domain_warp(seed: u32, p: vec4<f32>) -> vec4<f32> {
    var q = p;
    q.x += 0.5 * forge_perlin4(forge_octave_seed(seed, 100u), p + vec4<f32>(17.13, 3.71, 9.21, 5.43));
    q.y += 0.5 * forge_perlin4(forge_octave_seed(seed, 101u), p + vec4<f32>(-8.57, 12.29, -4.83, 7.11));
    q.z += 0.5 * forge_perlin4(forge_octave_seed(seed, 102u), p + vec4<f32>(5.97, -13.41, 21.07, -2.39));
    return vec4<f32>(q.xyz, p.w);
}
fn forge_fractal4(seed: u32, octaves: u32, p: vec4<f32>, use_abs: bool) -> f32 {
    var sum = 0.0;
    var amp_sum = 0.0;
    var amp = 1.0;
    var freq = 1.0;
    for (var i = 0u; i < max(octaves, 1u); i++) {
        var v = forge_perlin4(forge_octave_seed(seed, i), p * freq);
        if (use_abs) { v = abs(v); }
        sum += amp * v;
        amp_sum += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    return sum / amp_sum;
}
fn forge_turbulence4(seed: u32, octaves: u32, p: vec4<f32>) -> f32 {
    return 2.0 * forge_fractal4(seed, octaves, p, true) - 1.0;
}
fn forge_ridged4(seed: u32, octaves: u32, p: vec4<f32>) -> f32 {
    var sum = 0.0;
    var amp_sum = 0.0;
    var amp = 1.0;
    var freq = 1.0;
    var weight = 1.0;
    for (var i = 0u; i < max(octaves, 1u); i++) {
        let r = 1.0 - min(abs(forge_perlin4(forge_octave_seed(seed, i), p * freq)), 1.0);
        let signal = r * r;
        sum += amp * weight * signal;
        amp_sum += amp;
        weight = clamp(2.0 * signal, 0.0, 1.0);
        amp *= 0.5;
        freq *= 2.0;
    }
    return 2.0 * sum / amp_sum - 1.0;
}
// kind: the index in NoiseKind::ALL
fn forge_noise_eval(kind: u32, seed: u32, octaves: u32, p: vec4<f32>) -> f32 {
    var v = 0.0;
    switch (kind) {
        case 0u: { v = forge_perlin4(seed, p); }
        case 1u: { v = forge_value4(seed, p); }
        case 2u: { v = forge_simplex4(seed, p); }
        case 3u: { v = forge_value4(seed, floor(p)); }
        case 4u: { v = forge_worley4(seed, p).z; }
        case 5u: { v = 2.0 * forge_worley4(seed, p).x - 1.0; }
        case 6u: { v = 1.4 * forge_worley4(seed, p).y - 1.0; }
        case 7u: {
            let w = forge_worley4(seed, p);
            v = 2.0 * (w.y - w.x) - 1.0;
        }
        case 8u: { v = forge_fractal4(seed, octaves, p, false); }
        case 9u: { v = forge_turbulence4(seed, octaves, p); }
        case 10u: { v = forge_ridged4(seed, octaves, p); }
        case 11u: {
            var q = p;
            for (var i = 0u; i < 4u; i++) {
                q[i] += 0.5 * sin(2.0 * p[(i + 1u) % 4u] + f32(i));
            }
            v = forge_turbulence4(seed, octaves, q);
        }
        case 12u: {
            let f = abs(forge_fractal4(seed, octaves, p, false));
            v = 1.0 - 2.0 * sqrt(f);
        }
        case 13u: { v = forge_fractal4(seed, octaves, forge_domain_warp(seed, p), false); }
        case 14u: {
            let m = 0.5 + 0.5 * forge_perlin4(forge_octave_seed(seed, 200u), p * 0.5);
            v = forge_turbulence4(seed, octaves, p) * (0.5 + 0.5 * m);
        }
        case 15u: { v = forge_turbulence4(seed, octaves, forge_domain_warp(seed, p)); }
        default: { v = 2.0 * forge_worley4(seed, forge_domain_warp(seed, p)).x - 1.0; }
    }
    return clamp(v, -1.0, 1.0);
}
"#;
