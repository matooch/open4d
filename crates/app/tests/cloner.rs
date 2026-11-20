//! The cloner through the headless application core (clone-toolset spec C2): created
//! by `object.create`, parameters set through `param.set` (degrees and percent as typed),
//! undo, save/load and OBJ export of the expanded clones.

use forge_app::App;
use forge_core::NodeId;
use forge_parity_harness::parse_obj;
use serde_json::{Value, json};

fn create(app: &mut App, ty: &str) -> NodeId {
    let out = app.run("object.create", json!({"type": ty})).unwrap();
    NodeId(out["node"].as_u64().unwrap())
}

fn set(app: &mut App, node: NodeId, param: &str, value: Value) {
    app.run(
        "param.set",
        json!({"node": node.0, "param": param, "value": value}),
    )
    .unwrap();
}

/// A Linear cloner (count 5, Step Rotation H 10 degrees, S per step 90 %) over a 20 cm cube.
fn scene(app: &mut App) -> (NodeId, NodeId) {
    let cloner = create(app, "forge.cloner");
    let cube = create(app, "forge.cube");
    app.run(
        "object.reparent",
        json!({"node": cube.0, "parent": cloner.0}),
    )
    .unwrap();
    for axis in ["size_x", "size_y", "size_z"] {
        set(app, cube, axis, json!(20.0));
    }
    set(app, cloner, "mode", json!("linear"));
    set(app, cloner, "linear_count", json!(5));
    set(app, cloner, "step_rotation", json!([10.0, 0.0, 0.0]));
    set(app, cloner, "transform_scale", json!([90.0, 90.0, 90.0]));
    (cloner, cube)
}

#[test]
fn object_create_makes_a_cloner_in_grid_mode() {
    let mut app = App::new_default();
    let cloner = create(&mut app, "forge.cloner");
    let node = app.doc.scene().node(cloner).unwrap();
    assert_eq!(node.type_id, "forge.cloner");
    let ty = app.doc.scene().registry().get("forge.cloner").unwrap();
    assert_eq!(
        ty.find_param("mode").unwrap().default,
        forge_core::Value::Enum("grid".into())
    );
    // Parameters are descriptors: every one has a label and a group for the panel.
    assert!(ty.params.iter().all(|p| !p.label.is_empty()));
    assert!(ty.params.iter().any(|p| p.ui.group == "Transform"));
}

#[test]
fn parameters_set_in_display_units_drive_the_clones() {
    let mut app = App::new_default();
    let (cloner, _) = scene(&mut app);
    let data =
        forge_generators::clone_data(app.doc.scene(), cloner, &forge_core::EvalCtx::default())
            .unwrap()
            .unwrap();
    assert_eq!(data.len(), 5);
    let x1 = data.matrices[1].x_axis;
    assert!(
        (x1.x - 0.9 * 10f64.to_radians().cos()).abs() < 1e-9,
        "{x1:?}"
    );
    assert!(
        (x1.z - 0.9 * 10f64.to_radians().sin()).abs() < 1e-9,
        "{x1:?}"
    );
    let mesh = app.eval_mesh(cloner, 0).unwrap();
    assert_eq!(mesh.points.len(), 40);
}

#[test]
fn undo_redo_and_the_file_round_trip() {
    let mut app = App::new_default();
    let (cloner, _) = scene(&mut app);
    let before = app.eval_mesh(cloner, 0).unwrap();
    set(&mut app, cloner, "linear_count", json!(8));
    assert_eq!(app.eval_mesh(cloner, 0).unwrap().points.len(), 64);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(app.eval_mesh(cloner, 0).unwrap(), before);
    app.run("edit.redo", json!({})).unwrap();
    assert_eq!(app.eval_mesh(cloner, 0).unwrap().points.len(), 64);

    let dir = std::env::temp_dir().join(format!("forge-cloner-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cloner.forge").to_string_lossy().into_owned();
    app.save(&path).unwrap();
    let mut other = App::new_default();
    other.open(&path).unwrap();
    assert_eq!(
        other.eval_mesh(cloner, 0).unwrap(),
        app.eval_mesh(cloner, 0).unwrap()
    );
}

#[test]
fn export_holds_the_clones_not_the_prototype() {
    let mut app = App::new_default();
    scene(&mut app);
    let dir = std::env::temp_dir().join(format!("forge-cloner-obj-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("clones.obj").to_string_lossy().into_owned();
    assert_eq!(app.export_obj(&path, None).unwrap(), 1);
    let mesh = parse_obj(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!((mesh.positions.len(), mesh.faces.len()), (40, 30));
}

#[test]
fn make_editable_through_the_command() {
    let mut app = App::new_default();
    let (cloner, _) = scene(&mut app);
    let out = app
        .run("mesh.make_editable", json!({"nodes": [cloner.0]}))
        .unwrap();
    assert_eq!(out["skipped"], json!([]));
    assert_eq!(
        app.doc.scene().len(),
        2,
        "the polygon object and its smoothing tag"
    );
}

/// Brief 068: a Plain effector made by `object.create`, linked through `param.set` on the
/// cloner's `effectors` list (spec E1), moves the clones; undo, save and load keep the link.
#[test]
fn an_effector_is_created_and_linked_by_commands() {
    let mut app = App::new_default();
    let (cloner, _) = scene(&mut app);
    let plain = create(&mut app, "forge.effector.plain");
    let before = app.eval_mesh(cloner, 0).unwrap();
    set(&mut app, cloner, "effectors", json!([plain.0]));
    let after = app.eval_mesh(cloner, 0).unwrap();
    // Default Plain: P (0, 100, 0) at strength 100 %.
    assert!((after.points[0].y - before.points[0].y - 100.0).abs() < 1e-9);
    // Strength is set in percent like every percent parameter.
    set(&mut app, plain, "strength", json!(50.0));
    let half = app.eval_mesh(cloner, 0).unwrap();
    assert!((half.points[0].y - before.points[0].y - 50.0).abs() < 1e-9);
    // The query shows the list as node ids.
    let node = forge_app::query::node_json(app.doc.scene(), cloner);
    assert_eq!(node["params"]["effectors"], json!([plain.0]));
    // A cube is not an effector.
    let cube = create(&mut app, "forge.cube");
    let err = app.run(
        "param.set",
        json!({"node": cloner.0, "param": "effectors", "value": [cube.0]}),
    );
    assert!(err.is_err());

    // Undo takes the cube and then the strength edit back (the refused link changed nothing).
    app.run("edit.undo", json!({})).unwrap();
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(app.eval_mesh(cloner, 0).unwrap(), after);
    let dir = std::env::temp_dir().join(format!("forge-effector-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("effector.forge").to_string_lossy().into_owned();
    app.save(&path).unwrap();
    let mut other = App::new_default();
    other.open(&path).unwrap();
    assert_eq!(
        other.eval_mesh(cloner, 0).unwrap(),
        app.eval_mesh(cloner, 0).unwrap()
    );
}

/// Brief 069: a Random effector made by `object.create` moves the clones deterministically;
/// the seed changes the values, the Random Mode is an option set by its id.
#[test]
fn a_random_effector_is_created_and_seeded_by_commands() {
    let mut app = App::new_default();
    let (cloner, _) = scene(&mut app);
    let random = create(&mut app, "forge.effector.random");
    set(&mut app, cloner, "effectors", json!([random.0]));
    let a = app.eval_mesh(cloner, 0).unwrap();
    assert_eq!(app.eval_mesh(cloner, 0).unwrap(), a);
    set(&mut app, random, "seed", json!(7));
    let b = app.eval_mesh(cloner, 0).unwrap();
    assert_ne!(a, b);
    set(&mut app, random, "random_mode", json!("gaussian"));
    let g = app.eval_mesh(cloner, 0).unwrap();
    assert_ne!(g, b);
    // Sorted has no derived rules: the cloner reports it instead of guessing.
    set(&mut app, random, "random_mode", json!("sorted"));
    assert!(app.eval_mesh(cloner, 0).is_err());
}

/// Brief 072: an effector's falloff is a whole field list (spec E8): rows of any layer type, here a
/// Spherical under an Invert modifier, a disabled row and a transformed field, read the same way
/// for Plain and Random effectors.
#[test]
fn effector_falloff_takes_a_whole_field_list() {
    use forge_core::EvalCtx;
    use forge_fields::FieldList;
    use glam::DVec3;

    let mut app = App::new_default();
    let (cloner, _) = scene(&mut app);
    let plain = create(&mut app, "forge.effector.plain");
    set(&mut app, cloner, "effectors", json!([plain.0]));
    let base = {
        set(&mut app, plain, "strength", json!(0.0));
        app.eval_mesh(cloner, 0).unwrap()
    };
    set(&mut app, plain, "strength", json!(100.0));
    let per = base.points.len() / 5;
    let centre = |m: &forge_geometry::PolyMesh, k: usize| {
        m.points[k * per..(k + 1) * per]
            .iter()
            .fold(DVec3::ZERO, |a, p| a + *p)
            / per as f64
    };
    let sphere = create(&mut app, "forge.field.spherical");
    let invert = create(&mut app, "forge.field.invert_layer");
    set(&mut app, sphere, "transform.pos", json!([0.0, 0.0, 0.0]));
    let check = |app: &App, ids: &[forge_core::NodeId]| {
        let list = FieldList::from_nodes(app.doc.scene(), ids).unwrap();
        let moved = app.eval_mesh(cloner, 0).unwrap();
        let mut seen = std::collections::BTreeSet::new();
        for k in 0..5 {
            let v = list.sample(centre(&base, k), &EvalCtx::default()).value;
            let dy = centre(&moved, k).y - centre(&base, k).y;
            assert!((dy - 100.0 * v).abs() < 1e-6, "clone {k}: {dy} vs {v}");
            seen.insert((v * 1e6) as i64);
        }
        seen.len()
    };
    // One Spherical, then the same under an Invert layer: the clones' falloff flips.
    set(&mut app, plain, "fields", json!([sphere.0]));
    let a = check(&app, &[sphere]);
    set(&mut app, plain, "fields", json!([invert.0, sphere.0]));
    let b = check(&app, &[invert, sphere]);
    assert!(a > 1 && b > 1, "the field must vary over the clones");
    // A row switched off is left out, like an empty list: every clone moves by the full 100.
    set(
        &mut app,
        plain,
        "fields",
        json!([{"target": sphere.0, "enabled": false}]),
    );
    let moved = app.eval_mesh(cloner, 0).unwrap();
    for k in 0..5 {
        assert!((centre(&moved, k).y - centre(&base, k).y - 100.0).abs() < 1e-6);
    }
    // The same list on a Random effector: a clone with falloff 0 does not move.
    let random = create(&mut app, "forge.effector.random");
    set(&mut app, cloner, "effectors", json!([random.0]));
    set(&mut app, random, "fields", json!([invert.0, sphere.0]));
    let with = app.eval_mesh(cloner, 0).unwrap();
    set(&mut app, random, "fields", json!([]));
    let without = app.eval_mesh(cloner, 0).unwrap();
    assert_ne!(with, without);
}
