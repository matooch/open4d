//! ASCII FBX 7.4 writer (IO.md §8.8, brief 166). Written from the public description of the
//! FBX file structure (the `Objects` / `Connections` sections, `Properties70` lists, the
//! `LayerElement` mapping modes, skin clusters and animation curve nodes); no SDK and no
//! template copied from an existing file.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::FRAC_PI_2;
use std::fmt::Write as _;

use forge_core::transform::{POS, ROT, SCALE, Transform, hpb_from_rotation};
use forge_core::{EvalCtx, Interp, NodeId, Scene, Value};
use forge_geometry::PolyMesh;
use forge_geometry::normals::{Weighting, corner_normals};
use forge_material::node_material::{NODE_MATERIAL, SurfaceValues, load_graph};
use forge_rig::weights::{WeightMap, find_weight_tag, read_bind_pose, read_weights, tag_joints};
use glam::{DMat3, DMat4, DVec3, EulerRot};

use super::Error;

/// FBX time ticks per second.
const KTIME: f64 = 46_186_158_000.0;
const FIRST_ID: i64 = 100_000;

fn invalid(reason: impl Into<String>) -> Error {
    Error::Invalid(reason.into())
}

/// Shortest text that reads back to the same `f64`; no exponent, no `-0`.
fn num(v: f64) -> String {
    if v == 0.0 { "0".into() } else { v.to_string() }
}

fn clean(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c == '"' {
                '\''
            } else if c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// Our left-handed world to the file's right-handed one: `S = diag(1, 1, -1)`.
fn mirror() -> DMat3 {
    DMat3::from_diagonal(DVec3::new(1.0, 1.0, -1.0))
}

struct Space {
    /// File units per centimetre.
    scale: f64,
}

impl Space {
    fn point(&self, p: DVec3) -> DVec3 {
        DVec3::new(p.x, p.y, -p.z) * self.scale
    }

    /// `D M D^-1` with `D = scale * S`.
    fn matrix(&self, m: DMat4) -> DMat4 {
        let d = DMat4::from_scale(DVec3::splat(self.scale)) * DMat4::from_mat3(mirror());
        d * m * d.inverse()
    }
}

/// The local rotation of an object as XYZ Euler degrees of the file: `R = Rz Ry Rx`.
fn euler_xyz(m: DMat3) -> DVec3 {
    let (z, y, x) = m.to_euler(EulerRot::ZYX);
    DVec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Null,
    Mesh,
    Camera,
    Light,
    Joint,
}

impl Kind {
    fn of(type_id: &str, has_mesh: bool) -> Kind {
        match type_id {
            "forge.camera" => Kind::Camera,
            "forge.light" => Kind::Light,
            t if t == forge_rig::JOINT => Kind::Joint,
            _ if has_mesh => Kind::Mesh,
            _ => Kind::Null,
        }
    }

    fn model_class(self) -> &'static str {
        match self {
            Kind::Null => "Null",
            Kind::Mesh => "Mesh",
            Kind::Camera => "Camera",
            Kind::Light => "Light",
            Kind::Joint => "LimbNode",
        }
    }
}

fn array(out: &mut String, tabs: usize, name: &str, values: &[String]) {
    let pad = "\t".repeat(tabs);
    let _ = writeln!(out, "{pad}{name}: *{} {{", values.len());
    let _ = write!(out, "{pad}\ta: ");
    for (i, chunk) in values.chunks(24).enumerate() {
        if i > 0 {
            let _ = write!(out, ",\n{pad}\t");
        }
        out.push_str(&chunk.join(","));
    }
    let _ = writeln!(out, "\n{pad}}}");
}

#[derive(Default)]
struct Out {
    next: i64,
    objects: Vec<(&'static str, String)>,
    connections: Vec<String>,
}

impl Out {
    fn id(&mut self) -> i64 {
        self.next += 1;
        FIRST_ID + self.next
    }

    fn object(&mut self, class: &'static str, text: String) {
        self.objects.push((class, text));
    }

    fn connect(&mut self, child: i64, parent: i64) {
        self.connections
            .push(format!("\tC: \"OO\",{child},{parent}"));
    }

    fn connect_prop(&mut self, child: i64, parent: i64, prop: &str) {
        self.connections
            .push(format!("\tC: \"OP\",{child},{parent},\"{prop}\""));
    }
}

/// One `P:` line of a `Properties70` block: name, type, label, flags, values.
fn property(name: &str, kind: &str, animatable: bool, values: &[String]) -> String {
    let flags = if animatable { "A" } else { "" };
    let label = match kind {
        "double" => "Number",
        "int" => "Integer",
        _ => "",
    };
    format!(
        "\t\t\tP: \"{name}\", \"{kind}\", \"{label}\", \"{flags}\",{}\n",
        values.join(",")
    )
}

struct Material {
    id: i64,
}

#[derive(Default)]
struct Materials {
    by_node: BTreeMap<NodeId, Material>,
    default: Option<Material>,
}

fn surface(scene: &Scene, id: NodeId) -> Result<(String, SurfaceValues), Error> {
    let node = scene.node(id).ok_or_else(|| invalid("missing material"))?;
    let s = if node.type_id == NODE_MATERIAL {
        let graph = load_graph(node).map_err(|e| invalid(e.to_string()))?;
        SurfaceValues::from_graph(&graph).map_err(|e| invalid(e.to_string()))?
    } else {
        let m = forge_material::Material::from_node(node);
        SurfaceValues {
            base_color: m.c("color.color"),
            metallic: f64::from(u8::from(m.e("reflectance.spec.attenuation") == "metal")),
            roughness: m.f("color.roughness"),
            emission: if m.enabled("luminance") {
                m.c("luminance.color")
            } else {
                [0.0; 3]
            },
            ..SurfaceValues::default()
        }
    };
    Ok((node.name.clone(), s))
}

fn material_text(id: i64, name: &str, s: &SurfaceValues) -> String {
    let unit = |v: f64| v.clamp(0.0, 1.0);
    let rgb = |c: [f64; 3]| c.map(|v| num(v.max(0.0))).to_vec();
    // Lambert for roughness 1, else Phong with the exponent that gives the roughness back:
    // roughness = sqrt(2 / (shininess + 2))
    let rough = s.roughness.clamp(1e-3, 1.0);
    let lambert = rough >= 1.0 - 1e-12;
    let mut t = format!(
        "\tMaterial: {id}, \"Material::{}\", \"\" {{\n\t\tVersion: 102\n\t\tShadingModel: \"{}\"\n\t\tMultiLayer: 0\n\t\tProperties70:  {{\n",
        clean(name),
        if lambert { "lambert" } else { "phong" }
    );
    t += &property("DiffuseColor", "Color", true, &rgb(s.base_color));
    t += &property("DiffuseFactor", "Number", true, &["1".into()]);
    t += &property("EmissiveColor", "Color", true, &rgb(s.emission));
    t += &property("EmissiveFactor", "Number", true, &["1".into()]);
    t += &property(
        "TransparencyFactor",
        "Number",
        true,
        &[num(1.0 - unit(s.alpha))],
    );
    if !lambert {
        t += &property(
            "Shininess",
            "Number",
            true,
            &[num(2.0 / (rough * rough) - 2.0)],
        );
    }
    t += "\t\t}\n\t}\n";
    t
}

/// The texture tags of an object as material slots: `(slots, slot of every polygon)`.
/// Tags with a selection name a polygon selection tag of the object; the last tag without a
/// selection covers the rest. Polygons nothing covers get `None`.
fn assignment(scene: &Scene, id: NodeId, polys: usize) -> (Vec<NodeId>, Vec<Option<usize>>) {
    let Some(node) = scene.node(id) else {
        return (Vec::new(), vec![None; polys]);
    };
    let mut slots: Vec<NodeId> = Vec::new();
    let slot_of = |m: NodeId, slots: &mut Vec<NodeId>| {
        slots.iter().position(|&s| s == m).unwrap_or_else(|| {
            slots.push(m);
            slots.len() - 1
        })
    };
    let mut default = None;
    let mut selected: Vec<(usize, Vec<u32>)> = Vec::new();
    for &tag in &node.tags {
        let Some(t) = scene.node(tag) else { continue };
        if t.type_id != forge_material::TEXTURE_TAG {
            continue;
        }
        let Some(Value::Link(Some(m))) = scene.param(tag, "material") else {
            continue;
        };
        if !scene
            .node(*m)
            .is_some_and(|m| matches!(m.type_id.as_str(), forge_material::MATERIAL | NODE_MATERIAL))
        {
            continue;
        }
        let slot = slot_of(*m, &mut slots);
        match t.params.get("selection") {
            Some(Value::String(name)) if !name.is_empty() => {
                let list = node
                    .tags
                    .iter()
                    .filter_map(|&s| scene.node(s))
                    .find(|s| s.type_id == forge_core::tags::POLYGON_SELECTION && s.name == *name)
                    .and_then(|s| match s.params.get("indices") {
                        Some(Value::IndexList(l)) => Some(l.clone()),
                        _ => None,
                    });
                if let Some(list) = list {
                    selected.push((slot, list));
                }
            }
            _ => default = Some(slot),
        }
    }
    let mut face_slot = vec![default; polys];
    for (slot, list) in selected {
        for p in list {
            if let Some(f) = face_slot.get_mut(p as usize) {
                *f = Some(slot);
            }
        }
    }
    (slots, face_slot)
}

/// The corner order the file uses: first corner, then the rest reversed (the reflection of
/// the world flips the winding).
fn file_order(n: usize) -> impl Iterator<Item = usize> {
    std::iter::once(0).chain((1..n).rev())
}

struct MeshInputs<'a> {
    mesh: &'a PolyMesh,
    uvs: Option<forge_geometry::CornerUvs>,
    smoothing: Option<f64>,
    face_slot: Option<Vec<usize>>,
    slots: usize,
}

fn geometry_text(id: i64, name: &str, space: &Space, m: &MeshInputs<'_>) -> String {
    let mesh = m.mesh;
    let mut t = format!(
        "\tGeometry: {id}, \"Geometry::{}\", \"Mesh\" {{\n\t\tProperties70:  {{\n\t\t}}\n\t\tGeometryVersion: 124\n",
        clean(name)
    );
    let points: Vec<String> = mesh
        .points
        .iter()
        .flat_map(|&p| space.point(p).to_array())
        .map(num)
        .collect();
    array(&mut t, 2, "Vertices", &points);
    let mut indices = Vec::new();
    for poly in &mesh.polys {
        let n = poly.len();
        for (k, c) in file_order(n).enumerate() {
            let v = i64::from(poly[c]);
            indices.push((if k + 1 == n { -v - 1 } else { v }).to_string());
        }
    }
    array(&mut t, 2, "PolygonVertexIndex", &indices);
    let corner_normals = corner_normals(mesh, m.smoothing.unwrap_or(0.0), Weighting::Angle);
    let normals: Vec<String> = mesh
        .polys
        .iter()
        .zip(&corner_normals)
        .flat_map(|(poly, face)| {
            file_order(poly.len()).flat_map(move |c| {
                let n = face[c];
                [n.x, n.y, -n.z]
            })
        })
        .map(num)
        .collect();
    t += "\t\tLayerElementNormal: 0 {\n\t\t\tVersion: 101\n\t\t\tName: \"\"\n\t\t\tMappingInformationType: \"ByPolygonVertex\"\n\t\t\tReferenceInformationType: \"Direct\"\n";
    array(&mut t, 3, "Normals", &normals);
    t += "\t\t}\n";
    if let Some(uvs) = &m.uvs {
        let mut table: BTreeMap<[u64; 2], usize> = BTreeMap::new();
        let mut values: Vec<String> = Vec::new();
        let mut index: Vec<String> = Vec::new();
        for (poly, face) in mesh.polys.iter().zip(&uvs.corners) {
            for c in file_order(poly.len()) {
                let uv = face[c];
                let key = uv.map(|v| (v + 0.0).to_bits());
                let at = *table.entry(key).or_insert_with(|| {
                    values.push(num(uv[0]));
                    values.push(num(uv[1]));
                    values.len() / 2 - 1
                });
                index.push(at.to_string());
            }
        }
        t += "\t\tLayerElementUV: 0 {\n\t\t\tVersion: 101\n\t\t\tName: \"UVMap\"\n\t\t\tMappingInformationType: \"ByPolygonVertex\"\n\t\t\tReferenceInformationType: \"IndexToDirect\"\n";
        array(&mut t, 3, "UV", &values);
        array(&mut t, 3, "UVIndex", &index);
        t += "\t\t}\n";
    }
    if let Some(face_slot) = &m.face_slot {
        let all_same = m.slots == 1 || face_slot.iter().all(|&s| s == face_slot[0]);
        t += "\t\tLayerElementMaterial: 0 {\n\t\t\tVersion: 101\n\t\t\tName: \"\"\n";
        if all_same {
            t += "\t\t\tMappingInformationType: \"AllSame\"\n\t\t\tReferenceInformationType: \"IndexToDirect\"\n";
            array(&mut t, 3, "Materials", &[face_slot[0].to_string()]);
        } else {
            t += "\t\t\tMappingInformationType: \"ByPolygon\"\n\t\t\tReferenceInformationType: \"IndexToDirect\"\n";
            let slots: Vec<String> = face_slot.iter().map(|s| s.to_string()).collect();
            array(&mut t, 3, "Materials", &slots);
        }
        t += "\t\t}\n";
    }
    t += "\t\tLayer: 0 {\n\t\t\tVersion: 100\n";
    let mut layers = vec!["LayerElementNormal"];
    if m.face_slot.is_some() {
        layers.push("LayerElementMaterial");
    }
    if m.uvs.is_some() {
        layers.push("LayerElementUV");
    }
    for l in layers {
        let _ = writeln!(
            t,
            "\t\t\tLayerElement:  {{\n\t\t\t\tType: \"{l}\"\n\t\t\t\tTypedIndex: 0\n\t\t\t}}"
        );
    }
    t += "\t\t}\n\t}\n";
    t
}

fn float_param(scene: &Scene, id: NodeId, name: &str, default: f64) -> f64 {
    match scene.param(id, name) {
        Some(Value::Float(f)) => *f,
        _ => default,
    }
}

fn flag_param(scene: &Scene, id: NodeId, name: &str, default: bool) -> bool {
    match scene.param(id, name) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

fn camera_text(scene: &Scene, id: NodeId, attr: i64, name: &str, space: &Space) -> String {
    let sensor = float_param(scene, id, "sensor_size", 36.0);
    let fov = float_param(scene, id, "fov_horizontal", 2.0 * 0.5f64.atan());
    let focal = sensor * 0.5 / (fov * 0.5).tan();
    let film_width = sensor / 25.4;
    let ortho = !matches!(
        scene.param(id, "projection"),
        Some(Value::Enum(p)) if p == "perspective"
    ) && scene.param(id, "projection").is_some();
    let mut t = format!(
        "\tNodeAttribute: {attr}, \"NodeAttribute::{}\", \"Camera\" {{\n\t\tProperties70:  {{\n",
        clean(name)
    );
    t += &property("ApertureMode", "enum", false, &["3".into()]);
    t += &property("FocalLength", "Number", true, &[num(focal)]);
    t += &property("FilmWidth", "double", false, &[num(film_width)]);
    t += &property(
        "FilmHeight",
        "double",
        false,
        &[num(film_width * 9.0 / 16.0)],
    );
    t += &property(
        "FocusDistance",
        "double",
        false,
        &[num(
            float_param(scene, id, "focus_distance", 2000.0) * space.scale
        )],
    );
    let near = if flag_param(scene, id, "enable_near_clipping", true) {
        float_param(scene, id, "near_clipping", 0.0)
    } else {
        0.0
    };
    let far = if flag_param(scene, id, "enable_far_clipping", false) {
        float_param(scene, id, "far_clipping", 10_000.0)
    } else {
        10_000.0
    };
    t += &property("NearPlane", "double", false, &[num(near * space.scale)]);
    t += &property("FarPlane", "double", false, &[num(far * space.scale)]);
    if ortho {
        // the width at zoom 1 is 1000 cm; ufbx's orthographic extent is OrthoZoom * 30
        let zoom = float_param(scene, id, "zoom", 1.0);
        t += &property("CameraProjectionType", "enum", false, &["1".into()]);
        t += &property(
            "OrthoZoom",
            "double",
            false,
            &[num(1000.0 / zoom * space.scale / 30.0)],
        );
    }
    t += "\t\t}\n\t\tTypeFlags: \"Camera\"\n\t\tGeometryVersion: 124\n\t}\n";
    t
}

fn light_text(scene: &Scene, id: NodeId, attr: i64, name: &str) -> String {
    let kind = match scene.param(id, "type") {
        Some(Value::Enum(k)) => k.as_str(),
        _ => "omni",
    };
    // 0 point, 1 directional, 2 spot, 3 area
    let light_type = match kind {
        "spot" | "square_spot" | "parallel_spot" | "square_parallel_spot" => 2,
        "infinite" | "parallel" => 1,
        "area" => 3,
        _ => 0,
    };
    let color = match scene.param(id, "color") {
        Some(Value::Color(c)) => *c,
        _ => [1.0; 3],
    };
    let mut t = format!(
        "\tNodeAttribute: {attr}, \"NodeAttribute::{}\", \"Light\" {{\n\t\tProperties70:  {{\n",
        clean(name)
    );
    t += &property("LightType", "enum", false, &[light_type.to_string()]);
    t += &property("Color", "Color", true, &color.map(num));
    t += &property(
        "Intensity",
        "Number",
        true,
        &[num(float_param(scene, id, "intensity", 1.0) * 100.0)],
    );
    if light_type == 2 {
        t += &property(
            "InnerAngle",
            "Number",
            true,
            &[num(float_param(scene, id, "inner_angle", 0.0).to_degrees())],
        );
        t += &property(
            "OuterAngle",
            "Number",
            true,
            &[num(float_param(
                scene,
                id,
                "outer_angle",
                30f64.to_radians(),
            )
            .to_degrees())],
        );
    }
    t += "\t\t}\n\t\tTypeFlags: \"Light\"\n\t\tGeometryVersion: 124\n\t}\n";
    t
}

/// The keys of one transform parameter of an object: sample times and the values the
/// object's three tracks give there (the glTF writer's rule: key times, plus every frame
/// where an interpolation is not linear).
struct Channel {
    times: Vec<f64>,
    values: Vec<DVec3>,
    step: bool,
}

fn channel(scene: &Scene, id: NodeId, param: &str, base: DVec3) -> Result<Option<Channel>, Error> {
    let node = scene.node(id).expect("known node");
    let tracks = ["x", "y", "z"].map(|axis| node.tracks.get(&format!("{param}.{axis}")));
    let mut times: Vec<f64> = tracks
        .iter()
        .flatten()
        .flat_map(|t| t.keys.iter().map(|k| k.time))
        .collect();
    if times.is_empty() {
        return Ok(None);
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
        return Err(invalid("mixed step and continuous interpolation"));
    }
    let quaternion = tracks.iter().flatten().any(|t| t.quaternion);
    // FBX interpolates rotation as Euler angles, never as orientations: a rotation is
    // sampled on every frame, as are spline-interpolated values
    if modes.contains(&Interp::Spline) || (param == ROT && !step) {
        let fps = f64::from(scene.settings().fps);
        let first = (times[0] * fps).ceil() as i64;
        let last = (times[times.len() - 1] * fps).floor() as i64;
        if last.saturating_sub(first) > 1_000_000 {
            return Err(invalid("animation exceeds one million baked frames"));
        }
        times.extend((first..=last).map(|f| f as f64 / fps));
        times.sort_by(f64::total_cmp);
        times.dedup();
    }
    let values = times
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
    Ok(Some(Channel {
        times,
        values,
        step,
    }))
}

fn ktime(t: f64) -> String {
    ((t * KTIME).round() as i64).to_string()
}

/// One animation curve node of `model` with a curve per axis.
#[allow(clippy::too_many_arguments)]
fn curve_node(
    out: &mut Out,
    layer: i64,
    model: i64,
    name: &str,
    prop: &str,
    default: DVec3,
    ch: &Channel,
    values: &[DVec3],
) {
    let node = out.id();
    let mut t = format!(
        "\tAnimationCurveNode: {node}, \"AnimCurveNode::{name}\", \"\" {{\n\t\tProperties70:  {{\n"
    );
    for (axis, label) in ["d|X", "d|Y", "d|Z"].into_iter().enumerate() {
        t += &property(label, "Number", true, &[num(default[axis])]);
    }
    t += "\t\t}\n\t}\n";
    out.object("AnimationCurveNode", t);
    out.connect(node, layer);
    out.connect_prop(node, model, prop);
    let flags = if ch.step { 2 } else { 4 };
    for (axis, label) in ["d|X", "d|Y", "d|Z"].into_iter().enumerate() {
        let curve = out.id();
        let n = ch.times.len();
        let mut t = format!(
            "\tAnimationCurve: {curve}, \"AnimCurve::\", \"\" {{\n\t\tDefault: {}\n\t\tKeyVer: 4009\n",
            num(default[axis])
        );
        let times: Vec<String> = ch.times.iter().map(|&s| ktime(s)).collect();
        array(&mut t, 2, "KeyTime", &times);
        let v: Vec<String> = values.iter().map(|v| num(v[axis])).collect();
        array(&mut t, 2, "KeyValueFloat", &v);
        array(&mut t, 2, "KeyAttrFlags", &vec![flags.to_string(); n]);
        array(&mut t, 2, "KeyAttrDataFloat", &vec!["0".to_string(); 4 * n]);
        array(&mut t, 2, "KeyAttrRefCount", &vec!["1".to_string(); n]);
        t += "\t}\n";
        out.object("AnimationCurve", t);
        out.connect_prop(curve, node, label);
    }
}

/// The extra rotation of the local frame of a camera or light (the inverse of the import's
/// frame change): file = ours * `frame_change(kind)`, in the file's right-handed frame.
fn frame_change(kind: Kind) -> DMat3 {
    match kind {
        // our cameras look along +Z, FBX cameras along local +X (IO.md §8.2)
        Kind::Camera => DMat3::from_rotation_y(FRAC_PI_2),
        // our lights shine along +Z, FBX lights along local -Y
        Kind::Light => DMat3::from_rotation_x(FRAC_PI_2),
        _ => DMat3::IDENTITY,
    }
}

/// Our `(h, p, b)` as the file's XYZ Euler degrees for an object of `kind`.
fn file_euler(rot: DVec3, kind: Kind) -> DVec3 {
    let m = Transform {
        rot,
        ..Transform::IDENTITY
    }
    .rotation();
    euler_xyz(mirror() * m * mirror() * frame_change(kind))
}

/// `Euler` samples made continuous: each is the equivalent angle triple nearest the previous.
fn unwrap_degrees(samples: &mut [DVec3]) {
    for i in 1..samples.len() {
        let last = samples[i - 1];
        for a in 0..3 {
            samples[i][a] -= ((samples[i][a] - last[a]) / 360.0).round() * 360.0;
        }
    }
}

fn model_text(id: i64, name: &str, kind: Kind, pos: DVec3, euler: DVec3, scale: DVec3) -> String {
    let mut t = format!(
        "\tModel: {id}, \"Model::{}\", \"{}\" {{\n\t\tVersion: 232\n\t\tProperties70:  {{\n",
        clean(name),
        kind.model_class()
    );
    t += &property("RotationOrder", "enum", false, &["0".into()]);
    t += &property(
        "Lcl Translation",
        "Lcl Translation",
        true,
        &pos.to_array().map(num),
    );
    t += &property(
        "Lcl Rotation",
        "Lcl Rotation",
        true,
        &euler.to_array().map(num),
    );
    t += &property(
        "Lcl Scaling",
        "Lcl Scaling",
        true,
        &scale.to_array().map(num),
    );
    t += "\t\t}\n\t\tShading: T\n\t\tCulling: \"CullingOff\"\n\t}\n";
    t
}

fn matrix_values(m: DMat4) -> Vec<String> {
    m.to_cols_array().map(num).to_vec()
}

/// Writes ASCII FBX 7.4: the scene (or the selected roots with their descendants) as it is
/// evaluated, with its animation tracks. `scale` is the number of file units per centimetre
/// (declared as the unit of the file); returns the file and the number of objects.
pub fn write(
    scene: &Scene,
    roots: Option<&[NodeId]>,
    scale: f64,
) -> Result<(Vec<u8>, usize), Error> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(invalid("scale must be positive and finite"));
    }
    let space = Space { scale };
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
    if order.is_empty() {
        return Err(Error::Empty);
    }
    let mut out = Out::default();
    let infos: BTreeMap<NodeId, i64> = order.iter().map(|&id| (id, out.id())).collect();
    let layer = out.id();
    let stack = out.id();
    let mut materials = Materials::default();
    let mut animated = false;
    let (mut t_min, mut t_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for &id in &order {
        let node = scene.node(id).expect("known node");
        let model = infos[&id];
        let mut t = scene.local_transform(id).expect("object transform");
        let parent = node.parent.and_then(|p| infos.get(&p).copied());
        if parent.is_none() && node.parent.is_some() {
            if !node.tracks.is_empty() {
                return Err(invalid("animated export root with omitted parent"));
            }
            let world = scene.world_matrix(id).expect("known parents");
            let (scale, rotation, pos) = world.to_scale_rotation_translation();
            t = Transform {
                pos,
                rot: hpb_from_rotation(DMat3::from_quat(rotation)),
                scale,
            };
        }
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
                _ => return Err(invalid("skin without polygon bind mesh")),
            }
        } else {
            forge_generators::evaluate(scene, id, &EvalCtx::default())
                .ok()
                .flatten()
        }
        .filter(|m| !m.polys.is_empty());
        let kind = Kind::of(&node.type_id, mesh.is_some());
        out.object(
            "Model",
            model_text(
                model,
                &node.name,
                kind,
                space.point(t.pos),
                file_euler(t.rot, kind),
                t.scale,
            ),
        );
        out.connect(model, parent.unwrap_or(0));
        match kind {
            Kind::Camera => {
                let attr = out.id();
                out.object(
                    "NodeAttribute",
                    camera_text(scene, id, attr, &node.name, &space),
                );
                out.connect(attr, model);
            }
            Kind::Light => {
                let attr = out.id();
                out.object("NodeAttribute", light_text(scene, id, attr, &node.name));
                out.connect(attr, model);
            }
            Kind::Joint => {
                let attr = out.id();
                out.object(
                    "NodeAttribute",
                    format!(
                        "\tNodeAttribute: {attr}, \"NodeAttribute::{}\", \"LimbNode\" {{\n\t\tProperties70:  {{\n{}\t\t}}\n\t\tTypeFlags: \"Skeleton\"\n\t}}\n",
                        clean(&node.name),
                        property("Size", "double", false, &[num(100.0 * space.scale)])
                    ),
                );
                out.connect(attr, model);
            }
            _ => {}
        }
        if let Some(mesh) = &mesh {
            let geometry = out.id();
            let (slot_nodes, face_slot) = assignment(scene, id, mesh.polys.len());
            let mut slot_ids = Vec::new();
            for &m in &slot_nodes {
                if let Some(existing) = materials.by_node.get(&m) {
                    slot_ids.push(existing.id);
                    continue;
                }
                let (name, s) = surface(scene, m)?;
                let mid = out.id();
                out.object("Material", material_text(mid, &name, &s));
                materials.by_node.insert(m, Material { id: mid });
                slot_ids.push(mid);
            }
            let covered = face_slot.iter().any(Option::is_some);
            let face_slot = if covered {
                let mut slots = face_slot;
                if slots.iter().any(Option::is_none) {
                    // polygons no tag covers use the engine's default surface
                    let default = materials.default.get_or_insert_with(|| {
                        let mid = out.id();
                        out.object(
                            "Material",
                            material_text(mid, "Default", &SurfaceValues::default()),
                        );
                        Material { id: mid }
                    });
                    slot_ids.push(default.id);
                    let at = slot_ids.len() - 1;
                    for s in slots.iter_mut().filter(|s| s.is_none()) {
                        *s = Some(at);
                    }
                }
                Some(slots.into_iter().flatten().collect::<Vec<_>>())
            } else {
                None
            };
            let uvs = crate::scene::node_uvs(scene, id, mesh);
            out.object(
                "Geometry",
                geometry_text(
                    geometry,
                    &node.name,
                    &space,
                    &MeshInputs {
                        mesh,
                        uvs,
                        smoothing: forge_core::smoothing::smoothing_angle(scene, id),
                        face_slot,
                        slots: slot_ids.len(),
                    },
                ),
            );
            out.connect(geometry, model);
            for mid in slot_ids {
                out.connect(mid, model);
            }
            if let Some(tag) = skin_tag {
                skin_objects(&mut out, scene, id, tag, geometry, &infos, &space)?;
            }
        }
        // animation
        for (param, label, base, name) in [
            (POS, "Lcl Translation", t.pos, "T"),
            (ROT, "Lcl Rotation", t.rot, "R"),
            (SCALE, "Lcl Scaling", t.scale, "S"),
        ] {
            let Some(ch) = channel(scene, id, param, base)? else {
                continue;
            };
            animated = true;
            t_min = t_min.min(ch.times[0]);
            t_max = t_max.max(ch.times[ch.times.len() - 1]);
            let (default, values) = match param {
                POS => (
                    space.point(base),
                    ch.values
                        .iter()
                        .map(|&v| space.point(v))
                        .collect::<Vec<_>>(),
                ),
                ROT => {
                    let mut v: Vec<_> = ch.values.iter().map(|&v| file_euler(v, kind)).collect();
                    unwrap_degrees(&mut v);
                    (file_euler(base, kind), v)
                }
                _ => (base, ch.values.clone()),
            };
            curve_node(&mut out, layer, model, name, label, default, &ch, &values);
        }
    }
    let mut text = header(scene, &space);
    // Definitions
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (class, _) in &out.objects {
        *counts.entry(class).or_default() += 1;
    }
    if animated {
        counts.insert("AnimationLayer", 1);
        counts.insert("AnimationStack", 1);
    }
    let _ = writeln!(
        text,
        "Definitions:  {{\n\tVersion: 100\n\tCount: {}",
        counts.values().sum::<usize>() + 1
    );
    text += "\tObjectType: \"GlobalSettings\" {\n\t\tCount: 1\n\t}\n";
    for (class, count) in &counts {
        let _ = writeln!(
            text,
            "\tObjectType: \"{class}\" {{\n\t\tCount: {count}\n\t}}"
        );
    }
    text += "}\n\nObjects:  {\n";
    for (_, body) in &out.objects {
        text += body;
    }
    if animated {
        text += &format!(
            "\tAnimationStack: {stack}, \"AnimStack::Take 001\", \"\" {{\n\t\tProperties70:  {{\n{}{}\t\t}}\n\t}}\n\tAnimationLayer: {layer}, \"AnimLayer::BaseLayer\", \"\" {{\n\t}}\n",
            property("LocalStart", "KTime", false, &[ktime(t_min.min(0.0))]),
            property("LocalStop", "KTime", false, &[ktime(t_max)]),
        );
        out.connect(layer, stack);
    }
    text += "}\n\nConnections:  {\n";
    for c in &out.connections {
        text += c;
        text += "\n";
    }
    text += "}\n\nTakes:  {\n\tCurrent: \"Take 001\"\n}\n";
    Ok((text.into_bytes(), order.len()))
}

fn header(scene: &Scene, space: &Space) -> String {
    let fps = f64::from(scene.settings().fps);
    let (mode, custom) = match fps as i64 {
        120 => (1, None),
        100 => (2, None),
        60 => (3, None),
        50 => (4, None),
        48 => (5, None),
        30 => (6, None),
        24 => (11, None),
        _ => (14, Some(fps)),
    };
    let mut t = String::from(
        "; FBX 7.4.0 project file\n; Written by Forge3D\n; ------------------------------------------------\n\n",
    );
    t += "FBXHeaderExtension:  {\n\tFBXHeaderVersion: 1003\n\tFBXVersion: 7400\n\tCreator: \"Forge3D\"\n}\n\n";
    t += "GlobalSettings:  {\n\tVersion: 1000\n\tProperties70:  {\n";
    for (name, v) in [
        ("UpAxis", 1),
        ("UpAxisSign", 1),
        ("FrontAxis", 2),
        ("FrontAxisSign", 1),
        ("CoordAxis", 0),
        ("CoordAxisSign", 1),
    ] {
        t += &property(name, "int", false, &[v.to_string()]).replacen("\t\t\t", "\t\t", 1);
    }
    // centimetres per file unit
    t += &property(
        "UnitScaleFactor",
        "double",
        false,
        &[num(1.0 / space.scale)],
    )
    .replacen("\t\t\t", "\t\t", 1);
    t += &property("TimeMode", "enum", false, &[mode.to_string()]).replacen("\t\t\t", "\t\t", 1);
    if let Some(rate) = custom {
        t += &property("CustomFrameRate", "double", false, &[num(rate)])
            .replacen("\t\t\t", "\t\t", 1);
    }
    t += "\t}\n}\n\n";
    t
}

fn skin_objects(
    out: &mut Out,
    scene: &Scene,
    mesh: NodeId,
    tag: NodeId,
    geometry: i64,
    models: &BTreeMap<NodeId, i64>,
    space: &Space,
) -> Result<(), Error> {
    let joints = tag_joints(scene, tag);
    let node = scene.node(mesh).expect("known node");
    let normalise = node
        .children
        .iter()
        .filter_map(|&c| scene.node(c))
        .find(|c| c.type_id == forge_rig::SKIN)
        .is_none_or(|c| !matches!(c.params.get("normalise"), Some(Value::Bool(false))));
    let mut weights: WeightMap = read_weights(scene, tag);
    if normalise {
        weights.normalise();
    }
    let bind = read_bind_pose(scene, tag, joints.len());
    let world = scene.world_matrix(mesh).expect("known node");
    let skin = out.id();
    out.object(
        "Deformer",
        format!(
            "\tDeformer: {skin}, \"Deformer::\", \"Skin\" {{\n\t\tVersion: 101\n\t\tLink_DeformAcuracy: 50\n\t\tSkinningType: \"Linear\"\n\t}}\n"
        ),
    );
    out.connect(skin, geometry);
    for (slot, (&joint, bind)) in joints.iter().zip(&bind).enumerate() {
        let bone = *models
            .get(&joint)
            .ok_or_else(|| invalid("skin joints outside the exported selection"))?;
        let rows: Vec<(u32, f64)> = weights
            .points()
            .filter_map(|p| {
                weights
                    .get(p)
                    .iter()
                    .find(|(s, _)| *s as usize == slot)
                    .map(|(_, w)| (p, *w))
            })
            .collect();
        let cluster = out.id();
        let mut t = format!(
            "\tDeformer: {cluster}, \"SubDeformer::\", \"Cluster\" {{\n\t\tVersion: 100\n\t\tUserData: \"\", \"\"\n"
        );
        array(
            &mut t,
            2,
            "Indexes",
            &rows.iter().map(|(p, _)| p.to_string()).collect::<Vec<_>>(),
        );
        array(
            &mut t,
            2,
            "Weights",
            &rows.iter().map(|(_, w)| num(*w)).collect::<Vec<_>>(),
        );
        // `Transform` is the mesh node's matrix relative to the joint at bind time (the inverse
        // of our bind matrix), `TransformLink` the joint's world matrix at bind time (the
        // mesh's current world matrix stands for the mesh at bind time)
        array(
            &mut t,
            2,
            "Transform",
            &matrix_values(space.matrix(bind.inverse())),
        );
        array(
            &mut t,
            2,
            "TransformLink",
            &matrix_values(space.matrix(world * *bind)),
        );
        t += "\t}\n";
        out.object("Deformer", t);
        out.connect(cluster, skin);
        out.connect(bone, cluster);
    }
    Ok(())
}
