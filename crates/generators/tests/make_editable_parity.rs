//! Parity: docs/specs/EDITABLE.md §1.6 against tests/golden/make-editable (observation O3).
//!
//! Each case rebuilds its scene from `[params.input]` (the `child` table is the first child),
//! runs the command, then compares the scene as the exporter writes it (every non-consumed
//! object with a mesh in tree order, in world space, concatenated) and the observed tree.

use std::path::Path;
use std::sync::Arc;

mod common;

use common::{Table, resolved_params, typed_params};
use forge_core::{Document, EvalCtx, NodeId, Registry, Scene, Value};
use forge_generators::{
    EditableError, POLYGON, Primitives, current_state_to_object, evaluate, is_consumed,
    make_editable,
};
use forge_modifiers::Deformers;
use forge_parity_harness::{Mesh, compare_geometric, compare_meshes, discover_cases};

/// Cases whose exported mesh we build but whose tree we do not reproduce (EDITABLE.md §1.6).
const TREE_PENDING: &[&str] = &["array"];

/// Menu-created follow-ups whose resulting tree is only in the observation's prose readback
/// (`docs/parity/make-editable.md`, "<case> readback"), not in the case file: the mesh is
/// compared, the tree is `pending: tree not machine-readable`. The newer menu-created Extrude
/// also reports caps integrated in one polygon object where the older `extrude` reports Cap
/// children; the exported meshes agree (EDITABLE.md section 1.2).
const TREE_NOT_RECORDED: &[&str] = &[
    "array-tree",
    "cloner",
    "extrude-cap-off",
    "extrude-csto",
    "extrude-triangles",
    "lathe-caps",
    "two-objects",
    "two-objects-csto",
];

/// Cases with no mesh to compare (tree only).
const TREE_ONLY: &[&str] = &["spline"];

/// Cases whose mesh differs (none).
const KNOWN_MISMATCH: &[&str] = &[];

/// Cases not built (none).
const UNSUPPORTED: &[&str] = &[];

/// The observed tree after the command, without the UVW and data tags (not modelled).
/// `Name<type>{tags}[children]`.
fn expected_tree(case: &str) -> Option<&'static str> {
    Some(match case {
        "cube" => "Cube<polygon>{smoothing}",
        "cube-deformed" => "Cube<polygon>{smoothing}[Bend<bend>]",
        "cube-deformed-csto" => "Cube<cube>{smoothing}[Bend<bend>] Cube<polygon>{smoothing}",
        "subdivision" => "Subdivision Surface<polygon>{smoothing}",
        "subdivision-csto" => {
            "Subdivision Surface<subdivision>[Cube<cube>{smoothing}] \
             Subdivision Surface<polygon>{smoothing}"
        }
        "extrude" => {
            "Extrude<polygon>{smoothing}[Cap 1<polygon>{polygon_selection:C1,smoothing} \
             Cap 2<polygon>{polygon_selection:C2,smoothing}]"
        }
        "sphere-moved" => "Sphere<polygon>{smoothing}",
        "spline" => "Circle<spline>",
        _ => return None,
    })
}

fn registry() -> Arc<Registry> {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r.install(&Deformers).unwrap();
    r.install(&forge_clone_engine::CloneToolset).unwrap();
    Arc::new(r)
}

/// Our node type of a case file's `type` key.
fn type_id(key: &str) -> String {
    match key {
        "circle" | "rectangle" | "arc" => format!("forge.{key}_spline"),
        other => format!("forge.{other}"),
    }
}

fn short(type_id: &str) -> &str {
    type_id.strip_prefix("forge.").unwrap_or(type_id)
}

fn tree_of(scene: &Scene, id: NodeId) -> String {
    let n = scene.node(id).unwrap();
    let tags: Vec<String> = n
        .tags
        .iter()
        .map(|t| {
            let t = scene.node(*t).unwrap();
            if t.type_id == forge_generators::POLYGON_SELECTION {
                format!("{}:{}", short(&t.type_id), t.name)
            } else {
                short(&t.type_id).to_string()
            }
        })
        .collect();
    let kids: Vec<String> = n.children.iter().map(|c| tree_of(scene, *c)).collect();
    let kids = if kids.is_empty() {
        String::new()
    } else {
        format!("[{}]", kids.join(" "))
    };
    let tags = if tags.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", tags.join(","))
    };
    format!("{}<{}>{}{}", n.name, short(&n.type_id), tags, kids)
}

/// Sets a node's params from a case table: `position`, `rotation_hpb_deg`, `*_deg` keys
/// and plain params; `type` and the `child` table are skipped.
fn set_table(tx: &mut forge_core::Tx, reg: &Registry, id: NodeId, ty: &str, table: &Table) {
    let mut plain = Table::new();
    for (k, v) in table {
        match k.as_str() {
            "node" | "child" => {}
            "type" if !table.contains_key("node") => {}
            "position" => tx.set_param(id, "transform.pos", vec3(v, false)).unwrap(),
            "rotation_hpb_deg" => tx.set_param(id, "transform.rot", vec3(v, true)).unwrap(),
            "scale_xyz" => tx.set_param(id, "transform.scale", vec3(v, false)).unwrap(),
            _ => {
                plain.insert(k.clone(), v.clone());
            }
        }
    }
    for (k, v) in typed_params(reg, ty, &plain) {
        tx.set_param(id, &k, v).unwrap();
    }
}

fn vec3(v: &toml::Value, degrees: bool) -> Value {
    let f = |x: &toml::Value| x.as_float().or(x.as_integer().map(|i| i as f64)).unwrap();
    let a = v.as_array().unwrap();
    let k = if degrees { f64::to_radians(1.0) } else { 1.0 };
    Value::Vec3([f(&a[0]) * k, f(&a[1]) * k, f(&a[2]) * k])
}

/// Case keys in degrees (`strength_deg`) are stored as radians under the plain name.
fn rename_deg(table: &Table) -> Table {
    table
        .iter()
        .map(|(k, v)| match k.strip_suffix("_deg") {
            Some(base) if k != "rotation_hpb_deg" => (
                base.to_string(),
                toml::Value::Float(v.as_float().or(v.as_integer().map(|i| i as f64)).unwrap()),
            ),
            _ => (k.clone(), v.clone()),
        })
        .collect()
}

/// The node-type key of a case object: `node` (menu-created form, where `type` may be a
/// param such as the Sphere's) or `type` (the older files).
fn kind(t: &Table) -> &str {
    t.get("node")
        .or(t.get("type"))
        .and_then(|v| v.as_str())
        .unwrap()
}

/// Objects whose `input_2` is their child (the generators); anything else is a second root.
const PARENTS: &[&str] = &["extrude", "lathe", "cloner", "array"];

/// One menu-created object table (`object_type`, the app's labels, raw radians and fractions)
/// as the flat table `set_table` reads: display keys dropped, the switches of the second
/// bevel and cap tabs dropped where the type has no such param, `rotation` in radians.
fn flat_object(reg: &Registry, obj: &Table) -> Table {
    let key = obj
        .get("object_type")
        .or(obj.get("type"))
        .and_then(|v| v.as_str())
        .unwrap()
        .to_lowercase();
    let ty = type_id(&key);
    let node = reg.get(&ty).unwrap_or_else(|| panic!("no node type {ty}"));
    let mut out = Table::new();
    out.insert("node".into(), toml::Value::String(key.clone()));
    if key == "cloner" {
        // the Grid mode's own rows; the rest of the form is at its defaults in the observed case
        out.insert(
            "mode".into(),
            toml::Value::String(obj["mode"].as_str().unwrap().to_lowercase()),
        );
        for (from, to) in [("count", "grid_count"), ("size", "grid_size")] {
            if let Some(v) = obj.get(from) {
                out.insert(to.into(), v.clone());
            }
        }
        if let Some(p) = obj.get("position") {
            out.insert("position".into(), p.clone());
        }
        return out;
    }
    let radians = |v: &toml::Value| {
        toml::Value::Float(
            v.as_float()
                .or(v.as_integer().map(|i| i as f64))
                .unwrap()
                .to_degrees(),
        )
    };
    for (k, v) in obj {
        match k.as_str() {
            "object_type" => continue,
            "type" if node.find_param("type").is_none() => continue,
            "name"
            | "visible_in_editor"
            | "visible_in_renderer"
            | "enabled"
            | "display_color"
            | "x_ray"
            | "rotation_order"
            | "quaternion_rotation"
            | "icon_file_id"
            | "icon_color"
            | "frozen_position"
            | "frozen_rotation"
            | "frozen_scale" => continue,
            _ if k.starts_with("color") => continue,
            "direction" | "hierarchical" if node.find_param(k).is_none() => continue,
            "offset" if key == "extrude" => {
                // Direction Auto goes along +Z here (the sign is the export tests' business)
                let f = v.as_float().or(v.as_integer().map(|i| i as f64)).unwrap();
                out.insert(
                    "movement".into(),
                    toml::Value::Array(vec![0.0.into(), 0.0.into(), f.into()]),
                );
                continue;
            }
            "caps_type" => {
                out.insert("cap_type".into(), v.clone());
                continue;
            }
            "scale" if node.find_param("scale").is_none() => {
                out.insert("scale_xyz".into(), v.clone());
                continue;
            }
            "rotation" => {
                // raw radians: the test's `set_table` reads degrees
                out.insert(
                    "rotation_hpb_deg".into(),
                    toml::Value::Array(v.as_array().unwrap().iter().map(&radians).collect()),
                );
                continue;
            }
            // the splines' Intermediate Points angle and the Lathe's turn are raw radians
            "angle" => {
                let id = if node.find_param("intermediate_angle").is_some() {
                    "intermediate_angle"
                } else {
                    "angle"
                };
                out.insert(id.into(), radians(v));
                continue;
            }
            "intermediate_points" | "number" | "maximum_length" | "radius"
                if key == "rectangle" || key == "arc" || key == "circle" =>
            {
                let id = common::resolve_key(node, k);
                out.insert(id, v.clone());
                continue;
            }
            "scaling" if node.find_param("scaling").is_some() => {
                out.insert(
                    "scaling".into(),
                    toml::Value::Float(v.as_float().unwrap() * 100.0),
                );
                continue;
            }
            "start_cap" | "end_cap" => {
                let id = if k == "start_cap" {
                    "cap_start"
                } else {
                    "cap_end"
                };
                let on = v.as_integer().is_some_and(|i| i != 0);
                out.insert(
                    id.into(),
                    toml::Value::String(if on { "cap" } else { "none" }.into()),
                );
                continue;
            }
            _ => {}
        }
        // raw radians of an angle param (`start_angle`, `end_angle`): the readers below take degrees
        if node
            .find_param(k)
            .is_some_and(|d| d.ui.unit == forge_core::Unit::Angle)
            && v.is_float()
        {
            out.insert(k.clone(), radians(v));
            continue;
        }
        let known = node.find_param(k).is_some();
        let second_tab = k.ends_with("_2") || k.ends_with("_3");
        if !known && (second_tab || common::is_bevel_key(k) || k == "shape_depth") {
            continue;
        }
        out.insert(k.clone(), v.clone());
    }
    out
}

/// One object of a case (and its child) in the document; returns its id.
fn add_object(
    tx: &mut forge_core::Tx,
    reg: &Registry,
    input: &Table,
) -> Result<NodeId, forge_core::EditError> {
    let ty = type_id(kind(input));
    let id = tx.create(&ty, None, None)?;
    set_table(tx, reg, id, &ty, &rename_deg(input));
    // the app names its generator "Subdivision Surface"; our label is shorter
    if ty == "forge.subdivision" {
        tx.rename(id, "Subdivision Surface")?;
    }
    // objects that carry a smoothing tag when created in the app
    if ["forge.cube", "forge.sphere", "forge.extrude"].contains(&ty.as_str()) {
        tx.attach_tag(forge_core::smoothing::TAG, id, None)?;
    }
    if let Some(toml::Value::Table(child)) = input.get("child") {
        let cty = type_id(kind(child));
        let c = tx.create(&cty, Some(id), None)?;
        set_table(tx, reg, c, &cty, &rename_deg(child));
        if cty == "forge.cube" {
            tx.attach_tag(forge_core::smoothing::TAG, c, None)?;
        }
    }
    Ok(id)
}

/// The objects of a case: the legacy `[params.input]` (with its `child`) or the menu-created
/// `[params.input_1]`, `[params.input_2]`, ... (the second is the first's child when the first
/// is a generator, else another root).
fn objects_of(reg: &Registry, params: &Table) -> Vec<Table> {
    if let Some(toml::Value::Table(input)) = params.get("input") {
        // array-tree: the legacy `input`/`child` shape with the menu-created labels
        if input.contains_key("visible_in_editor") {
            let mut flat = flat_object(reg, input);
            if let Some(toml::Value::Table(c)) = input.get("child") {
                flat.insert("child".into(), toml::Value::Table(flat_object(reg, c)));
            }
            return vec![flat];
        }
        return vec![input.clone()];
    }
    let flat: Vec<Table> = (1..)
        .map_while(|n| params.get(&format!("input_{n}")).and_then(|v| v.as_table()))
        .map(|t| flat_object(reg, t))
        .collect();
    let mut roots: Vec<Table> = Vec::new();
    for obj in flat {
        let parent_type = roots.last().map(|r| kind(r).to_string());
        match parent_type {
            Some(t)
                if PARENTS.contains(&t.as_str())
                    && !roots.last().unwrap().contains_key("child") =>
            {
                roots
                    .last_mut()
                    .unwrap()
                    .insert("child".into(), toml::Value::Table(obj));
            }
            _ => roots.push(obj),
        }
    }
    roots
}

fn build_scene(reg: &Arc<Registry>, objects: &[Table]) -> (Document, Vec<NodeId>) {
    let mut d = Document::new(reg.clone());
    let ids = d
        .transact("case", |tx| {
            objects
                .iter()
                .map(|o| add_object(tx, reg, o))
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap();
    (d, ids)
}

/// The scene as the exporter writes it: pre-order, skipping consumed inputs, tags and
/// objects without a mesh; each object's evaluated mesh in world space.
fn export(scene: &Scene) -> Mesh {
    let mut out = Mesh {
        positions: Vec::new(),
        faces: Vec::new(),
    };
    for root in scene.roots() {
        for id in std::iter::once(*root).chain(scene.descendants(*root)) {
            if is_consumed(scene, id) {
                continue;
            }
            let Ok(Some(mesh)) = evaluate(scene, id, &EvalCtx::default()) else {
                continue;
            };
            let m = scene.world_matrix(id).unwrap();
            let base = out.positions.len() as u32;
            out.positions.extend(
                mesh.points
                    .iter()
                    .map(|p| m.transform_point3(*p).to_array()),
            );
            out.faces.extend(
                mesh.polys
                    .iter()
                    .map(|f| f.iter().map(|i| i + base).collect()),
            );
        }
    }
    out
}

fn tree_all(scene: &Scene) -> String {
    scene
        .roots()
        .iter()
        .map(|r| tree_of(scene, *r))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn make_editable_cases_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut mismatched) = (Vec::new(), Vec::new());
    let mut unsupported: Vec<String> = Vec::new();
    let mut trees = 0;
    for case in discover_cases(&root).unwrap() {
        if case.feature != "make-editable" {
            continue;
        }
        let key = case.name.clone();
        let m = case.manifest().unwrap();
        let params = resolved_params(&case, &m);
        let objects = objects_of(&reg, &params);
        let command = match params["command"].as_str().unwrap() {
            "Make Editable" => "make_editable",
            "Current State to Object" => "current_state_to_object",
            other => other,
        };
        let (mut d, srcs) = build_scene(&reg, &objects);
        let mut news = Vec::new();
        for src in srcs {
            let result = d.transact(command, |tx| {
                Ok(match command {
                    "make_editable" => make_editable(tx, src),
                    "current_state_to_object" => current_state_to_object(tx, src),
                    other => panic!("{key}: command {other}"),
                })
            });
            match result.unwrap() {
                Ok(new) => news.push(new),
                Err(EditableError::Build(what)) => {
                    println!("{key}: UNSUPPORTED {what}");
                    unsupported.push(key.clone());
                    break;
                }
                Err(e) => panic!("{key}: {e}"),
            }
        }
        if unsupported.last() == Some(&key) {
            continue;
        }
        let new = news[0];

        // observed tree
        if TREE_NOT_RECORDED.contains(&key.as_str()) {
            assert!(expected_tree(&key).is_none());
        } else if !TREE_PENDING.contains(&key.as_str()) {
            let want = expected_tree(&key).unwrap_or_else(|| panic!("{key}: no expected tree"));
            assert_eq!(tree_all(d.scene()), want, "{key}: tree");
            trees += 1;
        }
        // the object the command made
        let made = d.scene().node(new).unwrap();
        match key.as_str() {
            "spline" => assert_eq!(made.type_id, "forge.spline", "{key}"),
            _ => assert_eq!(made.type_id, POLYGON, "{key}"),
        }
        if TREE_ONLY.contains(&key.as_str()) {
            continue;
        }

        // transform stays on the object
        if key == "sphere-moved" {
            assert_eq!(
                d.scene().param(new, "transform.pos"),
                Some(&Value::Vec3([10.0, 20.0, 30.0]))
            );
        }

        let ours = export(d.scene());
        let expected = case.expected_mesh().unwrap().unwrap();
        let geometric = matches!(
            m.tolerance.compare,
            forge_parity_harness::Compare::Geometric
        );
        let passed = if geometric {
            let r = compare_geometric(&expected, &ours, &m.tolerance);
            if !r.passed() && !KNOWN_MISMATCH.contains(&key.as_str()) {
                panic!("{key}: {:?}", r.failures);
            }
            r.passed()
        } else {
            let r = compare_meshes(&expected, &ours, &m.tolerance);
            if !r.passed() && !KNOWN_MISMATCH.contains(&key.as_str()) {
                panic!("{key}: {:?} (max {:?})", r.failures, r.max_position_error);
            }
            r.passed()
        };
        if passed {
            matched.push(key);
        } else {
            mismatched.push(key);
        }
    }
    println!("matched: {matched:?}; mismatched: {mismatched:?}; trees: {trees}");
    assert_eq!(mismatched, KNOWN_MISMATCH, "known mismatch set changed");
    assert_eq!(matched.len(), 16, "{matched:?}");
    assert_eq!(unsupported, UNSUPPORTED);
    assert_eq!(trees, 8);
}

/// A cap that is off gives no cap object (EDITABLE.md §1.2 step 5); the other keeps its
/// number; without any cap the generator is converted whole.
#[test]
fn a_cap_that_is_off_gives_no_cap_object() {
    let reg = registry();
    for (start, end, names) in [
        ("none", "cap", vec!["Cap 2"]),
        ("cap", "none", vec!["Cap 1"]),
        ("none", "none", vec![]),
    ] {
        let mut d = Document::new(reg.clone());
        let ex = d
            .transact("build", |tx| {
                let ex = tx.create("forge.extrude", None, None)?;
                tx.create("forge.rectangle_spline", Some(ex), None)?;
                tx.set_param(ex, "cap_start", Value::Enum(start.into()))?;
                tx.set_param(ex, "cap_end", Value::Enum(end.into()))?;
                Ok(ex)
            })
            .unwrap();
        let before = evaluate(d.scene(), ex, &EvalCtx::default())
            .unwrap()
            .unwrap();
        let new = d
            .transact("me", |tx| Ok(make_editable(tx, ex).unwrap()))
            .unwrap();
        let s = d.scene();
        let kids: Vec<_> = s
            .node(new)
            .unwrap()
            .children
            .iter()
            .map(|c| s.node(*c).unwrap().name.as_str())
            .collect();
        assert_eq!(kids, names, "{start}/{end}");
        // sides plus caps hold every polygon of the generator's mesh
        let total: usize = std::iter::once(new)
            .chain(s.descendants(new))
            .map(|n| {
                evaluate(s, n, &EvalCtx::default())
                    .unwrap()
                    .map_or(0, |m| m.polys.len())
            })
            .sum();
        assert_eq!(total, before.polys.len(), "{start}/{end}");
    }
}
