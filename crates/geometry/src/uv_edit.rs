//! UV charts, least-squares conformal relaxation and skyline packing (UV.md U7).

use std::collections::{BTreeMap, BTreeSet};

use glam::{DVec2, DVec3};
use nalgebra_sparse::{CooMatrix, CsrMatrix};

use crate::CornerUvs;
use crate::mesh::{Edge, MeshError, PolyMesh};
use crate::triangulate::triangulate_poly;

#[derive(Debug, thiserror::Error)]
pub enum UvEditError {
    #[error(transparent)]
    Mesh(#[from] MeshError),
    #[error("UVs do not match the mesh or contain a non-finite coordinate")]
    Layout,
    #[error("a closed chart needs a seam before relaxing")]
    Closed,
    #[error("a chart is degenerate or the conformal solve did not converge")]
    Degenerate,
    #[error("islands do not fit with that padding")]
    Padding,
}

struct Groups(Vec<usize>);

impl Groups {
    fn new(n: usize) -> Self {
        Self((0..n).collect())
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.0[i] != i {
            self.0[i] = self.0[self.0[i]];
            i = self.0[i];
        }
        i
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        self.0[a.max(b)] = a.min(b);
    }
}

#[derive(Default)]
struct Chart {
    faces: Vec<usize>,
    boundary: BTreeSet<usize>,
}

struct Charts {
    islands: Vec<Chart>,
    starts: Vec<usize>,
    points: Vec<u32>,
    shared: Vec<usize>,
}

#[derive(Clone, Copy)]
struct EdgeUse {
    face: usize,
    corners: [usize; 2],
}

fn charts(
    mesh: &PolyMesh,
    uv: Option<&CornerUvs>,
    seams: &BTreeSet<u32>,
) -> Result<Charts, UvEditError> {
    mesh.validate()?;
    if uv.is_some_and(|uv| {
        !uv.matches(mesh)
            || uv
                .corners
                .iter()
                .flatten()
                .flatten()
                .any(|x| !x.is_finite())
    }) {
        return Err(UvEditError::Layout);
    }
    let coords: Option<Vec<_>> = uv.map(|u| {
        u.corners
            .iter()
            .flatten()
            .copied()
            .map(DVec2::from)
            .collect()
    });
    let mut starts = Vec::with_capacity(mesh.polys.len());
    let mut points = Vec::new();
    let mut edges: BTreeMap<Edge, Vec<EdgeUse>> = BTreeMap::new();
    for (face, poly) in mesh.polys.iter().enumerate() {
        let start = points.len();
        starts.push(start);
        points.extend_from_slice(poly);
        for j in 0..poly.len() {
            let k = (j + 1) % poly.len();
            let edge = Edge::new(poly[j], poly[k]);
            let corners = if poly[j] == edge.0 { [j, k] } else { [k, j] };
            edges.entry(edge).or_default().push(EdgeUse {
                face,
                corners: corners.map(|i| start + i),
            });
        }
    }
    let mut faces = Groups::new(mesh.polys.len());
    let mut corners = Groups::new(points.len());
    let mut border = Vec::new();
    for (id, edge) in mesh.edges().into_iter().enumerate() {
        let owners = &edges[&edge];
        if let [a, b] = owners.as_slice()
            && !seams.contains(&(id as u32))
            && coords.as_ref().is_none_or(|uv| {
                a.corners
                    .into_iter()
                    .zip(b.corners)
                    .all(|(i, j)| uv[i].abs_diff_eq(uv[j], 1e-9))
            })
        {
            faces.join(a.face, b.face);
            for (i, j) in a.corners.into_iter().zip(b.corners) {
                corners.join(i, j);
            }
        } else {
            border.extend_from_slice(owners);
        }
    }
    let mut islands = BTreeMap::<usize, Chart>::new();
    for f in 0..mesh.polys.len() {
        islands.entry(faces.find(f)).or_default().faces.push(f);
    }
    for edge in border {
        let chart = islands
            .get_mut(&faces.find(edge.face))
            .expect("face assigned above");
        chart.boundary.extend(edge.corners.map(|i| corners.find(i)));
    }
    Ok(Charts {
        islands: islands.into_values().collect(),
        starts,
        shared: (0..points.len()).map(|i| corners.find(i)).collect(),
        points,
    })
}

/// Solve min ||Ax-b|| by CGLS; multiply A and Aᵀ from the same sparse rows.
fn least_squares(matrix: &CsrMatrix<f64>, rhs: &[f64]) -> Result<Vec<f64>, UvEditError> {
    let n = matrix.ncols();
    let rows = matrix.row_offsets();
    let col = matrix.col_indices();
    let x = matrix.values();
    let mut norms = vec![0.0_f64; n];
    for (&j, &a) in col.iter().zip(x) {
        norms[j] += a * a;
    }
    if norms.iter().any(|&v| v <= 0.0 || !v.is_finite()) {
        return Err(UvEditError::Degenerate);
    }
    let scaled: Vec<_> = x
        .iter()
        .zip(col)
        .map(|(&a, &j)| a / norms[j].sqrt())
        .collect();
    let times = |v: &[f64], out: &mut [f64]| {
        for (y, row) in out.iter_mut().zip(rows.windows(2)) {
            *y = (row[0]..row[1]).map(|k| scaled[k] * v[col[k]]).sum();
        }
    };
    let transpose = |v: &[f64], out: &mut [f64]| {
        out.fill(0.0);
        for (v, row) in v.iter().zip(rows.windows(2)) {
            for k in row[0]..row[1] {
                out[col[k]] += scaled[k] * v;
            }
        }
    };
    let mut sol = vec![0.0; n];
    let mut residual = rhs.to_vec();
    let mut gradient = vec![0.0; n];
    transpose(&residual, &mut gradient);
    let squared = |x: &[f64]| x.iter().map(|x| x * x).sum::<f64>();
    let mut step = gradient.clone();
    let mut g = squared(&gradient);
    let threshold = (g * 1e-22).max(1e-30);
    let mut q = vec![0.0; rhs.len()];
    for _ in 0..(n.saturating_mul(8).clamp(128, 20_000)) {
        if g <= threshold {
            return Ok(sol
                .into_iter()
                .zip(norms)
                .map(|(x, d)| x / d.sqrt())
                .collect());
        }
        times(&step, &mut q);
        let q2 = squared(&q);
        if q2 <= 0.0 || !q2.is_finite() {
            break;
        }
        let a = g / q2;
        for (sol, s) in sol.iter_mut().zip(&step) {
            *sol += a * s;
        }
        for (r, q) in residual.iter_mut().zip(&q) {
            *r -= a * q;
        }
        transpose(&residual, &mut gradient);
        let new = squared(&gradient);
        let b = new / g;
        for (p, s) in step.iter_mut().zip(&gradient) {
            *p = *s + b * *p;
        }
        g = new;
    }
    Err(UvEditError::Degenerate)
}

fn solve_chart(
    mesh: &PolyMesh,
    charts: &Charts,
    chart: &Chart,
    out: &mut CornerUvs,
) -> Result<(), UvEditError> {
    let world = |id: usize| mesh.points[charts.points[id] as usize];
    let from = chart.boundary.first().copied().ok_or(UvEditError::Closed)?;
    let far = |v| {
        chart
            .boundary
            .iter()
            .copied()
            .max_by(|&a, &b| {
                world(a)
                    .distance_squared(world(v))
                    .total_cmp(&world(b).distance_squared(world(v)))
            })
            .unwrap_or(v)
    };
    let a = far(from);
    let b = far(a);
    let width = world(a).distance(world(b));
    if !width.is_finite() || width <= 1e-12 {
        return Err(UvEditError::Degenerate);
    }
    let mut free = BTreeMap::new();
    let get = |f: usize, c: usize| charts.shared[charts.starts[f] + c];
    for &f in &chart.faces {
        for c in 0..mesh.polys[f].len() {
            let v = get(f, c);
            if v != a && v != b {
                let next = free.len() * 2;
                free.entry(v).or_insert(next);
            }
        }
    }
    let mut equations = Vec::new();
    for &f in &chart.faces {
        let poly = &mesh.polys[f];
        for tri in triangulate_poly(&mesh.points, poly) {
            let ids = tri.map(|i| get(f, poly.iter().position(|&v| v == i).expect("corner")));
            let [o, p, q] = ids.map(world);
            if let Some(grad) = gradients(o, p, q) {
                equations.push((ids, grad));
            }
        }
    }
    if equations.is_empty() {
        return Err(UvEditError::Degenerate);
    }
    let mut coo = CooMatrix::new(equations.len() * 2, free.len() * 2);
    let mut rhs = vec![0.0; equations.len() * 2];
    for (t, (ids, grad)) in equations.into_iter().enumerate() {
        for (id, [gx, gy]) in ids.into_iter().zip(grad) {
            if id == b {
                rhs[t * 2] -= gx * width;
                rhs[t * 2 + 1] -= gy * width;
            } else if let Some(&j) = free.get(&id) {
                coo.push(t * 2, j, gx);
                coo.push(t * 2, j + 1, -gy);
                coo.push(t * 2 + 1, j, gy);
                coo.push(t * 2 + 1, j + 1, gx);
            }
        }
    }
    let sol = least_squares(&CsrMatrix::from(&coo), &rhs)?;
    for &f in &chart.faces {
        for (c, result) in out.corners[f].iter_mut().enumerate() {
            let id = get(f, c);
            *result = if id == a {
                [0.0, 0.0]
            } else if id == b {
                [width, 0.0]
            } else {
                let j = free[&id];
                [sol[j], sol[j + 1]]
            };
        }
    }
    Ok(())
}

fn gradients(o: DVec3, p: DVec3, q: DVec3) -> Option<[[f64; 2]; 3]> {
    let (e, f) = (p - o, q - o);
    let len = e.length();
    let area2 = e.cross(f).length();
    if !area2.is_finite() || area2 <= 1e-18 || len <= 1e-12 {
        return None;
    }
    let x = e.dot(f) / len;
    let y = area2 / len;
    let w = (0.5 / area2).sqrt(); // area-weighted barycentric gradients
    Some([[-y * w, (x - len) * w], [y * w, -x * w], [0.0, len * w]])
}

pub fn relax(
    mesh: &PolyMesh,
    existing: Option<&CornerUvs>,
    seams: &BTreeSet<u32>,
) -> Result<CornerUvs, UvEditError> {
    let layout = charts(mesh, existing, seams)?;
    let mut uv = CornerUvs {
        corners: mesh.polys.iter().map(|p| vec![[0.0; 2]; p.len()]).collect(),
    };
    for chart in &layout.islands {
        solve_chart(mesh, &layout, chart, &mut uv)?;
    }
    Ok(uv)
}

/// Skyline segments are [left, right, height], ordered left-to-right.
fn skyline(sizes: &[DVec2], order: &[usize], padding: f64, scale: f64) -> Option<Vec<DVec2>> {
    let mut skyline = vec![[0.0, 1.0, 0.0_f64]];
    let mut positions = vec![DVec2::ZERO; sizes.len()];
    for &id in order {
        let [w, h] = (sizes[id] * scale + DVec2::splat(2.0 * padding)).to_array();
        let mut best: Option<[f64; 2]> = None;
        for [x, _, _] in &skyline {
            let end = x + w;
            if end > 1.0 {
                break;
            }
            let y = skyline
                .iter()
                .filter(|[l, r, _]| *r > *x && *l < end)
                .map(|v| v[2])
                .fold(0.0, f64::max);
            if y + h <= 1.0 && best.is_none_or(|b| y < b[1]) {
                best = Some([*x, y]);
            }
        }
        let [x, y] = best?;
        let end = x + w;
        positions[id] = DVec2::new(x + padding, y + padding);
        let mut next = Vec::with_capacity(skyline.len() + 2);
        for [l, r, z] in skyline {
            if l < x {
                next.push([l, r.min(x), z]);
            }
            if r > end {
                next.push([l.max(end), r, z]);
            }
        }
        next.push([x, end, y + h]);
        next.sort_by(|a, b| a[0].total_cmp(&b[0]));
        skyline = next;
    }
    Some(positions)
}

pub fn pack(
    mesh: &PolyMesh,
    uv: &CornerUvs,
    seams: &BTreeSet<u32>,
    padding: f64,
) -> Result<CornerUvs, UvEditError> {
    if !padding.is_finite() || !(0.0..0.5).contains(&padding) {
        return Err(UvEditError::Padding);
    }
    let padding = padding.max(1e-10);
    let layout = charts(mesh, Some(uv), seams)?;
    let mut lows = Vec::new();
    let mut sizes = Vec::new();
    for chart in &layout.islands {
        let (mut lo, mut hi) = (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY));
        for point in chart.faces.iter().flat_map(|&f| &uv.corners[f]).copied() {
            let p = DVec2::from(point);
            lo = lo.min(p);
            hi = hi.max(p);
        }
        lows.push(lo);
        sizes.push(hi - lo);
    }
    let mut order: Vec<_> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| {
        sizes[b]
            .y
            .total_cmp(&sizes[a].y)
            .then(sizes[b].x.total_cmp(&sizes[a].x))
            .then(a.cmp(&b))
    });
    let extent = sizes.iter().map(|s| s.max_element()).fold(0.0, f64::max);
    let mut positions = skyline(&sizes, &order, padding, 0.0).ok_or(UvEditError::Padding)?;
    let (mut low, mut high) = (0.0, (1.0 - 2.0 * padding) / extent.max(1e-15));
    for _ in 0..42 {
        let scale = (low + high) * 0.5;
        if let Some(fit) = skyline(&sizes, &order, padding, scale) {
            low = scale;
            positions = fit;
        } else {
            high = scale;
        }
    }
    let mut out = uv.clone();
    for (i, chart) in layout.islands.iter().enumerate() {
        for &f in &chart.faces {
            for p in &mut out.corners[f] {
                *p = ((DVec2::from(*p) - lows[i]) * low + positions[i]).to_array();
            }
        }
    }
    Ok(out)
}
