//! Axis editing and snapping commands headless (docs/specs/MANIPULATE.md §6–7): the content
//! keeps its world place, one undo step per command, errors change nothing.

use forge_app::App;
use forge_command::{CommandError, schema};
use forge_core::{NodeId, Value};
use glam::{DMat4, DVec3};
use serde_json::{Value as Json, json};

const COMMANDS: &[&str] = &[
    "object.set_axis",
    "object.center_axis",
    "object.align_axis",
    "edit.axis_mode",
    "snap.set",
    "snap.toggle",
    "snap.reset",
    "snap.query",
];

fn editable_cube(app: &mut App) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    app.selection.active.unwrap()
}

fn child_of(app: &mut App, parent: NodeId, pos: [f64; 3]) -> NodeId {
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let child = app.selection.active.unwrap();
    app.run(
        "object.reparent",
        json!({"node": child.0, "parent": parent.0}),
    )
    .unwrap();
    app.run(
        "transform.set",
        json!({"node": child.0, "pos": pos, "rot": [10.0, 20.0, 30.0]}),
    )
    .unwrap();
    child
}

fn world_points(app: &App, id: NodeId) -> Vec<DVec3> {
    let w = app.doc.scene().world_matrix(id).unwrap();
    match app.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => m
            .points
            .iter()
            .map(|p| w.transform_point3(DVec3::from_array(*p)))
            .collect(),
        _ => panic!("no mesh"),
    }
}

fn world(app: &App, id: NodeId) -> DMat4 {
    app.doc.scene().world_matrix(id).unwrap()
}

fn steps(app: &App) -> usize {
    app.doc.history().undo_labels().len()
}

fn same_points(a: &[DVec3], b: &[DVec3]) {
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(b) {
        assert!((*p - *q).abs().max_element() < 1e-9, "{p} != {q}");
    }
}

fn same_matrix(a: DMat4, b: DMat4) {
    let d = (a - b)
        .to_cols_array()
        .iter()
        .fold(0.0f64, |m, x| m.max(x.abs()));
    assert!(d < 1e-9, "{a} != {b}");
}

fn local_pos(app: &App, id: NodeId) -> DVec3 {
    app.doc.scene().local_transform(id).unwrap().pos
}

/// Moves the object transform and the mesh so the cube is off-centre and rotated.
fn rotated_rig(app: &mut App) -> (NodeId, NodeId) {
    let cube = editable_cube(app);
    app.run(
        "transform.set",
        json!({"node": cube.0, "pos": [300.0, 50.0, -20.0], "rot": [30.0, -20.0, 10.0],
               "scale": [2.0, 2.0, 2.0]}),
    )
    .unwrap();
    let kid = child_of(app, cube, [10.0, 20.0, 30.0]);
    app.selection.click(cube);
    (cube, kid)
}

#[test]
fn set_axis_local_keeps_points_and_children_in_place() {
    let mut app = App::new_default();
    let (cube, kid) = rotated_rig(&mut app);
    let (pts, kw) = (world_points(&app, cube), world(&app, kid));
    let before = steps(&app);
    let out = app
        .run(
            "object.set_axis",
            json!({"node": cube.0, "pos": [0.0, 0.0, 0.0], "rot": [45.0, 10.0, -5.0]}),
        )
        .unwrap();
    assert_eq!(out["nodes"], json!([cube.0]));
    assert_eq!(local_pos(&app, cube), DVec3::ZERO);
    same_points(&pts, &world_points(&app, cube));
    same_matrix(world(&app, kid), kw);
    assert_eq!(steps(&app), before + 1, "one undo step");
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(steps(&app), before);
    assert_eq!(world_points(&app, cube), pts);
    assert_eq!(world(&app, kid), kw);
}

#[test]
fn set_axis_world_places_the_axis_at_a_world_point() {
    let mut app = App::new_default();
    let (cube, kid) = rotated_rig(&mut app);
    // under a rotated, scaled parent the world rule matters
    let parent = app.doc.scene().node(cube).unwrap().parent;
    assert!(parent.is_none());
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let group = app.selection.active.unwrap();
    app.run(
        "transform.set",
        json!({"node": group.0, "pos": [0.0, 100.0, 0.0], "rot": [90.0, 0.0, 0.0]}),
    )
    .unwrap();
    app.run(
        "object.reparent",
        json!({"node": cube.0, "parent": group.0}),
    )
    .unwrap();
    let (pts, kw) = (world_points(&app, cube), world(&app, kid));
    app.run(
        "object.set_axis",
        json!({"node": cube.0, "pos": [10.0, 20.0, 30.0], "rot": [0.0, 0.0, 0.0],
               "space": "world"}),
    )
    .unwrap();
    let w = world(&app, cube);
    assert!((w.w_axis.truncate() - DVec3::new(10.0, 20.0, 30.0)).length() < 1e-9);
    // the world rotation is identity (the scale is still 2)
    assert!((w.x_axis.truncate().normalize() - DVec3::X).length() < 1e-9);
    assert!((w.y_axis.truncate().normalize() - DVec3::Y).length() < 1e-9);
    same_points(&pts, &world_points(&app, cube));
    same_matrix(world(&app, kid), kw);
}

#[test]
fn set_axis_errors_change_nothing() {
    let mut app = App::new_default();
    let (cube, _) = rotated_rig(&mut app);
    let before = steps(&app);
    let pts = world_points(&app, cube);
    assert!(matches!(
        app.run("object.set_axis", json!({"node": cube.0})),
        Err(CommandError::InvalidArgs(_))
    ));
    assert!(matches!(
        app.run("object.set_axis", json!({"node": 9999, "pos": [0, 0, 0]})),
        Err(CommandError::Failed(_))
    ));
    assert!(
        app.run("object.set_axis", json!({"node": cube.0, "pos": [0, 0]}))
            .is_err()
    );
    assert_eq!(steps(&app), before);
    assert_eq!(world_points(&app, cube), pts);
}

#[test]
fn center_axis_moves_the_axis_to_the_centre_without_moving_points() {
    let mut app = App::new_default();
    let (cube, kid) = rotated_rig(&mut app);
    // shift the mesh so the bounding box centre is (50, 0, 0) in object space
    let mut mesh = match app.doc.scene().param(cube, "mesh") {
        Some(Value::Mesh(m)) => (**m).clone(),
        _ => panic!(),
    };
    for p in &mut mesh.points {
        p[0] += 50.0;
    }
    app.doc
        .transact("shift", |tx| {
            tx.set_param(cube, "mesh", Value::Mesh(std::sync::Arc::new(mesh)))
        })
        .unwrap();
    let (pts, kw) = (world_points(&app, cube), world(&app, kid));
    let before = steps(&app);
    let out = app
        .run("object.center_axis", json!({"nodes": [cube.0]}))
        .unwrap();
    assert_eq!(out["centers"][cube.0.to_string()], json!([50.0, 0.0, 0.0]));
    same_points(&pts, &world_points(&app, cube));
    same_matrix(world(&app, kid), kw);
    assert_eq!(steps(&app), before + 1);
    // the object origin now sits at the old bounding box centre
    let c = DVec3::new(150.0, 0.0, 0.0);
    let expect = pts_centre_world(&pts);
    assert!(
        (world(&app, cube).w_axis.truncate() - expect).length() < 1e-9,
        "{c}"
    );
    // the mesh is centred on the origin
    let local = match app.doc.scene().param(cube, "mesh") {
        Some(Value::Mesh(m)) => m.points.iter().map(|p| p[0]).sum::<f64>() / 8.0,
        _ => panic!(),
    };
    assert!(local.abs() < 1e-9);
}

fn pts_centre_world(pts: &[DVec3]) -> DVec3 {
    // the box is a transformed cube: the centre is the mean of all eight corners
    pts.iter().sum::<DVec3>() / pts.len() as f64
}

#[test]
fn center_axis_to_selection_points_and_children() {
    let mut app = App::new_default();
    let (cube, kid) = rotated_rig(&mut app);
    let pts = world_points(&app, cube);
    // select the points 0 and 1: their average is on the cube's -y/-z edge
    app.doc
        .transact("sel", |tx| {
            tx.set_param(cube, "selection.points", Value::IndexList(vec![0, 1]))
        })
        .unwrap();
    let out = app
        .run("object.center_axis", json!({"to": "selection"}))
        .unwrap();
    let c: Vec<f64> = serde_json::from_value(out["centers"][cube.0.to_string()].clone()).unwrap();
    let m = match app.doc.scene().param(cube, "mesh") {
        Some(Value::Mesh(m)) => m.clone(),
        _ => panic!(),
    };
    // the selected points average is now the local origin
    let avg = (DVec3::from_array(m.points[0]) + DVec3::from_array(m.points[1])) / 2.0;
    assert!(avg.length() < 1e-9, "{avg} (centre was {c:?})");
    same_points(&pts, &world_points(&app, cube));
    // children: the axis goes to the average child position
    let kw = world(&app, kid);
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let group = app.selection.active.unwrap();
    let a = child_of(&mut app, group, [100.0, 0.0, 0.0]);
    let b = child_of(&mut app, group, [0.0, 100.0, 0.0]);
    let (wa, wb) = (world(&app, a), world(&app, b));
    app.run(
        "object.center_axis",
        json!({"nodes": [group.0], "to": "children"}),
    )
    .unwrap();
    assert!((local_pos(&app, group) - DVec3::new(50.0, 50.0, 0.0)).length() < 1e-9);
    same_matrix(world(&app, a), wa);
    same_matrix(world(&app, b), wb);
    same_matrix(world(&app, kid), kw);
}

#[test]
fn center_axis_needs_a_polygon_object_and_is_all_or_nothing() {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let generator = app.selection.active.unwrap();
    let editable = editable_cube(&mut app);
    let before = steps(&app);
    let pos = local_pos(&app, editable);
    // the generator in the list fails the whole command
    let r = app.run(
        "object.center_axis",
        json!({"nodes": [editable.0, generator.0]}),
    );
    assert!(matches!(r, Err(CommandError::Failed(_))), "{r:?}");
    assert_eq!(steps(&app), before);
    assert_eq!(local_pos(&app, editable), pos);
    // children centre needs children
    assert!(
        app.run(
            "object.center_axis",
            json!({"nodes": [editable.0], "to": "children"})
        )
        .is_err()
    );
    // selection needs a selection
    assert!(
        app.run(
            "object.center_axis",
            json!({"nodes": [editable.0], "to": "selection"})
        )
        .is_err()
    );
}

#[test]
fn two_objects_are_one_undo_step() {
    let mut app = App::new_default();
    let a = editable_cube(&mut app);
    let b = editable_cube(&mut app);
    for id in [a, b] {
        app.run(
            "transform.set",
            json!({"node": id.0, "pos": [id.0 as f64 * 100.0, 0.0, 0.0]}),
        )
        .unwrap();
    }
    let before = steps(&app);
    let pts = (world_points(&app, a), world_points(&app, b));
    app.run(
        "object.align_axis",
        json!({"nodes": [a.0, b.0], "to": "world"}),
    )
    .unwrap();
    app.run(
        "object.center_axis",
        json!({"nodes": [a.0, b.0], "to": "points"}),
    )
    .unwrap();
    assert_eq!(steps(&app), before + 2);
    same_points(&pts.0, &world_points(&app, a));
    same_points(&pts.1, &world_points(&app, b));
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(steps(&app), before + 1);
}

#[test]
fn align_axis_to_world_parent_and_normal() {
    let mut app = App::new_default();
    let (cube, kid) = rotated_rig(&mut app);
    let (pts, kw) = (world_points(&app, cube), world(&app, kid));
    app.run("object.align_axis", json!({"to": "world"}))
        .unwrap();
    let w = world(&app, cube);
    assert!((w.x_axis.truncate().normalize() - DVec3::X).length() < 1e-9);
    assert!((w.y_axis.truncate().normalize() - DVec3::Y).length() < 1e-9);
    same_points(&pts, &world_points(&app, cube));
    same_matrix(world(&app, kid), kw);
    // parent: rotation zero (no parent: same as world)
    app.run(
        "transform.set",
        json!({"node": cube.0, "rot": [10.0, 20.0, 30.0]}),
    )
    .unwrap();
    let pts = world_points(&app, cube);
    app.run("object.align_axis", json!({"to": "parent"}))
        .unwrap();
    assert_eq!(
        app.doc.scene().local_transform(cube).unwrap().rot,
        DVec3::ZERO
    );
    same_points(&pts, &world_points(&app, cube));
    // normal: select the polygon facing +Y and align Z to it
    let m = match app.doc.scene().param(cube, "mesh") {
        Some(Value::Mesh(m)) => forge_generators::to_poly_mesh(m),
        _ => panic!(),
    };
    let top = 2usize;
    let n_obj = forge_geometry::normals::poly_normal(&m.points, &m.polys[top]);
    // uniform scale: the world normal is the world rotation of the object-space normal
    let expected = world(&app, cube).transform_vector3(n_obj).normalize();
    let top = top as u32;
    app.doc
        .transact("sel", |tx| {
            tx.set_param(cube, "selection.polys", Value::IndexList(vec![top]))
        })
        .unwrap();
    let pts = world_points(&app, cube);
    let before = steps(&app);
    app.run("object.align_axis", json!({"to": "normal"}))
        .unwrap();
    assert_eq!(steps(&app), before + 1);
    let w = world(&app, cube);
    let z = w.z_axis.truncate().normalize();
    // world Z is the polygon's world normal
    assert!((z - expected).length() < 1e-9, "{z} != {expected}");
    same_points(&pts, &world_points(&app, cube));
    // right-handed
    let (x, y) = (w.x_axis.truncate(), w.y_axis.truncate());
    assert!(x.cross(y).dot(w.z_axis.truncate()) > 0.0);
}

#[test]
fn align_axis_normal_errors() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    // nothing selected
    assert!(matches!(
        app.run("object.align_axis", json!({"to": "normal"})),
        Err(CommandError::Failed(_))
    ));
    // a generator has no stored polygons
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    assert!(matches!(
        app.run("object.align_axis", json!({"to": "normal"})),
        Err(CommandError::Failed(_))
    ));
    assert!(app.run("object.align_axis", json!({})).is_err());
    assert!(app.run("object.align_axis", json!({"to": "up"})).is_err());
    assert_eq!(steps(&app), before + 1, "only the create");
    let _ = cube;
}

#[test]
fn axis_mode_and_snap_settings_are_not_undoable() {
    let mut app = App::new_default();
    let before = steps(&app);
    assert_eq!(
        app.run("edit.axis_mode", json!({})).unwrap(),
        json!({"enabled": true})
    );
    assert!(app.transform_settings.axis_mode);
    assert_eq!(
        app.run("edit.axis_mode", json!({"enabled": true})).unwrap()["enabled"],
        true
    );
    assert_eq!(
        app.run("edit.axis_mode", json!({})).unwrap(),
        json!({"enabled": false})
    );
    let out = app
        .run(
            "snap.set",
            json!({"enabled": true, "mode": "2d", "radius": 25, "targets": {"polygon": true},
                   "quantize": {"move": 5}}),
        )
        .unwrap();
    assert_eq!(out["mode"], "2d");
    assert_eq!(out["targets"]["polygon"], true);
    assert_eq!(out["targets"]["vertex"], true);
    assert_eq!(out["quantize"]["move"], 5.0);
    assert!(app.transform_settings.snap.enabled);
    assert_eq!(app.transform_settings.snap.radius, 25.0);
    // reading
    assert_eq!(app.run("snap.set", json!({})).unwrap(), out);
    assert_eq!(
        app.run("snap.toggle", json!({})).unwrap(),
        json!({"enabled": false})
    );
    assert_eq!(
        app.run("snap.toggle", json!({"enabled": true})).unwrap(),
        json!({"enabled": true})
    );
    // bad values change nothing
    assert!(app.run("snap.set", json!({"radius": 0})).is_err());
    assert!(
        app.run("snap.set", json!({"targets": {"x": true}}))
            .is_err()
    );
    assert!(app.run("snap.toggle", json!({"enabled": 3})).is_err());
    assert_eq!(app.transform_settings.snap.radius, 25.0);
    let reset = app.run("snap.reset", json!({})).unwrap();
    assert_eq!(reset["enabled"], false);
    assert_eq!(app.transform_settings.snap, Default::default());
    assert_eq!(steps(&app), before);
}

#[test]
fn snap_query_finds_the_target_by_priority() {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let cube = app.selection.active.unwrap();
    // default targets: vertices, edges, midpoints, polygon centres
    let r = app
        .run(
            "snap.query",
            json!({"point": [95.0, 98.0, 103.0], "radius": 20}),
        )
        .unwrap();
    assert_eq!(r["hit"]["kind"], "vertex");
    assert_eq!(r["hit"]["node"], cube.0);
    let p: Vec<f64> = serde_json::from_value(r["hit"]["position"].clone()).unwrap();
    assert_eq!(p, vec![100.0, 100.0, 100.0]);
    // no hit outside the radius
    let r = app
        .run(
            "snap.query",
            json!({"point": [500.0, 0.0, 0.0], "radius": 20}),
        )
        .unwrap();
    assert_eq!(r["hit"], Json::Null);
    // a polygon target
    app.run(
        "snap.set",
        json!({"targets": {"vertex": false, "edge": false, "edge_midpoint": false,
                           "polygon_center": false, "polygon": true}}),
    )
    .unwrap();
    let r = app
        .run(
            "snap.query",
            json!({"point": [30.0, 130.0, 40.0], "radius": 50}),
        )
        .unwrap();
    assert_eq!(r["hit"]["kind"], "polygon");
    let p: Vec<f64> = serde_json::from_value(r["hit"]["position"].clone()).unwrap();
    assert_eq!(p, vec![30.0, 100.0, 40.0]);
    // 2D: projected onto the workplane (y = 0)
    app.run("snap.set", json!({"mode": "2d"})).unwrap();
    let r = app
        .run(
            "snap.query",
            json!({"point": [30.0, 130.0, 40.0], "radius": 50}),
        )
        .unwrap();
    let p: Vec<f64> = serde_json::from_value(r["hit"]["position"].clone()).unwrap();
    assert_eq!(p, vec![30.0, 0.0, 40.0]);
    // restricting the nodes
    let r = app
        .run(
            "snap.query",
            json!({"point": [30.0, 130.0, 40.0], "radius": 50, "nodes": [9999]}),
        )
        .unwrap();
    assert_eq!(r["hit"], Json::Null);
    assert!(app.run("snap.query", json!({})).is_err());
    assert!(
        app.run("snap.query", json!({"point": [0, 0, 0], "radius": 0}))
            .is_err()
    );
}

#[test]
fn schemas_and_samples_are_valid() {
    let app = App::new_default();
    for id in COMMANDS {
        let h = app.commands.get(id).unwrap_or_else(|| panic!("{id}"));
        let schema = h.args_schema();
        schema::validate(&schema, &h.sample_args()).unwrap();
    }
    for id in ["object.set_axis", "object.center_axis", "object.align_axis"] {
        assert!(app.commands.get(id).unwrap().undoable(), "{id}");
    }
    for id in [
        "edit.axis_mode",
        "snap.set",
        "snap.toggle",
        "snap.reset",
        "snap.query",
    ] {
        assert!(!app.commands.get(id).unwrap().undoable(), "{id}");
    }
}
