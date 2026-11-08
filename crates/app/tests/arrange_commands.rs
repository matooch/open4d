//! object.duplicate, object.arrange, object.randomize, object.transfer and mesh.array_command
//! headless (docs/specs/MODELING.md §9): one undo step each, deep copies, determinism, argument
//! errors, schemas.

use forge_app::App;
use forge_command::{CommandError, schema};
use forge_core::{NodeId, Value};
use serde_json::{Value as Json, json};

fn create(app: &mut App, ty: &str) -> NodeId {
    app.run("object.create", json!({"type": ty})).unwrap();
    app.selection.active.unwrap()
}

fn pos_of(app: &App, id: NodeId) -> [f64; 3] {
    match app.doc.scene().param(id, "transform.pos") {
        Some(Value::Vec3(v)) => *v,
        _ => panic!("no position"),
    }
}

fn vec_of(app: &App, id: NodeId, key: &str) -> [f64; 3] {
    match app.doc.scene().param(id, key) {
        Some(Value::Vec3(v)) => *v,
        _ => panic!("no {key}"),
    }
}

fn set_pos(app: &mut App, id: NodeId, p: [f64; 3]) {
    app.run("transform.set", json!({"node": id.0, "pos": p}))
        .unwrap();
}

fn steps(app: &App) -> usize {
    app.doc.history().undo_labels().len()
}

fn name_of(app: &App, id: NodeId) -> String {
    app.doc.scene().node(id).unwrap().name.clone()
}

fn failed(r: Result<Json, CommandError>) -> bool {
    matches!(r, Err(CommandError::Failed(_)))
}

fn ids(v: &Json, key: &str) -> Vec<NodeId> {
    v[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| NodeId(i.as_u64().unwrap()))
        .collect()
}

#[test]
fn duplicate_places_copies_after_the_source() {
    let mut app = App::new_default();
    let a = create(&mut app, "forge.cube");
    let b = create(&mut app, "forge.sphere");
    set_pos(&mut app, a, [10.0, 0.0, 0.0]);
    let before = steps(&app);
    let out = app
        .run(
            "object.duplicate",
            json!({"nodes": [a.0], "copies": 2, "offset": [100.0, 0.0, 0.0],
                   "rotation_deg": [0.0, 0.0, 90.0], "scale": [2.0, 1.0, 1.0]}),
        )
        .unwrap();
    assert_eq!(steps(&app), before + 1, "one undo step");
    assert_eq!(out["count"], 2);
    let created = ids(&out, "created");
    let roots = app.doc.scene().roots().to_vec();
    assert_eq!(roots, vec![a, created[0], created[1], b]);
    // step mode: pos 10 + k * 100, bank k * 90 degrees, scale 1 + k * (2 - 1)
    assert_eq!(pos_of(&app, created[0]), [110.0, 0.0, 0.0]);
    assert_eq!(pos_of(&app, created[1]), [210.0, 0.0, 0.0]);
    let rot = |id| match app.doc.scene().param(id, "transform.rot") {
        Some(Value::Vec3(v)) => *v,
        _ => panic!(),
    };
    assert!((rot(created[1])[2] - std::f64::consts::PI).abs() < 1e-12);
    let scale = |id| match app.doc.scene().param(id, "transform.scale") {
        Some(Value::Vec3(v)) => *v,
        _ => panic!(),
    };
    assert_eq!(scale(created[1]), [3.0, 1.0, 1.0]);
    // the source stays; names are `<name>.N`; the copies are the selection
    assert_eq!(pos_of(&app, a), [10.0, 0.0, 0.0]);
    assert_eq!(name_of(&app, created[0]), format!("{}.1", name_of(&app, a)));
    assert_eq!(name_of(&app, created[1]), format!("{}.2", name_of(&app, a)));
    assert_eq!(app.selection.nodes.len(), 2);
    assert_eq!(app.selection.active, Some(created[1]));
    // undo removes both copies
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(app.doc.scene().roots(), &[a, b]);
}

#[test]
fn duplicate_total_mode_and_defaults() {
    let mut app = App::new_default();
    let a = create(&mut app, "forge.cube");
    let out = app
        .run(
            "object.duplicate",
            json!({"copies": 4, "mode": "total", "offset": [400.0, 0.0, 0.0]}),
        )
        .unwrap();
    let c = ids(&out, "created");
    assert_eq!(pos_of(&app, c[0]), [100.0, 0.0, 0.0]);
    assert_eq!(pos_of(&app, c[3]), [400.0, 0.0, 0.0]);
    // defaults: one copy, no offset
    app.run("object.duplicate", json!({"nodes": [a.0]}))
        .unwrap();
    assert_eq!(app.doc.scene().roots().len(), 6);
}

#[test]
fn duplicate_is_deep_and_remaps_links() {
    let mut app = App::new_default();
    let cube = create(&mut app, "forge.cube");
    let out = app
        .run("object.create_cloner", json!({"nodes": [cube.0]}))
        .unwrap();
    let cloner = NodeId(out["node"].as_u64().unwrap());
    let fx = create(&mut app, "forge.effector.random");
    app.run("object.move", json!({"nodes": [fx.0], "parent": cloner.0}))
        .unwrap();
    app.run(
        "param.set",
        json!({"node": cloner.0, "param": "effectors", "value": [fx.0]}),
    )
    .unwrap();
    let before_nodes = app.doc.scene().len();
    let out = app
        .run("object.duplicate", json!({"nodes": [cloner.0]}))
        .unwrap();
    let copy = ids(&out, "created")[0];
    let scene = app.doc.scene();
    let kids = scene.node(copy).unwrap().children.clone();
    assert_eq!(kids.len(), 2, "both children are copied");
    // the whole subtree with its tags is copied
    let size = |root: NodeId| {
        let all: Vec<NodeId> = std::iter::once(root)
            .chain(scene.descendants(root))
            .collect();
        all.len()
            + all
                .iter()
                .map(|i| scene.node(*i).unwrap().tags.len())
                .sum::<usize>()
    };
    assert_eq!(size(copy), size(cloner));
    assert_eq!(scene.len(), before_nodes + size(cloner));
    // the copy's effector list points at the copied effector, not at the original
    let new_fx = kids
        .iter()
        .copied()
        .find(|k| scene.node(*k).unwrap().type_id == "forge.effector.random")
        .unwrap();
    assert_ne!(new_fx, fx);
    assert_eq!(
        scene.param(copy, "effectors"),
        Some(&Value::LinkList(vec![new_fx]))
    );
    // the original still points at the original
    assert_eq!(
        scene.param(cloner, "effectors"),
        Some(&Value::LinkList(vec![fx]))
    );
    // tags are copied (the cube's smoothing tag)
    let tags = |id: NodeId| scene.node(id).unwrap().tags.len();
    let cube_copy = kids
        .iter()
        .copied()
        .find(|k| scene.node(*k).unwrap().type_id == "forge.cube")
        .unwrap();
    assert_eq!(tags(cube_copy), tags(cube));
}

#[test]
fn duplicate_errors_change_nothing() {
    let mut app = App::new_default();
    let before = steps(&app);
    assert!(failed(app.run("object.duplicate", json!({}))));
    assert!(failed(app.run("object.duplicate", json!({"nodes": [999]}))));
    let a = create(&mut app, "forge.cube");
    let n = app.doc.scene().len();
    let steps0 = steps(&app);
    assert!(matches!(
        app.run("object.duplicate", json!({"nodes": [a.0], "nope": 1})),
        Err(CommandError::InvalidArgs(_))
    ));
    assert!(matches!(
        app.run("object.duplicate", json!({"nodes": [a.0], "mode": "x"})),
        Err(CommandError::InvalidArgs(_))
    ));
    assert!(matches!(
        app.run(
            "object.duplicate",
            json!({"nodes": [a.0], "offset": [1, 2]})
        ),
        Err(CommandError::InvalidArgs(_))
    ));
    assert_eq!(app.doc.scene().len(), n);
    assert_eq!(steps(&app), steps0);
    let _ = before;
}

#[test]
fn arrange_line_grid_and_circle() {
    let mut app = App::new_default();
    let n: Vec<NodeId> = (0..4).map(|_| create(&mut app, "forge.cube")).collect();
    set_pos(&mut app, n[0], [5.0, 1.0, 2.0]);
    let list: Vec<u64> = n.iter().map(|i| i.0).collect();
    let before = steps(&app);
    let out = app
        .run(
            "object.arrange",
            json!({"nodes": list, "spacing": [10.0, 0.0, 0.0]}),
        )
        .unwrap();
    assert_eq!(out["arranged"], 4);
    assert_eq!(steps(&app), before + 1);
    assert_eq!(pos_of(&app, n[0]), [5.0, 1.0, 2.0]);
    assert_eq!(pos_of(&app, n[3]), [35.0, 1.0, 2.0]);
    app.run(
        "object.arrange",
        json!({"nodes": list, "mode": "grid", "columns": 2,
               "spacing": [10.0, 0.0, 0.0], "row_spacing": [0.0, 0.0, 20.0]}),
    )
    .unwrap();
    assert_eq!(pos_of(&app, n[3]), [15.0, 1.0, 22.0]);
    // circle: the centre is the mean, then every node sits `radius` away from it
    app.run(
        "object.arrange",
        json!({"nodes": list, "mode": "circle", "radius": 100.0}),
    )
    .unwrap();
    let p: Vec<[f64; 3]> = n.iter().map(|i| pos_of(&app, *i)).collect();
    let c = [
        p.iter().map(|v| v[0]).sum::<f64>() / 4.0,
        p.iter().map(|v| v[1]).sum::<f64>() / 4.0,
        p.iter().map(|v| v[2]).sum::<f64>() / 4.0,
    ];
    for v in &p {
        let d = ((v[0] - c[0]).powi(2) + (v[2] - c[2]).powi(2)).sqrt();
        assert!((d - 100.0).abs() < 1e-9);
        assert!((v[1] - c[1]).abs() < 1e-9);
    }
    // undo restores the grid
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(pos_of(&app, n[3]), [15.0, 1.0, 22.0]);
    // no selection
    app.selection.clear();
    assert!(failed(app.run("object.arrange", json!({}))));
}

#[test]
fn arrange_defaults_to_the_selection() {
    let mut app = App::new_default();
    let a = create(&mut app, "forge.cube");
    let b = create(&mut app, "forge.cube");
    app.selection.nodes = [a, b].into_iter().collect();
    app.run("object.arrange", json!({})).unwrap();
    assert_eq!(pos_of(&app, b), [200.0, 0.0, 0.0]);
}

#[test]
fn randomize_is_seeded_and_deterministic() {
    let run = |seed: i64| {
        let mut app = App::new_default();
        let n: Vec<NodeId> = (0..3).map(|_| create(&mut app, "forge.cube")).collect();
        let list: Vec<u64> = n.iter().map(|i| i.0).collect();
        let before = steps(&app);
        let out = app
            .run(
                "object.randomize",
                json!({"nodes": list, "seed": seed, "position": [100.0, 0.0, 100.0],
                       "rotation_deg": [10.0, 0.0, 0.0], "scale": [0.5, 0.5, 0.5],
                       "uniform_scale": true}),
            )
            .unwrap();
        assert_eq!(out["randomized"], 3);
        assert_eq!(steps(&app), before + 1);
        let scale = |id: NodeId| match app.doc.scene().param(id, "transform.scale") {
            Some(Value::Vec3(v)) => *v,
            _ => panic!(),
        };
        for id in &n {
            let (p, s) = (pos_of(&app, *id), scale(*id));
            assert!(p[0].abs() <= 100.0 && p[1] == 0.0 && p[2].abs() <= 100.0);
            assert!(s[0] == s[1] && s[1] == s[2] && (0.5..=1.5).contains(&s[0]));
        }
        (n.iter().map(|i| pos_of(&app, *i)).collect::<Vec<_>>(), app)
    };
    let (a, _) = run(5);
    let (b, _) = run(5);
    let (c, _) = run(6);
    assert_eq!(a, b, "same seed, same result");
    assert_ne!(a, c, "another seed, another result");
    assert_ne!(a[0], a[1], "nodes differ");
}

#[test]
fn randomize_does_not_depend_on_the_other_nodes() {
    let mut app = App::new_default();
    let a = create(&mut app, "forge.cube");
    let b = create(&mut app, "forge.cube");
    app.run("object.randomize", json!({"nodes": [a.0, b.0], "seed": 3}))
        .unwrap();
    let both = pos_of(&app, b);
    app.run("edit.undo", json!({})).unwrap();
    app.run("object.randomize", json!({"nodes": [b.0], "seed": 3}))
        .unwrap();
    assert_eq!(pos_of(&app, b), both);
    assert!(failed(app.run("object.randomize", json!({"nodes": []}))));
}

#[test]
fn transfer_world_and_local() {
    let mut app = App::new_default();
    let src = create(&mut app, "forge.cube");
    let dst = create(&mut app, "forge.cube");
    app.run(
        "transform.set",
        json!({"node": src.0, "pos": [10.0, 20.0, 30.0], "rot": [30.0, 0.0, 0.0],
               "scale": [2.0, 3.0, 4.0]}),
    )
    .unwrap();
    let before = steps(&app);
    let out = app
        .run("object.transfer", json!({"source": src.0, "target": dst.0}))
        .unwrap();
    assert_eq!(out["node"], dst.0);
    assert_eq!(steps(&app), before + 1);
    assert_eq!(pos_of(&app, dst), [10.0, 20.0, 30.0]);
    let near = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
    assert!(near(vec_of(&app, dst, "transform.scale"), [2.0, 3.0, 4.0]));
    assert!(near(
        vec_of(&app, dst, "transform.rot"),
        vec_of(&app, src, "transform.rot")
    ));
    // under a rotated parent the world matrices still agree
    let g = app.run("object.group", json!({"nodes": [dst.0]})).unwrap();
    let group = NodeId(g["node"].as_u64().unwrap());
    app.run(
        "transform.set",
        json!({"node": group.0, "pos": [100.0, 0.0, 0.0], "rot": [45.0, 0.0, 0.0]}),
    )
    .unwrap();
    app.run("object.transfer", json!({"source": src.0, "target": dst.0}))
        .unwrap();
    let ws = app.doc.scene().world_matrix(src).unwrap();
    let wd = app.doc.scene().world_matrix(dst).unwrap();
    assert!(ws.abs_diff_eq(wd, 1e-9), "{ws:?} vs {wd:?}");
    // only the ticked components, local space
    app.run(
        "transform.set",
        json!({"node": dst.0, "pos": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]}),
    )
    .unwrap();
    app.run(
        "object.transfer",
        json!({"source": src.0, "target": dst.0, "space": "local",
               "position": true, "rotation": false, "scale": false}),
    )
    .unwrap();
    assert_eq!(pos_of(&app, dst), [10.0, 20.0, 30.0]);
    assert!(near(vec_of(&app, dst, "transform.scale"), [1.0, 1.0, 1.0]));
}

#[test]
fn transfer_uses_the_active_node_and_checks_arguments() {
    let mut app = App::new_default();
    let src = create(&mut app, "forge.cube");
    let dst = create(&mut app, "forge.cube");
    set_pos(&mut app, src, [1.0, 2.0, 3.0]);
    app.selection.click(src);
    app.run("object.transfer", json!({"target": dst.0}))
        .unwrap();
    assert_eq!(pos_of(&app, dst), [1.0, 2.0, 3.0]);
    let n = steps(&app);
    assert!(matches!(
        app.run("object.transfer", json!({})),
        Err(CommandError::InvalidArgs(_))
    ));
    assert!(failed(app.run("object.transfer", json!({"target": src.0}))));
    assert!(failed(app.run("object.transfer", json!({"target": 999}))));
    assert!(failed(app.run(
        "object.transfer",
        json!({"target": dst.0, "source": 999})
    )));
    assert!(matches!(
        app.run("object.transfer", json!({"target": dst.0, "nodes": [1]})),
        Err(CommandError::InvalidArgs(_))
    ));
    assert_eq!(steps(&app), n);
}

fn editable_cube(app: &mut App) -> NodeId {
    create(app, "forge.cube");
    app.run("mesh.make_editable", json!({})).unwrap();
    app.selection.active.unwrap()
}

fn counts(app: &App, id: NodeId) -> (usize, usize) {
    match app.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => {
            let m = forge_generators::to_poly_mesh(m);
            (m.points.len(), m.polys.len())
        }
        _ => panic!("no mesh"),
    }
}

#[test]
fn array_command_copies_the_mesh() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    let out = app
        .run(
            "mesh.array_command",
            json!({"settings": {"count_x": 4, "offset": [900.0, 0.0, 0.0]}}),
        )
        .unwrap();
    assert_eq!(steps(&app), before + 1);
    assert_eq!(counts(&app, cube), (32, 24));
    assert_eq!(out["selected"], json!((6..24).collect::<Vec<u32>>()));
    // the third copy reaches x = 900 + 100
    let Some(Value::Mesh(m)) = app.doc.scene().param(cube, "mesh") else {
        panic!()
    };
    let mesh = forge_generators::to_poly_mesh(m);
    let max_x = mesh.points.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    assert!((max_x - 1000.0).abs() < 1e-9, "{max_x}");
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(counts(&app, cube), (8, 6));
    // not a polygon object
    create(&mut app, "forge.cube");
    assert!(failed(app.run("mesh.array_command", json!({}))));
    assert!(matches!(
        app.run("mesh.array_command", json!({"settings": {"count_x": "x"}})),
        Err(CommandError::InvalidArgs(_))
    ));
}

#[test]
fn schemas_and_samples_are_valid() {
    let app = App::new_default();
    for id in [
        "object.duplicate",
        "object.arrange",
        "object.randomize",
        "object.transfer",
        "mesh.array_command",
    ] {
        let h = app.commands.get(id).unwrap_or_else(|| panic!("{id}"));
        let s = h.args_schema();
        schema::validate(&s, &h.sample_args()).unwrap();
        assert!(schema::validate(&s, &json!({"nope": 1})).is_err(), "{id}");
        assert!(h.undoable());
    }
    let dup = app.commands.get("object.duplicate").unwrap().args_schema();
    assert_eq!(dup["properties"]["copies"]["default"], 1);
    assert_eq!(dup["properties"]["mode"]["enum"], json!(["step", "total"]));
}
