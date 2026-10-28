//! Parity harness: loads golden reference data and diffs our output against it.
//! Method: docs/PARITY.md. Behaviour: docs/specs/INFRA.md §3–4.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Mesh + OBJ
// ---------------------------------------------------------------------------

/// Polygon soup as exported by the reference app: positions plus 0-based n-gon faces.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<[f64; 3]>,
    pub faces: Vec<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjError {
    /// 1-based line number.
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ObjError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ObjError {}

/// Reads `v` and `f` statements; everything else is ignored.
pub fn parse_obj(src: &str) -> Result<Mesh, ObjError> {
    let mut mesh = Mesh::default();
    for (i, raw) in src.lines().enumerate() {
        let err = |message: String| ObjError {
            line: i + 1,
            message,
        };
        let line = raw.split('#').next().unwrap_or("");
        let mut tokens = line.split_whitespace();
        match tokens.next() {
            Some("v") => {
                let coords: Vec<&str> = tokens.collect();
                if coords.len() < 3 {
                    return Err(err(format!(
                        "vertex needs 3 coordinates, got {}",
                        coords.len()
                    )));
                }
                let mut p = [0.0; 3];
                for (slot, tok) in p.iter_mut().zip(&coords) {
                    *slot = tok
                        .parse()
                        .map_err(|_| err(format!("bad coordinate `{tok}`")))?;
                }
                mesh.positions.push(p);
            }
            Some("f") => {
                let count = mesh.positions.len() as i64;
                let face = tokens
                    .map(|tok| {
                        let idx = tok.split('/').next().unwrap_or("");
                        let n: i64 = idx
                            .parse()
                            .map_err(|_| err(format!("bad vertex index `{tok}`")))?;
                        let resolved = match n {
                            0 => return Err(err("vertex index 0 is invalid".into())),
                            n if n > 0 => n - 1,
                            n => count + n,
                        };
                        if !(0..count).contains(&resolved) {
                            return Err(err(format!(
                                "vertex index {n} out of range ({count} vertices)"
                            )));
                        }
                        Ok(resolved as u32)
                    })
                    .collect::<Result<Vec<u32>, ObjError>>()?;
                if face.len() < 3 {
                    return Err(err(format!("face needs 3 vertices, got {}", face.len())));
                }
                mesh.faces.push(face);
            }
            _ => {}
        }
    }
    Ok(mesh)
}

/// Exporter settings recorded by the observer, undone on import.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Import {
    pub scale: f64,
    pub flip_z: bool,
}

impl Default for Import {
    fn default() -> Self {
        Import {
            scale: 1.0,
            flip_z: false,
        }
    }
}

/// Scales positions, then optionally negates Z and reverses face winding.
pub fn apply_import(mut mesh: Mesh, import: &Import) -> Mesh {
    for p in &mut mesh.positions {
        for c in p.iter_mut() {
            *c *= import.scale;
        }
        if import.flip_z {
            p[2] = -p[2];
        }
    }
    if import.flip_z {
        for f in &mut mesh.faces {
            f.reverse();
        }
    }
    mesh
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tolerance {
    /// Max abs per-component position error.
    pub position: f64,
    /// Min image PSNR in dB.
    pub psnr: f64,
    /// Which mesh comparison the feature's parity test applies.
    pub compare: Compare,
}

impl Default for Tolerance {
    fn default() -> Self {
        Tolerance {
            position: 1e-4,
            psnr: 40.0,
            compare: Compare::Strict,
        }
    }
}

/// `[tolerance] compare` (docs/specs/INFRA.md §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Compare {
    /// [`compare_meshes`]: order, topology and positions.
    #[default]
    Strict,
    /// [`compare_geometric`]: same points, surface per oriented plane, volume.
    Geometric,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshReport {
    /// (expected, actual)
    pub vertex_counts: (usize, usize),
    /// (expected, actual)
    pub face_counts: (usize, usize),
    pub topology_match: bool,
    /// `None` when either mesh is empty.
    pub bbox_error: Option<f64>,
    /// `None` when vertex counts differ.
    pub max_position_error: Option<f64>,
    pub failures: Vec<String>,
}

impl MeshReport {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

fn bbox(m: &Mesh) -> Option<([f64; 3], [f64; 3])> {
    let first = *m.positions.first()?;
    Some(
        m.positions
            .iter()
            .fold((first, first), |(mut lo, mut hi), p| {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
                (lo, hi)
            }),
    )
}

fn max_abs_diff<'a>(
    a: impl IntoIterator<Item = &'a f64>,
    b: impl IntoIterator<Item = &'a f64>,
) -> f64 {
    a.into_iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// Distance under which [`export_weld`] merges points (well below the 1e-4 position
/// tolerance, well above float noise).
pub const WELD_TOLERANCE: f64 = 1e-6;

/// Canonical weld for comparing against the reference exporter, which merges exactly
/// equal points (docs/specs/GENERATORS2.md §0): points within [`WELD_TOLERANCE`] merge
/// into their first occurrence, later points are renumbered in order, and faces keep
/// their corners (possibly repeated). Applied to both sides, it also removes the
/// reference's own near-duplicates that differ only by rounding noise.
pub fn export_weld(mesh: &Mesh) -> Mesh {
    let tol = WELD_TOLERANCE;
    let cell = |p: &[f64; 3]| p.map(|c| (c / tol).floor() as i64);
    let mut grid: std::collections::HashMap<[i64; 3], Vec<u32>> = std::collections::HashMap::new();
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut remap = Vec::with_capacity(mesh.positions.len());
    for p in &mesh.positions {
        let c = cell(p);
        let mut found = None;
        for d in 0..27 {
            let n = [c[0] + d % 3 - 1, c[1] + d / 3 % 3 - 1, c[2] + d / 9 - 1];
            for &i in grid.get(&n).into_iter().flatten() {
                let q = positions[i as usize];
                let dist = (0..3).map(|k| (q[k] - p[k]).powi(2)).sum::<f64>().sqrt();
                if dist <= tol && found.is_none_or(|f| i < f) {
                    found = Some(i);
                }
            }
        }
        let i = found.unwrap_or_else(|| {
            positions.push(*p);
            let i = (positions.len() - 1) as u32;
            grid.entry(c).or_default().push(i);
            i
        });
        remap.push(i);
    }
    let faces = mesh
        .faces
        .iter()
        .map(|f| f.iter().map(|&i| remap[i as usize]).collect())
        .collect();
    Mesh { positions, faces }
}

pub fn compare_meshes(expected: &Mesh, actual: &Mesh, tol: &Tolerance) -> MeshReport {
    let mut failures = Vec::new();
    let vertex_counts = (expected.positions.len(), actual.positions.len());
    let face_counts = (expected.faces.len(), actual.faces.len());
    if vertex_counts.0 != vertex_counts.1 {
        failures.push(format!(
            "vertex count: expected {}, got {}",
            vertex_counts.0, vertex_counts.1
        ));
    }
    if face_counts.0 != face_counts.1 {
        failures.push(format!(
            "face count: expected {}, got {}",
            face_counts.0, face_counts.1
        ));
    }

    let first_diff = expected
        .faces
        .iter()
        .zip(&actual.faces)
        .position(|(a, b)| a != b);
    let topology_match = face_counts.0 == face_counts.1 && first_diff.is_none();
    if let Some(i) = first_diff {
        failures.push(format!(
            "topology: face {i} expected {:?}, got {:?}",
            expected.faces[i], actual.faces[i]
        ));
    }

    let bbox_error = match (bbox(expected), bbox(actual)) {
        (Some((elo, ehi)), Some((alo, ahi))) => {
            Some(max_abs_diff(elo.iter().chain(&ehi), alo.iter().chain(&ahi)))
        }
        (None, None) => None,
        _ => {
            failures.push("bbox: exactly one mesh is empty".into());
            None
        }
    };
    if let Some(e) = bbox_error.filter(|e| *e > tol.position) {
        failures.push(format!(
            "bbox error {e:e} exceeds tolerance {:e}",
            tol.position
        ));
    }

    let max_position_error = (vertex_counts.0 == vertex_counts.1).then(|| {
        let mut worst = (0.0, 0);
        for (i, (a, b)) in expected.positions.iter().zip(&actual.positions).enumerate() {
            let e = max_abs_diff(a, b);
            if e > worst.0 {
                worst = (e, i);
            }
        }
        if worst.0 > tol.position {
            failures.push(format!(
                "position error {:e} at vertex {} exceeds tolerance {:e}",
                worst.0, worst.1, tol.position
            ));
        }
        worst.0
    });

    MeshReport {
        vertex_counts,
        face_counts,
        topology_match,
        bbox_error,
        max_position_error,
        failures,
    }
}

// ---------------------------------------------------------------------------
// Geometric comparison (docs/specs/INFRA.md §4.2)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct GeometricReport {
    /// Distinct used points (expected, actual).
    pub points: (usize, usize),
    /// Oriented planes (expected, actual).
    pub planes: (usize, usize),
    /// Closedness (expected, actual).
    pub closed: (bool, bool),
    /// Volumes (expected, actual) when both are closed.
    pub volume: Option<(f64, f64)>,
    /// Informational only.
    pub face_counts: (usize, usize),
    pub failures: Vec<String>,
}

impl GeometricReport {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

fn close(a: V3, b: V3, t: f64) -> bool {
    max_abs_diff(&a, &b) <= t
}

/// Per vertex index, the id of its distinct point (`usize::MAX` if unused); plus the
/// distinct points.
fn distinct_points(m: &Mesh, t: f64) -> (Vec<usize>, Vec<V3>) {
    let mut used = vec![false; m.positions.len()];
    for &v in m.faces.iter().flatten() {
        used[v as usize] = true;
    }
    let mut points: Vec<V3> = Vec::new();
    let ids = m
        .positions
        .iter()
        .zip(&used)
        .map(|(p, &u)| {
            if !u {
                return usize::MAX;
            }
            points
                .iter()
                .position(|q| close(*p, *q, t))
                .unwrap_or_else(|| {
                    points.push(*p);
                    points.len() - 1
                })
        })
        .collect();
    (ids, points)
}

struct Plane {
    n: V3,
    d: f64,
    area: f64,
    /// Sum of area × triangle centroid.
    moment: V3,
    perimeter: f64,
}

fn planes(m: &Mesh, t: f64) -> Vec<Plane> {
    let mut out: Vec<Plane> = Vec::new();
    for f in &m.faces {
        let p: Vec<V3> = f.iter().map(|&i| m.positions[i as usize]).collect();
        let k = p.len();
        let mut newell = [0.0; 3];
        let mut perimeter = 0.0;
        for i in 0..k {
            newell = add(newell, cross(p[i], p[(i + 1) % k]));
            perimeter += norm(sub(p[(i + 1) % k], p[i]));
        }
        let len = norm(newell);
        if len == 0.0 {
            continue;
        }
        let n = scale(newell, 1.0 / len);
        let c = scale(p.iter().fold([0.0; 3], |s, q| add(s, *q)), 1.0 / k as f64);
        let d = dot(n, c);
        let mut area = 0.0;
        let mut moment = [0.0; 3];
        for i in 1..k - 1 {
            let a = dot(n, cross(sub(p[i], p[0]), sub(p[i + 1], p[0]))) / 2.0;
            let tc = scale(add(add(p[0], p[i]), p[i + 1]), 1.0 / 3.0);
            area += a;
            moment = add(moment, scale(tc, a));
        }
        let nt = normal_tolerance(t, area);
        match out
            .iter_mut()
            .find(|q| close(q.n, n, nt) && (q.d - d).abs() <= t)
        {
            Some(q) => {
                q.area += area;
                q.moment = add(q.moment, moment);
                q.perimeter += perimeter;
            }
            None => out.push(Plane {
                n,
                d,
                area,
                moment,
                perimeter,
            }),
        }
    }
    out
}

/// Unit normals of a face of area `a` whose corners move by `t` turn by about `t/√a`.
fn normal_tolerance(t: f64, a: f64) -> f64 {
    (4.0 * t / a.abs().sqrt()).max(1e-6)
}

fn is_closed(m: &Mesh, ids: &[usize]) -> bool {
    let mut edges: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for f in &m.faces {
        for i in 0..f.len() {
            let (a, b) = (ids[f[i] as usize], ids[f[(i + 1) % f.len()] as usize]);
            *edges.entry((a, b)).or_insert(0) += 1;
        }
    }
    !edges.is_empty()
        && edges
            .iter()
            .all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1))
}

fn volume(m: &Mesh) -> f64 {
    m.faces
        .iter()
        .map(|f| {
            let p = |i: usize| m.positions[f[i] as usize];
            (1..f.len() - 1)
                .map(|i| dot(p(0), cross(p(i), p(i + 1))) / 6.0)
                .sum::<f64>()
        })
        .sum()
}

/// Same surface regardless of numbering and polygon split (docs/specs/INFRA.md §4.2).
pub fn compare_geometric(expected: &Mesh, actual: &Mesh, tol: &Tolerance) -> GeometricReport {
    let t = tol.position;
    let mut failures = Vec::new();

    let (eids, ep) = distinct_points(expected, t);
    let (aids, ap) = distinct_points(actual, t);
    if let Some(p) = ep.iter().find(|p| !ap.iter().any(|q| close(**p, *q, t))) {
        failures.push(format!("points: expected point {p:?} missing"));
    }
    if let Some(p) = ap.iter().find(|p| !ep.iter().any(|q| close(**p, *q, t))) {
        failures.push(format!("points: unexpected point {p:?}"));
    }
    if ep.len() != ap.len() {
        failures.push(format!(
            "points: expected {} distinct, got {}",
            ep.len(),
            ap.len()
        ));
    }

    let (epl, apl) = (planes(expected, t), planes(actual, t));
    let mut matched = vec![false; apl.len()];
    for e in &epl {
        let found = apl.iter().enumerate().find(|(i, a)| {
            !matched[*i] && close(a.n, e.n, normal_tolerance(t, e.area)) && (a.d - e.d).abs() <= t
        });
        let Some((i, a)) = found else {
            failures.push(format!(
                "planes: expected plane n {:?} d {} (area {}) missing",
                e.n, e.d, e.area
            ));
            continue;
        };
        matched[i] = true;
        if (a.area - e.area).abs() > t * e.perimeter + 1e-12 {
            failures.push(format!(
                "planes: n {:?} d {}: area expected {}, got {}",
                e.n, e.d, e.area, a.area
            ));
        } else if e.area > 0.0 && a.area > 0.0 {
            let ec = scale(e.moment, 1.0 / e.area);
            let ac = scale(a.moment, 1.0 / a.area);
            if !close(ec, ac, t + t * e.perimeter / e.area) {
                failures.push(format!(
                    "planes: n {:?} d {}: centroid expected {ec:?}, got {ac:?}",
                    e.n, e.d
                ));
            }
        }
    }
    for (a, _) in apl.iter().zip(&matched).filter(|(_, m)| !**m) {
        failures.push(format!(
            "planes: unexpected plane n {:?} d {} (area {})",
            a.n, a.d, a.area
        ));
    }

    let closed = (is_closed(expected, &eids), is_closed(actual, &aids));
    if closed.0 != closed.1 {
        failures.push(format!("closed: expected {}, got {}", closed.0, closed.1));
    }
    let volume = (closed.0 && closed.1).then(|| (volume(expected), volume(actual)));
    if let Some((ev, av)) = volume {
        let area: f64 = epl.iter().map(|p| p.area.abs()).sum();
        if (ev - av).abs() > t * area + 1e-12 {
            failures.push(format!("volume: expected {ev}, got {av}"));
        }
    }

    GeometricReport {
        points: (ep.len(), ap.len()),
        planes: (epl.len(), apl.len()),
        closed,
        volume,
        face_counts: (expected.faces.len(), actual.faces.len()),
        failures,
    }
}

/// Max abs element difference of two 4×4 matrices.
pub fn max_matrix_error(a: &[f64; 16], b: &[f64; 16]) -> f64 {
    max_abs_diff(a, b)
}

/// PSNR in dB for channel values in [0, 1]; `INFINITY` when identical.
///
/// # Panics
/// On empty input or length mismatch.
pub fn psnr(expected: &[f32], actual: &[f32]) -> f64 {
    assert_eq!(expected.len(), actual.len(), "psnr: length mismatch");
    assert!(!expected.is_empty(), "psnr: empty input");
    let sum: f64 = expected
        .iter()
        .zip(actual)
        .map(|(a, b)| (f64::from(*a) - f64::from(*b)).powi(2))
        .sum();
    let mse = sum / expected.len() as f64;
    if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (1.0 / mse).log10()
    }
}

// ---------------------------------------------------------------------------
// Golden cases
// ---------------------------------------------------------------------------

/// `tests/golden/<feature>/<case>/case.toml`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseManifest {
    pub feature: String,
    /// Repo-relative path under `docs/parity/`.
    pub observation: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Relative to the case directory.
    #[serde(default)]
    pub mesh: Option<String>,
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
    #[serde(default)]
    pub import: Import,
    #[serde(default)]
    pub tolerance: Tolerance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoldenCase {
    pub feature: String,
    pub name: String,
    pub dir: PathBuf,
}

impl GoldenCase {
    pub fn manifest(&self) -> Result<CaseManifest, String> {
        let path = self.dir.join("case.toml");
        let src = fs::read_to_string(&path).map_err(|e| format!("missing case.toml ({e})"))?;
        toml::from_str(&src).map_err(|e| format!("case.toml: {e}"))
    }

    /// The golden mesh with `[import]` applied, or `None` if the case has no mesh.
    pub fn expected_mesh(&self) -> Result<Option<Mesh>, String> {
        let manifest = self.manifest()?;
        let Some(rel) = &manifest.mesh else {
            return Ok(None);
        };
        let src = fs::read_to_string(self.dir.join(rel))
            .map_err(|_| format!("mesh {rel:?} does not exist"))?;
        let mesh = parse_obj(&src).map_err(|e| format!("mesh {rel:?}: {e}"))?;
        Ok(Some(apply_import(mesh, &manifest.import)))
    }
}

fn subdirs(dir: &Path) -> io::Result<Vec<(String, PathBuf)>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            out.push((
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            ));
        }
    }
    out.sort();
    Ok(out)
}

/// Every `<feature>/<case>/` directory under `golden_root`, sorted.
pub fn discover_cases(golden_root: &Path) -> io::Result<Vec<GoldenCase>> {
    let mut cases = Vec::new();
    for (feature, feature_dir) in subdirs(golden_root)? {
        for (name, dir) in subdirs(&feature_dir)? {
            cases.push(GoldenCase {
                feature: feature.clone(),
                name,
                dir,
            });
        }
    }
    Ok(cases)
}

// ---------------------------------------------------------------------------
// App version and the R20 defaults overlay (docs/specs/INFRA.md §3.1)
// ---------------------------------------------------------------------------

/// The reference app version an observation was made with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppVersion {
    R20,
    R25,
}

/// The version named in the `Status:` header line of the case's observation file: the
/// first `R2` + digit token after `App version:`. Errors name the observation file.
pub fn app_version(repo_root: &Path, manifest: &CaseManifest) -> Result<AppVersion, String> {
    let rel = &manifest.observation;
    let src =
        fs::read_to_string(repo_root.join(rel)).map_err(|e| format!("{rel}: unreadable ({e})"))?;
    let status = src
        .lines()
        .find(|l| l.starts_with("Status:"))
        .ok_or_else(|| format!("{rel}: no `Status:` header line"))?;
    let after = status
        .split_once("App version:")
        .map(|(_, rest)| rest)
        .ok_or_else(|| format!("{rel}: no `App version:` in the Status line"))?;
    let bytes = after.as_bytes();
    for i in 0..bytes.len().saturating_sub(2) {
        if bytes[i] == b'R' && bytes[i + 1] == b'2' && bytes[i + 2].is_ascii_digit() {
            return match &after[i..i + 3] {
                "R20" => Ok(AppVersion::R20),
                "R25" => Ok(AppVersion::R25),
                other => Err(format!("{rel}: unsupported app version {other}")),
            };
        }
    }
    Err(format!("{rel}: no R20 / R25 token after `App version:`"))
}

/// The params whose R20 default differs from our R25 descriptor defaults, for the node
/// type `node_type` (our type id without `forge.`), from `tests/defaults/r20.toml`, in
/// case-file units. Empty for R25 and for a type without a table. Panics if the file is
/// missing or malformed: it is test data.
pub fn defaults_overlay(
    repo_root: &Path,
    version: AppVersion,
    node_type: &str,
) -> BTreeMap<String, toml::Value> {
    if version == AppVersion::R25 {
        return BTreeMap::new();
    }
    let path = repo_root.join("tests/defaults/r20.toml");
    let src = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: unreadable ({e})", path.display()));
    let doc: toml::Table =
        toml::from_str(&src).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    match doc.get(node_type) {
        None => BTreeMap::new(),
        Some(toml::Value::Table(t)) => t.clone().into_iter().collect(),
        Some(_) => panic!("{}: [{node_type}] is not a table", path.display()),
    }
}

/// Problems with a case; empty means valid.
pub fn validate_case(repo_root: &Path, case: &GoldenCase) -> Vec<String> {
    let manifest = match case.manifest() {
        Ok(m) => m,
        Err(e) => return vec![e],
    };
    let mut problems = Vec::new();
    if manifest.feature != case.feature {
        problems.push(format!(
            "feature {:?} does not match directory {:?}",
            manifest.feature, case.feature
        ));
    }
    let obs = &manifest.observation;
    if !obs.starts_with("docs/parity/") || obs.split('/').any(|c| c == "..") {
        problems.push(format!("observation {obs:?} must be under docs/parity/"));
    } else if !repo_root.join(obs).is_file() {
        problems.push(format!("observation {obs:?} does not exist"));
    }
    if let Err(e) = case.expected_mesh() {
        problems.push(e);
    }
    problems
}

// ---------------------------------------------------------------------------
// Observation helpers (docs/specs/INFRA.md §4, §4.1)
// ---------------------------------------------------------------------------

/// Summary of a golden mesh, printed by `golden info`.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshInfo {
    pub vertices: usize,
    /// Points no face uses.
    pub unused_vertices: usize,
    pub faces: usize,
    /// Face size → count.
    pub face_sizes: BTreeMap<usize, usize>,
    pub bbox: Option<([f64; 3], [f64; 3])>,
}

pub fn mesh_info(mesh: &Mesh) -> MeshInfo {
    let mut used = vec![false; mesh.positions.len()];
    let mut face_sizes = BTreeMap::new();
    for f in &mesh.faces {
        *face_sizes.entry(f.len()).or_insert(0) += 1;
        for &v in f {
            used[v as usize] = true;
        }
    }
    MeshInfo {
        vertices: mesh.positions.len(),
        unused_vertices: used.iter().filter(|u| !**u).count(),
        faces: mesh.faces.len(),
        face_sizes,
        bbox: bbox(mesh),
    }
}

impl fmt::Display for MeshInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v3 = |v: [f64; 3]| format!("({}, {}, {})", v[0], v[1], v[2]);
        writeln!(
            f,
            "vertices: {} (unused {})",
            self.vertices, self.unused_vertices
        )?;
        let sizes: Vec<String> = self
            .face_sizes
            .iter()
            .map(|(k, n)| format!("{k}-gons {n}"))
            .collect();
        if sizes.is_empty() {
            writeln!(f, "faces: {}", self.faces)?;
        } else {
            writeln!(f, "faces: {} ({})", self.faces, sizes.join(", "))?;
        }
        match self.bbox {
            None => writeln!(f, "bbox: empty"),
            Some((lo, hi)) => {
                writeln!(f, "bbox min: {}", v3(lo))?;
                writeln!(f, "bbox max: {}", v3(hi))?;
                writeln!(
                    f,
                    "size: {}",
                    v3([hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]])
                )?;
                let c = |k: usize| (lo[k] + hi[k]) / 2.0;
                writeln!(f, "centre: {}", v3([c(0), c(1), c(2)]))
            }
        }
    }
}

/// Counts of OBJ statements the mesh parser ignores but observers should know about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ObjExtras {
    pub normals: usize,
    pub uvs: usize,
}

pub fn obj_extras(src: &str) -> ObjExtras {
    let mut e = ObjExtras::default();
    for line in src.lines() {
        match line.split_whitespace().next() {
            Some("vn") => e.normals += 1,
            Some("vt") => e.uvs += 1,
            _ => {}
        }
    }
    e
}

// ---------------------------------------------------------------------------
// Exported normals (docs/specs/INFRA.md §4)
// ---------------------------------------------------------------------------

/// Per face, per corner, the normal the corner references (`None` if none).
pub fn parse_obj_corner_normals(src: &str) -> Result<Vec<Vec<Option<[f64; 3]>>>, ObjError> {
    let mut normals: Vec<[f64; 3]> = Vec::new();
    let mut faces = Vec::new();
    for (i, raw) in src.lines().enumerate() {
        let err = |message: String| ObjError {
            line: i + 1,
            message,
        };
        let line = raw.split('#').next().unwrap_or("");
        let mut tokens = line.split_whitespace();
        match tokens.next() {
            Some("vn") => {
                let mut n = [0.0; 3];
                for slot in &mut n {
                    let tok = tokens
                        .next()
                        .ok_or_else(|| err("normal needs 3 components".into()))?;
                    *slot = tok
                        .parse()
                        .map_err(|_| err(format!("bad normal component `{tok}`")))?;
                }
                normals.push(n);
            }
            Some("f") => {
                let count = normals.len() as i64;
                let corners = tokens
                    .map(|tok| {
                        let Some(idx) = tok.split('/').nth(2).filter(|s| !s.is_empty()) else {
                            return Ok(None);
                        };
                        let n: i64 = idx
                            .parse()
                            .map_err(|_| err(format!("bad normal index `{tok}`")))?;
                        let resolved = match n {
                            0 => return Err(err("normal index 0 is invalid".into())),
                            n if n > 0 => n - 1,
                            n => count + n,
                        };
                        if !(0..count).contains(&resolved) {
                            return Err(err(format!(
                                "normal index {n} out of range ({count} normals)"
                            )));
                        }
                        Ok(Some(normals[resolved as usize]))
                    })
                    .collect::<Result<Vec<_>, ObjError>>()?;
                faces.push(corners);
            }
            _ => {}
        }
    }
    Ok(faces)
}

/// Max abs component difference over all corners; `None` if the shapes differ.
pub fn max_normal_error(expected: &[Vec<[f64; 3]>], actual: &[Vec<[f64; 3]>]) -> Option<f64> {
    if expected.len() != actual.len()
        || expected.iter().zip(actual).any(|(a, b)| a.len() != b.len())
    {
        return None;
    }
    Some(
        expected
            .iter()
            .flatten()
            .zip(actual.iter().flatten())
            .map(|(a, b)| max_abs_diff(a, b))
            .fold(0.0, f64::max),
    )
}
