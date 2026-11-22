use std::collections::{BTreeMap, BTreeSet};

use forge_core::transform::{POS, ROT, SCALE, Transform, hpb_from_rotation};
use forge_core::{EvalCtx, Interp, NodeId, Scene, Value};
use forge_geometry::{CornerUvs, PolyMesh, triangulate::triangulate};
use forge_rig::weights::{WeightMap, find_weight_tag, read_bind_pose, read_weights, tag_joints};
use glam::{DMat3, DQuat, DVec3};
use serde_json::{Value as Json, json};

use super::{Error, invalid, swap_matrix};

#[derive(Default)]
struct Buffer {
    bytes: Vec<u8>,
    views: Vec<Json>,
    accessors: Vec<Json>,
}

impl Buffer {
    fn accessor(
        &mut self,
        bytes: impl IntoIterator<Item = u8>,
        component: u32,
        count: usize,
        kind: &str,
        bounds: Option<(Json, Json)>,
    ) -> usize {
        self.bytes.resize(self.bytes.len().next_multiple_of(4), 0);
        let start = self.bytes.len();
        self.bytes.extend(bytes);
        let view = self.views.len();
        self.views
            .push(json!({"buffer":0,"byteOffset":start,"byteLength":self.bytes.len()-start}));
        let mut a = json!({"bufferView":view,"componentType":component,"count":count,"type":kind});
        if let Some((min, max)) = bounds {
            a["min"] = min;
            a["max"] = max;
        }
        self.accessors.push(a);
        self.accessors.len() - 1
    }

    fn floats<const N: usize>(&mut self, values: &[[f32; N]], kind: &str, bounds: bool) -> usize {
        let bounds = bounds.then(|| {
            let mut min = vec![f32::INFINITY; N];
            let mut max = vec![f32::NEG_INFINITY; N];
            for v in values {
                for i in 0..N {
                    min[i] = min[i].min(v[i]);
                    max[i] = max[i].max(v[i]);
                }
            }
            (json!(min), json!(max))
        });
        self.accessor(
            values.iter().flatten().flat_map(|n| n.to_le_bytes()),
            5126,
            values.len(),
            kind,
            bounds,
        )
    }
}

fn position(p: DVec3, scale: f64) -> [f32; 3] {
    (p * DVec3::new(scale, scale, -scale))
        .to_array()
        .map(|v| v as f32)
}

fn rotation(rot: DVec3) -> [f32; 4] {
    let [x, y, z, w] = DQuat::from_mat3(
        &Transform {
            rot,
            ..Transform::IDENTITY
        }
        .rotation(),
    )
    .to_array();
    [-x as f32, -y as f32, z as f32, w as f32]
}

fn primitive(
    buf: &mut Buffer,
    mesh: &PolyMesh,
    uv: Option<&CornerUvs>,
    weights: Option<&WeightMap>,
    mat: Option<usize>,
    scale: f64,
) -> Result<Json, Error> {
    let mut lookup = BTreeMap::new();
    let mut indices = Vec::new();
    let mut positions = Vec::new();
    let mut uv_coords = Vec::new();
    let mut joints = Vec::new();
    let mut influences = Vec::new();
    for (face, tri) in triangulate(mesh) {
        for i in [tri[0], tri[2], tri[1]] {
            let coord = uv.map_or([0.0; 2], |uv| {
                let corner = mesh.polys[face as usize]
                    .iter()
                    .position(|v| *v == i)
                    .unwrap();
                uv.corners[face as usize][corner]
            });
            let key = (i, coord.map(f64::to_bits));
            if let Some(&index) = lookup.get(&key) {
                indices.push(index);
                continue;
            }
            let index = u32::try_from(positions.len()).map_err(|_| invalid("too many vertices"))?;
            positions.push(position(mesh.points[i as usize], scale));
            uv_coords.push([coord[0] as f32, (1.0 - coord[1]) as f32]);
            lookup.insert(key, index);
            indices.push(index);
            if let Some(weights) = weights {
                let row = weights.get(i);
                if row.len() > 4 {
                    return Err(Error::Unsupported("more than four skin influences"));
                }
                let mut slots = [0; 4];
                let mut values = [0.0; 4];
                for (k, &(j, w)) in row.iter().enumerate() {
                    slots[k] = u16::try_from(j).map_err(|_| invalid("too many joints"))?;
                    values[k] = w as f32;
                }
                joints.push(slots);
                influences.push(values);
            }
        }
    }
    if indices.is_empty() || positions.iter().flatten().any(|v| !v.is_finite()) {
        return Err(invalid("mesh has no valid triangles"));
    }
    let mut attrs = json!({"POSITION": buf.floats(&positions, "VEC3", true)});
    if uv.is_some() {
        attrs["TEXCOORD_0"] = json!(buf.floats(&uv_coords, "VEC2", false));
    }
    if weights.is_some() {
        attrs["JOINTS_0"] = json!(buf.accessor(
            joints.iter().flatten().flat_map(|j| j.to_le_bytes()),
            5123,
            joints.len(),
            "VEC4",
            None
        ));
        attrs["WEIGHTS_0"] = json!(buf.floats(&influences, "VEC4", false));
    }
    let accessor = buf.accessor(
        indices.iter().flat_map(|i| i.to_le_bytes()),
        5125,
        indices.len(),
        "SCALAR",
        None,
    );
    let mut p = json!({"attributes":attrs,"indices":accessor});
    if let Some(mat) = mat {
        p["material"] = json!(mat);
    }
    Ok(p)
}

struct Animation {
    samplers: Vec<Json>,
    channels: Vec<Json>,
}

impl Animation {
    fn add_node(
        &mut self,
        scene: &Scene,
        id: NodeId,
        index: usize,
        base: Transform,
        scale: f64,
        buf: &mut Buffer,
    ) -> Result<(), Error> {
        let node = scene.node(id).expect("known node");
        for (param, path, base) in [
            (POS, "translation", base.pos),
            (ROT, "rotation", base.rot),
            (SCALE, "scale", base.scale),
        ] {
            let tracks = ["x", "y", "z"].map(|axis| node.tracks.get(&format!("{param}.{axis}")));
            let mut times: Vec<_> = tracks
                .iter()
                .flatten()
                .flat_map(|t| t.keys.iter().map(|k| k.time))
                .collect();
            if times.is_empty() {
                continue;
            }
            times.sort_by(f64::total_cmp);
            times.dedup();
            let modes: Vec<_> = tracks
                .iter()
                .flatten()
                .flat_map(|t| t.keys.iter().rev().skip(1).map(|k| k.interp))
                .collect();
            let step = modes.contains(&Interp::Step);
            if step && modes.iter().any(|m| *m != Interp::Step) {
                return Err(Error::Unsupported(
                    "mixed step and continuous interpolation",
                ));
            }
            let quaternion = tracks.iter().flatten().any(|t| t.quaternion);
            if modes.contains(&Interp::Spline) || (param == ROT && !step && !quaternion) {
                let fps = f64::from(scene.settings().fps);
                let first = (times[0] * fps).ceil() as i64;
                let last = (times[times.len() - 1] * fps).floor() as i64;
                if last.saturating_sub(first) > 1_000_000 {
                    return Err(Error::Unsupported(
                        "animation exceeds one million baked frames",
                    ));
                }
                times.extend((first..=last).map(|f| f as f64 / fps));
                times.sort_by(f64::total_cmp);
                times.dedup_by(|a, b| *a as f32 == *b as f32);
            }
            let seconds: Vec<_> = times.iter().map(|&t| [t as f32]).collect();
            if seconds.iter().any(|t| !t[0].is_finite() || t[0] < 0.0)
                || seconds.windows(2).any(|s| s[0] >= s[1])
            {
                return Err(Error::Unsupported("animation times outside glTF range"));
            }
            let input = buf.floats(&seconds, "SCALAR", true);
            let samples: Vec<_> = times
                .iter()
                .map(|&t| {
                    if param == ROT && quaternion {
                        forge_anim::quaternion_rotation_at(tracks, base, t)
                    } else {
                        DVec3::from_array(std::array::from_fn(|i| {
                            tracks[i].map_or(base[i], |tr| forge_anim::value_at_exact(tr, &[], t))
                        }))
                    }
                })
                .collect();
            let output = if param == ROT {
                buf.floats(
                    &samples.into_iter().map(rotation).collect::<Vec<_>>(),
                    "VEC4",
                    false,
                )
            } else {
                let data: Vec<_> = samples
                    .into_iter()
                    .map(|p| {
                        if param == POS {
                            position(p, scale)
                        } else {
                            p.to_array().map(|v| v as f32)
                        }
                    })
                    .collect();
                buf.floats(&data, "VEC3", false)
            };
            let sampler = self.samplers.len();
            let interpolation = if step { "STEP" } else { "LINEAR" };
            self.samplers
                .push(json!({"input":input,"output":output,"interpolation":interpolation}));
            self.channels
                .push(json!({"sampler":sampler,"target":{"node":index,"path":path}}));
        }
        Ok(())
    }
}

fn glb(doc: &Json, bytes: Vec<u8>) -> Result<Vec<u8>, Error> {
    let mut text = serde_json::to_vec(doc).map_err(|e| invalid(e.to_string()))?;
    text.resize(text.len().next_multiple_of(4), b' ');
    let bin = bytes.len().next_multiple_of(4);
    let size = 28 + text.len() + bin;
    let size = u32::try_from(size).map_err(|_| invalid("GLB exceeds file size limit"))?;
    let mut out = Vec::new();
    out.extend(b"glTF");
    out.extend(2_u32.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend((text.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(text);
    out.extend((bin as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(bytes);
    out.resize(size as usize, 0);
    Ok(out)
}

/// Write binary GLB. `scale=0.01` converts centimetres to metres.
pub fn write(
    scene: &Scene,
    roots: Option<&[NodeId]>,
    scale: f64,
) -> Result<(Vec<u8>, usize), Error> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(invalid("scale must be positive and finite"));
    }
    let roots = roots.unwrap_or_else(|| scene.roots());
    let mut order = Vec::new();
    let mut seen = BTreeSet::new();
    for r in roots.iter().copied().filter(|&id| scene.contains(id)) {
        for id in std::iter::once(r).chain(scene.descendants(r)) {
            if scene.local_transform(id).is_some()
                && !forge_generators::is_consumed(scene, id)
                && scene.node(id).is_some_and(|n| n.type_id != forge_rig::SKIN)
                && seen.insert(id)
            {
                order.push(id);
            }
        }
    }
    let slots: BTreeMap<_, _> = order.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    let mut top = Vec::new();
    let mut nodes = Vec::new();
    let mut meshes = Vec::new();
    let mut skins = Vec::new();
    let mut materials = Vec::new();
    let mut material_slots = BTreeMap::new();
    let mut buffer = Buffer::default();
    let mut anim = Animation {
        samplers: Vec::new(),
        channels: Vec::new(),
    };
    for (index, id) in order.into_iter().enumerate() {
        let node = scene.node(id).expect("known node");
        let mut t = scene.local_transform(id).expect("object transform");
        if !node.parent.is_some_and(|p| slots.contains_key(&p)) {
            top.push(index);
            if node.parent.is_some() {
                if !node.tracks.is_empty() {
                    return Err(Error::Unsupported(
                        "animated export root with omitted parent",
                    ));
                }
                let world = scene.world_matrix(id).expect("known parents");
                let (scale, rotation, pos) = world.to_scale_rotation_translation();
                t = Transform {
                    pos,
                    rot: hpb_from_rotation(DMat3::from_quat(rotation)),
                    scale,
                };
            }
        }
        let mut n = json!({
            "name":node.name, "translation":position(t.pos, scale), "rotation":rotation(t.rot),
            "scale": t.scale.to_array(),
        });
        let children: Vec<_> = node
            .children
            .iter()
            .filter_map(|child| slots.get(child))
            .collect();
        if !children.is_empty() {
            n["children"] = json!(children);
        }
        anim.add_node(scene, id, index, t, scale, &mut buffer)?;
        let skin_tag = find_weight_tag(scene, id).filter(|_| {
            node.children.iter().any(|&child| {
                scene
                    .node(child)
                    .is_some_and(|n| n.type_id == forge_rig::SKIN)
            })
        });
        let mesh = if skin_tag.is_some() {
            match scene.param(id, "mesh") {
                Some(Value::Mesh(mesh)) => Some(forge_generators::to_poly_mesh(mesh)),
                _ => return Err(Error::Unsupported("skin without polygon bind mesh")),
            }
        } else {
            forge_generators::evaluate(scene, id, &EvalCtx::default())
                .ok()
                .flatten()
        };
        if let Some(mesh) = mesh.filter(|m| !m.polys.is_empty()) {
            let mat = node.tags.iter().find_map(|&tag| {
                let Some(Value::Link(Some(m))) = scene.param(tag, "material") else {
                    return None;
                };
                scene
                    .node(*m)
                    .filter(|m| m.type_id == forge_material::MATERIAL)
                    .map(|_| *m)
            });
            let mat = mat.map(|id| *material_slots.entry(id).or_insert_with(|| {
                let [r,g,b] = match scene.param(id, "color.color") {
                    Some(Value::Color(c)) => c.map(|v| v.clamp(0.0, 1.0)),
                    _ => [0.8; 3],
                };
                let roughness = match scene.param(id, "color.roughness") {
                    Some(Value::Float(v)) => v.clamp(0.0, 1.0),
                    _ => 0.5,
                };
                materials.push(json!({"name":scene.node(id).unwrap().name,
                    "pbrMetallicRoughness":{"baseColorFactor":[r,g,b,1.0],"roughnessFactor":roughness,"metallicFactor":0.0}}));
                materials.len() - 1
            }));
            let weights = skin_tag.map(|tag| read_weights(scene, tag));
            if let Some(tag) = skin_tag {
                let joints = tag_joints(scene, tag);
                let joint_slots = joints
                    .iter()
                    .map(|j| {
                        slots.get(j).ok_or(Error::Unsupported(
                            "skin joints outside the exported selection",
                        ))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let bind: Vec<_> = read_bind_pose(scene, tag, joints.len())
                    .into_iter()
                    .map(|m| {
                        swap_matrix(m.inverse(), scale)
                            .to_cols_array()
                            .map(|v| v as f32)
                    })
                    .collect();
                if bind.is_empty() || bind.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(invalid("invalid bind pose"));
                }
                let accessor = buffer.floats(&bind, "MAT4", false);
                n["skin"] = json!(skins.len());
                skins.push(json!({"joints":joint_slots,"inverseBindMatrices":accessor}));
            }
            let uv = crate::scene::node_uvs(scene, id, &mesh);
            let p = primitive(
                &mut buffer,
                &mesh,
                uv.as_ref(),
                weights.as_ref(),
                mat,
                scale,
            )?;
            n["mesh"] = json!(meshes.len());
            meshes.push(json!({"primitives":[p]}));
        }
        nodes.push(n);
    }
    if meshes.is_empty() {
        return Err(invalid("no polygon meshes to export"));
    }
    let mut doc = json!({"asset":{"version":"2.0","generator":"Forge3D"},"scene":0,"scenes":[{"nodes":top}],
        "nodes":nodes,"meshes":meshes,"bufferViews":buffer.views,"accessors":buffer.accessors,
        "buffers":[{"byteLength":buffer.bytes.len()}]});
    if !skins.is_empty() {
        doc["skins"] = json!(skins);
    }
    if !materials.is_empty() {
        doc["materials"] = json!(materials);
    }
    if !anim.channels.is_empty() {
        doc["animations"] = json!([{"samplers":anim.samplers,"channels":anim.channels}]);
    }
    Ok((glb(&doc, buffer.bytes)?, meshes.len()))
}
