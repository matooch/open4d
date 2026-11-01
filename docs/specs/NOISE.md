# Noise library `forge-noise` (brief 063)

Wording: "noise", "octave", "lacunarity", "gain". Sources: published algorithms only. Perlin,
"Improving noise" (SIGGRAPH 2002: quintic fade `6t^5-15t^4+10t^3`, edge-vector gradients);
Gustavson, "Simplex noise demystified" (2005: skew `F=(sqrt(n+1)-1)/n`, unskew
`G=(1-1/sqrt(n+1))/n`, kernel `(r^2 - d^2)^4`); Worley, "A cellular texture basis function"
(SIGGRAPH 1996: feature points per cell, F1/F2); Musgrave, "Texturing and Modeling" ch. 12
(fBm, turbulence, ridged multifractal); Quilez's public articles for domain warping. All code
is written from those descriptions; **no third-party source or table is copied** and nothing
comes from the reference app. The hash is the public-domain SplitMix64 finalizer
(Steele, Lea, Flood 2014).
Parity: the type *list* comes from O13 [fields-2.md](../parity/fields-2.md) (Noise Type, 31
options); the *values* of the reference noise are **parity-pending O42**
([shaders.md](../parity/shaders.md)). Exact matching is not expected; compare statistically.

## N1 Contract
- Crate `crates/noise`, package `forge-noise`, depends only on `glam`. No global state, no
  `unsafe`, no randomness source other than the seed.
- `Noise { kind: NoiseKind, seed: u32, scale: f64, octaves: u32, lacunarity: f64, gain: f64,
  time: f64, warp: f64 }` (`Default`: Perlin, seed 0, scale 1, 4 octaves, lacunarity 2, gain
  0.5, time 0, warp 0.5). `scale` is the feature size in world units: the lattice
  coordinate is `p / scale`.
- `sample3(p: DVec3) -> f64` is true 3D noise. `sample4(p, t) -> f64` and `sample(p)`
  (= `sample4(p, self.time)`) use 4D noise with `t` as the fourth axis. `unit*` variants return `0.5 + 0.5 v`.
- Output range: every type returns a value in `[-1, 1]` (final `clamp`); `unit*` in `[0, 1]`.
- Free functions on the raw lattice noises: `perlin3/4`, `value3/4`, `simplex3/4`,
  `worley3/4` (-> `Worley { f1, f2, id }`), `hash_*`.

## N2 Determinism
- Pure `f64` arithmetic (`+ - * /`, `floor`, `sqrt`, `abs`, `powi`, `sin` only in Wavy), integer
  hashing with wrapping `u64` ops. No SIMD intrinsics, no `mul_add`, no `HashMap` iteration,
  no threads. Same seed and arguments give bit-identical values on every OS. `sin` is the only
  libm call; the one type using it (Wavy Turbulence) is pinned with a tolerance of 1e-12.
- Lattice hash `h(seed, cell) = fold(mix, seed * golden, cell_i as u64)`; different seeds give
  statistically independent fields (test: correlation of seeds 1 and 2 below 0.05).

## N3 Base noises (signed, nominal range `[-1, 1]`)
- **Perlin** (`perlin`): improved gradient noise in 3D and 4D. Gradients are the
  `n * 2^(n-1)` vectors with one zero component and the others `+-1` (12 in 3D, 32 in 4D),
  selected by the hash; the corner dot products are blended with the quintic fade. Zero at lattice
  points. Scale 1.0 (3D), 0.95 (4D) (N8).
- **Value** (`value`): hashed value in `[-1, 1]` at each lattice point, quintic interpolation.
- **Simplex** (`simplex`): Gustavson simplex in 3D and 4D with the one-zero gradients, radius
  `r^2 = 0.6`, scale 32 (3D) / 27 (4D) (N8).
- **Worley** (`worley`): one feature point per cell at `cell + jitter(hash)`, jitter in
  `[0,1)^n`. Searches the `3^n` neighbouring cells; returns the nearest and second nearest
  Euclidean distances `f1`, `f2` and a per-cell random `id in [-1, 1]` of the nearest cell.
  Limitation: with full jitter the 3^n search can in rare cases miss the true F2; accepted.

## N4 Types (`NoiseKind`)
Our own formulas; names follow the O13 list where a literature name exists. The remaining names of the
O13 list have no public definition and are not implemented until O42 (N7).

| Kind | Definition (`n` = Perlin, `a_i = gain^i`, `l_i = lacunarity^i`) |
|---|---|
| Perlin | `n(p)` |
| Value | value noise |
| Simplex | simplex noise |
| Box | value noise at `floor(p)`: constant per lattice cell |
| Cell | `id` of the nearest Worley cell: constant per Voronoi cell |
| Voronoi1 | `2 f1 - 1` (Worley F1) |
| Voronoi2 | `1.4 f2 - 1` (Worley F2) |
| Voronoi3 | `2 (f2 - f1) - 1` (Worley F2-F1) |
| Fbm | `sum a_i n_i(l_i p) / sum a_i`, per-octave seed `octave_seed(seed, i)` |
| Turbulence | `2 (sum a_i abs(n_i(l_i p)) / sum a_i) - 1` (biased low, mean about -0.56) |
| RidgedMultifractal | Musgrave: `s_i = (1 - abs(n_i))^2`, weight `w_0 = 1`, `w_{i+1} = clamp(2 s_i, 0, 1)`, result `2 (sum a_i w_i s_i / sum a_i) - 1` |
| WavyTurbulence | Turbulence at `q_i = p_i + warp * sin(2 p_{i+1} + i)` |
| Electric | `1 - 2 sqrt(abs(Fbm))`: sharp creases where Fbm crosses 0 |
| Gaseous | Fbm at the domain-warped point (N5) |
| BlisteredTurbulence | `Turbulence * (0.5 + 0.5 m)`, `m = 0.5 + 0.5 n(p / 2)` |
| DisplacedTurbulence | Turbulence at the domain-warped point |
| DisplacedVoronoi | Voronoi1 at the domain-warped point |

(Exact formulas are the doc comments in `crates/noise/src/lib.rs`; they are normative with the table.)

## N5 Domain warp
`warp(p, seed, amount) = p + amount * (perlin(p + o1), perlin(p + o2), perlin(p + o3))` with three
fixed decorrelating offsets and seeds derived from `seed`. Also public: `domain_warp`.

## N6 Tests (golden-free, `crates/noise/tests/noise.rs`)
- determinism: repeated calls identical; pinned values for fixed points per kind (regression pins
  of this implementation, not of any reference);
- range: 20 000 random points per kind inside `[-1, 1]`;
- continuity: for the smooth kinds, `|f(p+e) - f(p)| <= K e` with a Lipschitz bound, e = 1e-4;
- moments: mean near 0 and std in a band per kind; unit-variance-ish ordering (Perlin std 0.2..0.5);
- seed independence: low correlation between seeds, identical for equal seeds;
- 3D vs 4D: `sample4` continuous in `t`; `sample3` differs from time slices;
- lattice: Perlin is 0 at integer points; Worley f1 is 0 at a feature point; `f1 <= f2`.

## N7 Parity status and statistical comparison (O13)
- `tests/golden/fields/random-defaults` (Random field, mode Noise, Perlin, seed 123456, space
  Field, scale 100 %, remapping on) holds 729 samples on the grid `{-300..300 step 75}^3`,
  values in `[0.10056, 0.95476]`. `random-seed` gives `[0.10959, 0.88282]`.
  `crates/noise/tests/o13_statistics.rs` computes mean, std, min/max and the lag-1 correlation
  (neighbours 75 apart along x) of the reference and of our unit-mapped Perlin on the same grid.
  Measured reference: random-defaults mean 0.4918, std 0.1467, lag-1 -0.009; random-seed mean
  0.4995, std 0.1517, lag-1 -0.012. Ours (seed 123456): scale 50 std 0.133 lag-1 -0.05; scale 100
  std 0.129 lag-1 0.07; scale 200 lag-1 0.62. Result: the reference is centred on 0.5, spans a
  similar width, and is uncorrelated at step 75, so its feature size is at most about 100
  world units; our Perlin at scale 50-100 matches mean, std and decorrelation. The asserted
  relations are loose (mean within 0.1 of 0.5, lag-1 under 0.1 / 0.15, some scale
  reproduces the spread within 50 %). Why not tighter: O13 gives no feature size, octave count or
  seed hashing, only values that pass through the Random field's remapping chain; 729 points
  on a grid that coarse cannot pin more. A scale of 75 aligns the grid with lattice points and is
  degenerate (all zero), so it is not used.
- `shader-noise` (O23, Noise shader seed 665, 5 octaves; mean 0.4999, std 0.1179, lag-1 -0.003,
  range 0.1725..0.8682) is checked for range, mean and decorrelation only.
- Pending O42: exact per-type values, the 31-name list beyond the kinds above, the effect of
  seed on the lattice, the unit mapping. Names without a literature definition (Buya, Cranal,
  Dents, Hama, Luka, Mod. Noise, Naki, Nutous, Ober, Pezo, Poxo, Sema, Stupl, VL Noise, Zada)
  are not implemented.

## N8 Normalisation constants
Measured over 10^6 points and recorded in the code comments next to each constant; a test
asserts the extremes of every base noise stay inside `[-1, 1]` before clamping.
