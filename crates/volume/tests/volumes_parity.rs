//! Parity of the builder and the mesher against tests/golden/volumes (docs/specs/VOLUME.md).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, ParamKind, Registry, Tx, Value};
use forge_generators::Primitives;
use forge_parity_harness::{Mesh, compare_volume_mesh, discover_cases, export_weld};
use forge_volume::VolumePlugin;

type Table = toml::map::Map<String, toml::Value>;

fn registry() -> Arc<Registry> {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r.install(&forge_fields::FieldToolset).unwrap();
    r.install(&VolumePlugin).unwrap();
    Arc::new(r)
}

fn f(v: &toml::Value) -> f64 {
    v.as_float().or(v.as_integer().map(|i| i as f64)).unwrap()
}

/// Object keys every node has (position, colour, display state): not params of the type.
fn is_generic(k: &str) -> bool {
    ["color_", "frozen_", "global_", "transformed_"]
        .iter()
        .any(|p| k.starts_with(p))
        || matches!(
            k,
            "display_color"
                | "enabled"
                | "name"
                | "quaternion_rotation"
                | "rotation_order"
                | "visible_in_editor"
                | "visible_in_renderer"
                | "x_ray"
        )
}

fn node_type(name: &str) -> &'static str {
    match name {
        "Sphere" => "forge.sphere",
        "Platonic" => "forge.platonic",
        "Cube" => "forge.cube",
        "Null" => "forge.null",
        "Helix" => "forge.helix_spline",
        other => panic!("unmapped case object {other}"),
    }
}

/// Sets a node's params from a case table (transform keys, then the type's own keys).
fn apply(tx: &mut Tx, reg: &Registry, id: NodeId, type_id: &str, t: &Table) {
    let ty = reg.get(type_id).unwrap();
    let get = |k: &str| t.get(k).map(f).unwrap_or(0.0);
    let pos = [get("position_x"), get("position_y"), get("position_z")];
    if pos != [0.0; 3] {
        tx.set_param(id, "transform.pos", Value::Vec3(pos)).unwrap();
    }
    for k in ["rotation_x", "rotation_y", "rotation_z"] {
        assert_eq!(get(k), 0.0, "rotated case objects are not used");
    }
    let mut size = [None; 3];
    for (k, v) in t {
        if is_generic(k) || k.starts_with("position_") || k.starts_with("rotation_") {
            continue;
        }
        if k.starts_with("scale_") {
            assert_eq!(f(v), 1.0);
            continue;
        }
        if k == "type" {
            // the case form names the type by label or index; a Platonic's index is its option
            if let (Some(i), Some(d)) = (v.as_integer(), ty.find_param("type"))
                && let ParamKind::Enum { options } = &d.kind
                && type_id == "forge.platonic"
            {
                tx.set_param(id, "type", Value::Enum(options[i as usize].id.clone()))
                    .unwrap();
            }
            continue;
        }
        if let Some(axis) = k.strip_prefix("size_") {
            size["xyz".find(axis).unwrap()] = Some(f(v));
            continue;
        }
        // keys of the reference that our types do not store (bias fractions, display)
        let Some(desc) = ty.find_param(k) else {
            assert!(
                matches!(
                    k.as_str(),
                    "render_perfect"
                        | "fillet"
                        | "fillet_radius"
                        | "fillet_subdivision"
                        | "segments_x"
                        | "segments_y"
                        | "segments_z"
                        | "separate_surfaces"
                        | "aspect_ratio"
                        | "orientation"
                        | "radius"
                        | "shape"
                        | "angle"
                        | "close_spline"
                        | "height_bias"
                        | "radial_bias"
                        | "intermediate_points"
                        | "maximum_length"
                        | "number"
                        | "reverse"
                ),
                "{type_id}: unexpected case key {k}"
            );
            continue;
        };
        let value = match (&desc.kind, v) {
            // percent params of the scripted form are fractions (0.5 is 50 %)
            (ParamKind::Float { .. }, v) if desc.ui.unit == forge_core::Unit::Percent => {
                Value::Float(f(v) * 100.0)
            }
            (ParamKind::Float { .. }, v) => Value::Float(f(v)),
            (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
            (ParamKind::Bool, toml::Value::Integer(i)) => Value::Bool(*i != 0),
            (ParamKind::Enum { options }, toml::Value::Integer(i)) => {
                Value::Enum(options[*i as usize].id.clone())
            }
            other => panic!("{type_id}.{k}: {other:?}"),
        };
        tx.set_param(id, k, value).unwrap();
    }
    if size.iter().any(Option::is_some) && ty.find_param("size").is_some() {
        let d = [200.0; 3];
        let s = [0, 1, 2].map(|i| size[i].unwrap_or(d[i]));
        tx.set_param(id, "size", Value::Vec3(s)).unwrap();
    }
}

/// The operator index of a filter case: the description names it (`Operator 4`); the case table
/// of the mean, median and Laplacian cases still reads 0 (VOLUME.md section 8.2).
fn operator_index(t: &Table, description: &str) -> i64 {
    description
        .split("Operator ")
        .nth(1)
        .and_then(|rest| rest.chars().next()?.to_digit(10))
        .map_or_else(|| t["operator"].as_integer().unwrap(), i64::from)
}

fn apply_filter(tx: &mut Tx, id: NodeId, t: &Table, description: &str) {
    let kind = ["offset", "reshape", "smooth"][t["type"].as_integer().unwrap() as usize];
    tx.set_param(id, "kind", Value::Enum(kind.into())).unwrap();
    let op = match operator_index(t, description) {
        0 => "gaussian",
        1 => "mean",
        2 => "median",
        4 => "laplacian",
        other => panic!("operator {other}"),
    };
    tx.set_param(id, "smooth_operator", Value::Enum(op.into()))
        .unwrap();
    tx.set_param(id, "offset", Value::Float(f(&t["offset"])))
        .unwrap();
    tx.set_param(
        id,
        "iterations",
        Value::Int(t["iterations"].as_integer().unwrap()),
    )
    .unwrap();
    tx.set_param(id, "strength", Value::Float(f(&t["strength"])))
        .unwrap();
    tx.set_param(
        id,
        "voxel_distance",
        Value::Int(t["voxel_distance"].as_integer().unwrap()),
    )
    .unwrap();
    tx.set_param(
        id,
        "voxel_range",
        Value::Int(t["voxel_range"].as_integer().unwrap()),
    )
    .unwrap();
}

pub struct Built {
    pub doc: Document,
    pub mesher: NodeId,
    pub builder: NodeId,
}

/// The scene a case describes: mesher, builder child, the input objects under the builder.
pub fn build_case(reg: &Arc<Registry>, params: &Table, description: &str) -> Built {
    let mut doc = Document::new(reg.clone());
    let (mesher, builder) = doc
        .transact("case", |tx| {
            let mesher = tx.create("forge.volume_mesher", None, None)?;
            let builder = tx.create("forge.volume_builder", Some(mesher), None)?;
            let mt = params["mesher"].as_table().unwrap();
            tx.set_param(
                mesher,
                "surface_threshold",
                Value::Float(f(&mt["surface_threshold"])),
            )?;
            tx.set_param(
                mesher,
                "voxel_range_threshold",
                Value::Float(f(&mt["voxel_range_threshold"])),
            )?;
            tx.set_param(mesher, "adaptive", Value::Float(f(&mt["adaptive"])))?;
            tx.set_param(
                mesher,
                "use_absolute_value_iso",
                Value::Bool(mt["use_absolute_value_iso"].as_integer() != Some(0)),
            )?;
            let b = params["builder"].as_table().unwrap();
            tx.set_param(builder, "voxel_size", Value::Float(f(&b["voxel_size"])))?;
            for k in [
                "interior_voxel_range",
                "exterior_voxel_range",
                "spline_voxel_range",
                "particle_voxel_range",
            ] {
                tx.set_param(builder, k, Value::Int(b[k].as_integer().unwrap()))?;
            }
            let vt = match b["volume_type"].as_str().unwrap() {
                "Signed Distance Field" => "sdf",
                "Fog" => "fog",
                other => panic!("volume type {other}"),
            };
            tx.set_param(builder, "volume_type", Value::Enum(vt.into()))?;
            // a filter layer is listed above the objects, so it is the first child
            if let Some(t) = params.get("filter").and_then(|v| v.as_table()) {
                let id = tx.create("forge.volume_filter", Some(builder), None)?;
                apply_filter(tx, id, t, description);
            }
            // a field layer is listed above the objects too
            if let Some(t) = params.get("field").and_then(|v| v.as_table()) {
                assert_eq!(t["type"].as_str(), Some("spherical_field"));
                let id = tx.create("forge.field.spherical", Some(builder), None)?;
                tx.set_param(id, "radius", Value::Float(f(&t["size"])))?;
            }
            // inputs: input, input2, ..., or a Null `input` with children input_a, input_b
            let table = |k: &str| params.get(k).and_then(|v| v.as_table());
            let mut parents: BTreeMap<&str, NodeId> = BTreeMap::new();
            for key in ["input", "input2", "spline", "input_a", "input_b"] {
                let Some(t) = table(key) else { continue };
                let ty = node_type(t["name"].as_str().unwrap());
                let parent = if key.ends_with("_a") || key.ends_with("_b") {
                    parents["input"]
                } else {
                    builder
                };
                let id = tx.create(ty, Some(parent), None)?;
                apply(tx, reg, id, ty, t);
                parents.insert(key, id);
            }
            Ok((mesher, builder))
        })
        .unwrap();
    Built {
        doc,
        mesher,
        builder,
    }
}

fn mesh_of(b: &Built) -> Mesh {
    let m = forge_generators::evaluate(b.doc.scene(), b.mesher, &EvalCtx::default())
        .unwrap()
        .expect("mesh");
    Mesh {
        positions: m.points.iter().map(|p| p.to_array()).collect(),
        faces: m.polys,
    }
}

/// Observed cases whose options are not built; each is listed with the reason.
const UNSUPPORTED: &[(&str, &str)] = &[];

/// Built, but the geometric comparison fails.
const KNOWN_MISMATCH: &[(&str, &str)] = &[(
    "smooth-strength",
    "three Gaussian iterations: the reference rounds the cube to a near sphere, our iterated box means reach area +6.5 %, Hausdorff 7.5 (VOLUME.md 8.3)",
)];

/// Cases whose point and face counts equal the reference's and whose every vertex is within
/// 1e-3 of a reference vertex (VOLUME.md section 7.2).
const EXACT: &[&str] = &[
    "cube-defaults",
    "cube-voxel-5",
    "cube-voxel-20",
    "sphere-defaults",
    "sphere-voxel-5",
    "union-two",
    "hierarchy",
    "torus-defaults",
    "mesher-threshold",
    "mesher-adaptive",
];

/// Largest distance from a point of `a` to the nearest point of `b` (hash grid of 2 units;
/// a distance above 2 reports 2).
fn max_nearest(a: &Mesh, b: &Mesh) -> f64 {
    let cell = |p: &[f64; 3]| p.map(|c| (c / 2.0).floor() as i64);
    let mut grid: BTreeMap<[i64; 3], Vec<[f64; 3]>> = BTreeMap::new();
    for p in &b.positions {
        grid.entry(cell(p)).or_default().push(*p);
    }
    a.positions
        .iter()
        .map(|p| {
            let c = cell(p);
            let mut best: f64 = 2.0;
            for d in 0..27 {
                let n = [c[0] + d % 3 - 1, c[1] + d / 3 % 3 - 1, c[2] + d / 9 - 1];
                for q in grid.get(&n).into_iter().flatten() {
                    let dist = (0..3).map(|k| (p[k] - q[k]).powi(2)).sum::<f64>().sqrt();
                    best = best.min(dist);
                }
            }
            best
        })
        .fold(0.0, f64::max)
}

#[test]
fn volumes_match_the_reference_geometrically() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let reg = registry();
    let listed: BTreeSet<&str> = UNSUPPORTED
        .iter()
        .chain(KNOWN_MISMATCH)
        .map(|(n, _)| *n)
        .collect();
    let mut seen = BTreeSet::new();
    let mut matched = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "volumes" {
            continue;
        }
        seen.insert(case.name.clone());
        let man = case.manifest().unwrap();
        let params: Table = man.params.clone().into_iter().collect();
        if let Some((_, why)) = UNSUPPORTED.iter().find(|(n, _)| *n == case.name) {
            panic!("{}: unsupported ({why}) is not asserted yet", case.name);
        }
        let built = build_case(&reg, &params, man.description.as_deref().unwrap_or(""));
        let ours = mesh_of(&built);
        let exp = case.expected_mesh().unwrap().unwrap();
        let voxel = f(&params["builder"].as_table().unwrap()["voxel_size"]);
        let r = compare_volume_mesh(&exp, &ours, voxel);
        if std::env::var_os("VOLUME_REPORT").is_some() {
            println!(
                "{:<18} pts {:>5}/{:<5} haus {:.2}/{:.2} area {:.0}/{:.0}",
                case.name,
                exp.positions.len(),
                ours.positions.len(),
                r.hausdorff_expected_to_actual,
                r.hausdorff_actual_to_expected,
                r.area.0,
                r.area.1
            );
        }
        if let Some((_, why)) = KNOWN_MISMATCH.iter().find(|(n, _)| *n == case.name) {
            assert!(
                !r.passed(),
                "{} now matches ({why}): remove it from KNOWN_MISMATCH",
                case.name
            );
            continue;
        }
        assert!(r.passed(), "{}: {:?}", case.name, r.failures);
        if EXACT.contains(&case.name.as_str()) {
            assert_eq!(
                (exp.positions.len(), exp.faces.len()),
                (ours.positions.len(), ours.faces.len()),
                "{}: counts",
                case.name
            );
            let adaptive = params["mesher"].as_table().unwrap()["adaptive"].as_float() != Some(0.0);
            if adaptive {
                // O32 mesher-adaptive: 30 quads and 42 triangles
                let quads = |m: &Mesh| m.faces.iter().filter(|f| f.len() == 4).count();
                assert_eq!(quads(&exp), quads(&ours), "{}: quad count", case.name);
            } else {
                assert!(
                    ours.faces.iter().all(|f| f.len() == 4),
                    "{}: quads only",
                    case.name
                );
            }
            let (e, o) = (export_weld(&exp), export_weld(&ours));
            let d = max_nearest(&e, &o).max(max_nearest(&o, &e));
            assert!(d < 1e-3, "{}: vertices differ by {d}", case.name);
        }
        matched.push(case.name.clone());
    }
    assert!(listed.iter().all(|n| seen.contains(*n)), "stale list entry");
    assert_eq!(seen.len(), 21, "the volumes golden set changed");
    assert_eq!(matched.len(), 20, "matched: {matched:?}");
    for n in EXACT {
        assert!(matched.iter().any(|m| m == n), "{n} missing");
    }
}
