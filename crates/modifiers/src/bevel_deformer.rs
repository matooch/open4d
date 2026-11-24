//! Bevel deformer (docs/specs/MODIFIERS.md §22.4): the Bevel tool applied to the whole mesh.

use std::collections::BTreeMap;

use forge_core::Value;
use forge_geometry::PolyMesh;
use forge_geometry::normals::poly_normal;
use forge_geometry::selection::Selection;
use forge_modeling::{Settings, bevel, tool_desc};
use glam::DVec3;

pub struct BevelParams {
    /// `points`, `edges` or `polygons`.
    pub component: String,
    pub use_angle: bool,
    pub angle: f64,
    pub bevel_mode: String,
    pub offset_mode: String,
    pub offset: f64,
    pub subdivision: i64,
    pub depth: f64,
    pub shape: String,
    pub tension: f64,
    pub mitering: String,
    pub ending: String,
    pub partial_rounding: String,
    pub corner_ngons: bool,
    pub rounding_ngons: bool,
}

impl Default for BevelParams {
    fn default() -> Self {
        Self {
            component: "edges".into(),
            use_angle: true,
            angle: 40.0,
            bevel_mode: "chamfer".into(),
            offset_mode: "fixed_distance".into(),
            offset: 1.0,
            subdivision: 0,
            depth: 1.0,
            shape: "round".into(),
            tension: 1.0,
            mitering: "default".into(),
            ending: "default".into(),
            partial_rounding: "none".into(),
            corner_ngons: false,
            rounding_ngons: false,
        }
    }
}

/// Bevels every selected component (all of them for the deformer).
pub fn bevel_all(mesh: &mut PolyMesh, p: &BevelParams) {
    let Some(desc) = tool_desc("bevel") else {
        return;
    };
    let mut settings = Settings::defaults(&desc);
    let mut values: BTreeMap<&str, Value> = BTreeMap::new();
    values.insert("bevel_mode", Value::Enum(p.bevel_mode.clone()));
    values.insert("offset_mode", Value::Enum(p.offset_mode.clone()));
    values.insert("offset", Value::Float(p.offset));
    values.insert("subdivision", Value::Int(p.subdivision));
    values.insert("depth", Value::Float(p.depth));
    values.insert("shape", Value::Enum(p.shape.clone()));
    values.insert("tension", Value::Float(p.tension));
    values.insert("mitering", Value::Enum(p.mitering.clone()));
    values.insert("ending", Value::Enum(p.ending.clone()));
    values.insert("partial_rounding", Value::Enum(p.partial_rounding.clone()));
    values.insert("corner_ngons", Value::Bool(p.corner_ngons));
    values.insert("rounding_ngons", Value::Bool(p.rounding_ngons));
    for (id, value) in values {
        let _ = settings.set(&desc, id, value);
    }
    let mut selection = Selection::default();
    match p.component.as_str() {
        "points" => {
            let set = &mut selection.points;
            for i in 0..mesh.points.len() as u32 {
                set.insert(i);
            }
        }
        "polygons" => {
            let set = &mut selection.polys;
            for i in 0..mesh.polys.len() as u32 {
                set.insert(i);
            }
        }
        _ => {
            // The Bevel tool rejects three selected edges at one vertex (a full-mesh
            // chamfer), so the edge mode cuts every edge here (MODIFIERS.md §22.4).
            *mesh = chamfer_all(mesh, p.offset, p.use_angle, p.angle);
            return;
        }
    }
    if selection.points.is_empty() && selection.edges.is_empty() && selection.polys.is_empty() {
        return;
    }
    match bevel(mesh, &selection, &settings) {
        Ok((out, _)) => *mesh = out,
        Err(e) => eprintln!("bevel deformer: {e}"),
    }
}

/// A full-mesh chamfer: every face is inset by `offset` along its edges, every edge becomes
/// a quad between its two faces and every vertex a small polygon of its incident faces.
/// `use_angle`/`angle` keep only edges sharper than the threshold.
fn chamfer_all(mesh: &PolyMesh, offset: f64, use_angle: bool, angle: f64) -> PolyMesh {
    if offset <= 0.0 || mesh.polys.is_empty() {
        return mesh.clone();
    }
    let normals: Vec<DVec3> = mesh
        .polys
        .iter()
        .map(|f| poly_normal(&mesh.points, f).normalize_or_zero())
        .collect();
    let mut any_sharp = false;
    for f in &mesh.polys {
        for i in 0..f.len() {
            let (a, b) = (f[i], f[(i + 1) % f.len()]);
            let faces: Vec<usize> = mesh
                .polys
                .iter()
                .enumerate()
                .filter(|(_, g)| g.contains(&a) && g.contains(&b))
                .map(|(j, _)| j)
                .collect();
            let sharp = !use_angle
                || (faces.len() == 2
                    && normals[faces[0]]
                        .dot(normals[faces[1]])
                        .clamp(-1.0, 1.0)
                        .acos()
                        .to_degrees()
                        > angle);
            any_sharp |= sharp;
        }
    }
    if use_angle && !any_sharp {
        return mesh.clone();
    }
    let sharp = |a: u32, b: u32| -> bool {
        if !use_angle {
            return true;
        }
        let faces: Vec<usize> = mesh
            .polys
            .iter()
            .enumerate()
            .filter(|(_, f)| f.contains(&a) && f.contains(&b))
            .map(|(i, _)| i)
            .collect();
        faces.len() == 2
            && normals[faces[0]]
                .dot(normals[faces[1]])
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees()
                > angle
    };
    let mut points = Vec::new();
    let mut face_points: Vec<Vec<u32>> = Vec::with_capacity(mesh.polys.len());
    for f in &mesh.polys {
        let n = f.len();
        let mut row = Vec::with_capacity(n);
        for i in 0..n {
            let v = mesh.points[f[i] as usize];
            let prev = mesh.points[f[(i + n - 1) % n] as usize];
            let next = mesh.points[f[(i + 1) % n] as usize];
            let a = (prev - v).normalize_or_zero();
            let b = (next - v).normalize_or_zero();
            row.push(points.len() as u32);
            points.push(v + (a + b) * offset);
        }
        face_points.push(row);
    }
    let mut polys: Vec<Vec<u32>> = Vec::new();
    // inset faces
    for row in &face_points {
        polys.push(row.clone());
    }
    // edge quads
    for (fi, f) in mesh.polys.iter().enumerate() {
        for i in 0..f.len() {
            let a = f[i];
            let b = f[(i + 1) % f.len()];
            if a > b || !sharp(a, b) {
                continue;
            }
            let Some(fj) = mesh
                .polys
                .iter()
                .enumerate()
                .position(|(j, g)| j != fi && g.contains(&a) && g.contains(&b))
            else {
                continue;
            };
            let ja = mesh.polys[fj].iter().position(|v| *v == a).unwrap();
            let jb = mesh.polys[fj].iter().position(|v| *v == b).unwrap();
            let quad = vec![
                face_points[fi][i],
                face_points[fi][(i + 1) % f.len()],
                face_points[fj][jb],
                face_points[fj][ja],
            ];
            polys.push(quad);
        }
    }
    // vertex polygons: the face points around each vertex, ordered in its tangent plane
    let mut around: BTreeMap<u32, Vec<(u32, DVec3)>> = BTreeMap::new();
    for (fi, f) in mesh.polys.iter().enumerate() {
        for (i, v) in f.iter().enumerate() {
            around
                .entry(*v)
                .or_default()
                .push((face_points[fi][i], normals[fi]));
        }
    }
    for (v, list) in around {
        if list.len() < 3 {
            continue;
        }
        let n = list
            .iter()
            .map(|(_, n)| *n)
            .sum::<DVec3>()
            .normalize_or_zero();
        let centre = mesh.points[v as usize];
        let reference = (points[list[0].0 as usize] - centre).normalize_or_zero();
        let binormal = n.cross(reference);
        let mut sorted = list.clone();
        sorted.sort_by(|(a, _), (b, _)| {
            let pa = points[*a as usize] - centre;
            let pb = points[*b as usize] - centre;
            let aa = pa.dot(binormal).atan2(pa.dot(reference));
            let ab = pb.dot(binormal).atan2(pb.dot(reference));
            aa.partial_cmp(&ab).unwrap_or(std::cmp::Ordering::Equal)
        });
        polys.push(sorted.iter().map(|(id, _)| *id).collect());
    }
    let mut out = PolyMesh { points, polys };
    if signed_volume(&out) < 0.0 {
        for f in &mut out.polys {
            f.reverse();
        }
    }
    out
}

fn signed_volume(mesh: &PolyMesh) -> f64 {
    let mut v = 0.0;
    for f in &mesh.polys {
        for k in 1..f.len() - 1 {
            let a = mesh.points[f[0] as usize];
            let b = mesh.points[f[k] as usize];
            let c = mesh.points[f[k + 1] as usize];
            v += a.dot(b.cross(c)) / 6.0;
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> PolyMesh {
        let mut points = Vec::new();
        for z in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for x in [-1.0, 1.0] {
                    points.push(DVec3::new(x * 100.0, y * 100.0, z * 100.0));
                }
            }
        }
        PolyMesh {
            points,
            polys: vec![
                vec![0, 1, 3, 2],
                vec![4, 6, 7, 5],
                vec![0, 4, 5, 1],
                vec![2, 3, 7, 6],
                vec![0, 2, 6, 4],
                vec![1, 5, 7, 3],
            ],
        }
    }

    #[test]
    fn every_edge_of_a_cube_is_chamfered() {
        let mut mesh = cube();
        bevel_all(&mut mesh, &BevelParams::default());
        assert!(mesh.polys.len() > 6, "{} faces", mesh.polys.len());
        assert!(mesh.points.len() > 8, "{} points", mesh.points.len());
    }

    #[test]
    fn the_angle_threshold_can_skip_every_edge() {
        let mut mesh = cube();
        bevel_all(
            &mut mesh,
            &BevelParams {
                use_angle: false,
                ..Default::default()
            },
        );
        let with_all = mesh.polys.len();
        let mut mesh = cube();
        bevel_all(
            &mut mesh,
            &BevelParams {
                angle: 100.0,
                ..Default::default()
            },
        );
        assert_eq!(mesh.polys.len(), 6);
        assert!(with_all > 6);
    }
}
