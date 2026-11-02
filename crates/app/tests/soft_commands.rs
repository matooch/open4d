//! edit.soft_selection, edit.symmetry and mesh.transform headless (docs/specs/EDITABLE.md §5).

use forge_app::App;
use forge_command::{CommandError, schema};
use forge_core::{NodeId, Value};
use glam::DVec3;
use serde_json::{Value as Json, json};

fn editable(app: &mut App, ty: &str) -> NodeId {
    app.run("object.create", json!({"type": ty})).unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    app.selection.active.unwrap()
}

fn points(app: &App, id: NodeId) -> Vec<DVec3> {
    match app.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => m.points.iter().map(|p| DVec3::from_array(*p)).collect(),
        _ => panic!("no mesh"),
    }
}

fn steps(app: &App) -> usize {
    app.doc.history().undo_labels().len()
}

fn near_origin(pts: &[DVec3]) -> u32 {
    (0..pts.len())
        .min_by(|a, b| pts[*a].length().total_cmp(&pts[*b].length()))
        .unwrap() as u32
}

fn invalid(r: Result<Json, CommandError>) -> bool {
    matches!(r, Err(CommandError::InvalidArgs(_)))
}

fn failed(r: Result<Json, CommandError>) -> bool {
    matches!(r, Err(CommandError::Failed(_)))
}

#[test]
fn settings_commands_set_and_report_the_editor_settings() {
    let mut app = App::new_default();
    let r = app
        .run(
            "edit.soft_selection",
            json!({"enabled": true, "radius": 250, "falloff": "bell", "distance": "surface"}),
        )
        .unwrap();
    assert_eq!(r["radius"], json!(250.0));
    assert_eq!(r["falloff"], json!("bell"));
    assert!(app.component_settings.soft.enabled);
    assert_eq!(app.component_settings.soft.radius, 250.0);
    // omitted keys keep their value
    app.run("edit.soft_selection", json!({"radius": 80}))
        .unwrap();
    assert_eq!(app.component_settings.soft.falloff.id(), "bell");
    // an error changes nothing
    assert!(invalid(app.run(
        "edit.soft_selection",
        json!({"radius": -1, "falloff": "dome"})
    )));
    assert!(invalid(
        app.run("edit.soft_selection", json!({"falloff": "box"}))
    ));
    assert!(invalid(app.run("edit.soft_selection", json!({"junk": 1}))));
    assert_eq!(app.component_settings.soft.radius, 80.0);
    assert_eq!(app.component_settings.soft.falloff.id(), "bell");
    let r = app
        .run(
            "edit.symmetry",
            json!({"enabled": true, "axis": "z", "tolerance": 0.5}),
        )
        .unwrap();
    assert_eq!(r["axis"], json!("z"));
    assert!(app.component_settings.symmetry.enabled);
    assert!(invalid(app.run("edit.symmetry", json!({"axis": "q"}))));
    assert_eq!(app.component_settings.symmetry.tolerance, 0.5);
    // settings are not document edits
    assert_eq!(steps(&app), 0);
}

#[test]
fn a_move_with_radius_100_on_a_plane_follows_the_falloff() {
    let mut app = App::new_default();
    let plane = editable(&mut app, "forge.plane");
    let before = points(&app, plane);
    let c = near_origin(&before);
    let steps_before = steps(&app);
    let out = app
        .run(
            "mesh.transform",
            json!({"points": [c], "move": [0.0, 50.0, 0.0],
                   "soft": {"radius": 100.0, "falloff": "linear"}}),
        )
        .unwrap();
    assert_eq!(steps(&app), steps_before + 1);
    let after = points(&app, plane);
    let mut reached = 0;
    for (b, a) in before.iter().zip(&after) {
        let t = b.distance(before[c as usize]) / 100.0;
        let expect = if t < 1.0 { 50.0 * (1.0 - t) } else { 0.0 };
        assert!((a.y - b.y - expect).abs() < 1e-9, "{b} -> {a}");
        assert_eq!((a.x, a.z), (b.x, b.z));
        if expect > 0.0 {
            reached += 1;
        }
    }
    assert!(reached > 1, "neighbours move");
    assert_eq!(out["moved"], json!(reached));
    // editor settings are untouched by the override
    assert!(!app.component_settings.soft.enabled);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(points(&app, plane), before);
}

#[test]
fn editor_settings_apply_and_soft_false_overrides_them() {
    let mut app = App::new_default();
    let plane = editable(&mut app, "forge.plane");
    let before = points(&app, plane);
    let c = near_origin(&before);
    app.run(
        "edit.soft_selection",
        json!({"enabled": true, "radius": 100.0}),
    )
    .unwrap();
    app.run(
        "mesh.transform",
        json!({"points": [c], "move": [0.0, 10.0, 0.0]}),
    )
    .unwrap();
    let moved = |a: &[DVec3]| (0..a.len()).filter(|i| a[*i] != before[*i]).count();
    assert!(moved(&points(&app, plane)) > 1);
    app.run("edit.undo", json!({})).unwrap();
    app.run(
        "mesh.transform",
        json!({"points": [c], "move": [0.0, 10.0, 0.0], "soft": false}),
    )
    .unwrap();
    assert_eq!(moved(&points(&app, plane)), 1);
}

#[test]
fn the_stored_selection_of_the_current_mode_is_used() {
    let mut app = App::new_default();
    let cube = editable(&mut app, "forge.cube");
    app.run("select.mode", json!({"mode": "polys"})).unwrap();
    app.doc
        .transact("sel", |tx| {
            tx.set_param(cube, "selection.polys", Value::IndexList(vec![0]))
        })
        .unwrap();
    let before = points(&app, cube);
    let out = app
        .run("mesh.transform", json!({"move": [0.0, 0.0, 10.0]}))
        .unwrap();
    assert_eq!(out["moved"], json!(4));
    let after = points(&app, cube);
    assert_eq!((0..8).filter(|i| after[*i] != before[*i]).count(), 4);
}

#[test]
fn symmetry_mirrors_a_move_across_the_plane() {
    let mut app = App::new_default();
    let cube = editable(&mut app, "forge.cube");
    let before = points(&app, cube);
    // a corner at +x, +y, +z and its mirror at -x
    let a = before
        .iter()
        .position(|p| p.x > 0.0 && p.y > 0.0 && p.z > 0.0)
        .unwrap();
    let b = before
        .iter()
        .position(|p| p.x < 0.0 && p.y > 0.0 && p.z > 0.0)
        .unwrap();
    let out = app
        .run(
            "mesh.transform",
            json!({"points": [a], "move": [10.0, 20.0, 30.0], "symmetry": {"axis": "x"}}),
        )
        .unwrap();
    assert_eq!(out["moved"], json!(2));
    let after = points(&app, cube);
    assert_eq!(after[a], before[a] + DVec3::new(10.0, 20.0, 30.0));
    assert_eq!(after[b], before[b] + DVec3::new(-10.0, 20.0, 30.0));
    // symmetry false restores a one-sided move
    app.run("edit.undo", json!({})).unwrap();
    app.run("edit.symmetry", json!({"enabled": true})).unwrap();
    let out = app
        .run(
            "mesh.transform",
            json!({"points": [a], "move": [0.0, 5.0, 0.0], "symmetry": false}),
        )
        .unwrap();
    assert_eq!(out["moved"], json!(1));
}

#[test]
fn rotate_and_scale_arguments_use_the_pivot() {
    let mut app = App::new_default();
    let cube = editable(&mut app, "forge.cube");
    let before = points(&app, cube);
    let all: Vec<u32> = (0..8).collect();
    app.run(
        "mesh.transform",
        json!({"points": all, "scale": 2.0, "pivot": [0.0, 0.0, 0.0]}),
    )
    .unwrap();
    let after = points(&app, cube);
    for (b, a) in before.iter().zip(&after) {
        assert!((*a - *b * 2.0).length() < 1e-9);
    }
    app.run(
        "mesh.transform",
        json!({"points": all, "rotate": {"axis": [0, 1, 0], "angle": 90.0},
               "pivot": [0.0, 0.0, 0.0]}),
    )
    .unwrap();
    let rotated = points(&app, cube);
    for (b, a) in after.iter().zip(&rotated) {
        assert!((a.x - b.z).abs() < 1e-9 && (a.z + b.x).abs() < 1e-9 && a.y == b.y);
    }
    assert_eq!(
        app.doc.history().undo_labels().last().copied(),
        Some("Rotate")
    );
}

#[test]
fn errors_change_nothing() {
    let mut app = App::new_default();
    let cube = editable(&mut app, "forge.cube");
    let before = points(&app, cube);
    let n = steps(&app);
    // Model mode and no points
    assert!(failed(
        app.run("mesh.transform", json!({"move": [1, 0, 0]}))
    ));
    assert!(failed(app.run(
        "mesh.transform",
        json!({"points": [99], "move": [1, 0, 0]})
    )));
    assert!(invalid(app.run("mesh.transform", json!({"points": [0]}))));
    assert!(invalid(
        app.run("mesh.transform", json!({"points": [0], "move": [1, 0]}))
    ));
    assert!(invalid(app.run(
        "mesh.transform",
        json!({"points": [0], "move": [1, 0, 0], "soft": {"radius": 0}})
    )));
    assert!(invalid(app.run(
        "mesh.transform",
        json!({"points": [0], "move": [1, 0, 0], "symmetry": {"axis": "w"}})
    )));
    assert!(invalid(app.run(
        "mesh.transform",
        json!({"points": [0], "rotate": {"axis": [0, 0, 0], "angle": 1}})
    )));
    assert!(invalid(
        app.run("mesh.transform", json!({"points": [0], "junk": 1}))
    ));
    assert_eq!(points(&app, cube), before);
    assert_eq!(steps(&app), n);
    // not a polygon object
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    assert!(failed(app.run(
        "mesh.transform",
        json!({"points": [0], "move": [1, 0, 0]})
    )));
}

#[test]
fn schemas_accept_the_samples() {
    let app = App::new_default();
    for id in ["mesh.transform", "edit.soft_selection", "edit.symmetry"] {
        let h = app.commands.get(id).expect(id);
        let s = h.args_schema();
        schema::validate(&s, &h.sample_args()).unwrap();
        assert!(schema::validate(&s, &json!({"bogus": 1})).is_err(), "{id}");
    }
}
