// Deterministic 64-bit emulation, PCG32 and the sampling helpers of R9 in WGSL
// (docs/specs/RENDER.md R20.2). Every function mirrors forge_render_cpu::sampling.

const PI_F: f32 = 3.14159265358979323846;
const TAU_F: f32 = 6.28318530717958647692;
const INV_4PI_F: f32 = 0.07957747154594766788;

fn u64_add(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let lo = a.x + b.x;
    var carry = 0u;
    if (lo < a.x) {
        carry = 1u;
    }
    return vec2<u32>(lo, a.y + b.y + carry);
}

fn u64_shl(a: vec2<u32>, n: u32) -> vec2<u32> {
    if (n == 0u) {
        return a;
    }
    if (n >= 32u) {
        return vec2<u32>(0u, a.x << (n - 32u));
    }
    return vec2<u32>(a.x << n, (a.y << n) | (a.x >> (32u - n)));
}

fn u64_shr(a: vec2<u32>, n: u32) -> vec2<u32> {
    if (n == 0u) {
        return a;
    }
    if (n >= 32u) {
        return vec2<u32>(a.y >> (n - 32u), 0u);
    }
    return vec2<u32>((a.x >> n) | (a.y << (32u - n)), a.y >> n);
}

fn u64_xor(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(a.x ^ b.x, a.y ^ b.y);
}

fn u64_mul(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    var r = vec2<u32>(0u, 0u);
    var acc = a;
    for (var i = 0u; i < 64u; i = i + 1u) {
        var bit = 0u;
        if (i < 32u) {
            bit = (b.x >> i) & 1u;
        } else {
            bit = (b.y >> (i - 32u)) & 1u;
        }
        if (bit == 1u) {
            r = u64_add(r, acc);
        }
        acc = u64_shl(acc, 1u);
    }
    return r;
}

fn splitmix(x: vec2<u32>) -> vec2<u32> {
    var z = u64_add(x, vec2<u32>(0x7F4A7C15u, 0x9E3779B9u));
    z = u64_mul(u64_xor(z, u64_shr(z, 30u)), vec2<u32>(0x1CE4E5B9u, 0xBF58476Du));
    z = u64_mul(u64_xor(z, u64_shr(z, 27u)), vec2<u32>(0x133111EBu, 0x94D049BBu));
    return u64_xor(z, u64_shr(z, 31u));
}

struct Rng {
    state: vec2<u32>,
    inc: vec2<u32>,
}

fn rng_next(r: ptr<function, Rng>) -> u32 {
    let old = (*r).state;
    (*r).state = u64_add(
        u64_mul(old, vec2<u32>(0x4C957F2Du, 0x5851F42Du)),
        (*r).inc,
    );
    let w = u64_shr(u64_xor(u64_shr(old, 18u), old), 27u);
    let x = w.x;
    let rot = old.y >> 27u;
    return (x >> rot) | (x << ((32u - rot) & 31u));
}

fn rng_new(seed: vec2<u32>, stream: vec2<u32>) -> Rng {
    var r: Rng;
    r.state = vec2<u32>(0u, 0u);
    r.inc = vec2<u32>((stream.x << 1u) | 1u, (stream.y << 1u) | (stream.x >> 31u));
    let _a = rng_next(&r);
    r.state = u64_add(r.state, seed);
    let _b = rng_next(&r);
    return r;
}

// ((hi << 21) ^ lo) & (2^53 - 1), over 2^53, rounded to f32: the CPU value to ~1e-7.
fn rng_f(r: ptr<function, Rng>) -> f32 {
    let hi = rng_next(r);
    let lo = rng_next(r);
    let x_lo = (hi << 21u) ^ lo;
    let x_hi = hi >> 11u;
    return f32(x_hi) * 4.76837158203125e-7 + f32(x_lo) * 1.1102230246251565e-16;
}

fn rng_f2(r: ptr<function, Rng>) -> vec2<f32> {
    let a = rng_f(r);
    let b = rng_f(r);
    return vec2<f32>(a, b);
}

fn basis(n: vec3<f32>) -> mat3x3<f32> {
    let s = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    let t = vec3<f32>(1.0 + s * n.x * n.x * a, s * b, -s * n.x);
    let u = vec3<f32>(b, s + n.y * n.y * a, -n.y);
    return mat3x3<f32>(t, u, n);
}

fn to_world(n: vec3<f32>, l: vec3<f32>) -> vec3<f32> {
    return basis(n) * l;
}

fn cosine_hemisphere(u: vec2<f32>) -> vec3<f32> {
    let r = sqrt(u.x);
    let phi = TAU_F * u.y;
    return vec3<f32>(r * cos(phi), r * sin(phi), sqrt(max(1.0 - u.x, 0.0)));
}

fn uniform_sphere(u: vec2<f32>) -> vec3<f32> {
    let z = 1.0 - 2.0 * u.x;
    let r = sqrt(max(1.0 - z * z, 0.0));
    let phi = TAU_F * u.y;
    return vec3<f32>(r * cos(phi), r * sin(phi), z);
}

fn uniform_ball(r: ptr<function, Rng>) -> vec3<f32> {
    for (var i = 0u; i < 64u; i = i + 1u) {
        let p = vec3<f32>(
            rng_f(r) * 2.0 - 1.0,
            rng_f(r) * 2.0 - 1.0,
            rng_f(r) * 2.0 - 1.0,
        );
        if (dot(p, p) <= 1.0) {
            return p;
        }
    }
    return vec3<f32>(0.0);
}

fn uniform_cone(axis: vec3<f32>, half: f32, u: vec2<f32>) -> vec3<f32> {
    let cos_t = 1.0 - u.x * (1.0 - cos(half));
    let sin_t = sqrt(max(1.0 - cos_t * cos_t, 0.0));
    let phi = TAU_F * u.y;
    return to_world(axis, vec3<f32>(sin_t * cos(phi), sin_t * sin(phi), cos_t));
}

fn power_heuristic(a: f32, b: f32) -> f32 {
    let a2 = a * a;
    let b2 = b * b;
    if (a2 + b2 <= 0.0) {
        return 0.0;
    }
    return a2 / (a2 + b2);
}

fn lum(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}
