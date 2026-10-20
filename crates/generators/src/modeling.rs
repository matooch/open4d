//! Symmetry, array, connect and instance (docs/specs/GENERATORS2.md).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::{BuildError, Params, evaluate};

pub const SYMMETRY: &str = "forge.symmetry";
pub const ARRAY: &str = "forge.array";
pub const CONNECT: &str = "forge.connect";
pub const INSTANCE: &str = "forge.instance";

fn distance(id: &str, label: &str, default: f64) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(0.0, f64::MAX)
        .unit(Unit::Distance)
}

pub(crate) fn symmetry_type() -> NodeType {
    NodeType::new(SYMMETRY, "Symmetry", Category::Generator)
        .param(ParamDesc::enumeration(
            "mirror_plane",
            "Mirror plane",
            "zy",
            &[("xy", "XY"), ("zy", "ZY"), ("xz", "XZ")],
        ))
        .param(ParamDesc::bool("weld_points", "Weld points", true))
        .param(distance("tolerance", "Tolerance", 0.01))
        .param(ParamDesc::bool("symmetrical", "Symmetrical", true))
}

pub(crate) fn array_type() -> NodeType {
    NodeType::new(ARRAY, "Array", Category::Generator)
        .param(distance("radius", "Radius", 250.0))
        .param(ParamDesc::int("copies", "Copies", 7).range(0, 100_000))
        .param(ParamDesc::float("amplitude", "Amplitude", 0.0).unit(Unit::Distance))
        .param(ParamDesc::float("frequency", "Frequency", 0.0))
        .param(ParamDesc::float("array_frequency", "Array frequency", 10.0))
        .param(ParamDesc::bool("render_instance", "Render instance", false))
}

pub(crate) fn connect_type() -> NodeType {
    NodeType::new(CONNECT, "Connect", Category::Generator)
        .param(ParamDesc::bool("weld", "Weld", true))
        .param(distance("tolerance", "Tolerance", 0.1))
        .param(ParamDesc::enumeration(
            "phong_mode",
            "Phong mode",
            "as_breaks",
            &[
                ("manual", "Manual"),
                ("average", "Average"),
                ("lowest", "Lowest"),
                ("highest", "Highest"),
                ("as_breaks", "As breaks"),
            ],
        ))
        .param(ParamDesc::bool("textures", "Textures", true))
        .param(ParamDesc::bool("center_axis", "Center axis", false))
}

pub(crate) fn instance_type() -> NodeType {
    NodeType::new(INSTANCE, "Instance", Category::Generator)
        .param(ParamDesc::link("reference", "Reference object", &[]))
        .param(ParamDesc::enumeration(
            "instance_mode",
            "Instance mode",
            "instance",
            &[
                ("instance", "Instance"),
                ("render_instance", "Render instance"),
                ("multi_instance", "Multi-instance"),
            ],
        ))
}

/// Number of children a generator consumes, if it is one of these (§0).
pub(crate) fn input_count(type_id: &str) -> Option<usize> {
    match type_id {
        SYMMETRY | ARRAY => Some(1),
        CONNECT => Some(usize::MAX),
        _ => None,
    }
}

/// The mesh children a generator consumes, in child order (§0).
pub fn inputs_of(scene: &Scene, id: NodeId) -> Vec<NodeId> {
    let Some(node) = scene.node(id) else {
        return Vec::new();
    };
    let take = input_count(&node.type_id).unwrap_or(0);
    node.children
        .iter()
        .copied()
        .filter(|c| matches!(evaluate(scene, *c), Ok(Some(_)) | Err(_)))
        .take(take)
        .collect()
}

/// An input's mesh placed by its local transform.
fn placed(scene: &Scene, child: NodeId) -> Result<Option<PolyMesh>, BuildError> {
    let Some(mut mesh) = evaluate(scene, child)? else {
        return Ok(None);
    };
    let m = scene.local_transform(child).expect("child exists").matrix();
    for p in &mut mesh.points {
        *p = m.transform_point3(*p);
    }
    Ok(Some(mesh))
}

pub(crate) fn evaluate_modeling(
    scene: &Scene,
    id: NodeId,
    type_id: &str,
    values: &BTreeMap<String, Value>,
) -> Result<Option<PolyMesh>, BuildError> {
    if type_id == INSTANCE {
        let ty = instance_type();
        let p = Params { ty: &ty, values };
        // Follow instance chains to a non-instance; a cycle has no geometry.
        let mut seen = vec![id];
        let mut target = p.link("reference");
        while let Some(r) = target {
            let Some(node) = scene.node(r) else {
                return Ok(None);
            };
            if node.type_id != INSTANCE {
                return evaluate(scene, r);
            }
            if seen.contains(&r) {
                return Ok(None);
            }
            seen.push(r);
            target = match node.params.get("reference") {
                Some(Value::Link(l)) => *l,
                _ => None,
            };
        }
        return Ok(None);
    }
    let mut inputs = Vec::new();
    for c in inputs_of(scene, id) {
        if let Some(m) = placed(scene, c)? {
            inputs.push(m);
        }
    }
    if inputs.is_empty() {
        return Ok(None);
    }
    match type_id {
        SYMMETRY => symmetry(&inputs[0], values).map(Some),
        ARRAY => array(&inputs[0], values).map(Some),
        _ => connect(&inputs, values).map(Some),
    }
}

/// Symmetry of one mesh (§1).
pub fn symmetry(mesh: &PolyMesh, values: &BTreeMap<String, Value>) -> Result<PolyMesh, BuildError> {
    let ty = symmetry_type();
    let p = Params { ty: &ty, values };
    let axis = match p.enumeration("mirror_plane").as_str() {
        "xy" => 2,
        "xz" => 1,
        _ => 0,
    };
    let tol = if p.bool("weld_points") {
        p.float("tolerance")
    } else {
        -1.0
    };
    let mut points = mesh.points.clone();
    let mirror: Vec<u32> = mesh
        .points
        .iter()
        .enumerate()
        .map(|(i, q)| {
            if q[axis].abs() <= tol {
                i as u32
            } else {
                let mut m = *q;
                m[axis] = -m[axis];
                points.push(m);
                (points.len() - 1) as u32
            }
        })
        .collect();
    let mut polys = mesh.polys.clone();
    polys.extend(mesh.polys.iter().map(|f| {
        std::iter::once(f[0])
            .chain(f[1..].iter().rev().copied())
            .map(|v| mirror[v as usize])
            .collect()
    }));
    Ok(PolyMesh { points, polys })
}

/// Array of one mesh (§2).
pub fn array(mesh: &PolyMesh, values: &BTreeMap<String, Value>) -> Result<PolyMesh, BuildError> {
    let ty = array_type();
    let p = Params { ty: &ty, values };
    let n = p.int("copies") + 1;
    let (radius, amplitude, af) = (
        p.float("radius"),
        p.float("amplitude"),
        p.float("array_frequency"),
    );
    let mut out = PolyMesh::default();
    for k in 0..n {
        let theta = std::f64::consts::TAU * k as f64 / n as f64;
        let (s, c) = theta.sin_cos();
        let ry = |q: DVec3| DVec3::new(q.x * c + q.z * s, q.y, -q.x * s + q.z * c);
        let lift = amplitude * (std::f64::consts::TAU * af * k as f64 / n as f64).sin();
        let offset = ry(DVec3::new(0.0, 0.0, radius)) + DVec3::new(0.0, lift, 0.0);
        let base = out.points.len() as u32;
        out.points
            .extend(mesh.points.iter().map(|&q| ry(q) + offset));
        out.polys.extend(
            mesh.polys
                .iter()
                .map(|f| f.iter().map(|&v| v + base).collect()),
        );
    }
    Ok(out)
}

/// Connect of several meshes (§3).
pub fn connect(
    meshes: &[PolyMesh],
    values: &BTreeMap<String, Value>,
) -> Result<PolyMesh, BuildError> {
    let ty = connect_type();
    let p = Params { ty: &ty, values };
    if p.bool("center_axis") {
        return Err(BuildError::Unsupported("connect center axis"));
    }
    let mut out = PolyMesh::default();
    for m in meshes {
        let base = out.points.len() as u32;
        out.points.extend(m.points.iter().copied());
        out.polys.extend(
            m.polys
                .iter()
                .map(|f| f.iter().map(|&v| v + base).collect()),
        );
    }
    if !p.bool("weld") {
        return Ok(out);
    }
    // Merge points within tolerance into their first occurrence.
    let tol = p.float("tolerance").max(1e-9);
    let cell = |q: DVec3| (q / tol).floor().as_i64vec3().to_array();
    let mut grid: HashMap<[i64; 3], Vec<u32>> = HashMap::new();
    let mut points: Vec<DVec3> = Vec::new();
    let mut remap = Vec::with_capacity(out.points.len());
    for &q in &out.points {
        let c = cell(q);
        let near = (-1..=1)
            .flat_map(|dx| (-1..=1).flat_map(move |dy| (-1..=1).map(move |dz| [dx, dy, dz])))
            .filter_map(|d| grid.get(&[c[0] + d[0], c[1] + d[1], c[2] + d[2]]))
            .flatten()
            .copied()
            .filter(|&i| (points[i as usize] - q).length() <= tol)
            .min();
        let i = near.unwrap_or_else(|| {
            points.push(q);
            let i = (points.len() - 1) as u32;
            grid.entry(c).or_default().push(i);
            i
        });
        remap.push(i);
    }
    let polys: Vec<Vec<u32>> = out
        .polys
        .iter()
        .map(|f| f.iter().map(|&v| remap[v as usize]).collect())
        .collect();
    // Drop a face whose corner set equals that of a later face.
    let sets: Vec<BTreeSet<u32>> = polys.iter().map(|f| f.iter().copied().collect()).collect();
    let mut last: HashMap<&BTreeSet<u32>, usize> = HashMap::new();
    for (i, s) in sets.iter().enumerate() {
        last.insert(s, i);
    }
    let polys = polys
        .into_iter()
        .enumerate()
        .filter(|(i, _)| last[&sets[*i]] == *i)
        .map(|(_, f)| f)
        .collect();
    Ok(PolyMesh { points, polys })
}
