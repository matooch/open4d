//! Component editing across several selected polygon objects, headless
//! (docs/specs/EDITABLE.md §6, MODELING.md §0.1): selection commands, `mesh.transform`
//! and the modeling commands work on every target in one undo step; each object keeps
//! its own topology; soft selection does not cross objects.

use forge_app::App;
use forge_app::select::{EditMode, selection_of, targets};
use forge_command::{CommandError, schema};
use forge_core::{NodeId, Value};
use glam::DVec3;
use serde_json::{Value as Json, json};

/// An editable default cube (200 wide) at `x`.
fn cube_at(app: &mut App, x: f64) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    let id = app.selection.active.unwrap();
    app.run("transform.set", json!({"node": id.0, "pos": [x, 0.0, 0.0]}))
        .unwrap();
    id
}

/// Two cubes (A at the origin, B at x = 500), both selected, A < B, B active, in `mode`.
fn two(mode: &str) -> (App, NodeId, NodeId) {
    let mut app = App::new_default();
    let a = cube_at(&mut app, 0.0);
    let b = cube_at(&mut app, 500.0);
    app.selection.nodes = [a, b].into_iter().collect();
    app.selection.active = Some(b);
    app.run("select.mode", json!({"mode": mode})).unwrap();
    (app, a, b)
}

fn mesh_points(app: &App, id: NodeId) -> Vec<DVec3> {
    match app.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => m.points.iter().map(|p| DVec3::from_array(*p)).collect(),
        _ => panic!("no mesh"),
    }
}

fn counts(app: &App, id: NodeId) -> (usize, usize) {
    match app.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => (m.points.len(), m.polys.len()),
        _ => panic!("no mesh"),
    }
}

fn world_points(app: &App, id: NodeId) -> Vec<DVec3> {
    let w = app.doc.scene().world_matrix(id).unwrap();
    mesh_points(app, id)
        .into_iter()
        .map(|p| w.transform_point3(p))
        .collect()
}

fn sel(app: &App, id: NodeId, mode: EditMode) -> Vec<u32> {
    selection_of(app.doc.scene(), id, mode).iter().collect()
}

fn set_sel(app: &mut App, id: NodeId, param: &str, items: &[u32]) {
    app.doc
        .transact("Start", |tx| {
            tx.set_param(id, param, Value::IndexList(items.to_vec()))
        })
        .unwrap();
}

fn steps(app: &App) -> usize {
    app.doc.history().undo_labels().len()
}

fn close(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 1e-9, "{a} != {b}");
}

fn failed(r: &Result<Json, CommandError>) -> bool {
    matches!(r, Err(CommandError::Failed(_)))
}

fn invalid(r: &Result<Json, CommandError>) -> bool {
    matches!(r, Err(CommandError::InvalidArgs(_)))
}

#[test]
fn targets_are_the_selected_polygon_objects_while_the_active_node_is_one() {
    let (mut app, a, b) = two("points");
    assert_eq!(targets(&app), vec![a, b]);
    // another type among the selection is not a target
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let plain = app.selection.active.unwrap();
    app.selection.nodes.extend([a, b]);
    app.selection.active = Some(b);
    assert_eq!(targets(&app), vec![a, b]);
    // an active node that is not a polygon object: no targets
    app.selection.active = Some(plain);
    assert!(targets(&app).is_empty());
    // Model mode: none
    app.selection.active = Some(b);
    app.run("select.mode", json!({"mode": "model"})).unwrap();
    assert!(targets(&app).is_empty());
}

#[test]
fn select_all_invert_and_none_work_on_every_target_in_one_step() {
    let (mut app, a, b) = two("polys");
    let before = steps(&app);
    app.run("select.all", json!({})).unwrap();
    assert_eq!(steps(&app), before + 1, "two objects, one step");
    assert_eq!(sel(&app, a, EditMode::Polys).len(), 6);
    assert_eq!(sel(&app, b, EditMode::Polys).len(), 6);

    app.run("select.none", json!({"nodes": [a.0]})).unwrap();
    assert!(sel(&app, a, EditMode::Polys).is_empty());
    assert_eq!(
        sel(&app, b, EditMode::Polys).len(),
        6,
        "only `nodes` changed"
    );

    app.run("select.invert", json!({})).unwrap();
    assert_eq!(sel(&app, a, EditMode::Polys).len(), 6);
    assert!(sel(&app, b, EditMode::Polys).is_empty());

    // one undo reverts both objects of one command
    app.run("select.all", json!({})).unwrap();
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(sel(&app, a, EditMode::Polys).len(), 6);
    assert!(sel(&app, b, EditMode::Polys).is_empty());

    // no change in any object writes no step
    app.run("select.none", json!({})).unwrap();
    let n = steps(&app);
    app.run("select.none", json!({})).unwrap();
    assert_eq!(steps(&app), n);
}

#[test]
fn grow_convert_and_loop_use_each_objects_own_topology() {
    let (mut app, a, b) = two("points");
    set_sel(&mut app, a, "selection.points", &[0]);
    set_sel(&mut app, b, "selection.points", &[0, 1]);
    app.run("select.grow", json!({})).unwrap();
    let (ga, gb) = (
        sel(&app, a, EditMode::Points),
        sel(&app, b, EditMode::Points),
    );
    assert_eq!(ga.len(), 4, "a point of a cube plus its three neighbours");
    assert!(gb.len() > ga.len(), "B grew from two points");
    // convert: the polygons of each object come from its own points
    let before = steps(&app);
    app.run("select.convert", json!({"to": "polys", "mode": "tolerant"}))
        .unwrap();
    assert_eq!(steps(&app), before + 1);
    assert_eq!(app.mode, EditMode::Polys);
    assert_eq!(
        sel(&app, a, EditMode::Polys).len(),
        6,
        "all touch the grown set"
    );
    assert!(!sel(&app, b, EditMode::Polys).is_empty());
    // the source stays selected in both
    assert_eq!(sel(&app, a, EditMode::Points), ga);
    assert_eq!(sel(&app, b, EditMode::Points), gb);

    // an error in one object writes nothing: Loop on points
    app.run("select.mode", json!({"mode": "points"})).unwrap();
    let n = steps(&app);
    let r = app.run("select.loop", json!({}));
    assert!(failed(&r));
    assert_eq!(steps(&app), n);
}

#[test]
fn nothing_crosses_between_objects() {
    let (mut app, a, b) = two("points");
    set_sel(&mut app, a, "selection.points", &[0]);
    for cmd in ["select.grow", "select.connected"] {
        app.run(cmd, json!({})).unwrap();
        assert!(sel(&app, b, EditMode::Points).is_empty(), "{cmd}");
    }
    assert_eq!(
        sel(&app, a, EditMode::Points).len(),
        8,
        "connected = the whole cube"
    );
}

#[test]
fn explicit_nodes_must_be_polygon_objects_and_need_a_component_mode() {
    let (mut app, a, _b) = two("polys");
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let plain = app.selection.active.unwrap();
    let n = steps(&app);
    let r = app.run("select.all", json!({"nodes": [a.0, plain.0]}));
    assert!(failed(&r));
    assert!(sel(&app, a, EditMode::Polys).is_empty(), "nothing written");
    assert!(failed(&app.run("select.all", json!({"nodes": []}))));
    assert_eq!(steps(&app), n);
    app.run("select.mode", json!({"mode": "model"})).unwrap();
    assert!(failed(&app.run("select.all", json!({"nodes": [a.0]}))));
}

#[test]
fn a_move_of_points_in_two_objects_is_one_undo_step() {
    let (mut app, a, b) = two("points");
    set_sel(&mut app, a, "selection.points", &[0]);
    set_sel(&mut app, b, "selection.points", &[0, 1]);
    let (wa, wb) = (world_points(&app, a), world_points(&app, b));
    let before = steps(&app);
    let r = app
        .run("mesh.transform", json!({"move": [0.0, 10.0, 0.0]}))
        .unwrap();
    assert_eq!(steps(&app), before + 1, "one step for both objects");
    assert_eq!(r["moved"], json!(3));
    assert_eq!(r["nodes"].as_array().unwrap().len(), 2);
    for (name, id, old, moved) in [("a", a, &wa, vec![0]), ("b", b, &wb, vec![0, 1])] {
        let now = world_points(&app, id);
        for (i, (n, o)) in now.iter().zip(old).enumerate() {
            let want = if moved.contains(&i) {
                *o + DVec3::Y * 10.0
            } else {
                *o
            };
            close(*n, want);
            let _ = name;
        }
    }
    assert_eq!(*app.doc.history().undo_labels().last().unwrap(), "Move");
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(world_points(&app, a), wa);
    assert_eq!(world_points(&app, b), wb);
    app.run("edit.redo", json!({})).unwrap();
    close(world_points(&app, a)[0], wa[0] + DVec3::Y * 10.0);
}

#[test]
fn a_world_rotation_turns_both_objects_about_the_common_centroid() {
    let (mut app, a, b) = two("points");
    app.run("select.all", json!({})).unwrap();
    let (wa, wb) = (world_points(&app, a), world_points(&app, b));
    // the common centroid is (250, 0, 0): A's centre (0,0,0) goes round it
    app.run(
        "mesh.transform",
        json!({"rotate": {"axis": [0.0, 1.0, 0.0], "angle": 90.0}}),
    )
    .unwrap();
    let centre = |pts: &[DVec3]| pts.iter().copied().sum::<DVec3>() / pts.len() as f64;
    close(
        centre(&world_points(&app, a)),
        DVec3::new(250.0, 0.0, 250.0),
    );
    close(
        centre(&world_points(&app, b)),
        DVec3::new(250.0, 0.0, -250.0),
    );
    // a rigid turn: sizes are kept
    let width = |pts: &[DVec3]| (pts[0] - pts[1]).length();
    assert!((width(&world_points(&app, a)) - width(&wa)).abs() < 1e-9);
    assert!((width(&world_points(&app, b)) - width(&wb)).abs() < 1e-9);

    // space "object": the same arguments about each object's own centroid
    app.run("edit.undo", json!({})).unwrap();
    app.run(
        "mesh.transform",
        json!({"space": "object", "rotate": {"axis": [0.0, 1.0, 0.0], "angle": 90.0}}),
    )
    .unwrap();
    close(centre(&world_points(&app, a)), DVec3::ZERO);
    close(centre(&world_points(&app, b)), DVec3::new(500.0, 0.0, 0.0));
}

#[test]
fn transform_argument_checks_change_nothing() {
    let (mut app, a, b) = two("points");
    set_sel(&mut app, a, "selection.points", &[0]);
    let n = steps(&app);
    // points name one object's indices
    let r = app.run(
        "mesh.transform",
        json!({"points": [0], "move": [1.0, 0.0, 0.0]}),
    );
    assert!(invalid(&r));
    // node and nodes together
    let r = app.run(
        "mesh.transform",
        json!({"node": a.0, "nodes": [a.0, b.0], "move": [1.0, 0.0, 0.0]}),
    );
    assert!(invalid(&r));
    let r = app.run(
        "mesh.transform",
        json!({"space": "moon", "move": [1.0, 0.0, 0.0]}),
    );
    assert!(invalid(&r));
    // B has no points selected, A has: only A moves (B is skipped)
    let r = app
        .run("mesh.transform", json!({"move": [1.0, 0.0, 0.0]}))
        .unwrap();
    assert_eq!(r["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(steps(&app), n + 1);
    // nothing selected anywhere
    app.run("select.none", json!({})).unwrap();
    let n = steps(&app);
    let r = app.run("mesh.transform", json!({"move": [1.0, 0.0, 0.0]}));
    assert!(failed(&r));
    assert_eq!(steps(&app), n);
}

#[test]
fn soft_selection_does_not_cross_objects() {
    let (mut app, a, b) = two("points");
    // radius 1000 reaches B (500 away) in world space; the falloff must stay inside A
    app.run(
        "edit.soft_selection",
        json!({"enabled": true, "radius": 1000.0, "falloff": "linear"}),
    )
    .unwrap();
    set_sel(&mut app, a, "selection.points", &[0]);
    let (b_before, a_before) = (mesh_points(&app, b), mesh_points(&app, a));
    app.run("mesh.transform", json!({"move": [0.0, 10.0, 0.0]}))
        .unwrap();
    assert_eq!(
        mesh_points(&app, b),
        b_before,
        "B has no selection and stays put"
    );
    let a_after = mesh_points(&app, a);
    assert!(
        (a_after[0] - a_before[0]).y > 9.999,
        "the selected point moves fully"
    );
    let others = (1..8).filter(|i| a_after[*i] != a_before[*i]).count();
    assert!(others > 0, "A's own neighbours follow the soft falloff");

    // with a selection in both objects each falloff uses its own selection
    set_sel(&mut app, b, "selection.points", &[7]);
    let a_mid = mesh_points(&app, a);
    app.run("mesh.transform", json!({"move": [0.0, 10.0, 0.0]}))
        .unwrap();
    // A's point 0 is not selected any more? It still is: it moved again
    assert!((mesh_points(&app, a)[0] - a_mid[0]).y > 9.999);
    assert!((mesh_points(&app, b)[7] - b_before[7]).y > 9.999);
}

#[test]
fn extrude_runs_on_every_object_with_its_own_selection_in_one_step() {
    let (mut app, a, b) = two("polys");
    set_sel(&mut app, a, "selection.polys", &[0]);
    set_sel(&mut app, b, "selection.polys", &[1, 2]);
    let (ca, cb) = (counts(&app, a), counts(&app, b));
    let before = steps(&app);
    let r = app
        .run("mesh.extrude", json!({"settings": {"offset": 20.0}}))
        .unwrap();
    assert_eq!(steps(&app), before + 1);
    assert_eq!(*app.doc.history().undo_labels().last().unwrap(), "Extrude");
    assert!(counts(&app, a).0 > ca.0 && counts(&app, b).0 > cb.0);
    // each keeps its own topology: different counts
    assert_ne!(counts(&app, a), counts(&app, b));
    assert_eq!(r["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(r["skipped"], json!([]));
    // the extruded polygons are selected per object
    assert!(!sel(&app, a, EditMode::Polys).is_empty());
    assert!(!sel(&app, b, EditMode::Polys).is_empty());
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!((counts(&app, a), counts(&app, b)), (ca, cb));
    assert_eq!(sel(&app, a, EditMode::Polys), vec![0]);
}

#[test]
fn an_object_without_a_usable_selection_is_skipped_when_others_work() {
    let (mut app, a, b) = two("polys");
    set_sel(&mut app, a, "selection.polys", &[0]);
    let cb = counts(&app, b);
    let r = app
        .run("mesh.extrude", json!({"settings": {"offset": 20.0}}))
        .unwrap();
    assert_eq!(r["skipped"], json!([b.0]));
    assert_eq!(r["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(counts(&app, b), cb);
    assert!(counts(&app, a).0 > 8);
    // nobody has a selection: the error stays
    app.run("select.none", json!({})).unwrap();
    let n = steps(&app);
    assert!(failed(&app.run("mesh.extrude", json!({}))));
    assert_eq!(steps(&app), n);
}

#[test]
fn bevel_and_subdivide_run_on_two_objects_in_one_step() {
    let (mut app, a, b) = two("edges");
    set_sel(&mut app, a, "selection.edges", &[0]);
    set_sel(&mut app, b, "selection.edges", &[0, 1]);
    let (ca, cb) = (counts(&app, a), counts(&app, b));
    let before = steps(&app);
    app.run("mesh.bevel", json!({"settings": {"offset": 10.0}}))
        .unwrap();
    assert_eq!(steps(&app), before + 1);
    assert!(counts(&app, a).1 > ca.1 && counts(&app, b).1 > cb.1);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!((counts(&app, a), counts(&app, b)), (ca, cb));

    // Subdivide works on the whole mesh without a selection
    let before = steps(&app);
    app.run("mesh.subdivide", json!({"nodes": [a.0, b.0]}))
        .unwrap();
    assert_eq!(steps(&app), before + 1);
    assert!(counts(&app, a).1 > ca.1 && counts(&app, b).1 > cb.1);
    assert_eq!(
        counts(&app, a),
        counts(&app, b),
        "same input, same topology"
    );
}

#[test]
fn split_creates_one_object_per_source_and_selects_them() {
    let (mut app, a, b) = two("polys");
    set_sel(&mut app, a, "selection.polys", &[0]);
    set_sel(&mut app, b, "selection.polys", &[0]);
    let nodes = app.doc.scene().roots().len();
    let before = steps(&app);
    app.run("mesh.split", json!({})).unwrap();
    assert_eq!(steps(&app), before + 1);
    assert_eq!(app.doc.scene().roots().len(), nodes + 2);
    assert_eq!(app.selection.nodes.len(), 2);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(app.doc.scene().roots().len(), nodes);
}

#[test]
fn argument_rules_of_the_multi_object_tools() {
    let (mut app, a, b) = two("polys");
    set_sel(&mut app, a, "selection.polys", &[0]);
    // indices name one object's components
    let r = app.run("mesh.extrude", json!({"polys": [0]}));
    assert!(invalid(&r));
    let r = app.run("mesh.extrude", json!({"node": a.0, "nodes": [a.0, b.0]}));
    assert!(invalid(&r));
    // with a single node they still work
    app.run("mesh.extrude", json!({"node": a.0, "polys": [1]}))
        .unwrap();
    // the single-object tools say so
    for (cmd, args) in [
        ("mesh.loop_cut", json!({"nodes": [a.0, b.0], "edge": 0})),
        (
            "mesh.knife",
            json!({"nodes": [a.0], "mode": "loop", "edge": 0}),
        ),
        ("mesh.bridge", json!({"nodes": [a.0, b.0]})),
        ("mesh.close_hole", json!({"nodes": [a.0, b.0], "edge": 0})),
        ("mesh.stitch", json!({"nodes": [a.0, b.0]})),
        ("mesh.pen", json!({"nodes": [a.0, b.0]})),
    ] {
        match app.run(cmd, args) {
            Err(CommandError::InvalidArgs(m)) => {
                assert!(m.contains("one object"), "{cmd}: {m}");
            }
            other => panic!("{cmd}: {other:?}"),
        }
    }
}

#[test]
fn the_schemas_list_nodes() {
    let app = App::new_default();
    for cmd in [
        "mesh.extrude",
        "mesh.bevel",
        "mesh.subdivide",
        "mesh.transform",
        "select.all",
        "select.convert",
        "select.loop",
    ] {
        let h = app.commands.get(cmd).expect(cmd);
        let s = h.args_schema();
        assert!(
            s.to_string().contains("\"nodes\""),
            "{cmd} documents nodes: {s}"
        );
        let mut args = h.sample_args();
        args["nodes"] = json!([1, 2]);
        schema::validate(&s, &args).unwrap_or_else(|e| panic!("{cmd}: {e}"));
    }
}
