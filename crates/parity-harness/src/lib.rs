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
}

impl Default for Tolerance {
    fn default() -> Self {
        Tolerance {
            position: 1e-4,
            psnr: 40.0,
        }
    }
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
