//! Seeded, deterministic 3D/4D noise (docs/specs/NOISE.md).
//!
//! Written from the published algorithms (improved Perlin noise, simplex noise, Worley
//! cellular noise, Musgrave fractals). No global state, no randomness other than the seed,
//! only wrapping integer hashing and plain `f64` arithmetic, so results are bit-identical on
//! every platform.

use glam::DVec3;

/// SplitMix64 finalizer (public domain).
#[inline]
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Hash of a lattice cell and a seed.
#[inline]
pub fn hash_cell<const N: usize>(seed: u32, cell: &[i64; N]) -> u64 {
    let mut h = mix(u64::from(seed).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0x1234_5678_9abc_def0);
    for &c in cell {
        h = mix(h ^ (c as u64));
    }
    h
}

/// Hash mapped to `[0, 1)` (53 bits).
#[inline]
fn unit_from(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Gradient `g` (one zero component, others `+-1`; `N * 2^(N-1)` choices) dotted with `d`.
#[inline]
fn grad_dot<const N: usize>(h: u64, d: &[f64; N]) -> f64 {
    let per = 1usize << (N - 1);
    let g = (h >> 8) as usize % (N * per);
    let axis = g / per;
    let mut bits = g % per;
    let mut s = 0.0;
    for (i, &x) in d.iter().enumerate() {
        if i == axis {
            continue;
        }
        s += if bits & 1 == 1 { -x } else { x };
        bits >>= 1;
    }
    s
}

fn floor_split<const N: usize>(p: [f64; N]) -> ([i64; N], [f64; N]) {
    let mut c = [0i64; N];
    let mut f = [0.0; N];
    for i in 0..N {
        let fl = p[i].floor();
        c[i] = fl as i64;
        f[i] = p[i] - fl;
    }
    (c, f)
}

/// Improved Perlin gradient noise in `N` dimensions, scaled to roughly `[-1, 1]`.
pub fn perlin<const N: usize>(seed: u32, p: [f64; N]) -> f64 {
    let (c, f) = floor_split(p);
    let w: Vec<f64> = f.iter().map(|&x| fade(x)).collect();
    // Corners are blended one axis at a time.
    let mut vals = vec![0.0; 1 << N];
    for (m, v) in vals.iter_mut().enumerate() {
        let mut cell = c;
        let mut d = f;
        for i in 0..N {
            if (m >> i) & 1 == 1 {
                cell[i] += 1;
                d[i] -= 1.0;
            }
        }
        *v = grad_dot(hash_cell(seed, &cell), &d);
    }
    let mut n = 1usize << N;
    // Pairs (2j, 2j+1) differ in the lowest remaining axis, so axes reduce in order.
    for wi in w {
        n >>= 1;
        for j in 0..n {
            vals[j] = lerp(vals[2 * j], vals[2 * j + 1], wi);
        }
    }
    vals[0] * perlin_scale(N)
}

fn perlin_scale(n: usize) -> f64 {
    match n {
        3 => 1.0,
        _ => 0.95,
    }
}

/// Value noise in `N` dimensions in `[-1, 1]`.
pub fn value<const N: usize>(seed: u32, p: [f64; N]) -> f64 {
    let (c, f) = floor_split(p);
    let mut vals = vec![0.0; 1 << N];
    for (m, v) in vals.iter_mut().enumerate() {
        let mut cell = c;
        for (i, ci) in cell.iter_mut().enumerate() {
            *ci += ((m >> i) & 1) as i64;
        }
        *v = unit_from(hash_cell(seed, &cell)) * 2.0 - 1.0;
    }
    let mut n = 1usize << N;
    for fi in f {
        n >>= 1;
        let t = fade(fi);
        for j in 0..n {
            vals[j] = lerp(vals[2 * j], vals[2 * j + 1], t);
        }
    }
    vals[0]
}

/// Simplex noise in `N` dimensions (3 or 4 are tuned), roughly `[-1, 1]`.
pub fn simplex<const N: usize>(seed: u32, p: [f64; N]) -> f64 {
    let nf = N as f64;
    let skew = ((nf + 1.0).sqrt() - 1.0) / nf;
    let unskew = (1.0 - 1.0 / (nf + 1.0).sqrt()) / nf;
    let s: f64 = p.iter().sum::<f64>() * skew;
    let (cell, _) = floor_split(std::array::from_fn::<f64, N, _>(|i| p[i] + s));
    let t: f64 = cell.iter().map(|&c| c as f64).sum::<f64>() * unskew;
    let d0: [f64; N] = std::array::from_fn(|i| p[i] - (cell[i] as f64 - t));
    // Visit axes by decreasing offset: that selects the simplex containing p.
    let mut order: [usize; N] = std::array::from_fn(|i| i);
    for i in 1..N {
        let mut j = i;
        while j > 0 && d0[order[j - 1]] < d0[order[j]] {
            order.swap(j - 1, j);
            j -= 1;
        }
    }
    let mut total = 0.0;
    let mut cur = cell;
    for k in 0..=N {
        if k > 0 {
            cur[order[k - 1]] += 1;
        }
        let shift = unskew * k as f64;
        let mut d = [0.0; N];
        let mut r2 = 0.0;
        for i in 0..N {
            let stepped = order[..k].contains(&i) as i64 as f64;
            d[i] = d0[i] - stepped + shift;
            r2 += d[i] * d[i];
        }
        let a = 0.6 - r2;
        if a > 0.0 {
            let a2 = a * a;
            total += a2 * a2 * grad_dot(hash_cell(seed, &cur), &d);
        }
    }
    total * simplex_scale(N)
}

fn simplex_scale(n: usize) -> f64 {
    match n {
        3 => 32.0,
        _ => 27.0,
    }
}

/// Result of a Worley search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Worley {
    /// Distance to the nearest feature point (cell units).
    pub f1: f64,
    /// Distance to the second nearest feature point.
    pub f2: f64,
    /// Random value in `[-1, 1]` tied to the nearest feature point's cell.
    pub id: f64,
}

/// Worley cellular noise (one jittered feature point per cell, 3^N neighbourhood).
pub fn worley<const N: usize>(seed: u32, p: [f64; N]) -> Worley {
    let (c, f) = floor_split(p);
    let mut f1 = f64::MAX;
    let mut f2 = f64::MAX;
    let mut id = 0.0;
    let total = 3usize.pow(N as u32);
    for m in 0..total {
        let mut cell = c;
        let mut off = [0.0; N];
        let mut mm = m;
        for i in 0..N {
            let o = (mm % 3) as i64 - 1;
            mm /= 3;
            cell[i] += o;
            off[i] = o as f64;
        }
        let h = hash_cell(seed, &cell);
        let mut g = h;
        let mut d2 = 0.0;
        for i in 0..N {
            g = mix(g.wrapping_add(0x9e37_79b9_7f4a_7c15));
            let q = off[i] + unit_from(g) - f[i];
            d2 += q * q;
        }
        if d2 < f1 {
            f2 = f1;
            f1 = d2;
            id = unit_from(mix(h ^ 0xa5a5_a5a5)) * 2.0 - 1.0;
        } else if d2 < f2 {
            f2 = d2;
        }
    }
    Worley {
        f1: f1.sqrt(),
        f2: f2.sqrt(),
        id,
    }
}

/// Which noise a [`Noise`] evaluates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoiseKind {
    Perlin,
    Value,
    Simplex,
    /// Constant value per lattice cell.
    Box,
    /// Constant random value per Worley cell.
    Cell,
    /// Worley `2 F1 - 1`.
    Voronoi1,
    /// Worley `1.4 F2 - 1`.
    Voronoi2,
    /// Worley `2 (F2 - F1) - 1`.
    Voronoi3,
    Fbm,
    Turbulence,
    RidgedMultifractal,
    WavyTurbulence,
    Electric,
    Gaseous,
    BlisteredTurbulence,
    DisplacedTurbulence,
    DisplacedVoronoi,
}

impl NoiseKind {
    /// All kinds, in declaration order.
    pub const ALL: [NoiseKind; 17] = [
        NoiseKind::Perlin,
        NoiseKind::Value,
        NoiseKind::Simplex,
        NoiseKind::Box,
        NoiseKind::Cell,
        NoiseKind::Voronoi1,
        NoiseKind::Voronoi2,
        NoiseKind::Voronoi3,
        NoiseKind::Fbm,
        NoiseKind::Turbulence,
        NoiseKind::RidgedMultifractal,
        NoiseKind::WavyTurbulence,
        NoiseKind::Electric,
        NoiseKind::Gaseous,
        NoiseKind::BlisteredTurbulence,
        NoiseKind::DisplacedTurbulence,
        NoiseKind::DisplacedVoronoi,
    ];

    /// True if the output is continuous (no jumps at cell borders).
    pub fn is_continuous(self) -> bool {
        !matches!(self, NoiseKind::Box | NoiseKind::Cell)
    }
}

/// A configured noise. All fields are plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct Noise {
    pub kind: NoiseKind,
    pub seed: u32,
    /// Feature size in world units; lattice coordinate is `p / scale`.
    pub scale: f64,
    pub octaves: u32,
    pub lacunarity: f64,
    /// Amplitude factor per octave.
    pub gain: f64,
    /// Fourth coordinate used by [`Noise::sample`].
    pub time: f64,
    /// Domain-warp amount (kinds that warp).
    pub warp: f64,
}

impl Default for Noise {
    fn default() -> Self {
        Noise {
            kind: NoiseKind::Perlin,
            seed: 0,
            scale: 1.0,
            octaves: 4,
            lacunarity: 2.0,
            gain: 0.5,
            time: 0.0,
            warp: 0.5,
        }
    }
}

/// Seed of octave `i` of a fractal (public so tests can reproduce a single octave).
pub fn octave_seed(seed: u32, i: u32) -> u32 {
    (mix(u64::from(seed) ^ (u64::from(i) << 32) ^ 0x51ed_270b) >> 32) as u32
}

/// Offsets that decorrelate the three warp channels.
const WARP_OFFSETS: [[f64; 4]; 3] = [
    [17.13, 3.71, 9.21, 5.43],
    [-8.57, 12.29, -4.83, 7.11],
    [5.97, -13.41, 21.07, -2.39],
];

/// Domain warp: `p + amount * (n1, n2, n3)` with three decorrelated Perlin samples (4D point).
pub fn domain_warp(seed: u32, p: [f64; 4], amount: f64) -> [f64; 4] {
    let mut q = p;
    for (k, off) in WARP_OFFSETS.iter().enumerate() {
        let s = std::array::from_fn(|i| p[i] + off[i]);
        q[k] += amount * perlin::<4>(octave_seed(seed, 100 + k as u32), s);
    }
    q
}

impl Noise {
    /// Pure 3D noise at `p` in `[-1, 1]` (ignores `time`).
    pub fn sample3(&self, p: DVec3) -> f64 {
        let q = p / self.scale;
        self.eval::<3>([q.x, q.y, q.z])
    }

    /// 4D noise with `t` as the fourth axis (not divided by `scale`).
    pub fn sample4(&self, p: DVec3, t: f64) -> f64 {
        let q = p / self.scale;
        self.eval::<4>([q.x, q.y, q.z, t])
    }

    /// `sample4(p, self.time)`.
    pub fn sample(&self, p: DVec3) -> f64 {
        self.sample4(p, self.time)
    }

    /// `0.5 + 0.5 * sample3(p)` in `[0, 1]`.
    pub fn unit3(&self, p: DVec3) -> f64 {
        0.5 + 0.5 * self.sample3(p)
    }

    /// `0.5 + 0.5 * sample(p)` in `[0, 1]`.
    pub fn unit(&self, p: DVec3) -> f64 {
        0.5 + 0.5 * self.sample(p)
    }

    fn eval<const N: usize>(&self, p: [f64; N]) -> f64 {
        let s = self.seed;
        let v = match self.kind {
            NoiseKind::Perlin => perlin(s, p),
            NoiseKind::Value => value(s, p),
            NoiseKind::Simplex => simplex(s, p),
            NoiseKind::Box => value(s, p.map(f64::floor)),
            NoiseKind::Cell => worley(s, p).id,
            NoiseKind::Voronoi1 => 2.0 * worley(s, p).f1 - 1.0,
            NoiseKind::Voronoi2 => 1.4 * worley(s, p).f2 - 1.0,
            NoiseKind::Voronoi3 => {
                let w = worley(s, p);
                2.0 * (w.f2 - w.f1) - 1.0
            }
            NoiseKind::Fbm => self.fractal(p, |n| n),
            NoiseKind::Turbulence => self.turbulence(p),
            NoiseKind::RidgedMultifractal => self.ridged(p),
            NoiseKind::WavyTurbulence => {
                let mut q = p;
                for i in 0..N {
                    q[i] += self.warp * (2.0 * p[(i + 1) % N] + i as f64).sin();
                }
                self.turbulence(q)
            }
            NoiseKind::Electric => {
                let f = self.fractal(p, |n| n).abs();
                1.0 - 2.0 * f.sqrt()
            }
            NoiseKind::Gaseous => self.fractal(self.warped(p), |n| n),
            NoiseKind::BlisteredTurbulence => {
                let low: [f64; N] = p.map(|x| x * 0.5);
                let m = 0.5 + 0.5 * perlin(octave_seed(s, 200), low);
                self.turbulence(p) * (0.5 + 0.5 * m)
            }
            NoiseKind::DisplacedTurbulence => self.turbulence(self.warped(p)),
            NoiseKind::DisplacedVoronoi => 2.0 * worley(s, self.warped(p)).f1 - 1.0,
        };
        v.clamp(-1.0, 1.0)
    }

    fn warped<const N: usize>(&self, p: [f64; N]) -> [f64; N] {
        let mut four = [0.0; 4];
        for (i, x) in p.iter().enumerate().take(4) {
            four[i] = *x;
        }
        let q = domain_warp(self.seed, four, self.warp);
        let mut out = p;
        for (i, o) in out.iter_mut().enumerate().take(3) {
            *o = q[i];
        }
        out
    }

    /// Weighted octave sum of `f(perlin)`, normalised by the amplitude sum.
    fn fractal<const N: usize>(&self, p: [f64; N], f: impl Fn(f64) -> f64) -> f64 {
        let n = self.octaves.max(1);
        let (mut sum, mut amp_sum, mut amp, mut freq) = (0.0, 0.0, 1.0, 1.0);
        for i in 0..n {
            let q = p.map(|x| x * freq);
            sum += amp * f(perlin(octave_seed(self.seed, i), q));
            amp_sum += amp;
            amp *= self.gain;
            freq *= self.lacunarity;
        }
        sum / amp_sum
    }

    fn turbulence<const N: usize>(&self, p: [f64; N]) -> f64 {
        // sum of |n| lies in [0, 1]; map to [-1, 1]
        2.0 * self.fractal(p, f64::abs) - 1.0
    }

    fn ridged<const N: usize>(&self, p: [f64; N]) -> f64 {
        let n = self.octaves.max(1);
        let (mut sum, mut amp_sum, mut amp, mut freq, mut weight) = (0.0, 0.0, 1.0, 1.0, 1.0);
        for i in 0..n {
            let q = p.map(|x| x * freq);
            let r = 1.0 - perlin(octave_seed(self.seed, i), q).abs().min(1.0);
            let signal = r * r;
            sum += amp * weight * signal;
            amp_sum += amp;
            weight = (2.0 * signal).clamp(0.0, 1.0);
            amp *= self.gain;
            freq *= self.lacunarity;
        }
        2.0 * sum / amp_sum - 1.0
    }
}
