//! Headless application core (docs/specs/APP.md §4).

use forge_app::{App, AppError, default_registry};
use forge_command::{CommandError, ModeArg};
use forge_core::{Interp, Key, NodeId};
use forge_parity_harness::parse_obj;
use serde_json::{Value, json};

fn temp(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("forge-app-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name).to_string_lossy().into_owned()
}

fn create(app: &mut App, ty: &str) -> NodeId {
    let out = app.run("object.create", json!({"type": ty})).unwrap();
    NodeId(out["node"].as_u64().unwrap())
}

/// A cube moving along x: 0 at frame 0 to 300 at frame 30 (linear keys, 30 fps).
fn animated_cube(app: &mut App) -> NodeId {
    let cube = create(app, "forge.cube");
    app.run(
        "timeline.settings",
        json!({"fps": 30, "start": 0, "end": 60}),
    )
    .unwrap();
    app.doc
        .transact("Keys", |tx| {
            for (time, x) in [(0.0, 0.0), (1.0, 300.0)] {
                let key = Key {
                    interp: Interp::Linear,
                    ..Key::new(time, x)
                };
                tx.set_key(cube, "transform.pos.x", key)?;
            }
            Ok(())
        })
        .unwrap();
    cube
}

fn center_x(app: &App, node: NodeId, frame: i64) -> f64 {
    let mesh = app.eval_mesh(node, frame).unwrap();
    let (lo, hi) = mesh
        .points
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), p| {
            (lo.min(p.x), hi.max(p.x))
        });
    (lo + hi) / 2.0
}

#[test]
fn default_app_has_plugins_and_commands() {
    let app = App::new_default();
    let registry = default_registry().unwrap();
    let ids = |r: &forge_core::Registry| r.types().map(|t| t.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(app.doc.scene().registry()), ids(&registry));
    assert!(registry.get("forge.cube").is_some());
    assert!(app.commands.get("object.create").is_some());
}

#[test]
fn open_a_saved_scene() {
    let path = temp("open.forge");
    let mut app = App::new_default();
    let cube = create(&mut app, "forge.cube");
    app.run(
        "param.set",
        json!({"node": cube.0, "param": "size_x", "value": 40.0}),
    )
    .unwrap();
    app.save(&path).unwrap();
    assert_eq!(app.file_path.as_deref(), Some(path.as_str()));
    assert_eq!(app.saved_revision, app.doc.scene().revision());

    let mut other = App::new_default();
    other.open(&path).unwrap();
    assert_eq!(other.doc.scene().len(), app.doc.scene().len());
    assert_eq!(other.file_path.as_deref(), Some(path.as_str()));
    let (a, b) = (
        app.eval_mesh(cube, 0).unwrap(),
        other.eval_mesh(cube, 0).unwrap(),
    );
    assert_eq!(a.points, b.points);
    assert!(matches!(
        other.open(&temp("missing.forge")),
        Err(CommandError::Failed(_))
    ));
}

#[test]
fn create_and_set_param_are_undo_steps() {
    let mut app = App::new_default();
    let cube = create(&mut app, "forge.cube");
    let before = app.eval_mesh(cube, 0).unwrap();
    app.run(
        "param.set",
        json!({"node": cube.0, "param": "size_x", "value": 10.0}),
    )
    .unwrap();
    let after = app.eval_mesh(cube, 0).unwrap();
    assert_ne!(before.points, after.points);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(app.eval_mesh(cube, 0).unwrap().points, before.points);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(app.doc.scene().len(), 0);
    assert!(matches!(
        app.run(
            "param.set",
            json!({"node": cube.0, "param": "size_x", "value": 1.0})
        ),
        Err(CommandError::Failed(_) | CommandError::InvalidArgs(_))
    ));
    assert!(matches!(
        app.run("no.such", Value::Null),
        Err(CommandError::Unknown(_))
    ));
}

#[test]
fn eval_mesh_at_frame_15_of_an_animated_cube() {
    let mut app = App::new_default();
    let cube = animated_cube(&mut app);
    assert_eq!(app.timeline.current, 0);
    let (x0, x15, x30) = (
        center_x(&app, cube, 0),
        center_x(&app, cube, 15),
        center_x(&app, cube, 30),
    );
    assert!(x0.abs() < 1e-9, "{x0}");
    assert!((x30 - 300.0).abs() < 1e-9, "{x30}");
    assert!((x15 - 150.0).abs() < 1e-6, "{x15}");
    // Evaluating does not move the timeline; `frame` does.
    assert_eq!(app.timeline.current, 0);
    let scene = app.frame(15);
    assert_eq!(app.timeline.current, 15);
    assert!(scene.contains(cube));
    assert_eq!(app.frame(1000).settings().end, 60);
    assert_eq!(app.timeline.current, 60);
    assert!(matches!(
        app.eval_mesh(NodeId(9999), 0),
        Err(AppError::UnknownNode(9999))
    ));
}

#[test]
fn eval_mesh_without_geometry_fails() {
    let mut app = App::new_default();
    let null = create(&mut app, "forge.null");
    assert_eq!(app.eval_mesh(null, 0), Err(AppError::NoGeometry(null.0)));
}

#[test]
fn export_obj_matches_eval_mesh() {
    let path = temp("frame15.obj");
    let mut app = App::new_default();
    let cube = animated_cube(&mut app);
    app.frame(15);
    assert_eq!(app.export_obj(&path, None).unwrap(), 1);
    let parsed = parse_obj(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let expected = app.eval_mesh(cube, 15).unwrap();
    assert_eq!(parsed.positions.len(), expected.points.len());
    for (got, want) in parsed.positions.iter().zip(&expected.points) {
        for (g, w) in got.iter().zip(want.to_array()) {
            assert!((g - w).abs() < 1e-4, "{got:?} vs {want:?}");
        }
    }
    assert_eq!(parsed.faces.len(), expected.polys.len());
    // Listing no existing node exports nothing.
    assert_eq!(
        app.export_obj(&path, Some(&[NodeId(9999)])),
        Err(AppError::NothingToExport)
    );
}

#[test]
fn headless_component_selection_and_keys() {
    let mut app = App::new_default();
    let cube = create(&mut app, "forge.cube");
    app.run("mesh.make_editable", json!({})).unwrap();
    let poly = app.selection.active.unwrap();
    assert_ne!(poly, cube);
    app.run("select.mode", json!({"mode": "points"})).unwrap();
    assert_eq!(app.mode, forge_app::select::EditMode::Points);
    app.run("select.all", json!({})).unwrap();
    let scene = app.doc.scene();
    let set = forge_app::select::selection_of(scene, poly, app.mode);
    assert_eq!(set.len(), 8);
    app.run("select.none", json!({})).unwrap();
    assert!(forge_app::select::selection_of(app.doc.scene(), poly, app.mode).is_empty());

    app.run("select.mode", json!({"mode": "model"})).unwrap();
    let keys = app.run("key.record", json!({})).unwrap();
    assert_eq!(keys["keys"], 9);
    app.run("timeline.set_frame", json!({"frame": 10})).unwrap();
    app.run("key.record", json!({})).unwrap();
    app.run("key.jump", json!({"forward": false})).unwrap();
    assert_eq!(app.timeline.current, 0);
    assert_eq!(
        forge_command::CommandContext::edit_mode(&app),
        ModeArg::Model
    );
}

#[test]
fn export_obj_command_writes_the_evaluated_frame() {
    let path = temp("command-frame15.obj");
    let mut app = App::new_default();
    let cube = animated_cube(&mut app);
    // Frame 0: the stored position (x = 0), identical to the stored scene.
    app.run("file.export_obj", json!({"path": path})).unwrap();
    let at0 = parse_obj(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(at0.positions.iter().all(|p| p[0].abs() <= 100.0 + 1e-9));
    // Frame 15: the cube has moved to x = 150.
    app.frame(15);
    app.run("file.export_obj", json!({"path": path})).unwrap();
    let at15 = parse_obj(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let want = app.eval_mesh(cube, 15).unwrap();
    assert_eq!(at15.positions.len(), want.points.len());
    for (got, w) in at15.positions.iter().zip(&want.points) {
        for (g, w) in got.iter().zip(w.to_array()) {
            assert!((g - w).abs() < 1e-4, "{got:?} vs {w:?}");
        }
    }
    let min_x = at15.positions.iter().map(|p| p[0]).fold(f64::MAX, f64::min);
    assert!((min_x - 50.0).abs() < 1e-4, "{min_x}");
}
