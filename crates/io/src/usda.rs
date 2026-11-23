//! Single-layer OpenUSD text exchange (docs/specs/IO.md §6).

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_core::transform::{POS, ROT, SCALE, Transform, hpb_from_rotation};
use forge_core::{EditError, Interp, Key, NodeId, Track, Tx, Value};
use forge_geometry::{CornerUvs, PolyMesh};
use glam::{DMat3, DVec3, EulerRot};

mod parse;
mod write;
pub use write::write;

use parse::{Data, Prim};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid USDA: {0}")]
    Invalid(String),
    #[error("unsupported USDA feature: {0}")]
    Unsupported(&'static str),
    #[error("Nothing to export")]
    Empty,
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::Invalid(reason.into())
}

const OPS: [&str; 3] = ["xformOp:translate", "xformOp:rotateXYZ", "xformOp:scale"];
const PARAMS: [&str; 3] = [POS, ROT, SCALE];
const COMPONENTS: [&str; 3] = ["x", "y", "z"];
const BASIS: DMat3 = DMat3::from_diagonal(DVec3::new(1.0, 1.0, -1.0));

struct Mesh {
    geometry: PolyMesh,
    uv: Option<CornerUvs>,
}

struct Node {
    name: String,
    parent: Option<usize>,
    mesh: Option<Mesh>,
    material: Option<String>,
    transform: Transform,
    tracks: Vec<(String, Track)>,
}

struct Material {
    name: String,
    color: [f64; 3],
    roughness: f64,
}

/// Parsed, validated USDA layer, independent of any document.
#[derive(Default)]
pub struct ImportScene {
    nodes: Vec<Node>,
    materials: BTreeMap<String, Material>,
}

/// Parse the supported ASCII layer subset.
pub fn read(text: &str) -> Result<ImportScene, Error> {
    let (metadata, roots) = parse::parse(text)?;
    if metadata
        .get("upAxis")
        .map(Data::text)
        .transpose()?
        .is_some_and(|v| v != "Y")
    {
        return Err(Error::Unsupported("non-Y-up stage"));
    }
    let scale = metadata
        .get("metersPerUnit")
        .map(Data::number)
        .transpose()?
        .unwrap_or(0.01)
        / 0.01;
    let rate = metadata
        .get("timeCodesPerSecond")
        .or_else(|| metadata.get("framesPerSecond"))
        .map(Data::number)
        .transpose()?
        .unwrap_or(24.0);
    if !scale.is_finite() || scale <= 0.0 || rate <= 0.0 {
        return Err(invalid("stage units and frame rate must be positive"));
    }
    let mut scene = ImportScene::default();
    scene.decode(&roots, "", None, scale, rate)?;
    if scene
        .nodes
        .iter()
        .filter_map(|n| n.material.as_ref())
        .any(|p| !scene.materials.contains_key(p))
    {
        return Err(invalid("material binding has no in-layer preview surface"));
    }
    Ok(scene)
}

impl ImportScene {
    fn decode(
        &mut self,
        prims: &[Prim],
        path: &str,
        parent: Option<usize>,
        scale: f64,
        rate: f64,
    ) -> Result<(), Error> {
        for p in prims {
            let path = format!("{path}/{}", p.name);
            let next = match p.kind.as_str() {
                "Scope" => parent,
                "Material" => {
                    self.materials
                        .insert(path.clone(), read_material(p, &path)?);
                    continue;
                }
                "Mesh" | "Xform" => {
                    let mesh = (p.kind == "Mesh")
                        .then(|| read_mesh(p, scale))
                        .transpose()?;
                    let (transform, tracks) = read_transform(p, scale, rate)?;
                    let id = self.nodes.len();
                    self.nodes.push(Node {
                        name: p.display_name()?.into(),
                        parent,
                        mesh,
                        material: p
                            .attr("material:binding")
                            .map(Data::path)
                            .transpose()?
                            .map(str::to_owned),
                        transform,
                        tracks,
                    });
                    Some(id)
                }
                _ => {
                    return Err(Error::Unsupported(
                        "prim type outside the mesh/Xform/Material subset",
                    ));
                }
            };
            self.decode(&p.children, &path, next, scale, rate)?;
        }
        Ok(())
    }

    /// Build native geometry, hosted materials, and tracks in the caller's transaction.
    pub fn apply(&self, tx: &mut Tx<'_>) -> Result<Vec<NodeId>, EditError> {
        if self.nodes.is_empty() {
            return Ok(Vec::new());
        }
        let mut materials = BTreeMap::new();
        for (path, m) in &self.materials {
            let mat = tx.create_material(forge_material::MATERIAL, None)?;
            tx.rename(mat, &m.name)?;
            tx.set_param(mat, "color.color", Value::Color(m.color))?;
            tx.set_param(mat, "color.roughness", Value::Float(m.roughness))?;
            materials.insert(path, mat);
        }
        let mut ids = Vec::new();
        let mut roots = Vec::new();
        for n in &self.nodes {
            let kind = if n.mesh.is_some() {
                forge_generators::POLYGON
            } else {
                "forge.null"
            };
            let id = tx.create(kind, n.parent.map(|i| ids[i]), None)?;
            ids.push(id);
            if n.parent.is_none() {
                roots.push(id);
            }
            tx.rename(id, &n.name)?;
            for (param, component) in
                PARAMS
                    .into_iter()
                    .zip([n.transform.pos, n.transform.rot, n.transform.scale])
            {
                tx.set_param(id, param, Value::Vec3(component.to_array()))?;
            }
            for (track, value) in &n.tracks {
                tx.set_track(id, track, Some(value.clone()))?;
            }
            if let Some(m) = &n.mesh {
                tx.set_param(
                    id,
                    "mesh",
                    Value::Mesh(Arc::new(forge_generators::from_poly_mesh(&m.geometry))),
                )?;
                if let Some(uv) = &m.uv {
                    let tag = tx.attach_tag(forge_material::UVW_TAG, id, None)?;
                    tx.set_param(
                        tag,
                        "uvw",
                        Value::FloatList(uv.corners.iter().flatten().flatten().copied().collect()),
                    )?;
                }
            }
            if let Some(m) = &n.material {
                let tag = tx.attach_tag(forge_material::TEXTURE_TAG, id, None)?;
                tx.set_param(tag, "material", Value::Link(Some(materials[m])))?;
            }
        }
        Ok(roots)
    }
}

fn read_material(p: &Prim, path: &str) -> Result<Material, Error> {
    let link = p
        .attr("outputs:surface.connect")
        .ok_or(Error::Unsupported("unconnected material"))?
        .path()?;
    let source = link
        .strip_prefix(path)
        .and_then(|s| s.strip_prefix('/'))
        .and_then(|s| s.strip_suffix(".outputs:surface"));
    let shader = p
        .children
        .iter()
        .find(|c| Some(c.name.as_str()) == source && c.kind == "Shader")
        .ok_or(Error::Unsupported("nonlocal surface connection"))?;
    if shader.attr("info:id").map(Data::text).transpose()? != Some("UsdPreviewSurface")
        || shader.attr("inputs:diffuseColor.connect").is_some()
        || shader.attr("inputs:roughness.connect").is_some()
    {
        return Err(Error::Unsupported(
            "surface other than constant UsdPreviewSurface",
        ));
    }
    let color = shader
        .attr("inputs:diffuseColor")
        .map(Data::tuple::<3>)
        .transpose()?
        .unwrap_or([0.18; 3]);
    let roughness = shader
        .attr("inputs:roughness")
        .map(Data::number)
        .transpose()?
        .unwrap_or(0.5);
    if !(0.0..=1.0).contains(&roughness) {
        return Err(invalid("roughness outside 0..1"));
    }
    Ok(Material {
        name: p.display_name()?.into(),
        color,
        roughness,
    })
}

fn topology_field<'a>(p: &'a Prim, name: &str) -> Result<&'a [Data], Error> {
    p.attr(name)
        .ok_or_else(|| invalid(format!("Mesh missing {name}")))?
        .list()
}

fn read_mesh(p: &Prim, scale: f64) -> Result<Mesh, Error> {
    if [
        "points.timeSamples",
        "faceVertexCounts.timeSamples",
        "faceVertexIndices.timeSamples",
        "primvars:st.timeSamples",
        "normals.timeSamples",
    ]
    .iter()
    .any(|k| p.attr(k).is_some())
    {
        return Err(Error::Unsupported("time-varying mesh data"));
    }
    if p.attr("subdivisionScheme")
        .map(Data::text)
        .transpose()?
        .is_some_and(|s| s != "none")
    {
        return Err(Error::Unsupported("subdivision scheme"));
    }
    let points = topology_field(p, "points")?
        .iter()
        .map(|v| Ok(BASIS * DVec3::from_array(v.tuple()?) * scale))
        .collect::<Result<Vec<_>, Error>>()?;
    if points.iter().any(|v| !v.is_finite()) {
        return Err(invalid("scaled point is not finite"));
    }
    let indices = topology_field(p, "faceVertexIndices")?
        .iter()
        .map(Data::index)
        .collect::<Result<Vec<_>, _>>()?;
    let mut offset = 0;
    let mut polys = Vec::new();
    for n in topology_field(p, "faceVertexCounts")? {
        let n = n.index()? as usize;
        let face = indices
            .get(offset..offset + n)
            .ok_or_else(|| invalid("face counts exceed index count"))?;
        polys.push(face.to_vec());
        offset += n;
    }
    if offset != indices.len() {
        return Err(invalid("face counts differ from index count"));
    }
    let mut geometry = PolyMesh { points, polys };
    validate_topology(&geometry)?;
    if let Some((values, meta)) = p.fields.get("normals") {
        let mode = meta
            .get("interpolation")
            .map(Data::text)
            .transpose()?
            .unwrap_or("vertex");
        if values.list()?.len() != interpolation_count(&geometry, mode)? {
            return Err(invalid("normal array length does not match interpolation"));
        }
        for n in values.list()? {
            n.tuple::<3>()?;
        }
    }
    let mut uv = p
        .fields
        .get("primvars:st")
        .map(|(values, meta)| {
            let mode = meta
                .get("interpolation")
                .map(Data::text)
                .transpose()?
                .unwrap_or("constant");
            let coords = values
                .list()?
                .iter()
                .map(Data::tuple::<2>)
                .collect::<Result<Vec<_>, _>>()?;
            let coords = if let Some(indices) = p.attr("primvars:st:indices") {
                indices
                    .list()?
                    .iter()
                    .map(|i| {
                        coords
                            .get(i.index()? as usize)
                            .copied()
                            .ok_or_else(|| invalid("ST index out of range"))
                    })
                    .collect::<Result<Vec<_>, Error>>()?
            } else {
                coords
            };
            if coords.len() != interpolation_count(&geometry, mode)? {
                return Err(invalid("ST array length does not match interpolation"));
            }
            let mut corner = 0;
            let corners = geometry
                .polys
                .iter()
                .enumerate()
                .map(|(f, face)| {
                    face.iter()
                        .map(|&i| {
                            let index = match mode {
                                "faceVarying" => corner,
                                "vertex" | "varying" => i as usize,
                                "uniform" => f,
                                _ => 0,
                            };
                            corner += 1;
                            coords[index]
                        })
                        .collect()
                })
                .collect();
            Ok::<_, Error>(CornerUvs { corners })
        })
        .transpose()?;
    match p
        .attr("orientation")
        .map(Data::text)
        .transpose()?
        .unwrap_or("rightHanded")
    {
        "rightHanded" => {
            geometry.polys.iter_mut().for_each(|p| p.reverse());
            if let Some(uv) = &mut uv {
                uv.corners.iter_mut().for_each(|p| p.reverse());
            }
        }
        "leftHanded" => {}
        _ => return Err(invalid("unknown orientation")),
    }
    Ok(Mesh { geometry, uv })
}

fn validate_topology(mesh: &PolyMesh) -> Result<(), Error> {
    // Preserve repeated corner indices in primitive output; USDA topology only
    // requires consistent counts and in-range indices.
    if mesh
        .polys
        .iter()
        .any(|face| face.len() < 3 || face.iter().any(|&i| i as usize >= mesh.points.len()))
    {
        return Err(invalid("invalid face counts or point indices"));
    }
    Ok(())
}

fn interpolation_count(mesh: &PolyMesh, mode: &str) -> Result<usize, Error> {
    match mode {
        "constant" => Ok(1),
        "uniform" => Ok(mesh.polys.len()),
        "vertex" | "varying" => Ok(mesh.points.len()),
        "faceVarying" => Ok(mesh.polys.iter().map(Vec::len).sum()),
        _ => Err(Error::Unsupported("interpolation token")),
    }
}

type Tracks = Vec<(String, Track)>;

fn read_transform(p: &Prim, scale: f64, rate: f64) -> Result<(Transform, Tracks), Error> {
    let order = p
        .attr("xformOpOrder")
        .map(Data::list)
        .transpose()?
        .unwrap_or(&[]);
    let mut t = Transform::IDENTITY;
    let mut tracks = Vec::new();
    let mut previous = 0;
    for name in order {
        let name = name.text()?;
        let Some(op) = OPS.iter().position(|n| *n == name) else {
            return Err(Error::Unsupported("transform operation"));
        };
        if op < previous {
            return Err(Error::Unsupported("non-TRS xformOpOrder"));
        }
        previous = op + 1;
        let convert = |v: &Data| -> Result<DVec3, Error> {
            let [x, y, z] = v.tuple::<3>()?;
            Ok(match op {
                0 => BASIS * DVec3::new(x, y, z) * scale,
                1 => {
                    let r = DMat3::from_euler(
                        EulerRot::ZYX,
                        z.to_radians(),
                        y.to_radians(),
                        x.to_radians(),
                    );
                    hpb_from_rotation(BASIS * r * BASIS)
                }
                _ => DVec3::new(x, y, z),
            })
        };
        let mut samples = match p.attr(&format!("{name}.timeSamples")) {
            Some(Data::Map(m)) => m
                .iter()
                .map(|(t, v)| Ok((*t / rate, convert(v)?)))
                .collect::<Result<Vec<_>, Error>>()?,
            None => Vec::new(),
            _ => return Err(invalid("expected dictionary of samples")),
        };
        samples.sort_by(|a, b| a.0.total_cmp(&b.0));
        let default = p
            .attr(name)
            .map(convert)
            .transpose()?
            .or_else(|| samples.first().map(|(_, v)| *v))
            .ok_or_else(|| invalid(format!("no value declared for {name}")))?;
        match op {
            0 => t.pos = default,
            1 => t.rot = default,
            _ => t.scale = default,
        }
        if !samples.is_empty() {
            for (i, axis) in COMPONENTS.into_iter().enumerate() {
                let track = Track::new(
                    samples
                        .iter()
                        .map(|(t, v)| Key::new(*t, v[i]).with_interp(Interp::Linear))
                        .collect(),
                );
                track.validate().map_err(|e| invalid(e.to_string()))?;
                tracks.push((format!("{}.{axis}", PARAMS[op]), track));
            }
        }
    }
    Ok((t, tracks))
}
