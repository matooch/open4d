//! Wavefront OBJ writer and reader (docs/specs/IO.md §1).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;

use forge_geometry::PolyMesh;
use forge_geometry::normals::{Weighting, corner_normals};
use glam::{DMat4, DVec3};

/// One named mesh. `mesh` is in the object's space, `world` takes it to world space.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjObject {
    pub name: String,
    pub mesh: PolyMesh,
    pub world: DMat4,
    /// Smoothing angle in radians for the written corner normals; `None` is faceted.
    pub smoothing_angle: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteOptions {
    /// Write one `vn` per corner (default on).
    pub normals: bool,
    /// Merge exactly equal points across the file (default on).
    pub weld: bool,
}

impl Default for WriteOptions {
    fn default() -> Self {
        WriteOptions {
            normals: true,
            weld: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReadOptions {
    /// Negate z and reverse each polygon's winding.
    pub flip_z: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {message}")]
pub struct ObjError {
    /// 1-based line number.
    pub line: usize,
    pub message: String,
}

/// The default smoothing tag's angle, given to read objects.
const DEFAULT_SMOOTHING_DEG: f64 = 40.0;

/// Shortest round-trip form; `-0` is written `0`.
fn num(x: f64) -> String {
    let x = if x == 0.0 { 0.0 } else { x };
    x.to_string()
}

fn key(v: DVec3) -> [u64; 3] {
    let bits = |x: f64| if x == 0.0 { 0 } else { x.to_bits() };
    [bits(v.x), bits(v.y), bits(v.z)]
}

pub fn write(objects: &[ObjObject], opts: &WriteOptions) -> String {
    let mut out = String::from("# Forge3D OBJ export\n");
    let mut points: HashMap<[u64; 3], usize> = HashMap::new();
    let mut point_count = 0usize;
    let mut normals: HashMap<[u64; 3], usize> = HashMap::new();
    let mut normal_count = 0usize;
    for obj in objects {
        let name: String = obj
            .name
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .collect();
        let _ = writeln!(out, "o {name}");
        let world: Vec<DVec3> = obj
            .mesh
            .points
            .iter()
            .map(|p| obj.world.transform_point3(*p))
            .collect();
        // global 0-based index of each point of this object
        let mut index = Vec::with_capacity(world.len());
        for p in &world {
            let known = opts.weld.then(|| points.get(&key(*p)).copied()).flatten();
            let i = known.unwrap_or_else(|| {
                let _ = writeln!(out, "v {} {} {}", num(p.x), num(p.y), num(p.z));
                point_count += 1;
                if opts.weld {
                    points.insert(key(*p), point_count - 1);
                }
                point_count - 1
            });
            index.push(i);
        }
        let corner = if opts.normals {
            let m = PolyMesh {
                points: world,
                polys: obj.mesh.polys.clone(),
            };
            let per_corner =
                corner_normals(&m, obj.smoothing_angle.unwrap_or(0.0), Weighting::Angle);
            per_corner
                .into_iter()
                .map(|face| {
                    face.into_iter()
                        .map(|n| {
                            *normals.entry(key(n)).or_insert_with(|| {
                                let _ = writeln!(out, "vn {} {} {}", num(n.x), num(n.y), num(n.z));
                                normal_count += 1;
                                normal_count - 1
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        } else {
            Vec::new()
        };
        for (f, poly) in obj.mesh.polys.iter().enumerate() {
            out.push('f');
            for (c, &v) in poly.iter().enumerate() {
                if opts.normals {
                    let _ = write!(out, " {}//{}", index[v as usize] + 1, corner[f][c] + 1);
                } else {
                    let _ = write!(out, " {}", index[v as usize] + 1);
                }
            }
            out.push('\n');
        }
    }
    out
}

/// An object being read: where its own `v` lines start and its faces (global indices).
struct Pending {
    name: String,
    declared_from: usize,
    declared_to: usize,
    faces: Vec<Vec<usize>>,
}

impl Pending {
    fn new(name: &str, declared_from: usize) -> Pending {
        Pending {
            name: name.to_string(),
            declared_from,
            declared_to: declared_from,
            faces: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.declared_to == self.declared_from && self.faces.is_empty()
    }

    /// Points: declared inside the object plus used by its faces, ascending, re-indexed.
    fn finish(self, global: &[DVec3], flip_z: bool) -> Option<ObjObject> {
        if self.faces.is_empty() {
            return None;
        }
        let mut used: BTreeSet<usize> = (self.declared_from..self.declared_to).collect();
        used.extend(self.faces.iter().flatten());
        let local: BTreeMap<usize, u32> = used
            .iter()
            .enumerate()
            .map(|(i, g)| (*g, i as u32))
            .collect();
        let mut points: Vec<DVec3> = used.iter().map(|g| global[*g]).collect();
        let mut polys: Vec<Vec<u32>> = self
            .faces
            .iter()
            .map(|f| f.iter().map(|g| local[g]).collect())
            .collect();
        if flip_z {
            for p in &mut points {
                p.z = -p.z;
            }
            for poly in &mut polys {
                poly.reverse();
            }
        }
        Some(ObjObject {
            name: self.name,
            mesh: PolyMesh { points, polys },
            world: DMat4::IDENTITY,
            smoothing_angle: Some(DEFAULT_SMOOTHING_DEG.to_radians()),
        })
    }
}

/// Lines with a trailing `\` joined to the next; each with its 1-based start line.
fn logical_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut joined: Option<(usize, String)> = None;
    for (i, raw) in text.lines().enumerate() {
        let (start, mut s) = joined.take().unwrap_or((i + 1, String::new()));
        match raw.trim_end().strip_suffix('\\') {
            Some(head) => {
                s.push_str(head);
                s.push(' ');
                joined = Some((start, s));
            }
            None => {
                s.push_str(raw);
                out.push((start, s));
            }
        }
    }
    if let Some(j) = joined {
        out.push(j);
    }
    out
}

pub fn read(text: &str, opts: &ReadOptions) -> Result<Vec<ObjObject>, ObjError> {
    let mut global: Vec<DVec3> = Vec::new();
    let mut done: Vec<Pending> = Vec::new();
    let mut current: Option<Pending> = None;
    for (line, content) in logical_lines(text) {
        let err = |message: String| ObjError { line, message };
        let content = content.split('#').next().unwrap_or("");
        let mut words = content.split_whitespace();
        let Some(word) = words.next() else { continue };
        match word {
            "v" => {
                let c: Vec<&str> = words.collect();
                if c.len() < 3 {
                    return Err(err("a point needs x y z".into()));
                }
                let mut v = [0.0; 3];
                for (slot, s) in v.iter_mut().zip(&c) {
                    *slot = s
                        .parse::<f64>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .ok_or_else(|| err(format!("bad coordinate {s:?}")))?;
                }
                global.push(DVec3::from_array(v));
                let cur = current.get_or_insert_with(|| Pending::new("Object", global.len() - 1));
                cur.declared_to = global.len();
            }
            "f" => {
                let mut face = Vec::new();
                for tok in words {
                    let first = tok.split('/').next().unwrap_or("");
                    let i: i64 = first
                        .parse()
                        .map_err(|_| err(format!("bad face index {tok:?}")))?;
                    let n = global.len() as i64;
                    let g = match i {
                        0 => return Err(err("face index 0".into())),
                        i if i > 0 => i - 1,
                        i => n + i,
                    };
                    if g < 0 || g >= n {
                        return Err(err(format!("face index {i} out of range ({n} points)")));
                    }
                    face.push(g as usize);
                }
                if face.len() < 3 {
                    return Err(err("a face needs at least 3 corners".into()));
                }
                current
                    .get_or_insert_with(|| Pending::new("Object", global.len()))
                    .faces
                    .push(face);
            }
            "o" | "g" => {
                let rest = content.trim_start()[word.len()..].trim();
                let name = if rest.is_empty() { "Object" } else { rest };
                match current.as_mut() {
                    Some(cur) if cur.is_empty() => cur.name = name.to_string(),
                    _ => {
                        done.extend(current.take());
                        current = Some(Pending::new(name, global.len()));
                    }
                }
            }
            _ => {}
        }
    }
    done.extend(current);
    Ok(done
        .into_iter()
        .filter_map(|p| p.finish(&global, opts.flip_z))
        .collect())
}
