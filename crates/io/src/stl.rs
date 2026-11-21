//! STL writer and reader, binary and ASCII (docs/specs/IO.md §4.1).

use std::fmt::Write as _;

use forge_geometry::PolyMesh;
use forge_geometry::triangulate::triangulate_poly;
use glam::{DMat4, DVec3};

use crate::obj::ObjObject;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WriteOptions {
    /// Binary (the observed form) or ASCII.
    pub binary: bool,
    /// Multiplier of every coordinate.
    pub scale: f64,
}

impl Default for WriteOptions {
    fn default() -> Self {
        WriteOptions {
            binary: true,
            scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadOptions {
    /// Multiplier of every coordinate.
    pub scale: f64,
}

impl Default for ReadOptions {
    fn default() -> Self {
        ReadOptions { scale: 1.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct StlError(pub String);

const HEADER: &[u8] = b"Forge3D STL export";

type Tri = [DVec3; 3];

/// The file's coordinates of a scene point: y and z swapped (§4.1).
fn to_file(p: DVec3) -> DVec3 {
    DVec3::new(p.x, p.z, p.y)
}

/// The triangles of one object as written: world space, axes swapped, winding reversed.
fn triangles(o: &ObjObject, scale: f64) -> Vec<Tri> {
    let world: DMat4 = o.world;
    let pts: Vec<DVec3> = o
        .mesh
        .points
        .iter()
        .map(|p| world.transform_point3(*p) * scale)
        .collect();
    let at = |i: u32| to_file(pts[i as usize]);
    let mut out = Vec::new();
    for poly in &o.mesh.polys {
        match poly.len() {
            0..=2 => {}
            3 => out.push([at(poly[0]), at(poly[2]), at(poly[1])]),
            4 => {
                out.push([at(poly[0]), at(poly[2]), at(poly[1])]);
                out.push([at(poly[0]), at(poly[3]), at(poly[2])]);
            }
            _ => {
                for t in triangulate_poly(&pts, poly) {
                    out.push([at(t[0]), at(t[2]), at(t[1])]);
                }
            }
        }
    }
    out
}

/// The unit normal `(v1 - v0) x (v2 - v0)`; zero for a degenerate triangle.
fn normal(t: &Tri) -> DVec3 {
    (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero()
}

/// Shortest `f32` form; `-0` is written `0`.
fn num(x: f64) -> String {
    let x = x as f32;
    let x = if x == 0.0 { 0.0 } else { x };
    x.to_string()
}

pub fn write(objects: &[ObjObject], opts: &WriteOptions) -> Vec<u8> {
    if opts.binary {
        let tris: Vec<Tri> = objects
            .iter()
            .flat_map(|o| triangles(o, opts.scale))
            .collect();
        let mut out = vec![0u8; 80];
        out[..HEADER.len()].copy_from_slice(HEADER);
        out.extend((tris.len() as u32).to_le_bytes());
        for t in &tris {
            for v in std::iter::once(normal(t)).chain(t.iter().copied()) {
                for c in v.to_array() {
                    out.extend((c as f32).to_le_bytes());
                }
            }
            out.extend(0u16.to_le_bytes());
        }
        return out;
    }
    let mut out = String::new();
    for o in objects {
        let name = o.name.replace(['\n', '\r'], " ");
        let _ = writeln!(out, "solid {name}");
        for t in triangles(o, opts.scale) {
            let n = normal(&t);
            let _ = writeln!(
                out,
                "facet normal {} {} {}\nouter loop",
                num(n.x),
                num(n.y),
                num(n.z)
            );
            for v in t {
                let _ = writeln!(out, "vertex {} {} {}", num(v.x), num(v.y), num(v.z));
            }
            out.push_str("endloop\nendfacet\n");
        }
        let _ = writeln!(out, "endsolid {name}");
    }
    out.into_bytes()
}

/// Builds an object from file-space triangles: points merged when bitwise equal, the inverse
/// axis swap and winding of the writer.
fn object(name: &str, tris: &[Tri], scale: f64) -> ObjObject {
    let mut points: Vec<DVec3> = Vec::new();
    let mut index = std::collections::HashMap::new();
    let mut polys = Vec::new();
    for t in tris {
        let mut corner = [0u32; 3];
        for (k, v) in t.iter().enumerate() {
            let key = v.to_array().map(|c| (c as f32).to_bits());
            corner[k] = *index.entry(key).or_insert_with(|| {
                points.push(to_file(*v) * scale);
                (points.len() - 1) as u32
            });
        }
        polys.push(vec![corner[0], corner[2], corner[1]]);
    }
    ObjObject {
        name: name.to_string(),
        mesh: PolyMesh { points, polys },
        world: DMat4::IDENTITY,
        smoothing_angle: Some(40f64.to_radians()),
        uvs: None,
    }
}

pub fn read(bytes: &[u8], opts: &ReadOptions) -> Result<Vec<ObjObject>, StlError> {
    if bytes.is_empty() {
        return Err(StlError("empty file".into()));
    }
    if bytes.len() >= 84 {
        let n = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
        if bytes.len() == 84 + 50 * n {
            return read_binary(bytes, n, opts);
        }
    }
    if bytes.starts_with(b"solid") {
        return read_ascii(bytes, opts);
    }
    Err(StlError(
        "size does not match the triangle count of a binary STL".into(),
    ))
}

fn read_binary(bytes: &[u8], n: usize, opts: &ReadOptions) -> Result<Vec<ObjObject>, StlError> {
    let mut tris = Vec::with_capacity(n);
    for i in 0..n {
        let rec = &bytes[84 + 50 * i..134 + 50 * i];
        let f = |k: usize| {
            f64::from(f32::from_le_bytes(
                rec[4 * k..4 * k + 4].try_into().unwrap(),
            ))
        };
        let mut t = [DVec3::ZERO; 3];
        for (c, v) in t.iter_mut().enumerate() {
            *v = DVec3::new(f(3 + 3 * c), f(4 + 3 * c), f(5 + 3 * c));
            if !v.is_finite() {
                return Err(StlError(format!("triangle {i}: a number is not finite")));
            }
        }
        tris.push(t);
    }
    Ok(vec![object("Object", &tris, opts.scale)])
}

fn read_ascii(bytes: &[u8], opts: &ReadOptions) -> Result<Vec<ObjObject>, StlError> {
    let text = std::str::from_utf8(bytes).map_err(|_| StlError("not valid text".into()))?;
    let mut out = Vec::new();
    let mut name = String::from("Object");
    let mut tris: Vec<Tri> = Vec::new();
    let mut verts: Vec<DVec3> = Vec::new();
    let mut open = false;
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        match word.to_ascii_lowercase().as_str() {
            "solid" => {
                name = if rest.trim().is_empty() {
                    "Object".into()
                } else {
                    rest.trim().to_string()
                };
                open = true;
            }
            "vertex" => {
                let v: Vec<f64> = rest
                    .split_whitespace()
                    .map(|t| t.parse::<f64>().ok().filter(|x| x.is_finite()))
                    .collect::<Option<_>>()
                    .ok_or_else(|| StlError(format!("line {}: bad number", n + 1)))?;
                if v.len() != 3 {
                    return Err(StlError(format!(
                        "line {}: a vertex needs 3 numbers",
                        n + 1
                    )));
                }
                verts.push(DVec3::new(v[0], v[1], v[2]));
            }
            "endfacet" => {
                if verts.len() != 3 {
                    return Err(StlError(format!(
                        "line {}: a facet needs three vertices",
                        n + 1
                    )));
                }
                tris.push([verts[0], verts[1], verts[2]]);
                verts.clear();
            }
            "endsolid" if open => {
                out.push(object(&name, &tris, opts.scale));
                tris.clear();
                open = false;
            }
            _ => {}
        }
    }
    if open && !tris.is_empty() {
        out.push(object(&name, &tris, opts.scale));
    }
    Ok(out)
}
