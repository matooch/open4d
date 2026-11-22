//! PLY writer and reader, ASCII and binary (docs/specs/IO.md §4.2). Own design: PLY was not
//! observed.

use std::fmt::Write as _;

use forge_geometry::PolyMesh;
use glam::{DMat4, DVec3};

use crate::obj::ObjObject;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WriteOptions {
    /// `binary_little_endian` (default) or ASCII.
    pub binary: bool,
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
    pub scale: f64,
}

impl Default for ReadOptions {
    fn default() -> Self {
        ReadOptions { scale: 1.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct PlyError(pub String);

fn num(x: f64) -> String {
    let x = x as f32;
    let x = if x == 0.0 { 0.0 } else { x };
    x.to_string()
}

pub fn write(objects: &[ObjObject], opts: &WriteOptions) -> Vec<u8> {
    let mut points: Vec<DVec3> = Vec::new();
    let mut faces: Vec<Vec<u32>> = Vec::new();
    for o in objects {
        let base = points.len() as u32;
        let world: DMat4 = o.world;
        points.extend(
            o.mesh
                .points
                .iter()
                .map(|p| world.transform_point3(*p) * opts.scale),
        );
        faces.extend(
            o.mesh
                .polys
                .iter()
                .map(|f| f.iter().map(|i| i + base).collect::<Vec<u32>>()),
        );
    }
    let mut head = String::from("ply\n");
    head.push_str(if opts.binary {
        "format binary_little_endian 1.0\n"
    } else {
        "format ascii 1.0\n"
    });
    let _ = write!(
        head,
        "comment Forge3D PLY export\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nelement face {}\nproperty list uchar uint vertex_indices\nend_header\n",
        points.len(),
        faces.len()
    );
    let mut out = head.into_bytes();
    if opts.binary {
        for p in &points {
            for c in p.to_array() {
                out.extend((c as f32).to_le_bytes());
            }
        }
        for f in &faces {
            out.push(f.len().min(255) as u8);
            for i in f {
                out.extend(i.to_le_bytes());
            }
        }
    } else {
        let mut body = String::new();
        for p in &points {
            let _ = writeln!(body, "{} {} {}", num(p.x), num(p.y), num(p.z));
        }
        for f in &faces {
            let _ = write!(body, "{}", f.len());
            for i in f {
                let _ = write!(body, " {i}");
            }
            body.push('\n');
        }
        out.extend(body.into_bytes());
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
enum Ty {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}

impl Ty {
    fn parse(s: &str) -> Option<Ty> {
        Some(match s {
            "char" | "int8" => Ty::I8,
            "uchar" | "uint8" => Ty::U8,
            "short" | "int16" => Ty::I16,
            "ushort" | "uint16" => Ty::U16,
            "int" | "int32" => Ty::I32,
            "uint" | "uint32" => Ty::U32,
            "float" | "float32" => Ty::F32,
            "double" | "float64" => Ty::F64,
            _ => return None,
        })
    }

    fn size(self) -> usize {
        match self {
            Ty::I8 | Ty::U8 => 1,
            Ty::I16 | Ty::U16 => 2,
            Ty::I32 | Ty::U32 | Ty::F32 => 4,
            Ty::F64 => 8,
        }
    }
}

enum Prop {
    Scalar(String, Ty),
    List(String, Ty, Ty),
}

struct Element {
    name: String,
    count: usize,
    props: Vec<Prop>,
}

#[derive(Clone, Copy, PartialEq)]
enum Enc {
    Ascii,
    Little,
    Big,
}

/// A cursor over the body: binary values, or whitespace separated ASCII tokens.
struct Body<'a> {
    enc: Enc,
    bytes: &'a [u8],
    pos: usize,
    tokens: std::str::SplitAsciiWhitespace<'a>,
}

impl Body<'_> {
    fn value(&mut self, ty: Ty) -> Result<f64, PlyError> {
        let short = || PlyError("the file is shorter than its header declares".into());
        if self.enc == Enc::Ascii {
            let t = self.tokens.next().ok_or_else(short)?;
            return t
                .parse::<f64>()
                .ok()
                .filter(|x| x.is_finite())
                .ok_or_else(|| PlyError(format!("bad number \"{t}\"")));
        }
        let n = ty.size();
        let raw = self.bytes.get(self.pos..self.pos + n).ok_or_else(short)?;
        self.pos += n;
        let mut b = [0u8; 8];
        b[..n].copy_from_slice(raw);
        if self.enc == Enc::Big {
            b[..n].reverse();
        }
        Ok(match ty {
            Ty::I8 => f64::from(b[0] as i8),
            Ty::U8 => f64::from(b[0]),
            Ty::I16 => f64::from(i16::from_le_bytes([b[0], b[1]])),
            Ty::U16 => f64::from(u16::from_le_bytes([b[0], b[1]])),
            Ty::I32 => f64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            Ty::U32 => f64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            Ty::F32 => f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            Ty::F64 => f64::from_le_bytes(b),
        })
    }
}

pub fn read(bytes: &[u8], opts: &ReadOptions) -> Result<Vec<ObjObject>, PlyError> {
    if !bytes.starts_with(b"ply") {
        return Err(PlyError("not a PLY file: missing the ply magic".into()));
    }
    let end = b"end_header";
    let at = bytes
        .windows(end.len())
        .position(|w| w == end)
        .ok_or_else(|| PlyError("missing end_header".into()))?;
    let mut body_start = at + end.len();
    // the header ends at the line break after `end_header` (`\n` or `\r\n`)
    if bytes.get(body_start) == Some(&b'\r') {
        body_start += 1;
    }
    if bytes.get(body_start) == Some(&b'\n') {
        body_start += 1;
    }
    let header =
        std::str::from_utf8(&bytes[..at]).map_err(|_| PlyError("the header is not text".into()))?;
    let mut lines = header.lines().map(str::trim);
    if lines.next() != Some("ply") {
        return Err(PlyError("not a PLY file: missing the ply magic".into()));
    }
    let mut enc = None;
    let mut elements: Vec<Element> = Vec::new();
    for line in lines {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.as_slice() {
            ["format", kind, "1.0"] => {
                enc = Some(match *kind {
                    "ascii" => Enc::Ascii,
                    "binary_little_endian" => Enc::Little,
                    "binary_big_endian" => Enc::Big,
                    other => return Err(PlyError(format!("unknown format \"{other}\""))),
                });
            }
            ["format", ..] => return Err(PlyError(format!("unsupported format line \"{line}\""))),
            ["element", name, count] => elements.push(Element {
                name: (*name).to_string(),
                count: count
                    .parse()
                    .map_err(|_| PlyError(format!("bad element count in \"{line}\"")))?,
                props: Vec::new(),
            }),
            ["property", "list", ct, it, name] => {
                let (Some(ct), Some(it)) = (Ty::parse(ct), Ty::parse(it)) else {
                    return Err(PlyError(format!("unknown type in \"{line}\"")));
                };
                elements
                    .last_mut()
                    .ok_or_else(|| PlyError("a property before any element".into()))?
                    .props
                    .push(Prop::List((*name).to_string(), ct, it));
            }
            ["property", ty, name] => {
                let Some(ty) = Ty::parse(ty) else {
                    return Err(PlyError(format!("unknown type in \"{line}\"")));
                };
                elements
                    .last_mut()
                    .ok_or_else(|| PlyError("a property before any element".into()))?
                    .props
                    .push(Prop::Scalar((*name).to_string(), ty));
            }
            _ => {}
        }
    }
    let enc =
        enc.ok_or_else(|| PlyError("no format line: the format keyword is missing".into()))?;
    let text = if enc == Enc::Ascii {
        std::str::from_utf8(&bytes[body_start..])
            .map_err(|_| PlyError("the body is not text".into()))?
    } else {
        ""
    };
    let mut body = Body {
        enc,
        bytes,
        pos: body_start,
        tokens: text.split_ascii_whitespace(),
    };
    let mut points: Vec<DVec3> = Vec::new();
    let mut polys: Vec<Vec<u32>> = Vec::new();
    for el in &elements {
        for row in 0..el.count {
            let mut xyz = [0.0f64; 3];
            for prop in &el.props {
                match prop {
                    Prop::Scalar(name, ty) => {
                        let v = body.value(*ty)?;
                        if el.name == "vertex" {
                            match name.as_str() {
                                "x" => xyz[0] = v,
                                "y" => xyz[1] = v,
                                "z" => xyz[2] = v,
                                _ => {}
                            }
                        }
                    }
                    Prop::List(name, ct, it) => {
                        let n = body.value(*ct)? as usize;
                        let mut idx = Vec::with_capacity(n);
                        for _ in 0..n {
                            idx.push(body.value(*it)?);
                        }
                        let face_list = el.name == "face"
                            && (name == "vertex_indices" || name == "vertex_index");
                        if face_list {
                            if n < 3 {
                                return Err(PlyError(format!("face {row}: fewer than 3 corners")));
                            }
                            polys.push(idx.iter().map(|i| *i as u32).collect());
                        }
                    }
                }
            }
            if el.name == "vertex" {
                points.push(DVec3::from_array(xyz) * opts.scale);
            }
        }
    }
    for (row, f) in polys.iter().enumerate() {
        if f.iter().any(|i| *i as usize >= points.len()) {
            return Err(PlyError(format!("face {row}: index out of range")));
        }
    }
    Ok(vec![ObjObject {
        name: "Object".into(),
        mesh: PolyMesh { points, polys },
        world: DMat4::IDENTITY,
        smoothing_angle: Some(40f64.to_radians()),
        uvs: None,
    }])
}
