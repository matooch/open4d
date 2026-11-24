//! glTF 2.0 exchange (IO.md §5).

use std::collections::BTreeMap;
use std::sync::Arc;

use base64::Engine;
use forge_core::transform::{POS, ROT, SCALE, Transform, hpb_from_rotation};
use forge_core::{EditError, Interp, Key, NodeId, Track, Tx, Value};
use forge_geometry::{CornerUvs, PolyMesh};
use forge_material::node_material::{GRAPH, NODE_MATERIAL, SurfaceValues};
use forge_rig::weights::{WeightMap, matrix_to_entries};
use glam::{DMat3, DMat4, DQuat, DVec3};
use gltf::animation::{Interpolation, util::ReadOutputs};
use gltf::mesh::{Mode, util::ReadTexCoords};

mod write;
pub use write::write;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid glTF: {0}")]
    Invalid(String),
    #[error("unsupported glTF feature: {0}")]
    Unsupported(&'static str),
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::Invalid(reason.into())
}

struct Primitive {
    mesh: PolyMesh,
    uv: Option<CornerUvs>,
    material: Option<usize>,
    weights: WeightMap,
}

struct Node {
    name: String,
    children: Vec<usize>,
    transform: Transform,
    primitives: Vec<Primitive>,
    skin: Option<usize>,
    tracks: BTreeMap<String, Track>,
}

struct Material {
    name: String,
    graph: String,
}

struct Skin {
    joints: Vec<usize>,
    inverse_bind: Vec<DMat4>,
}

/// Parsed exchange data. Applying it is a separate, undoable scene edit.
pub struct ImportScene {
    roots: Vec<usize>,
    nodes: Vec<Node>,
    materials: Vec<Material>,
    skins: Vec<Skin>,
}

fn point([x, y, z]: [f32; 3], scale: f64) -> DVec3 {
    DVec3::new(f64::from(x), f64::from(y), -f64::from(z)) * scale
}

fn rotation(q: [f32; 4]) -> DVec3 {
    let [x, y, z, w] = q.map(f64::from);
    hpb_from_rotation(DMat3::from_quat(DQuat::from_xyzw(-x, -y, z, w).normalize()))
}

fn swap_matrix(m: DMat4, scale: f64) -> DMat4 {
    let basis = DMat4::from_scale(DVec3::new(1.0, 1.0, -1.0));
    let mut out = basis * m * basis;
    out.w_axis.x *= scale;
    out.w_axis.y *= scale;
    out.w_axis.z *= scale;
    out
}

fn read_primitive(
    p: gltf::Primitive<'_>,
    buffers: &[Vec<u8>],
    scale: f64,
) -> Result<Primitive, Error> {
    if p.morph_targets().len() != 0 {
        return Err(Error::Unsupported("morph targets"));
    }
    if p.mode() != Mode::Triangles {
        return Err(Error::Unsupported("non-triangle primitive topology"));
    }
    let read = p.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
    let points: Vec<_> = read
        .read_positions()
        .ok_or_else(|| invalid("missing positions"))?
        .map(|p| point(p, scale))
        .collect();
    let indices: Vec<_> = read.read_indices().map_or_else(
        || (0..points.len() as u32).collect(),
        |i| i.into_u32().collect(),
    );
    let (triangles, remainder) = indices.as_chunks::<3>();
    if !remainder.is_empty() || indices.iter().any(|&i| i as usize >= points.len()) {
        return Err(invalid("invalid triangle indices"));
    }
    let polys: Vec<Vec<u32>> = triangles.iter().map(|t| vec![t[0], t[2], t[1]]).collect();
    let uv = read
        .read_tex_coords(0)
        .map(|coords: ReadTexCoords<'_>| -> Result<CornerUvs, Error> {
            let coords: Vec<_> = coords
                .into_f32()
                .map(|[u, v]| [f64::from(u), 1.0 - f64::from(v)])
                .collect();
            if coords.len() != points.len() {
                return Err(invalid("UV count differs from position count"));
            }
            Ok(CornerUvs {
                corners: polys
                    .iter()
                    .map(|t| t.iter().map(|&i| coords[i as usize]).collect())
                    .collect(),
            })
        })
        .transpose()?;
    let mut weights = WeightMap::new();
    match (read.read_joints(0), read.read_weights(0)) {
        (Some(j), Some(w)) => {
            let (j, w) = (j.into_u16(), w.into_f32());
            if j.len() != points.len() || w.len() != points.len() {
                return Err(invalid("invalid skin weights"));
            }
            for (i, (j, w)) in j.zip(w).enumerate() {
                weights.set(
                    i as u32,
                    j.into_iter()
                        .zip(w)
                        .map(|(j, w)| (u32::from(j), f64::from(w)))
                        .collect(),
                );
            }
        }
        (None, None) => {}
        _ => return Err(invalid("invalid skin weights")),
    }
    if read.read_joints(1).is_some() || read.read_weights(1).is_some() {
        return Err(Error::Unsupported("more than one set of skin influences"));
    }
    Ok(Primitive {
        mesh: PolyMesh { points, polys },
        uv,
        material: p.material().index(),
        weights,
    })
}

fn read_buffers(file: &gltf::Gltf) -> Result<Vec<Vec<u8>>, Error> {
    file.buffers()
        .map(|b| {
            let data = match b.source() {
                gltf::buffer::Source::Bin => file
                    .blob
                    .clone()
                    .ok_or_else(|| invalid("missing BIN chunk"))?,
                gltf::buffer::Source::Uri(uri) => {
                    let Some((_, data)) = uri
                        .strip_prefix("data:")
                        .and_then(|s| s.split_once(";base64,"))
                    else {
                        return Err(Error::Unsupported("external buffers"));
                    };
                    base64::engine::general_purpose::STANDARD
                        .decode(data)
                        .map_err(|e| invalid(e.to_string()))?
                }
            };
            if data.len() < b.length() {
                return Err(invalid("short buffer"));
            }
            Ok(data)
        })
        .collect()
}

/// Decode an embedded core glTF image. Non-data URIs are left to the caller.
pub fn decode_embedded_image(uri: &str) -> Result<Option<(&str, Vec<u8>)>, Error> {
    let Some(data) = uri.strip_prefix("data:") else {
        return Ok(None);
    };
    let (mime, data) = data
        .split_once(";base64,")
        .ok_or_else(|| invalid("invalid image data URI"))?;
    if !matches!(mime, "image/png" | "image/jpeg") {
        return Err(Error::Unsupported("embedded image MIME type"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| invalid(e.to_string()))?;
    Ok(Some((mime, bytes)))
}

fn texture_source(
    texture: gltf::Texture<'_>,
    coords: u32,
    buffers: &[Vec<u8>],
) -> Result<String, Error> {
    if coords != 0 {
        return Err(Error::Unsupported("material texture using a second UV set"));
    }
    match texture.source().source() {
        gltf::image::Source::Uri { uri, .. } => Ok(uri.to_owned()),
        gltf::image::Source::View { view, mime_type } => {
            if !matches!(mime_type, "image/png" | "image/jpeg") {
                return Err(Error::Unsupported("embedded image MIME type"));
            }
            let bytes = view
                .offset()
                .checked_add(view.length())
                .and_then(|end| buffers.get(view.buffer().index())?.get(view.offset()..end))
                .ok_or_else(|| invalid("image buffer view outside buffer"))?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            Ok(format!("data:{mime_type};base64,{encoded}"))
        }
    }
}

fn read_material(mat: gltf::Material<'_>, buffers: &[Vec<u8>]) -> Result<Material, Error> {
    use gltf::material::AlphaMode;
    let pbr = mat.pbr_metallic_roughness();
    let [r, g, b, alpha] = pbr.base_color_factor().map(f64::from);
    let info = |info: Option<gltf::texture::Info<'_>>| {
        info.map_or_else(
            || Ok(String::new()),
            |t| texture_source(t.texture(), t.tex_coord(), buffers),
        )
    };
    let normal = mat.normal_texture();
    let occlusion = mat.occlusion_texture();
    let surface = SurfaceValues {
        base_color: [r, g, b],
        alpha,
        metallic: f64::from(pbr.metallic_factor()),
        roughness: f64::from(pbr.roughness_factor()),
        emission: mat.emissive_factor().map(f64::from),
        alpha_mode: match mat.alpha_mode() {
            AlphaMode::Opaque => "OPAQUE",
            AlphaMode::Mask => "MASK",
            AlphaMode::Blend => "BLEND",
        }
        .into(),
        alpha_cutoff: f64::from(mat.alpha_cutoff().unwrap_or(0.5)),
        double_sided: mat.double_sided(),
        base_texture: info(pbr.base_color_texture())?,
        metallic_roughness_texture: info(pbr.metallic_roughness_texture())?,
        normal_texture: normal
            .as_ref()
            .map(|t| texture_source(t.texture(), t.tex_coord(), buffers))
            .transpose()?
            .unwrap_or_default(),
        normal_strength: normal.as_ref().map_or(1.0, |t| f64::from(t.scale())),
        occlusion_texture: occlusion
            .as_ref()
            .map(|t| texture_source(t.texture(), t.tex_coord(), buffers))
            .transpose()?
            .unwrap_or_default(),
        occlusion_strength: occlusion.as_ref().map_or(1.0, |t| f64::from(t.strength())),
        emissive_texture: info(mat.emissive_texture())?,
    };
    let graph = surface.to_graph().map_err(|e| invalid(e.to_string()))?;
    Ok(Material {
        name: mat.name().unwrap_or("Material").into(),
        graph: graph.to_json(),
    })
}

fn read_animation(
    file: &gltf::Gltf,
    nodes: &mut [Node],
    buffers: &[Vec<u8>],
    scale: f64,
) -> Result<(), Error> {
    let mut animations = file.animations();
    let Some(animation) = animations.next() else {
        return Ok(());
    };
    if animations.next().is_some() {
        return Err(Error::Unsupported("multiple animation clips"));
    }
    for channel in animation.channels() {
        let interp = match channel.sampler().interpolation() {
            Interpolation::Step => Interp::Step,
            Interpolation::Linear => Interp::Linear,
            _ => return Err(Error::Unsupported("cubic animation")),
        };
        let tracks = &mut nodes[channel.target().node().index()].tracks;
        let r = channel.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
        let time: Vec<_> = r
            .read_inputs()
            .ok_or_else(|| invalid("animation input"))?
            .map(f64::from)
            .collect();
        let (param, vals): (_, Vec<DVec3>) = match r
            .read_outputs()
            .ok_or_else(|| invalid("animation output"))?
        {
            ReadOutputs::Translations(v) => (POS, v.map(|p| point(p, scale)).collect()),
            ReadOutputs::Rotations(v) => (ROT, v.into_f32().map(rotation).collect()),
            ReadOutputs::Scales(v) => (
                SCALE,
                v.map(|p| DVec3::from_array(p.map(f64::from))).collect(),
            ),
            ReadOutputs::MorphTargetWeights(_) => return Err(Error::Unsupported("morph targets")),
        };
        if vals.len() != time.len() {
            return Err(invalid("animation input and output count differ"));
        }
        for (i, axis) in ["x", "y", "z"].into_iter().enumerate() {
            let mut track = Track::new(
                time.iter()
                    .zip(&vals)
                    .map(|(&t, v)| Key::new(t, v[i]).with_interp(interp))
                    .collect(),
            );
            track.quaternion = param == ROT;
            track.validate().map_err(|e| invalid(e.to_string()))?;
            tracks.insert(format!("{param}.{axis}"), track);
        }
    }
    Ok(())
}

/// Reads GLB or self-contained JSON glTF. `scale=100` converts metres to centimetres.
pub fn read(bytes: &[u8], scale: f64) -> Result<ImportScene, Error> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(invalid("scale must be positive and finite"));
    }
    let file = gltf::Gltf::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
    let buffers = read_buffers(&file)?;
    let roots = file
        .default_scene()
        .or_else(|| file.scenes().next())
        .ok_or_else(|| invalid("no scene"))?
        .nodes()
        .map(|n| n.index())
        .collect();
    let materials = file
        .materials()
        .map(|mat| read_material(mat, &buffers))
        .collect::<Result<_, _>>()?;
    let skins = file
        .skins()
        .map(|s| -> Result<Skin, Error> {
            let joints: Vec<_> = s.joints().map(|j| j.index()).collect();
            let reader = s.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
            let inverse_bind = reader.read_inverse_bind_matrices().map_or_else(
                || vec![DMat4::IDENTITY; joints.len()],
                |ms| {
                    ms.map(|m| {
                        swap_matrix(
                            DMat4::from_cols_array_2d(&m.map(|c| c.map(f64::from))),
                            scale,
                        )
                    })
                    .collect()
                },
            );
            if joints.len() != inverse_bind.len()
                || inverse_bind
                    .iter()
                    .any(|m| !m.is_finite() || m.determinant() == 0.0)
            {
                return Err(invalid("invalid inverse bind matrices"));
            }
            Ok(Skin {
                joints,
                inverse_bind,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut nodes = file
        .nodes()
        .map(|n| -> Result<Node, Error> {
            let (pos, rot, size) = n.transform().decomposed();
            let primitives = n
                .mesh()
                .map(|m| {
                    m.primitives()
                        .map(|p| read_primitive(p, &buffers, scale))
                        .collect()
                })
                .transpose()?
                .unwrap_or_default();
            Ok(Node {
                name: n.name().unwrap_or("Node").into(),
                children: n.children().map(|c| c.index()).collect(),
                transform: Transform {
                    pos: point(pos, scale),
                    rot: rotation(rot),
                    scale: DVec3::from_array(size.map(f64::from)),
                },
                primitives,
                skin: n.skin().map(|s| s.index()),
                tracks: BTreeMap::new(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    read_animation(&file, &mut nodes, &buffers, scale)?;
    Ok(ImportScene {
        roots,
        nodes,
        materials,
        skins,
    })
}

impl ImportScene {
    /// Install the scene as a transaction, returning the imported roots.
    pub fn apply(&self, tx: &mut Tx<'_>) -> Result<Vec<NodeId>, EditError> {
        let mats = self
            .materials
            .iter()
            .map(|m| {
                let id = tx.create_material(NODE_MATERIAL, None)?;
                tx.rename(id, &m.name)?;
                tx.set_param(id, GRAPH, Value::String(m.graph.clone()))?;
                Ok(id)
            })
            .collect::<Result<Vec<_>, EditError>>()?;
        let mut ids = BTreeMap::new();
        let mut meshes = Vec::new();
        let mut pending: Vec<_> = self.roots.iter().rev().map(|&i| (i, None)).collect();
        let mut roots = Vec::new();
        while let Some((i, parent)) = pending.pop() {
            if ids.contains_key(&i) {
                return Err(EditError::WrongCategory(
                    "glTF node appears twice in the hierarchy".into(),
                ));
            }
            let n = &self.nodes[i];
            let joint = self.skins.iter().any(|s| s.joints.contains(&i));
            let inline_mesh = !joint && n.primitives.len() == 1;
            let kind = if joint {
                forge_rig::JOINT
            } else if inline_mesh {
                forge_generators::POLYGON
            } else {
                "forge.null"
            };
            let id = tx.create(kind, parent, None)?;
            if parent.is_none() {
                roots.push(id);
            }
            ids.insert(i, id);
            tx.rename(id, &n.name)?;
            for (param, val) in [
                (POS, n.transform.pos),
                (ROT, n.transform.rot),
                (SCALE, n.transform.scale),
            ] {
                tx.set_param(id, param, Value::Vec3(val.to_array()))?;
            }
            for (key, track) in &n.tracks {
                tx.set_track(id, key, Some(track.clone()))?;
            }
            for (part, p) in n.primitives.iter().enumerate() {
                let mesh = if inline_mesh {
                    id
                } else {
                    let mesh = tx.create(forge_generators::POLYGON, Some(id), None)?;
                    tx.rename(mesh, &format!("{} {}", n.name, part + 1))?;
                    mesh
                };
                tx.set_param(
                    mesh,
                    "mesh",
                    Value::Mesh(Arc::new(forge_generators::from_poly_mesh(&p.mesh))),
                )?;
                if let Some(uv) = &p.uv {
                    let tag = tx.attach_tag(forge_material::UVW_TAG, mesh, None)?;
                    tx.set_param(
                        tag,
                        "uvw",
                        Value::FloatList(uv.corners.iter().flatten().flatten().copied().collect()),
                    )?;
                }
                if let Some(m) = p.material {
                    let tag = tx.attach_tag(forge_material::TEXTURE_TAG, mesh, None)?;
                    tx.set_param(tag, "material", Value::Link(Some(mats[m])))?;
                    if p.uv.is_some() {
                        tx.set_param(tag, "projection", Value::Enum("uvw".into()))?;
                    }
                }
                if let Some(s) = n.skin {
                    meshes.push((mesh, s, &p.weights));
                }
            }
            pending.extend(n.children.iter().rev().map(|&c| (c, Some(id))));
        }
        for (mesh, skin, weights) in meshes {
            let skin = &self.skins[skin];
            let joints = skin
                .joints
                .iter()
                .map(|i| {
                    ids.get(i).copied().ok_or_else(|| {
                        EditError::WrongCategory(
                            "glTF skin references joint outside the imported scene".into(),
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let tag = tx.attach_tag(forge_rig::WEIGHT_TAG, mesh, None)?;
            tx.set_param(tag, "joints", Value::LinkList(joints))?;
            tx.set_param(tag, "weights", weights.to_value())?;
            let bind = skin
                .inverse_bind
                .iter()
                .flat_map(|m| matrix_to_entries(&m.inverse()))
                .collect();
            tx.set_param(tag, "bind_pose", Value::Vec3List(bind))?;
            tx.create(forge_rig::SKIN, Some(mesh), None)?;
        }
        Ok(roots)
    }
}
