//! Snapping 2.0: settings, targets, priority rules and the world-space solver
//! (docs/specs/MANIPULATE.md §6). The screen-space resolution lives in `forge-viewport`.
//! Ref: Ericson, *Real-Time Collision Detection* (2005) §5.1.

use std::cmp::Ordering;

use forge_geometry::PolyMesh;
use forge_geometry::triangulate::triangulate;
use glam::{DMat3, DVec3};
use serde_json::{Map, Value as Json, json};

/// Largest radius, spacing and step accepted.
pub const MAX_RANGE: f64 = 1e9;
/// Grid spacing used by headless queries when `grid_spacing` is automatic.
pub const AUTO_GRID_HEADLESS: f64 = 100.0;

/// What a snap landed on. Declaration order is the tie-break order of the same rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SnapKind {
    Vertex,
    EdgeMidpoint,
    PolygonCenter,
    ObjectAxis,
    Edge,
    Polygon,
    GridPoint,
    GridLine,
    Workplane,
}

impl SnapKind {
    pub const ALL: [SnapKind; 9] = [
        SnapKind::Vertex,
        SnapKind::EdgeMidpoint,
        SnapKind::PolygonCenter,
        SnapKind::ObjectAxis,
        SnapKind::Edge,
        SnapKind::Polygon,
        SnapKind::GridPoint,
        SnapKind::GridLine,
        SnapKind::Workplane,
    ];

    /// Priority class: lower wins (points, edge, polygon, grid point, grid line, workplane).
    pub fn rank(self) -> u8 {
        match self {
            SnapKind::Vertex
            | SnapKind::EdgeMidpoint
            | SnapKind::PolygonCenter
            | SnapKind::ObjectAxis => 0,
            SnapKind::Edge => 1,
            SnapKind::Polygon => 2,
            SnapKind::GridPoint => 3,
            SnapKind::GridLine => 4,
            SnapKind::Workplane => 5,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            SnapKind::Vertex => "vertex",
            SnapKind::EdgeMidpoint => "edge_midpoint",
            SnapKind::PolygonCenter => "polygon_center",
            SnapKind::ObjectAxis => "object_axis",
            SnapKind::Edge => "edge",
            SnapKind::Polygon => "polygon",
            SnapKind::GridPoint => "grid_point",
            SnapKind::GridLine => "grid_line",
            SnapKind::Workplane => "workplane",
        }
    }

    pub fn from_id(id: &str) -> Option<SnapKind> {
        SnapKind::ALL.into_iter().find(|k| k.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            SnapKind::Vertex => "Vertex",
            SnapKind::EdgeMidpoint => "Edge midpoint",
            SnapKind::PolygonCenter => "Polygon center",
            SnapKind::ObjectAxis => "Object axis",
            SnapKind::Edge => "Edge",
            SnapKind::Polygon => "Polygon",
            SnapKind::GridPoint => "Grid point",
            SnapKind::GridLine => "Grid line",
            SnapKind::Workplane => "Workplane",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapMode {
    D3,
    D2,
}

impl SnapMode {
    pub fn id(self) -> &'static str {
        match self {
            SnapMode::D3 => "3d",
            SnapMode::D2 => "2d",
        }
    }
}

/// The plane grid and workplane targets live on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Workplane {
    pub origin: DVec3,
    /// Unit length.
    pub normal: DVec3,
}

impl Default for Workplane {
    fn default() -> Self {
        Workplane {
            origin: DVec3::ZERO,
            normal: DVec3::Y,
        }
    }
}

impl Workplane {
    /// Tangent basis `(u, v)`: `u = normalize(Y × n)` (`X` when `n` is nearly `±Y`), `v = u × n`.
    pub fn basis(&self) -> (DVec3, DVec3) {
        let n = self.normal;
        let u = if n.dot(DVec3::Y).abs() > 0.999 {
            DVec3::X
        } else {
            DVec3::Y.cross(n).normalize()
        };
        (u, u.cross(n))
    }

    /// `p` moved along the normal onto the plane.
    pub fn project(&self, p: DVec3) -> DVec3 {
        p - self.normal * (p - self.origin).dot(self.normal)
    }

    /// Plane coordinates `(u, v)` of `p`.
    pub fn coords(&self, p: DVec3) -> (f64, f64) {
        let (u, v) = self.basis();
        ((p - self.origin).dot(u), (p - self.origin).dot(v))
    }

    pub fn at(&self, u: f64, v: f64) -> DVec3 {
        let (bu, bv) = self.basis();
        self.origin + bu * u + bv * v
    }

    /// The grid point nearest the projection of `p` (`spacing > 0`).
    pub fn grid_point(&self, p: DVec3, spacing: f64) -> DVec3 {
        let (u, v) = self.coords(p);
        self.at(
            (u / spacing).round() * spacing,
            (v / spacing).round() * spacing,
        )
    }

    /// The point of the grid line nearest the projection of `p` (`spacing > 0`): the plane
    /// coordinate with the smaller offset to a line is rounded, the other one is kept.
    pub fn grid_line(&self, p: DVec3, spacing: f64) -> DVec3 {
        let (u, v) = self.coords(p);
        let (ru, rv) = (
            (u / spacing).round() * spacing,
            (v / spacing).round() * spacing,
        );
        if (u - ru).abs() <= (v - rv).abs() {
            self.at(ru, v)
        } else {
            self.at(u, rv)
        }
    }
}

/// Which targets take part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Targets {
    pub vertex: bool,
    pub edge: bool,
    pub edge_midpoint: bool,
    pub polygon_center: bool,
    pub polygon: bool,
    pub grid_point: bool,
    pub grid_line: bool,
    pub workplane: bool,
    pub object_axis: bool,
}

impl Default for Targets {
    fn default() -> Self {
        Targets {
            vertex: true,
            edge: true,
            edge_midpoint: true,
            polygon_center: true,
            polygon: false,
            grid_point: false,
            grid_line: false,
            workplane: false,
            object_axis: false,
        }
    }
}

impl Targets {
    pub fn get(&self, kind: SnapKind) -> bool {
        match kind {
            SnapKind::Vertex => self.vertex,
            SnapKind::EdgeMidpoint => self.edge_midpoint,
            SnapKind::PolygonCenter => self.polygon_center,
            SnapKind::ObjectAxis => self.object_axis,
            SnapKind::Edge => self.edge,
            SnapKind::Polygon => self.polygon,
            SnapKind::GridPoint => self.grid_point,
            SnapKind::GridLine => self.grid_line,
            SnapKind::Workplane => self.workplane,
        }
    }

    pub fn set(&mut self, kind: SnapKind, on: bool) {
        match kind {
            SnapKind::Vertex => self.vertex = on,
            SnapKind::EdgeMidpoint => self.edge_midpoint = on,
            SnapKind::PolygonCenter => self.polygon_center = on,
            SnapKind::ObjectAxis => self.object_axis = on,
            SnapKind::Edge => self.edge = on,
            SnapKind::Polygon => self.polygon = on,
            SnapKind::GridPoint => self.grid_point = on,
            SnapKind::GridLine => self.grid_line = on,
            SnapKind::Workplane => self.workplane = on,
        }
    }
}

/// Quantize steps (MANIPULATE.md §6.6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantize {
    pub enabled: bool,
    /// Units.
    pub move_step: f64,
    /// Degrees.
    pub angle: f64,
    /// Percent.
    pub percent: f64,
}

impl Default for Quantize {
    fn default() -> Self {
        Quantize {
            enabled: false,
            move_step: 10.0,
            angle: 15.0,
            percent: 10.0,
        }
    }
}

/// The step sizes a drag uses: units, radians, scale-factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Steps {
    pub translate: f64,
    pub rotate: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnapSettings {
    pub enabled: bool,
    pub mode: SnapMode,
    pub radius: f64,
    pub visible_only: bool,
    pub targets: Targets,
    /// `0`: automatic.
    pub grid_spacing: f64,
    pub workplane: Workplane,
    pub quantize: Quantize,
}

impl Default for SnapSettings {
    fn default() -> Self {
        SnapSettings {
            enabled: false,
            mode: SnapMode::D3,
            radius: 10.0,
            visible_only: true,
            targets: Targets::default(),
            grid_spacing: 0.0,
            workplane: Workplane::default(),
            quantize: Quantize::default(),
        }
    }
}

/// The editor's transform helpers: snapping and axis mode. Not saved, not undoable.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TransformSettings {
    pub snap: SnapSettings,
    /// Gizmo drags edit the object's axis instead of the object (MANIPULATE.md §7.4).
    pub axis_mode: bool,
}

fn number(key: &str, v: &Json) -> Result<f64, String> {
    v.as_f64()
        .filter(|f| f.is_finite())
        .ok_or_else(|| format!("{key} must be a number"))
}

fn flag(key: &str, v: &Json) -> Result<bool, String> {
    v.as_bool()
        .ok_or_else(|| format!("{key} must be true or false"))
}

fn vec3(key: &str, v: &Json) -> Result<DVec3, String> {
    let a = v
        .as_array()
        .filter(|a| a.len() == 3)
        .ok_or_else(|| format!("{key} must be three numbers"))?;
    Ok(DVec3::new(
        number(key, &a[0])?,
        number(key, &a[1])?,
        number(key, &a[2])?,
    ))
}

fn object<'a>(key: &str, v: &'a Json) -> Result<&'a Map<String, Json>, String> {
    v.as_object()
        .ok_or_else(|| format!("{key} must be an object"))
}

impl SnapSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !(1.0..=100.0).contains(&self.radius) {
            return Err("radius must be in 1..100".into());
        }
        if !(0.0..=MAX_RANGE).contains(&self.grid_spacing) {
            return Err("grid_spacing must be in 0..1e9".into());
        }
        let q = &self.quantize;
        for (name, v) in [
            ("move", q.move_step),
            ("angle", q.angle),
            ("percent", q.percent),
        ] {
            if !(v > 0.0 && v <= MAX_RANGE) {
                return Err(format!("quantize.{name} must be greater than 0"));
            }
        }
        if (self.workplane.normal.length() - 1.0).abs() > 1e-9 {
            return Err("workplane.normal must not be zero".into());
        }
        Ok(())
    }

    /// Sets the keys present in `obj`; nothing changes on an error.
    pub fn apply_json(&mut self, obj: &Map<String, Json>) -> Result<(), String> {
        let mut next = self.clone();
        for (key, v) in obj {
            match key.as_str() {
                "enabled" => next.enabled = flag(key, v)?,
                "mode" => {
                    next.mode = match v.as_str() {
                        Some("3d") => SnapMode::D3,
                        Some("2d") => SnapMode::D2,
                        _ => return Err("mode must be 3d or 2d".into()),
                    }
                }
                "radius" => next.radius = number(key, v)?,
                "visible_only" => next.visible_only = flag(key, v)?,
                "grid_spacing" => next.grid_spacing = number(key, v)?,
                "targets" => {
                    for (k, on) in object(key, v)? {
                        let kind =
                            SnapKind::from_id(k).ok_or_else(|| format!("unknown target {k}"))?;
                        next.targets.set(kind, flag(k, on)?);
                    }
                }
                "workplane" => {
                    for (k, val) in object(key, v)? {
                        match k.as_str() {
                            "origin" => next.workplane.origin = vec3("workplane.origin", val)?,
                            "normal" => {
                                let n = vec3("workplane.normal", val)?;
                                if n.length() < 1e-9 {
                                    return Err("workplane.normal must not be zero".into());
                                }
                                next.workplane.normal = n.normalize();
                            }
                            _ => return Err(format!("unknown key workplane.{k}")),
                        }
                    }
                }
                "quantize" => {
                    for (k, val) in object(key, v)? {
                        match k.as_str() {
                            "enabled" => next.quantize.enabled = flag("quantize.enabled", val)?,
                            "move" => next.quantize.move_step = number("quantize.move", val)?,
                            "angle" => next.quantize.angle = number("quantize.angle", val)?,
                            "percent" => next.quantize.percent = number("quantize.percent", val)?,
                            _ => return Err(format!("unknown key quantize.{k}")),
                        }
                    }
                }
                _ => return Err(format!("unknown key {key}")),
            }
        }
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn to_json(&self) -> Json {
        let mut targets = Map::new();
        for k in SnapKind::ALL {
            targets.insert(k.id().into(), json!(self.targets.get(k)));
        }
        let w = &self.workplane;
        let q = &self.quantize;
        json!({
            "enabled": self.enabled,
            "mode": self.mode.id(),
            "radius": self.radius,
            "visible_only": self.visible_only,
            "targets": targets,
            "grid_spacing": self.grid_spacing,
            "workplane": {"origin": w.origin.to_array(), "normal": w.normal.to_array()},
            "quantize": {"enabled": q.enabled, "move": q.move_step, "angle": q.angle,
                         "percent": q.percent},
        })
    }

    /// The quantize steps when quantizing is on (the setting xor the held Ctrl/Cmd key).
    pub fn quantize_steps(&self, ctrl: bool) -> Option<Steps> {
        (self.quantize.enabled != ctrl).then(|| Steps {
            translate: self.quantize.move_step,
            rotate: self.quantize.angle.to_radians(),
            scale: self.quantize.percent / 100.0,
        })
    }

    /// The grid spacing to use: the setting, else `auto`.
    pub fn spacing_or(&self, auto: f64) -> f64 {
        if self.grid_spacing > 0.0 {
            self.grid_spacing
        } else {
            auto
        }
    }
}

/// A mesh in view that snapping may land on (world space).
#[derive(Debug, Clone, Default)]
pub struct SnapSource {
    pub mesh: PolyMesh,
    /// The object's origin (the `ObjectAxis` target).
    pub origin: Option<DVec3>,
    /// Points that are being moved: sorted ascending.
    pub skip_points: Vec<u32>,
}

impl SnapSource {
    pub fn skips(&self, point: u32) -> bool {
        self.skip_points.binary_search(&point).is_ok()
    }

    pub fn skips_any(&self, poly: &[u32]) -> bool {
        !self.skip_points.is_empty() && poly.iter().any(|p| self.skips(*p))
    }
}

/// One thing the cursor (or a point) may land on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub kind: SnapKind,
    pub position: DVec3,
    pub distance: f64,
    pub depth: f64,
    pub source: Option<usize>,
    pub index: Option<u32>,
}

/// The ordering rule of MANIPULATE.md §6.2: `(rank, distance, depth, kind, source, index)`.
pub fn compare(a: &Candidate, b: &Candidate) -> Ordering {
    a.kind
        .rank()
        .cmp(&b.kind.rank())
        .then(a.distance.total_cmp(&b.distance))
        .then(a.depth.total_cmp(&b.depth))
        .then(a.kind.cmp(&b.kind))
        .then(a.source.cmp(&b.source))
        .then(a.index.cmp(&b.index))
}

/// What snapping resolved to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapHit {
    pub kind: SnapKind,
    /// The snapped position (projected onto the workplane in 2D mode).
    pub position: DVec3,
    /// The 3D target position.
    pub raw: DVec3,
    pub source: Option<usize>,
    pub index: Option<u32>,
    pub distance: f64,
}

/// The winner of `candidates` (empty: `None`), with the 2D projection applied.
pub fn best(candidates: &[Candidate], settings: &SnapSettings) -> Option<SnapHit> {
    let c = candidates.iter().min_by(|a, b| compare(a, b))?;
    Some(SnapHit {
        kind: c.kind,
        position: match settings.mode {
            SnapMode::D3 => c.position,
            SnapMode::D2 => settings.workplane.project(c.position),
        },
        raw: c.position,
        source: c.source,
        index: c.index,
        distance: c.distance,
    })
}

/// The point-like targets of every source (vertices, edge midpoints, polygon centres, axes)
/// as `(kind, position, source, index)`, in scene order.
pub fn point_targets(
    sources: &[SnapSource],
    settings: &SnapSettings,
) -> Vec<(SnapKind, DVec3, usize, Option<u32>)> {
    let t = &settings.targets;
    let mut out = Vec::new();
    for (si, s) in sources.iter().enumerate() {
        let m = &s.mesh;
        if t.vertex {
            for (i, p) in m.points.iter().enumerate() {
                if !s.skips(i as u32) {
                    out.push((SnapKind::Vertex, *p, si, Some(i as u32)));
                }
            }
        }
        if t.edge_midpoint {
            for (i, e) in m.edges().iter().enumerate() {
                if !s.skips(e.0) && !s.skips(e.1) {
                    let mid = (m.points[e.0 as usize] + m.points[e.1 as usize]) * 0.5;
                    out.push((SnapKind::EdgeMidpoint, mid, si, Some(i as u32)));
                }
            }
        }
        if t.polygon_center {
            for (i, poly) in m.polys.iter().enumerate() {
                if poly.is_empty() || s.skips_any(poly) {
                    continue;
                }
                let sum: DVec3 = poly.iter().map(|p| m.points[*p as usize]).sum();
                out.push((
                    SnapKind::PolygonCenter,
                    sum / poly.len() as f64,
                    si,
                    Some(i as u32),
                ));
            }
        }
        if t.object_axis
            && let Some(o) = s.origin
        {
            out.push((SnapKind::ObjectAxis, o, si, None));
        }
    }
    out
}

/// Closest point of the triangle `abc` to `p` (Ericson 2005 §5.1.5).
pub fn closest_on_triangle(p: DVec3, a: DVec3, b: DVec3, c: DVec3) -> DVec3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + (c - b) * w;
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

/// Closest point of the segment `ab` to `p`.
pub fn closest_on_segment(p: DVec3, a: DVec3, b: DVec3) -> DVec3 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= 0.0 {
        return a;
    }
    a + ab * ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
}

/// Snaps the world point `p` to the targets within `radius` world units (MANIPULATE.md §6.4).
/// `enabled` is not looked at; an empty target set gives `None`.
pub fn snap_point(
    sources: &[SnapSource],
    p: DVec3,
    radius: f64,
    settings: &SnapSettings,
) -> Option<SnapHit> {
    let t = &settings.targets;
    let mut cands = Vec::new();
    let mut push = |kind, position: DVec3, source, index| {
        let distance = (position - p).length();
        if distance <= radius {
            cands.push(Candidate {
                kind,
                position,
                distance,
                depth: 0.0,
                source,
                index,
            });
        }
    };
    for (kind, pos, si, idx) in point_targets(sources, settings) {
        push(kind, pos, Some(si), idx);
    }
    for (si, s) in sources.iter().enumerate() {
        let m = &s.mesh;
        if t.edge {
            for (i, e) in m.edges().iter().enumerate() {
                if s.skips(e.0) || s.skips(e.1) {
                    continue;
                }
                let q = closest_on_segment(p, m.points[e.0 as usize], m.points[e.1 as usize]);
                push(SnapKind::Edge, q, Some(si), Some(i as u32));
            }
        }
        if t.polygon {
            for (poly, tri) in triangulate(m) {
                if s.skips_any(&m.polys[poly as usize]) {
                    continue;
                }
                let [a, b, c] = tri.map(|i| m.points[i as usize]);
                push(
                    SnapKind::Polygon,
                    closest_on_triangle(p, a, b, c),
                    Some(si),
                    Some(poly),
                );
            }
        }
    }
    let wp = &settings.workplane;
    let spacing = settings.spacing_or(AUTO_GRID_HEADLESS);
    if t.grid_point {
        push(SnapKind::GridPoint, wp.grid_point(p, spacing), None, None);
    }
    if t.grid_line {
        push(SnapKind::GridLine, wp.grid_line(p, spacing), None, None);
    }
    if t.workplane {
        push(SnapKind::Workplane, wp.project(p), None, None);
    }
    best(&cands, settings)
}

/// A right-handed basis `[X, Y, Z]` with `Z = n`; `X` is `hint` made orthogonal to `n`
/// (`Y × n` when `hint` is parallel to `n`), `Y = Z × X` (MANIPULATE.md §7.2).
pub fn normal_basis(n: DVec3, hint: DVec3) -> DMat3 {
    let z = n.normalize_or_zero();
    let mut x = hint - z * hint.dot(z);
    if x.length() < 1e-9 {
        x = DVec3::Y.cross(z);
        if x.length() < 1e-9 {
            x = DVec3::X;
        }
    }
    let x = x.normalize();
    DMat3::from_cols(x, z.cross(x), z)
}
