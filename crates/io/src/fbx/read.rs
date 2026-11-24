//! FBX reader: ufbx scene to the engine-neutral import scene (IO.md §8.1 to §8.6).

use std::collections::{BTreeMap, HashMap};
use std::f64::consts::{FRAC_PI_2, TAU};

use forge_core::transform::hpb_from_rotation;
use forge_core::{Interp, Key, Track};
use forge_geometry::{CornerUvs, PolyMesh};
use forge_material::node_material::SurfaceValues;
use forge_rig::weights::WeightMap;
use glam::{DMat3, DMat4, DQuat, DVec3};

use super::{Camera, Error, Kind, Light, MAX_BYTES, Material, MeshPart, Node, Skin};

/// Decoded file, applied to a document by [`ImportScene::apply`].
pub struct ImportScene {
    pub(super) roots: Vec<usize>,
    pub(super) nodes: Vec<Node>,
    pub(super) materials: Vec<Material>,
    warnings: Vec<String>,
}

impl ImportScene {
    /// Content of the file that was skipped (IO.md §8.6).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::Invalid(reason.into())
}

fn dvec(v: ufbx::Vec3) -> DVec3 {
    DVec3::new(v.x, v.y, v.z)
}

fn quat(q: ufbx::Quat) -> DQuat {
    DQuat::from_xyzw(q.x, q.y, q.z, q.w).normalize()
}

fn matrix(m: &ufbx::Matrix) -> DMat4 {
    DMat4::from_cols(
        DVec3::new(m.m00, m.m10, m.m20).extend(0.0),
        DVec3::new(m.m01, m.m11, m.m21).extend(0.0),
        DVec3::new(m.m02, m.m12, m.m22).extend(0.0),
        DVec3::new(m.m03, m.m13, m.m23).extend(1.0),
    )
}

/// The file axis `axis` as a unit vector.
fn axis_vector(axis: ufbx::CoordinateAxis) -> Option<DVec3> {
    use ufbx::CoordinateAxis as A;
    Some(match axis {
        A::PositiveX => DVec3::X,
        A::NegativeX => DVec3::NEG_X,
        A::PositiveY => DVec3::Y,
        A::NegativeY => DVec3::NEG_Y,
        A::PositiveZ => DVec3::Z,
        A::NegativeZ => DVec3::NEG_Z,
        A::Unknown => return None,
    })
}

/// The change of basis from the file's world to ours (IO.md §8.2): the file's axes are
/// turned into right-handed Y-up (`P`), reflected into our left-handed world (`S`) and the
/// file's unit becomes centimeters times `scale` (`k`). Every point is `k S P p`, every node
/// or bind matrix `M` becomes `C M C^-1`, so a rotation about a file axis becomes the rotation
/// about the image of that axis.
struct Basis {
    /// `C = k S P` as a matrix.
    to_ours: DMat4,
    /// `S P`: a signed permutation.
    linear: DMat3,
    /// The entries of `S P` in absolute value: maps scale components.
    permute: DMat3,
    /// Whether `C` mirrors (the polygon winding must be reversed).
    mirrored: bool,
}

impl Basis {
    fn of(scene: &ufbx::Scene, scale: f64, warnings: &mut Vec<String>) -> Basis {
        let axes = scene.settings.axes;
        let vectors = [axes.right, axes.up, axes.front].map(axis_vector);
        // P maps the file's right, up and front axes to +X, +Y and +Z
        let p = match vectors {
            [Some(r), Some(u), Some(f)]
                if r.dot(u) == 0.0 && u.dot(f) == 0.0 && r.dot(f) == 0.0 =>
            {
                DMat3::from_cols(r, u, f).transpose()
            }
            // no axes in the file: the FBX default, right-handed Y-up
            [None, None, None] => DMat3::IDENTITY,
            _ => {
                warnings.push("invalid coordinate axes: assuming right-handed Y-up".into());
                DMat3::IDENTITY
            }
        };
        let s = DMat3::from_diagonal(DVec3::new(1.0, 1.0, -1.0));
        let linear = s * p;
        let unit = scene.settings.unit_meters;
        let unit = if unit.is_finite() && unit > 0.0 {
            unit
        } else {
            warnings.push("unknown unit: assuming centimeters".into());
            0.01
        };
        let k = unit / 0.01 * scale;
        let to_ours = DMat4::from_scale(DVec3::splat(k)) * DMat4::from_mat3(linear);
        Basis {
            to_ours,
            linear,
            permute: DMat3::from_cols(
                linear.x_axis.abs(),
                linear.y_axis.abs(),
                linear.z_axis.abs(),
            ),
            mirrored: linear.determinant() < 0.0,
        }
    }

    fn point(&self, p: DVec3) -> DVec3 {
        self.to_ours.transform_point3(p)
    }

    /// `(h, p, b)` of the rotation `q` seen in our basis.
    fn rotation(&self, q: DQuat) -> DVec3 {
        let m = DMat3::from_quat(q.normalize());
        hpb_from_rotation(self.linear * m * self.linear.transpose())
    }

    fn scale(&self, s: DVec3) -> DVec3 {
        self.permute * s
    }

    fn matrix(&self, m: DMat4) -> DMat4 {
        self.to_ours * m * self.to_ours.inverse()
    }

    /// A length of the file (a clip distance) in our units.
    fn length(&self, v: f64) -> f64 {
        v * self.to_ours.x_axis.length()
    }
}

/// How a node's local frame differs from the one our object of that kind uses: the rotation
/// `c` with `ours = file * c` (file frame, before the change of basis).
fn frame_change(node: &ufbx::Node) -> DQuat {
    if node.camera.is_some() {
        // FBX cameras look along local +X with +Y up and +Z to the right; ours look along
        // local +Z (+X right) in the mirrored frame, which is the right-handed frame
        // looking along -Z with +X right: the change maps x -> z and z -> -x
        DQuat::from_rotation_y(-FRAC_PI_2)
    } else if let Some(light) = node.light.as_ref() {
        // lights shine along `local_direction`; ours along local +Z, the mirror of -Z
        let d = dvec(light.local_direction);
        if d.length_squared() > 0.0 {
            DQuat::from_rotation_arc(DVec3::NEG_Z, d.normalize())
        } else {
            DQuat::IDENTITY
        }
    } else {
        DQuat::IDENTITY
    }
}

fn load_error(e: &ufbx::Error) -> Error {
    let info = e.info();
    if info.is_empty() {
        invalid(e.description.to_string())
    } else {
        invalid(format!("{}: {info}", e.description))
    }
}

/// Reads binary or ASCII FBX. `scale` multiplies the centimetre values (IO.md §8.2).
pub fn read(bytes: &[u8], scale: f64) -> Result<ImportScene, Error> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(invalid("scale must be positive and finite"));
    }
    if bytes.len() > MAX_BYTES {
        return Err(invalid("file exceeds the size limit"));
    }
    // the file stays in its own axes and unit: the change of basis is ours (`Basis`)
    let opts = ufbx::LoadOpts {
        geometry_transform_handling: ufbx::GeometryTransformHandling::ModifyGeometry,
        pivot_handling: ufbx::PivotHandling::AdjustToPivot,
        inherit_mode_handling: ufbx::InheritModeHandling::Compensate,
        ..Default::default()
    };
    let root = ufbx::load_memory(bytes, opts).map_err(|e| load_error(&e))?;
    Convert::new(&root, scale).run()
}

struct Convert<'a> {
    scene: &'a ufbx::Scene,
    basis: Basis,
    warnings: Vec<String>,
    /// ufbx node typed id to index in `nodes`.
    index: HashMap<u32, usize>,
    /// ufbx material typed id to index in `materials`.
    material_index: HashMap<u32, usize>,
    materials: Vec<Material>,
    /// Nodes named as a bone by a skin cluster.
    bones: Vec<bool>,
    baked: HashMap<u32, BakedChannels>,
}

#[derive(Default)]
struct BakedChannels {
    pos: Option<Vec<(f64, DVec3)>>,
    rot: Option<Vec<(f64, DQuat)>>,
    scale: Option<Vec<(f64, DVec3)>>,
}

impl<'a> Convert<'a> {
    fn new(scene: &'a ufbx::Scene, scale: f64) -> Self {
        let mut warnings = Vec::new();
        let basis = Basis::of(scene, scale, &mut warnings);
        Self {
            scene,
            basis,
            warnings,
            index: HashMap::new(),
            material_index: HashMap::new(),
            materials: Vec::new(),
            bones: vec![false; scene.nodes.len()],
            baked: HashMap::new(),
        }
    }

    fn run(mut self) -> Result<ImportScene, Error> {
        self.collect_warnings();
        for cluster in self.scene.skin_clusters.iter() {
            if let Some(bone) = cluster.bone_node.as_ref() {
                self.bones[bone.element.typed_id as usize] = true;
            }
        }
        self.bake();
        // depth-first order: a parent is numbered before its children
        let mut order: Vec<&ufbx::Node> = Vec::new();
        let mut roots = Vec::new();
        let mut stack: Vec<(&ufbx::Node, Option<usize>)> = self
            .scene
            .root_node
            .children
            .iter()
            .rev()
            .map(|c| (&**c, None))
            .collect();
        let mut parents: Vec<Option<usize>> = Vec::new();
        while let Some((node, parent)) = stack.pop() {
            let at = order.len();
            self.index.insert(node.element.typed_id, at);
            order.push(node);
            parents.push(parent);
            if parent.is_none() {
                roots.push(at);
            }
            stack.extend(node.children.iter().rev().map(|c| (&**c, Some(at))));
        }
        let mut nodes: Vec<Node> = order
            .iter()
            .map(|n| self.node(n))
            .collect::<Result<_, _>>()?;
        for (at, parent) in parents.iter().enumerate() {
            if let Some(p) = parent {
                nodes[*p].children.push(at);
            }
        }
        Ok(ImportScene {
            roots,
            nodes,
            materials: self.materials,
            warnings: self.warnings,
        })
    }

    fn collect_warnings(&mut self) {
        let s = self.scene;
        let mut note = |count: usize, what: &str| {
            if count > 0 {
                self.warnings.push(format!("{count} {what} skipped"));
            }
        };
        note(s.nurbs_curves.len(), "NURBS curve(s)");
        note(s.nurbs_surfaces.len(), "NURBS surface(s)");
        note(s.line_curves.len(), "line curve(s)");
        note(
            s.procedural_geometries.len(),
            "procedural geometry object(s)",
        );
        note(s.blend_deformers.len(), "blend shape deformer(s) (morphs)");
        note(s.cache_deformers.len(), "geometry cache deformer(s)");
        note(s.constraints.len(), "constraint(s)");
        note(s.textures.len(), "texture(s) not read");
        note(
            s.anim_stacks.len().saturating_sub(1),
            "extra animation stack(s)",
        );
    }

    fn bake(&mut self) {
        let scene = self.scene;
        if scene.anim_curves.is_empty() {
            return;
        }
        let baked = match ufbx::bake_anim(scene, &scene.anim, ufbx::BakeOpts::default()) {
            Ok(b) => b,
            Err(e) => {
                self.warnings
                    .push(format!("animation not imported: {}", e.description));
                return;
            }
        };
        for node in baked.nodes.iter() {
            let channels = BakedChannels {
                pos: (!node.translation_keys.is_empty()).then(|| {
                    node.translation_keys
                        .iter()
                        .map(|k| (k.time, dvec(k.value)))
                        .collect()
                }),
                rot: (!node.rotation_keys.is_empty()).then(|| {
                    node.rotation_keys
                        .iter()
                        .map(|k| (k.time, quat(k.value)))
                        .collect()
                }),
                scale: (!node.scale_keys.is_empty()).then(|| {
                    node.scale_keys
                        .iter()
                        .map(|k| (k.time, dvec(k.value)))
                        .collect()
                }),
            };
            self.baked.insert(node.typed_id, channels);
        }
    }

    fn node(&mut self, n: &ufbx::Node) -> Result<Node, Error> {
        let change = frame_change(n);
        let local = n.local_transform;
        let b = &self.basis;
        let mut pos = b.point(dvec(local.translation));
        let mut rot = b.rotation(quat(local.rotation) * change);
        let mut scale = b.scale(dvec(local.scale));
        let mut tracks = BTreeMap::new();
        if let Some(baked) = self.baked.get(&n.element.typed_id) {
            if let Some(keys) = &baked.pos {
                let keys: Vec<_> = keys.iter().map(|(t, p)| (*t, b.point(*p))).collect();
                pos = channel(&mut tracks, "transform.pos", &keys, false);
            }
            if let Some(keys) = &baked.rot {
                let mut last = DVec3::ZERO;
                let keys: Vec<_> = keys
                    .iter()
                    .enumerate()
                    .map(|(i, (t, q))| {
                        let mut v = b.rotation(*q * change);
                        if i > 0 {
                            for a in 0..3 {
                                v[a] -= ((v[a] - last[a]) / TAU).round() * TAU;
                            }
                        }
                        last = v;
                        (*t, v)
                    })
                    .collect();
                rot = channel(&mut tracks, "transform.rot", &keys, true);
            }
            if let Some(keys) = &baked.scale {
                let keys: Vec<_> = keys.iter().map(|(t, s)| (*t, b.scale(*s))).collect();
                scale = channel(&mut tracks, "transform.scale", &keys, false);
            }
        }
        let kind = self.kind(n);
        Ok(Node {
            name: n.element.name.to_string(),
            kind,
            children: Vec::new(),
            pos,
            rot,
            scale,
            tracks,
        })
    }

    fn kind(&mut self, n: &ufbx::Node) -> Kind {
        if let Some(mesh) = n.mesh.as_ref() {
            if let Some(part) = self.mesh(n, mesh) {
                return Kind::Mesh(Box::new(part));
            }
            return Kind::Null;
        }
        if let Some(cam) = n.camera.as_ref() {
            return Kind::Camera(camera(cam, &self.basis));
        }
        if let Some(light) = n.light.as_ref() {
            return Kind::Light(light_of(light));
        }
        if n.bone.is_some() || self.bones[n.element.typed_id as usize] {
            return Kind::Joint;
        }
        Kind::Null
    }

    fn material(&mut self, m: &ufbx::Material) -> usize {
        if let Some(&i) = self.material_index.get(&m.element.typed_id) {
            return i;
        }
        let surface = surface_of(m);
        let graph = surface
            .to_graph()
            .map(|g| g.to_json())
            .unwrap_or_else(|_| String::new());
        let at = self.materials.len();
        self.materials.push(Material {
            name: m.element.name.to_string(),
            graph,
        });
        self.material_index.insert(m.element.typed_id, at);
        at
    }

    fn mesh(&mut self, n: &ufbx::Node, m: &ufbx::Mesh) -> Option<MeshPart> {
        let vertices: &[ufbx::Vec3] = &m.vertices;
        let points: Vec<DVec3> = vertices
            .iter()
            .map(|v| self.basis.point(dvec(*v)))
            .collect();
        let mut polys: Vec<Vec<u32>> = Vec::new();
        let mut kept_faces: Vec<usize> = Vec::new();
        let mut kept_corners: Vec<Vec<usize>> = Vec::new();
        let mut kept_order: Vec<Vec<usize>> = Vec::new();
        let corner_vertex: &[u32] = &m.vertex_indices;
        let faces: &[ufbx::Face] = &m.faces;
        for (fi, face) in faces.iter().enumerate() {
            let begin = face.index_begin as usize;
            let count = face.num_indices as usize;
            if count < 3 {
                continue;
            }
            // a mirroring change of basis flips the winding: reversed after the first corner
            let order: Vec<usize> = if self.basis.mirrored {
                std::iter::once(begin)
                    .chain((begin + 1..begin + count).rev())
                    .collect()
            } else {
                (begin..begin + count).collect()
            };
            let poly: Vec<u32> = order.iter().map(|&c| corner_vertex[c]).collect();
            let mut distinct = poly.clone();
            distinct.sort_unstable();
            distinct.dedup();
            if distinct.len() < 3 || poly.iter().any(|&v| v as usize >= points.len()) {
                continue;
            }
            polys.push(poly);
            kept_faces.push(fi);
            kept_corners.push((begin..begin + count).collect());
            kept_order.push(order);
        }
        if polys.is_empty() {
            return None;
        }
        let uv = m.vertex_uv.exists.then(|| CornerUvs {
            corners: kept_order
                .iter()
                .map(|corners| {
                    corners
                        .iter()
                        .map(|&c| {
                            let t = m.vertex_uv[c];
                            [t.x, t.y]
                        })
                        .collect()
                })
                .collect(),
        });
        let smooth = !(m.vertex_normal.exists && is_faceted(m, &kept_faces, &kept_corners));
        // materials: the node's list overrides the mesh's
        let list: &ufbx::RefList<ufbx::Material> = if n.materials.is_empty() {
            &m.materials
        } else {
            &n.materials
        };
        let slots: Vec<Option<usize>> = list.iter().map(|mat| Some(self.material(mat))).collect();
        let face_material: &[u32] = &m.face_material;
        let face_slot = kept_faces
            .iter()
            .map(|&f| face_material.get(f).copied().unwrap_or(0))
            .collect();
        let skin = self.skin(m, points.len());
        Some(MeshPart {
            mesh: PolyMesh { points, polys },
            uv,
            smooth,
            face_slot,
            slots,
            skin,
        })
    }

    fn skin(&mut self, m: &ufbx::Mesh, points: usize) -> Option<Skin> {
        let deformer = m.skin_deformers.first()?;
        if m.skin_deformers.len() > 1 {
            self.warnings.push(format!(
                "mesh with {} skin deformers: only the first is imported",
                m.skin_deformers.len()
            ));
        }
        let mut slot_of: Vec<Option<u32>> = Vec::new();
        let mut joints = Vec::new();
        let mut bind = Vec::new();
        for cluster in deformer.clusters.iter() {
            let slot = cluster
                .bone_node
                .as_ref()
                .and_then(|b| self.index.get(&b.element.typed_id).copied())
                .map(|joint| {
                    joints.push(joint);
                    let to_bone = matrix(&cluster.geometry_to_bone);
                    bind.push(self.basis.matrix(to_bone.inverse()));
                    joints.len() as u32 - 1
                });
            slot_of.push(slot);
        }
        if joints.is_empty() {
            return None;
        }
        let vertices: &[ufbx::SkinVertex] = &deformer.vertices;
        let all: &[ufbx::SkinWeight] = &deformer.weights;
        let mut weights = WeightMap::new();
        for (v, sv) in vertices.iter().enumerate().take(points) {
            let begin = sv.weight_begin as usize;
            let row: Vec<(u32, f64)> = all[begin..begin + sv.num_weights as usize]
                .iter()
                .filter_map(|w| Some((slot_of.get(w.cluster_index as usize).copied()??, w.weight)))
                .collect();
            weights.set(v as u32, row);
        }
        Some(Skin {
            joints,
            weights,
            bind,
        })
    }
}

/// The value of a transform channel at its first key, plus one track per axis that changes.
fn channel(
    tracks: &mut BTreeMap<String, Track>,
    param: &str,
    keys: &[(f64, DVec3)],
    quaternion: bool,
) -> DVec3 {
    let first = keys[0].1;
    for (axis, suffix) in ["x", "y", "z"].into_iter().enumerate() {
        if keys.iter().all(|(_, v)| v[axis] == first[axis]) {
            continue;
        }
        let mut track = Track::new(
            keys.iter()
                .map(|(t, v)| Key::new(*t, v[axis]).with_interp(Interp::Linear))
                .collect(),
        );
        track.quaternion = quaternion;
        if track.validate().is_ok() {
            tracks.insert(format!("{param}.{suffix}"), track);
        }
    }
    first
}

/// Whether every corner normal equals its polygon's normal.
fn is_faceted(m: &ufbx::Mesh, faces: &[usize], corners: &[Vec<usize>]) -> bool {
    let vertices: &[ufbx::Vec3] = &m.vertices;
    let corner_vertex: &[u32] = &m.vertex_indices;
    faces.iter().zip(corners).all(|(_, order)| {
        // Newell normal of the polygon in file space: outward for the file's counter-clockwise
        // polygons
        let p = |c: usize| dvec(vertices[corner_vertex[c] as usize]);
        let mut n = DVec3::ZERO;
        for i in 0..order.len() {
            let (a, b) = (p(order[i]), p(order[(i + 1) % order.len()]));
            n += DVec3::new(
                (a.y - b.y) * (a.z + b.z),
                (a.z - b.z) * (a.x + b.x),
                (a.x - b.x) * (a.y + b.y),
            );
        }
        let Some(face) = n.try_normalize() else {
            return true;
        };
        order.iter().all(|&c| {
            dvec(m.vertex_normal[c])
                .try_normalize()
                .is_none_or(|v| v.dot(face) > 0.99999)
        })
    })
}

fn camera(c: &ufbx::Camera, basis: &Basis) -> Camera {
    let sensor = c.aperture_size_inch.x * 25.4;
    let sensor = if sensor.is_finite() && sensor > 0.0 {
        sensor
    } else {
        36.0
    };
    let fov = if c.focal_length_mm.is_finite() && c.focal_length_mm > 0.0 {
        2.0 * (0.5 * sensor / c.focal_length_mm).atan()
    } else {
        c.field_of_view_deg.x.to_radians()
    };
    let focus = ufbx::find_prop(&c.element.props, "FocusDistance")
        .map(|p| basis.length(p.value_vec4.x))
        .filter(|v| v.is_finite() && *v > 0.0);
    let ortho_zoom = (c.projection_mode == ufbx::ProjectionMode::Orthographic
        && c.orthographic_size.x > 0.0)
        .then(|| 1000.0 / basis.length(c.orthographic_size.x));
    Camera {
        sensor,
        fov,
        near: basis.length(c.near_plane),
        far: basis.length(c.far_plane),
        focus,
        ortho_zoom,
    }
}

fn light_of(l: &ufbx::Light) -> Light {
    let kind = match l.type_ {
        ufbx::LightType::Spot => "spot",
        ufbx::LightType::Directional => "infinite",
        ufbx::LightType::Area => "area",
        _ => "omni",
    };
    Light {
        kind,
        color: [l.color.x, l.color.y, l.color.z].map(|c| c.max(0.0)),
        intensity: l.intensity,
        inner: l.inner_angle.to_radians(),
        outer: l.outer_angle.to_radians(),
    }
}

fn surface_of(m: &ufbx::Material) -> SurfaceValues {
    let mut surface = SurfaceValues::default();
    let rgb = |map: &ufbx::MaterialMap, factor: &ufbx::MaterialMap| -> Option<[f64; 3]> {
        map.has_value.then(|| {
            let f = if factor.has_value {
                factor.value_vec4.x
            } else {
                1.0
            };
            [
                map.value_vec4.x * f,
                map.value_vec4.y * f,
                map.value_vec4.z * f,
            ]
            .map(|c| c.max(0.0))
        })
    };
    if let Some(c) = rgb(&m.fbx.diffuse_color, &m.fbx.diffuse_factor) {
        surface.base_color = c;
    }
    if let Some(c) = rgb(&m.fbx.emission_color, &m.fbx.emission_factor) {
        surface.emission = c;
    }
    if m.fbx.transparency_factor.has_value {
        let alpha = (1.0 - m.fbx.transparency_factor.value_vec4.x).clamp(0.0, 1.0);
        if alpha < 1.0 {
            surface.alpha = alpha;
            surface.alpha_mode = "BLEND".into();
        }
    }
    match m.shader_type {
        ufbx::ShaderType::FbxLambert => surface.roughness = 1.0,
        ufbx::ShaderType::FbxPhong if m.fbx.specular_exponent.has_value => {
            let shininess = m.fbx.specular_exponent.value_vec4.x.max(0.0);
            surface.roughness = (2.0 / (shininess + 2.0)).sqrt();
        }
        _ => {
            if m.pbr.roughness.has_value {
                surface.roughness = m.pbr.roughness.value_vec4.x.clamp(0.0, 1.0);
            }
            if m.pbr.metalness.has_value {
                surface.metallic = m.pbr.metalness.value_vec4.x.clamp(0.0, 1.0);
            }
        }
    }
    surface
}
