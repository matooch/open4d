//! Filters on distance fields: smoothing, offset, reshape (VOLUME.md section 8).

use glam::IVec3;

use crate::grid::SparseGrid;

/// Active voxels of a grid with their neighbourhood read through `get` (inactive voxels
/// read their stored value, or the background).
fn for_each_active(grid: &SparseGrid<f32>, mut f: impl FnMut(IVec3) -> f32) -> SparseGrid<f32> {
    let mut out = grid.clone();
    for (i, _) in grid.active_voxels() {
        out.set(i, f(i));
    }
    out
}

/// Normalised Gaussian taps of half width `radius` and standard deviation `sigma`.
pub fn gaussian_kernel(radius: u32, sigma: f64) -> Vec<f64> {
    let r = radius as i32;
    let mut k: Vec<f64> = (-r..=r)
        .map(|i| (-(f64::from(i * i)) / (2.0 * sigma * sigma)).exp())
        .collect();
    let sum: f64 = k.iter().sum();
    for w in &mut k {
        *w /= sum;
    }
    k
}

fn convolve_axis(grid: &SparseGrid<f32>, kernel: &[f64], axis: IVec3) -> SparseGrid<f32> {
    let r = (kernel.len() / 2) as i32;
    for_each_active(grid, |i| {
        let mut acc = 0.0;
        for (t, w) in kernel.iter().enumerate() {
            acc += w * f64::from(grid.get(i + axis * (t as i32 - r)));
        }
        acc as f32
    })
}

/// Separable Gaussian smoothing of the active voxels, `iterations` times.
pub fn gaussian(
    grid: &SparseGrid<f32>,
    radius: u32,
    sigma: f64,
    iterations: u32,
) -> SparseGrid<f32> {
    let k = gaussian_kernel(radius, sigma);
    let mut g = grid.clone();
    for _ in 0..iterations {
        for axis in [IVec3::X, IVec3::Y, IVec3::Z] {
            g = convolve_axis(&g, &k, axis);
        }
    }
    g
}

/// Box (mean) filter of half width `radius`, `iterations` times.
pub fn mean(grid: &SparseGrid<f32>, radius: u32, iterations: u32) -> SparseGrid<f32> {
    let k = vec![1.0 / f64::from(2 * radius + 1); (2 * radius + 1) as usize];
    let mut g = grid.clone();
    for _ in 0..iterations {
        for axis in [IVec3::X, IVec3::Y, IVec3::Z] {
            g = convolve_axis(&g, &k, axis);
        }
    }
    g
}

/// Median over the cube of half width `radius`, `iterations` times.
pub fn median(grid: &SparseGrid<f32>, radius: u32, iterations: u32) -> SparseGrid<f32> {
    let r = radius as i32;
    let mut g = grid.clone();
    for _ in 0..iterations {
        let src = g.clone();
        g = for_each_active(&src, |i| {
            let mut v: Vec<f32> = Vec::with_capacity(((2 * r + 1).pow(3)) as usize);
            for x in -r..=r {
                for y in -r..=r {
                    for z in -r..=r {
                        v.push(src.get(i + IVec3::new(x, y, z)));
                    }
                }
            }
            v.sort_by(f32::total_cmp);
            v[v.len() / 2]
        });
    }
    g
}

/// Laplacian flow: `value += strength * dt * laplacian(value)` with the 7-point Laplacian in
/// voxel units and `dt = 1/6` (the explicit stability limit), `iterations` times.
pub fn laplacian_flow(grid: &SparseGrid<f32>, strength: f64, iterations: u32) -> SparseGrid<f32> {
    let mut g = grid.clone();
    for _ in 0..iterations {
        let src = g.clone();
        g = for_each_active(&src, |i| {
            let c = f64::from(src.get(i));
            let mut sum = -6.0 * c;
            for d in [
                IVec3::X,
                IVec3::NEG_X,
                IVec3::Y,
                IVec3::NEG_Y,
                IVec3::Z,
                IVec3::NEG_Z,
            ] {
                sum += f64::from(src.get(i + d));
            }
            (c + strength / 6.0 * sum) as f32
        });
    }
    g
}

/// Band of a distance field in world units: exterior (positive side) and interior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub exterior: f64,
    pub interior: f64,
}

fn inside(v: f32) -> bool {
    v < 0.0
}

/// Re-distance: a true signed distance field of the zero level set of `grid`, within `band`
/// (VOLUME.md section 8.1). The interface distance of the voxels next to a sign change comes
/// from the linear crossing along the lattice edges; the rest follows from a first order fast
/// sweeping solve of `|grad d| = 1` (Zhao 2005) on the dense box of the active voxels grown by
/// the band. Voxels beyond the band are dropped and the interior is filled.
pub fn redistance(grid: &SparseGrid<f32>, band: Band) -> SparseGrid<f32> {
    let h = grid.voxel_size();
    let mut out = SparseGrid::new(band.exterior as f32, h, grid.origin());
    let Some((lo, hi)) = grid.bounds() else {
        return out;
    };
    let pad = (band.exterior.max(band.interior) / h).ceil() as i32 + 1;
    let (lo, hi) = (lo - IVec3::splat(pad), hi + IVec3::splat(pad));
    let size = hi - lo + IVec3::ONE;
    let (nx, ny, nz) = (size.x as usize, size.y as usize, size.z as usize);
    let at = |x: usize, y: usize, z: usize| x + nx * (y + ny * z);
    let mut sign = vec![false; nx * ny * nz];
    let mut vals = vec![0.0f32; nx * ny * nz];
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let v = grid.get(lo + IVec3::new(x as i32, y as i32, z as i32));
                vals[at(x, y, z)] = v;
                sign[at(x, y, z)] = inside(v);
            }
        }
    }
    let big = f32::MAX;
    let mut d = vec![big; nx * ny * nz];
    let mut fixed = vec![false; nx * ny * nz];
    // interface distance along edges, in voxel units
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let i = at(x, y, z);
                for (dx, dy, dz) in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
                    let (ux, uy, uz) = (x + dx, y + dy, z + dz);
                    if ux >= nx || uy >= ny || uz >= nz {
                        continue;
                    }
                    let j = at(ux, uy, uz);
                    if sign[i] != sign[j] {
                        let (a, b) = (f64::from(vals[i]).abs(), f64::from(vals[j]).abs());
                        let t = if a + b > 0.0 { a / (a + b) } else { 0.5 };
                        d[i] = d[i].min(t as f32);
                        d[j] = d[j].min((1.0 - t) as f32);
                        fixed[i] = true;
                        fixed[j] = true;
                    }
                }
            }
        }
    }
    // refine the interface distance of the voxels next to the surface with the gradient of the
    // input: `|value| / |grad value|` is exact for a linear field and removes the over-estimate
    // of the axis-aligned edge crossing at oblique surfaces
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let i = at(x, y, z);
                if !fixed[i] {
                    continue;
                }
                let diff = |lo_ok: bool, hi_ok: bool, step: [usize; 3]| {
                    let here = f64::from(vals[i]);
                    let lo = if lo_ok {
                        f64::from(vals[at(x - step[0], y - step[1], z - step[2])])
                    } else {
                        here
                    };
                    let hi = if hi_ok {
                        f64::from(vals[at(x + step[0], y + step[1], z + step[2])])
                    } else {
                        here
                    };
                    let span = f64::from(u8::from(lo_ok) + u8::from(hi_ok));
                    if span == 0.0 { 0.0 } else { (hi - lo) / span }
                };
                let gx = diff(x > 0, x + 1 < nx, [1, 0, 0]);
                let gy = diff(y > 0, y + 1 < ny, [0, 1, 0]);
                let gz = diff(z > 0, z + 1 < nz, [0, 0, 1]);
                let g = (gx * gx + gy * gy + gz * gz).sqrt();
                if g > 1e-6 {
                    let est = (f64::from(vals[i]).abs() / g) as f32;
                    // never farther than the edge crossing, which bounds the true distance
                    d[i] = d[i].min(est);
                }
            }
        }
    }
    // fast sweeping, eight orderings, twice
    for _ in 0..2 {
        for order in 0..8 {
            let xs: Vec<usize> = if order & 1 == 0 {
                (0..nx).collect()
            } else {
                (0..nx).rev().collect()
            };
            let ys: Vec<usize> = if order & 2 == 0 {
                (0..ny).collect()
            } else {
                (0..ny).rev().collect()
            };
            let zs: Vec<usize> = if order & 4 == 0 {
                (0..nz).collect()
            } else {
                (0..nz).rev().collect()
            };
            for &z in &zs {
                for &y in &ys {
                    for &x in &xs {
                        let i = at(x, y, z);
                        if fixed[i] {
                            continue;
                        }
                        let get = |xx: isize, yy: isize, zz: isize| -> f32 {
                            if xx < 0
                                || yy < 0
                                || zz < 0
                                || xx >= nx as isize
                                || yy >= ny as isize
                                || zz >= nz as isize
                            {
                                big
                            } else {
                                d[at(xx as usize, yy as usize, zz as usize)]
                            }
                        };
                        let (xi, yi, zi) = (x as isize, y as isize, z as isize);
                        let mut a = [
                            get(xi - 1, yi, zi).min(get(xi + 1, yi, zi)),
                            get(xi, yi - 1, zi).min(get(xi, yi + 1, zi)),
                            get(xi, yi, zi - 1).min(get(xi, yi, zi + 1)),
                        ];
                        a.sort_by(f32::total_cmp);
                        if a[0] >= big {
                            continue;
                        }
                        // upwind update with 1, 2 or 3 neighbours
                        let mut new = a[0] + 1.0;
                        if a[1] < big && new > a[1] {
                            let disc = 2.0 - (a[0] - a[1]).powi(2);
                            if disc >= 0.0 {
                                new = (a[0] + a[1] + disc.sqrt()) * 0.5;
                            }
                            if a[2] < big && new > a[2] {
                                let sum = a[0] + a[1] + a[2];
                                let sq = a[0] * a[0] + a[1] * a[1] + a[2] * a[2];
                                let disc = sum * sum - 3.0 * (sq - 1.0);
                                if disc >= 0.0 {
                                    new = (sum + disc.sqrt()) / 3.0;
                                }
                            }
                        }
                        if new < d[i] {
                            d[i] = new;
                        }
                    }
                }
            }
        }
    }
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let i = at(x, y, z);
                if d[i] >= big {
                    continue;
                }
                let dist = f64::from(d[i]) * h;
                let idx = lo + IVec3::new(x as i32, y as i32, z as i32);
                if sign[i] {
                    if dist <= band.interior {
                        out.set(idx, -(dist as f32));
                    }
                } else if dist <= band.exterior {
                    out.set(idx, dist as f32);
                }
            }
        }
    }
    out.signed_flood_fill(band.interior as f32);
    out
}

/// Moves the surface outwards by `distance` (inwards when negative): values minus `distance`,
/// then re-distanced.
pub fn offset(grid: &SparseGrid<f32>, distance: f64, band: Band) -> SparseGrid<f32> {
    let shifted = grid.map(grid.background() - distance as f32, |v| v - distance as f32);
    redistance(&shifted, band)
}

/// Morphological closing (`distance > 0`: dilate, then erode) or opening (`distance < 0`:
/// erode, then dilate) by a ball of radius `|distance|` (VOLUME.md section 8.3), the radius
/// limited to one voxel less than the narrower band.
pub fn reshape(grid: &SparseGrid<f32>, distance: f64, band: Band) -> SparseGrid<f32> {
    // the shifted field must keep its zero level set inside the band: the radius is limited to
    // one voxel less than the narrower side of the band
    let limit = (band.exterior.min(band.interior) - grid.voxel_size()).max(0.0);
    let distance = distance.clamp(-limit, limit);
    if distance == 0.0 {
        return grid.clone();
    }
    let d = distance as f32;
    let first = redistance(&grid.map(grid.background() - d, |v| v - d), band);
    let back = first.map(first.background() + d, |v| v + d);
    redistance(&back, band)
}

/// `a + t * (b - a)` on the active voxels of `a`.
fn blend(a: &SparseGrid<f32>, b: &SparseGrid<f32>, t: f64) -> SparseGrid<f32> {
    let mut out = a.clone();
    for (i, v) in a.active_voxels() {
        let w = b.get(i);
        out.set(i, v + (w - v) * t as f32);
    }
    out
}

/// The smoothing operators of a filter layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmoothOp {
    Gaussian,
    Mean,
    Median,
    Laplacian,
}

/// What a filter layer does (VOLUME.md section 8).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterKind {
    Offset {
        distance: f64,
    },
    Reshape {
        distance: f64,
    },
    Smooth {
        op: SmoothOp,
        width: u32,
        strength: f64,
    },
}

/// Applies a filter layer `iterations` times, each step followed by the re-distance.
pub fn apply(
    grid: &SparseGrid<f32>,
    kind: FilterKind,
    iterations: u32,
    band: Band,
) -> SparseGrid<f32> {
    let mut g = grid.clone();
    for _ in 0..iterations.max(1) {
        g = match kind {
            FilterKind::Offset { distance } => offset(&g, distance, band),
            FilterKind::Reshape { distance } => reshape(&g, distance, band),
            FilterKind::Smooth {
                op,
                width,
                strength,
            } => {
                let filtered = match op {
                    SmoothOp::Mean => mean(&g, width, 1),
                    SmoothOp::Median => median(&g, width, 1),
                    SmoothOp::Laplacian => laplacian_flow(&g, 1.0, 1),
                    SmoothOp::Gaussian => {
                        let mut f = g.clone();
                        for w in (1..=2 * width).rev() {
                            f = mean(&f, w, 1);
                        }
                        f
                    }
                };
                redistance(&blend(&g, &filtered, strength.clamp(0.0, 1.0)), band)
            }
        };
    }
    g
}

/// How a field layer combines with the distance field below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldMode {
    /// The field grows the volume: `value - K t`.
    Add,
    /// The field shrinks it: `value + K t`.
    Subtract,
    /// The field scales the distances: `value (1 + t)` (the surface does not move).
    Multiply,
}

/// Field strength to distance: `K = FIELD_DISTANCE_PER_VOXEL * voxel_size` at field value 1 above
/// the threshold [`FIELD_THRESHOLD`] (VOLUME.md section 9, O33 field-layer).
pub const FIELD_DISTANCE_PER_VOXEL: f64 = 0.625;
/// Field values below this are ignored; `t = (f - threshold) / (1 - threshold)` above it.
pub const FIELD_THRESHOLD: f64 = 0.5;

/// Applies sampled field values to the given active voxels: `t = max(0, (f - 0.5) / 0.5)`
/// times `strength`, then the mode.
pub fn apply_field_values(
    grid: &SparseGrid<f32>,
    voxels: &[(IVec3, f32)],
    values: &[f64],
    mode: FieldMode,
    strength: f64,
    voxel_size: f64,
) -> SparseGrid<f32> {
    let k = FIELD_DISTANCE_PER_VOXEL * voxel_size;
    let mut out = grid.clone();
    for ((i, v), f) in voxels.iter().zip(values) {
        let t = ((f - FIELD_THRESHOLD) / (1.0 - FIELD_THRESHOLD)).max(0.0) * strength;
        let value = f64::from(*v);
        let new = match mode {
            FieldMode::Add => value - k * t,
            FieldMode::Subtract => value + k * t,
            FieldMode::Multiply => value * (1.0 + t),
        };
        out.set(*i, new as f32);
    }
    out
}
