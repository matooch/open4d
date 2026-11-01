//! Soft selection and symmetry for component transforms (docs/specs/EDITABLE.md §5).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use forge_geometry::PolyMesh;
use glam::{DMat4, DVec3};
use serde_json::{Map, Number, Value as Json, json};

/// Largest radius and tolerance accepted.
pub const MAX_RANGE: f64 = 1e9;
/// Shortest and longest falloff curve.
pub const CURVE_LEN: std::ops::RangeInclusive<usize> = 2..=64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Falloff {
    Linear,
    Dome,
    Bell,
    Spline,
}

impl Falloff {
    pub const ALL: [Falloff; 4] = [
        Falloff::Linear,
        Falloff::Dome,
        Falloff::Bell,
        Falloff::Spline,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Falloff::Linear => "linear",
            Falloff::Dome => "dome",
            Falloff::Bell => "bell",
            Falloff::Spline => "spline",
        }
    }

    pub fn from_id(id: &str) -> Option<Falloff> {
        Falloff::ALL.into_iter().find(|f| f.id() == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distance {
    Volume,
    Surface,
}

impl Distance {
    pub const ALL: [Distance; 2] = [Distance::Volume, Distance::Surface];

    pub fn id(self) -> &'static str {
        match self {
            Distance::Volume => "volume",
            Distance::Surface => "surface",
        }
    }

    pub fn from_id(id: &str) -> Option<Distance> {
        Distance::ALL.into_iter().find(|d| d.id() == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    pub fn id(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }

    pub fn from_id(id: &str) -> Option<Axis> {
        Axis::ALL.into_iter().find(|a| a.id() == id)
    }

    fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }
}

/// Soft selection settings (EDITABLE.md §5.1).
#[derive(Debug, Clone, PartialEq)]
pub struct SoftSelection {
    pub enabled: bool,
    pub radius: f64,
    pub falloff: Falloff,
    pub distance: Distance,
    pub curve: Vec<f64>,
}

impl Default for SoftSelection {
    fn default() -> Self {
        SoftSelection {
            enabled: false,
            radius: 100.0,
            falloff: Falloff::Linear,
            distance: Distance::Volume,
            curve: vec![1.0, 0.896, 0.5, 0.104, 0.0],
        }
    }
}

/// Symmetry settings (EDITABLE.md §5.1).
#[derive(Debug, Clone, PartialEq)]
pub struct Symmetry {
    pub enabled: bool,
    pub axis: Axis,
    pub offset: f64,
    pub tolerance: f64,
}

impl Default for Symmetry {
    fn default() -> Self {
        Symmetry {
            enabled: false,
            axis: Axis::X,
            offset: 0.0,
            tolerance: 0.1,
        }
    }
}

/// The editor's component transform settings.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ComponentSettings {
    pub soft: SoftSelection,
    pub symmetry: Symmetry,
}

fn number(key: &str, v: &Json) -> Result<f64, String> {
    v.as_f64()
        .filter(|f| f.is_finite())
        .ok_or_else(|| format!("{key} must be a number"))
}

fn text<'a>(key: &str, v: &'a Json) -> Result<&'a str, String> {
    v.as_str().ok_or_else(|| format!("{key} must be a string"))
}

fn flag(key: &str, v: &Json) -> Result<bool, String> {
    v.as_bool()
        .ok_or_else(|| format!("{key} must be true or false"))
}

fn unknown(key: &str) -> String {
    format!("unknown key {key}")
}

impl SoftSelection {
    /// Checks the values (radius, curve).
    pub fn validate(&self) -> Result<(), String> {
        if !(self.radius >= 0.001 && self.radius <= MAX_RANGE) {
            return Err("radius must be in 0.001..1e9".into());
        }
        if !CURVE_LEN.contains(&self.curve.len()) {
            return Err("curve needs 2 to 64 values".into());
        }
        if self.curve.iter().any(|c| !(0.0..=1.0).contains(c)) {
            return Err("curve values must be in 0..1".into());
        }
        Ok(())
    }

    /// Sets the keys present in `obj`; nothing changes on an error.
    pub fn apply_json(&mut self, obj: &Map<String, Json>) -> Result<(), String> {
        let mut next = self.clone();
        for (key, v) in obj {
            match key.as_str() {
                "enabled" => next.enabled = flag(key, v)?,
                "radius" => next.radius = number(key, v)?,
                "falloff" => {
                    next.falloff = Falloff::from_id(text(key, v)?)
                        .ok_or("falloff must be linear, dome, bell or spline")?
                }
                "distance" => {
                    next.distance = Distance::from_id(text(key, v)?)
                        .ok_or("distance must be volume or surface")?
                }
                "curve" => {
                    let a = v.as_array().ok_or("curve must be a list of numbers")?;
                    next.curve = a.iter().map(|c| number(key, c)).collect::<Result<_, _>>()?;
                }
                _ => return Err(unknown(key)),
            }
        }
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn to_json(&self) -> Json {
        json!({"enabled": self.enabled, "radius": self.radius, "falloff": self.falloff.id(),
               "distance": self.distance.id(), "curve": self.curve})
    }

    /// The weight at normalised distance `t` (EDITABLE.md §5.2).
    pub fn weight_at(&self, t: f64) -> f64 {
        if t >= 1.0 {
            return 0.0;
        }
        let t = t.max(0.0);
        match self.falloff {
            Falloff::Linear => 1.0 - t,
            Falloff::Dome => (1.0 - t * t).sqrt(),
            Falloff::Bell => 0.5 * (1.0 + (std::f64::consts::PI * t).cos()),
            Falloff::Spline => sample_curve(&self.curve, t),
        }
    }
}

/// Catmull-Rom through `c` at equally spaced `t` in 0..1, clamped to 0..1.
fn sample_curve(c: &[f64], t: f64) -> f64 {
    let n = c.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return c[0];
    }
    let x = t.clamp(0.0, 1.0) * (n - 1) as f64;
    let i = (x.floor() as usize).min(n - 2);
    let u = x - i as f64;
    let at = |k: isize| c[k.clamp(0, n as isize - 1) as usize];
    let (p0, p1, p2, p3) = (
        at(i as isize - 1),
        at(i as isize),
        at(i as isize + 1),
        at(i as isize + 2),
    );
    let v = 0.5
        * (2.0 * p1
            + (p2 - p0) * u
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u * u
            + (3.0 * p1 - p0 - 3.0 * p2 + p3) * u * u * u);
    v.clamp(0.0, 1.0)
}

impl Symmetry {
    pub fn validate(&self) -> Result<(), String> {
        if !self.offset.is_finite() || self.offset.abs() > MAX_RANGE {
            return Err("offset must be a finite number".into());
        }
        if !(0.0..=1e6).contains(&self.tolerance) {
            return Err("tolerance must be in 0..1e6".into());
        }
        Ok(())
    }

    /// Sets the keys present in `obj`; nothing changes on an error.
    pub fn apply_json(&mut self, obj: &Map<String, Json>) -> Result<(), String> {
        let mut next = self.clone();
        for (key, v) in obj {
            match key.as_str() {
                "enabled" => next.enabled = flag(key, v)?,
                "axis" => {
                    next.axis = Axis::from_id(text(key, v)?).ok_or("axis must be x, y or z")?
                }
                "offset" => next.offset = number(key, v)?,
                "tolerance" => next.tolerance = number(key, v)?,
                _ => return Err(unknown(key)),
            }
        }
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn to_json(&self) -> Json {
        json!({"enabled": self.enabled, "axis": self.axis.id(), "offset": self.offset,
               "tolerance": Json::Number(Number::from_f64(self.tolerance).unwrap_or(0.into()))})
    }

    fn normal(&self) -> DVec3 {
        let mut n = DVec3::ZERO;
        n[self.axis.index()] = 1.0;
        n
    }

    /// The mirror image of `p`.
    pub fn reflect(&self, p: DVec3) -> DVec3 {
        let n = self.normal();
        p - 2.0 * (p.dot(n) - self.offset) * n
    }

    /// The linear part of the reflection (for displacements).
    pub fn reflect_vector(&self, d: DVec3) -> DVec3 {
        let n = self.normal();
        d - 2.0 * d.dot(n) * n
    }
}

/// Min-heap entry for the surface distance.
struct Item(f64, u32);

impl PartialEq for Item {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Item {}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0).then_with(|| o.1.cmp(&self.1))
    }
}

/// Distance from every point to the nearest of `selected` (`f64::INFINITY` when unreachable),
/// capped searches stop beyond `limit`.
pub fn distances(mesh: &PolyMesh, selected: &[u32], mode: Distance, limit: f64) -> Vec<f64> {
    let n = mesh.points.len();
    let sources: Vec<u32> = selected
        .iter()
        .copied()
        .filter(|i| (*i as usize) < n)
        .collect();
    let mut dist = vec![f64::INFINITY; n];
    match mode {
        Distance::Volume => {
            for (i, p) in mesh.points.iter().enumerate() {
                dist[i] = sources
                    .iter()
                    .map(|s| p.distance(mesh.points[*s as usize]))
                    .fold(f64::INFINITY, f64::min);
            }
        }
        Distance::Surface => {
            let mut adj: Vec<Vec<(u32, f64)>> = vec![Vec::new(); n];
            for e in mesh.edges() {
                let (a, b) = (e.0 as usize, e.1 as usize);
                if a < n && b < n {
                    let len = mesh.points[a].distance(mesh.points[b]);
                    adj[a].push((e.1, len));
                    adj[b].push((e.0, len));
                }
            }
            let mut heap = BinaryHeap::new();
            for &s in &sources {
                dist[s as usize] = 0.0;
                heap.push(Item(0.0, s));
            }
            while let Some(Item(d, i)) = heap.pop() {
                if d > dist[i as usize] || d > limit {
                    continue;
                }
                for &(j, len) in &adj[i as usize] {
                    let nd = d + len;
                    if nd < dist[j as usize] {
                        dist[j as usize] = nd;
                        heap.push(Item(nd, j));
                    }
                }
            }
        }
    }
    dist
}

/// Soft selection weights per point (EDITABLE.md §5.2); selected points get 1. With soft
/// selection off, only the selected points have a weight.
pub fn weights(mesh: &PolyMesh, selected: &[u32], soft: &SoftSelection) -> Vec<f64> {
    let n = mesh.points.len();
    let mut w = vec![0.0; n];
    if soft.enabled {
        let d = distances(mesh, selected, soft.distance, soft.radius);
        for (wi, di) in w.iter_mut().zip(&d) {
            if di.is_finite() {
                *wi = soft.weight_at(di / soft.radius);
            }
        }
    }
    for &s in selected {
        if let Some(wi) = w.get_mut(s as usize) {
            *wi = 1.0;
        }
    }
    w
}

/// For every point the point nearest to its mirror image within the tolerance (EDITABLE.md
/// §5.4); ties go to the lowest index.
pub fn partners(mesh: &PolyMesh, sym: &Symmetry) -> Vec<Option<u32>> {
    let cell = sym.tolerance.max(1e-6);
    let key = |p: DVec3| {
        [
            (p.x / cell).floor() as i64,
            (p.y / cell).floor() as i64,
            (p.z / cell).floor() as i64,
        ]
    };
    let mut grid: HashMap<[i64; 3], Vec<u32>> = HashMap::new();
    for (i, p) in mesh.points.iter().enumerate() {
        grid.entry(key(*p)).or_default().push(i as u32);
    }
    mesh.points
        .iter()
        .map(|p| {
            let r = sym.reflect(*p);
            let k = key(r);
            let mut best: Option<(f64, u32)> = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let Some(list) = grid.get(&[k[0] + dx, k[1] + dy, k[2] + dz]) else {
                            continue;
                        };
                        for &q in list {
                            let d = mesh.points[q as usize].distance(r);
                            let better = best.is_none_or(|(bd, bq)| d < bd || (d == bd && q < bq));
                            if d <= sym.tolerance && better {
                                best = Some((d, q));
                            }
                        }
                    }
                }
            }
            best.map(|(_, q)| q)
        })
        .collect()
}

/// Weights and mirror partners fixed for one edit (a whole drag): everything that depends
/// only on the start mesh and the selection (EDITABLE.md §5.6).
#[derive(Debug, Clone)]
pub struct ComponentPlan {
    weights: Vec<f64>,
    symmetry: Option<(Symmetry, Vec<Option<u32>>)>,
}

impl ComponentPlan {
    pub fn new(mesh: &PolyMesh, selected: &[u32], settings: &ComponentSettings) -> ComponentPlan {
        ComponentPlan {
            weights: weights(mesh, selected, &settings.soft),
            symmetry: settings.symmetry.enabled.then(|| {
                (
                    settings.symmetry.clone(),
                    partners(mesh, &settings.symmetry),
                )
            }),
        }
    }

    /// The weight of each point.
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    /// The partner of each point, when symmetry is on.
    pub fn partners(&self) -> Option<&[Option<u32>]> {
        self.symmetry.as_ref().map(|(_, p)| p.as_slice())
    }

    /// `points` after the matrix `m`, blended by the weights and mirrored (§5.3, §5.4).
    pub fn apply(&self, points: &[DVec3], m: DMat4) -> Vec<DVec3> {
        let d_a = |i: usize| self.weights[i] * (m.transform_point3(points[i]) - points[i]);
        (0..points.len().min(self.weights.len()))
            .map(|i| {
                let d = match &self.symmetry {
                    None => d_a(i),
                    Some((sym, partner)) => match partner[i] {
                        None => d_a(i),
                        Some(q) if q as usize == i => {
                            let d = d_a(i);
                            (d + sym.reflect_vector(d)) * 0.5
                        }
                        Some(q) => {
                            let (wp, wq) = (self.weights[i], self.weights[q as usize]);
                            let own =
                                wp > wq + 1e-12 || ((wp - wq).abs() <= 1e-12 && i < q as usize);
                            if own {
                                d_a(i)
                            } else {
                                sym.reflect_vector(d_a(q as usize))
                            }
                        }
                    },
                };
                points[i] + d
            })
            .collect()
    }
}

/// One-shot form of [`ComponentPlan`]: the new positions of all points.
pub fn transform_components(
    mesh: &PolyMesh,
    selected: &[u32],
    m: DMat4,
    settings: &ComponentSettings,
) -> Vec<DVec3> {
    ComponentPlan::new(mesh, selected, settings).apply(&mesh.points, m)
}
