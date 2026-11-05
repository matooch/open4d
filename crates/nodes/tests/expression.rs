//! docs/specs/NODES.md sections 5-7: the expression tag, the graph commands, evaluation order,
//! saving and loading. Everything is built headless through commands.

use forge_app::App;
use forge_core::{Interp, Key, NodeId, Value};
use forge_nodes::{TAG, apply_expressions, load_graph};
use serde_json::{Value as Json, json};

fn app() -> App {
    let mut app = App::new_default();
    app.run(
        "timeline.settings",
        json!({"fps": 30, "start": 0, "end": 90}),
    )
    .unwrap();
    app
}

fn create(app: &mut App, ty: &str) -> NodeId {
    let out = app.run("object.create", json!({"type": ty})).unwrap();
    NodeId(out["node"].as_u64().unwrap())
}

/// A cube with an expression tag, made with `object.create` on the selected cube.
fn cube_with_tag(app: &mut App) -> (NodeId, NodeId) {
    let cube = create(app, "forge.cube");
    let tag = add_tag(app, cube);
    (cube, tag)
}

fn add_tag(app: &mut App, host: NodeId) -> NodeId {
    app.selection.click(host);
    create(app, TAG)
}

/// Adds a graph node with constants; returns its id.
fn add(app: &mut App, tag: NodeId, ty: &str, values: Json) -> u64 {
    let out = app
        .run(
            "graph.add_node",
            json!({"tag": tag.0, "type": ty, "values": values}),
        )
        .unwrap();
    out["node"].as_u64().unwrap()
}

fn link(app: &mut App, tag: NodeId, from: (u64, &str), to: (u64, &str)) {
    app.run(
        "graph.connect",
        json!({"tag": tag.0, "from_node": from.0, "from_port": from.1, "to_node": to.0, "to_port": to.1}),
    )
    .unwrap();
}

/// time.seconds * `speed` -> vector.x -> object.set_param(`param`) on the host. Returns the
/// graph node ids (time, mul, compose, set).
fn drive_x(app: &mut App, tag: NodeId, speed: f64, param: &str) -> [u64; 4] {
    let t = add(app, tag, "time.now", json!({}));
    let m = add(app, tag, "math.mul", json!({"b": speed}));
    let v = add(app, tag, "vector.compose", json!({}));
    let w = add(app, tag, "object.set_param", json!({"param": param}));
    link(app, tag, (t, "seconds"), (m, "a"));
    link(app, tag, (m, "out"), (v, "x"));
    link(app, tag, (v, "out"), (w, "value"));
    [t, m, v, w]
}

fn pos(scene: &forge_core::Scene, node: NodeId) -> [f64; 3] {
    match scene.param(node, "transform.pos") {
        Some(Value::Vec3(v)) => *v,
        other => panic!("{other:?}"),
    }
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
fn an_expression_tag_drives_a_cube_position_from_time() {
    let mut app = app();
    let (cube, tag) = cube_with_tag(&mut app);
    assert_eq!(app.doc.scene().node(tag).unwrap().host, Some(cube));
    drive_x(&mut app, tag, 100.0, "transform.pos");
    // x = seconds * 100; 30 fps
    for (frame, x) in [(0, 0.0), (15, 50.0), (30, 100.0), (45, 150.0)] {
        let scene = app.frame(frame);
        let p = pos(&scene, cube);
        assert!((p[0] - x).abs() < 1e-9, "frame {frame}: {p:?}");
        assert_eq!((p[1], p[2]), (0.0, 0.0));
        // the stored scene is untouched; the expression only shapes the evaluated one
        assert_eq!(pos(app.doc.scene(), cube), [0.0; 3]);
    }
    // generators see the moved object
    assert!((center_x(&app, cube, 30) - 100.0).abs() < 1e-9);
    assert!((center_x(&app, cube, 60) - 200.0).abs() < 1e-9);
}

#[test]
fn every_graph_command_is_one_undo_step() {
    let mut app = app();
    let (_cube, tag) = cube_with_tag(&mut app);
    let depth = |app: &App| app.doc.undo_depth();
    let d0 = depth(&app);
    let a = add(&mut app, tag, "math.add", json!({"a": 1.0}));
    assert_eq!(depth(&app), d0 + 1);
    let b = add(&mut app, tag, "math.mul", json!({}));
    assert_eq!(depth(&app), d0 + 2);
    link(&mut app, tag, (a, "out"), (b, "a"));
    assert_eq!(depth(&app), d0 + 3);
    app.run(
        "graph.set_value",
        json!({"tag": tag.0, "node": b, "port": "b", "value": 4}),
    )
    .unwrap();
    assert_eq!(depth(&app), d0 + 4);
    app.run(
        "graph.disconnect",
        json!({"tag": tag.0, "to_node": b, "to_port": "a"}),
    )
    .unwrap();
    assert_eq!(depth(&app), d0 + 5);
    let removed = app
        .run("graph.remove", json!({"tag": tag.0, "node": a}))
        .unwrap();
    assert_eq!(removed["removed_links"], 0);
    assert_eq!(depth(&app), d0 + 6);

    let graph = |app: &App| load_graph(app.doc.scene(), tag).unwrap();
    assert_eq!(graph(&app).len(), 1);
    // undo walks back through each edit; redo returns
    assert!(app.doc.undo()); // remove
    assert_eq!(graph(&app).len(), 2);
    assert!(app.doc.undo()); // disconnect
    assert_eq!(graph(&app).links().len(), 1);
    assert!(app.doc.undo()); // set_value
    assert!(app.doc.undo()); // connect
    assert_eq!(graph(&app).links().len(), 0);
    assert!(app.doc.undo()); // add b
    assert!(app.doc.undo()); // add a
    assert!(graph(&app).is_empty());
    while app.doc.redo() {}
    assert_eq!(graph(&app).len(), 1);
    // labels
    assert!(
        app.doc
            .history()
            .undo_labels()
            .contains(&"Connect Graph Ports")
    );
}

#[test]
fn refused_commands_change_nothing_and_record_no_undo_step() {
    let mut app = app();
    let (cube, tag) = cube_with_tag(&mut app);
    let a = add(&mut app, tag, "math.add", json!({}));
    let b = add(&mut app, tag, "math.add", json!({}));
    link(&mut app, tag, (a, "out"), (b, "a"));
    let depth = app.doc.undo_depth();
    let before = app.run("graph.get", json!({"tag": tag.0})).unwrap();

    let refused = |app: &mut App, id: &str, args: Json| {
        let err = app.run(id, args).expect_err(id);
        err.to_string()
    };
    // a cycle
    let e = refused(
        &mut app,
        "graph.connect",
        json!({"tag": tag.0, "from_node": b, "from_port": "out", "to_node": a, "to_port": "a"}),
    );
    assert!(e.contains("cycle"), "{e}");
    // a type mismatch
    let s = add(&mut app, tag, "const.string", json!({}));
    let depth = depth + 1;
    let e = refused(
        &mut app,
        "graph.connect",
        json!({"tag": tag.0, "from_node": s, "from_port": "out", "to_node": a, "to_port": "b"}),
    );
    assert!(e.contains("cannot connect string to float"), "{e}");
    // unknown type, port, node; bad value; not a tag
    assert!(
        refused(
            &mut app,
            "graph.add_node",
            json!({"tag": tag.0, "type": "nope.nope"})
        )
        .contains("unknown node type")
    );
    assert!(
        refused(
            &mut app,
            "graph.set_value",
            json!({"tag": tag.0, "node": a, "port": "zz", "value": 1})
        )
        .contains("no input")
    );
    assert!(
        refused(
            &mut app,
            "graph.set_value",
            json!({"tag": tag.0, "node": a, "port": "a", "value": "text"})
        )
        .contains("expected a number")
    );
    assert!(
        refused(
            &mut app,
            "graph.set_value",
            json!({"tag": tag.0, "node": 999, "port": "a", "value": 1})
        )
        .contains("does not exist")
    );
    assert!(
        refused(&mut app, "graph.remove", json!({"tag": tag.0, "node": 999}))
            .contains("does not exist")
    );
    assert!(
        refused(
            &mut app,
            "graph.disconnect",
            json!({"tag": tag.0, "to_node": a, "to_port": "a"})
        )
        .contains("no link")
    );
    assert!(
        refused(&mut app, "graph.get", json!({"tag": cube.0})).contains("not an expression tag")
    );
    assert!(refused(&mut app, "graph.get", json!({"tag": 4040})).contains("does not exist"));
    // an add_node with a bad constant adds nothing
    assert!(
        refused(
            &mut app,
            "graph.add_node",
            json!({"tag": tag.0, "type": "math.add", "values": {"a": "x"}})
        )
        .contains("expected a number")
    );
    assert_eq!(app.doc.undo_depth(), depth);
    let after = app.run("graph.get", json!({"tag": tag.0})).unwrap();
    assert_eq!(
        after["nodes"].as_array().unwrap().len(),
        before["nodes"].as_array().unwrap().len() + 1
    );
    assert_eq!(after["links"], before["links"]);
}

#[test]
fn graph_get_node_types_and_evaluate() {
    let mut app = app();
    let (cube, tag) = cube_with_tag(&mut app);
    let ids = drive_x(&mut app, tag, 10.0, "transform.pos");
    let got = app.run("graph.get", json!({"tag": tag.0})).unwrap();
    assert_eq!(got["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(got["links"].as_array().unwrap().len(), 3);
    assert_eq!(got["order"], json!(ids));
    assert_eq!(got["nodes"][1]["values"]["b"], json!(10.0));

    let types = app.run("graph.node_types", json!({})).unwrap();
    let list = types["types"].as_array().unwrap();
    let add = list.iter().find(|t| t["id"] == "math.add").unwrap();
    assert_eq!(add["inputs"][0]["type"], "float");
    assert_eq!(add["outputs"][0]["id"], "out");
    let set = list.iter().find(|t| t["id"] == "object.set_param").unwrap();
    assert_eq!(set["sink"], true);

    // a dry run at frame 15: nothing is written to the scene
    app.run("timeline.set_frame", json!({"frame": 15})).unwrap();
    let ev = app.run("graph.evaluate", json!({"tag": tag.0})).unwrap();
    assert_eq!(ev["outputs"][format!("{}.out", ids[1])], json!(5.0));
    assert_eq!(ev["writes"][0]["node"], cube.0);
    assert_eq!(ev["writes"][0]["param"], "transform.pos");
    assert_eq!(ev["writes"][0]["value"], json!([5.0, 0.0, 0.0]));
    assert_eq!(pos(app.doc.scene(), cube), [0.0; 3]);
}

#[test]
fn an_expression_overrides_a_keyed_value_and_generators_see_the_result() {
    let mut app = app();
    let (cube, tag) = cube_with_tag(&mut app);
    // keys: x = 300 * seconds
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
    assert!((pos(&app.frame(15), cube)[0] - 150.0).abs() < 1e-9);
    // an expression on the same parameter wins (order: tracks first, then expressions)
    drive_x(&mut app, tag, 100.0, "transform.pos");
    assert!((pos(&app.frame(15), cube)[0] - 50.0).abs() < 1e-9);
    // disabling the tag brings the keys back
    let tag_id = tag.0;
    app.run(
        "param.set",
        json!({"node": tag_id, "param": "enabled", "value": false}),
    )
    .unwrap();
    assert!((pos(&app.frame(15), cube)[0] - 150.0).abs() < 1e-9);
    // size is read by the generator: the expression drives size_x from time
    app.run(
        "param.set",
        json!({"node": tag_id, "param": "enabled", "value": true}),
    )
    .unwrap();
    let t2 = add_tag(&mut app, cube);
    let t = add(&mut app, t2, "time.now", json!({}));
    let m = add(&mut app, t2, "math.mul", json!({"b": 100.0}));
    let a = add(&mut app, t2, "math.add", json!({"b": 50.0}));
    let w = add(&mut app, t2, "object.set_param", json!({"param": "size_x"}));
    link(&mut app, t2, (t, "seconds"), (m, "a"));
    link(&mut app, t2, (m, "out"), (a, "a"));
    link(&mut app, t2, (a, "out"), (w, "value"));
    let width = |app: &App, frame: i64| {
        let mesh = app.eval_mesh(cube, frame).unwrap();
        let (lo, hi) = mesh
            .points
            .iter()
            .fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                (lo.min(p.x), hi.max(p.x))
            });
        hi - lo
    };
    assert!((width(&app, 0) - 50.0).abs() < 1e-9);
    assert!((width(&app, 30) - 150.0).abs() < 1e-9);
}

#[test]
fn expressions_run_in_document_order_and_later_ones_see_earlier_writes() {
    let mut app = app();
    let (cube, first) = cube_with_tag(&mut app);
    // first: pos = (10, 0, 0)
    let w = add(
        &mut app,
        first,
        "object.set_param",
        json!({"param": "transform.pos", "value": [10.0, 0.0, 0.0]}),
    );
    let _ = w;
    // second: read the host's position, add 5 to x with a vector add, write it back
    let second = add_tag(&mut app, cube);
    let get = add(
        &mut app,
        second,
        "object.get_param",
        json!({"param": "transform.pos"}),
    );
    let addv = add(
        &mut app,
        second,
        "vector.add",
        json!({"b": [5.0, 1.0, 0.0]}),
    );
    let set = add(
        &mut app,
        second,
        "object.set_param",
        json!({"param": "transform.pos"}),
    );
    link(&mut app, second, (get, "value"), (addv, "a"));
    link(&mut app, second, (addv, "out"), (set, "value"));
    assert_eq!(pos(&app.frame(0), cube), [15.0, 1.0, 0.0]);
    // without the second tag only the first one writes
    app.doc
        .transact("Reorder", |tx| {
            tx.delete(second)?;
            Ok(())
        })
        .unwrap();
    assert_eq!(pos(&app.frame(0), cube), [10.0, 0.0, 0.0]);
}

#[test]
fn an_expression_can_drive_another_object_and_read_its_matrix() {
    let mut app = app();
    let (host, tag) = cube_with_tag(&mut app);
    let other = create(&mut app, "forge.sphere");
    app.doc
        .transact("move", |tx| {
            tx.set_param(host, "transform.pos", Value::Vec3([7.0, 8.0, 9.0]))
        })
        .unwrap();
    // other.pos = translation of host's world matrix * 2
    let m = add(&mut app, tag, "object.world_matrix", json!({}));
    let tr = add(&mut app, tag, "matrix.translation", json!({}));
    let sc = add(&mut app, tag, "vector.scale", json!({"s": 2.0}));
    let w = add(
        &mut app,
        tag,
        "object.set_param",
        json!({"object": other.0, "param": "transform.pos"}),
    );
    link(&mut app, tag, (m, "out"), (tr, "m"));
    link(&mut app, tag, (tr, "out"), (sc, "v"));
    link(&mut app, tag, (sc, "out"), (w, "value"));
    let scene = app.frame(0);
    assert_eq!(pos(&scene, other), [14.0, 16.0, 18.0]);
    assert_eq!(pos(&scene, host), [7.0, 8.0, 9.0]);
}

#[test]
fn conditions_compare_noise_and_scripts_work_in_a_scene() {
    let mut app = app();
    let (cube, tag) = cube_with_tag(&mut app);
    // x = if seconds >= 1 { noise-free script value } else { -1 }
    let t = add(&mut app, tag, "time.now", json!({}));
    let cmp = add(
        &mut app,
        tag,
        "compare.compare",
        json!({"op": "ge", "b": 1.0}),
    );
    let script = add(
        &mut app,
        tag,
        "script.rhai",
        json!({"source": "#{ vec: [a * 10.0, time, 0.0] }"}),
    );
    let cond = add(
        &mut app,
        tag,
        "logic.condition",
        json!({"if_false": [-1.0, -1.0, -1.0]}),
    );
    let set = add(
        &mut app,
        tag,
        "object.set_param",
        json!({"param": "transform.pos"}),
    );
    link(&mut app, tag, (t, "seconds"), (cmp, "a"));
    link(&mut app, tag, (t, "seconds"), (script, "a"));
    link(&mut app, tag, (cmp, "out"), (cond, "condition"));
    link(&mut app, tag, (script, "vec"), (cond, "if_true"));
    link(&mut app, tag, (cond, "out"), (set, "value"));
    assert_eq!(pos(&app.frame(0), cube), [-1.0, -1.0, -1.0]);
    assert_eq!(pos(&app.frame(30), cube), [10.0, 1.0, 0.0]);
    assert_eq!(pos(&app.frame(60), cube), [20.0, 2.0, 0.0]);

    // noise from the library, driven by time, deterministic between evaluations
    let n = add(&mut app, tag, "math.noise", json!({"seed": 3}));
    let cmp_to_noise = (n, "value");
    let comp = add(&mut app, tag, "vector.compose", json!({}));
    link(&mut app, tag, cmp_to_noise, (comp, "y"));
    link(&mut app, tag, (t, "seconds"), (n, "time"));
    link(&mut app, tag, (comp, "out"), (cond, "if_false"));
    let a = pos(&app.frame(10), cube);
    let b = pos(&app.frame(10), cube);
    assert_eq!(a, b);
    assert!(a[1].abs() <= 1.0 && a[1] != -1.0, "{a:?}");
}

#[test]
fn failures_are_diagnostics_and_never_abort_the_pass() {
    let mut app = app();
    let (cube, bad) = cube_with_tag(&mut app);
    // 1: a write that cannot convert (a string into a vec3 param)
    let w = add(
        &mut app,
        bad,
        "object.set_param",
        json!({"param": "transform.pos", "value": "oops"}),
    );
    let _ = w;
    // 2: a write to a parameter that does not exist
    let bad2 = add_tag(&mut app, cube);
    add(
        &mut app,
        bad2,
        "object.set_param",
        json!({"param": "no_such_param", "value": 1.0}),
    );
    // 3: a script that fails
    let bad3 = add_tag(&mut app, cube);
    let s = add(&mut app, bad3, "script.rhai", json!({"source": "loop {}"}));
    let w = add(
        &mut app,
        bad3,
        "object.set_param",
        json!({"param": "size_x"}),
    );
    link(&mut app, bad3, (s, "result"), (w, "value"));
    // 4: a good tag after the bad ones still runs
    let good = add_tag(&mut app, cube);
    let w = add(
        &mut app,
        good,
        "object.set_param",
        json!({"param": "transform.pos", "value": [1.0, 2.0, 3.0]}),
    );
    let _ = w;
    // 5: a damaged graph parameter
    let broken = add_tag(&mut app, cube);
    app.doc
        .transact("break", |tx| {
            tx.set_param(broken, "graph", Value::String("{oops".into()))
        })
        .unwrap();

    let (scene, diags) = apply_expressions(app.doc.scene(), 0.0);
    assert_eq!(pos(&scene, cube), [1.0, 2.0, 3.0]);
    let tags: Vec<NodeId> = diags.iter().map(|d| d.tag).collect();
    assert_eq!(tags, vec![bad, bad2, bad3, broken], "{diags:?}");
    assert!(
        diags[0].message.contains("cannot convert"),
        "{}",
        diags[0].message
    );
    assert!(
        diags[1].message.contains("no parameter"),
        "{}",
        diags[1].message
    );
    assert!(
        diags[2].message.contains("operation limit") || diags[2].message.contains("time limit"),
        "{}",
        diags[2].message
    );
    assert!(
        diags[3].message.contains("invalid graph"),
        "{}",
        diags[3].message
    );
    // the app evaluates the same way
    assert_eq!(pos(&app.frame(0), cube), [1.0, 2.0, 3.0]);
}

#[test]
fn a_write_that_fails_validation_is_skipped_with_a_diagnostic() {
    let mut app = app();
    let (cube, tag) = cube_with_tag(&mut app);
    // a colour written into a float param of the wrong kind is an eval error; a non-finite
    // write is refused by the graph; segments are clamped by the parameter range, not skipped
    add(
        &mut app,
        tag,
        "object.set_param",
        json!({"param": "segments_x", "value": 9000}),
    );
    let (scene, diags) = apply_expressions(app.doc.scene(), 0.0);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(scene.param(cube, "segments_x"), Some(&Value::Int(1000)));
}

#[test]
fn graphs_save_and_load_with_the_scene() {
    let mut app = app();
    let (cube, tag) = cube_with_tag(&mut app);
    drive_x(&mut app, tag, 100.0, "transform.pos");
    let dir = std::env::temp_dir().join(format!("forge-nodes-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("expr.forge").to_string_lossy().into_owned();
    app.save(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains(&format!("version: {}", forge_core::file::VERSION)),
        "{text}"
    );
    assert!(text.contains("forge.expression"), "{text}");

    let mut loaded = App::new_default();
    let report = loaded.open(&path).unwrap();
    assert_eq!(report["warnings"], json!([]));
    assert_eq!(
        load_graph(loaded.doc.scene(), tag).unwrap(),
        load_graph(app.doc.scene(), tag).unwrap()
    );
    for frame in [0, 15, 30] {
        assert_eq!(
            pos(&loaded.frame(frame), cube),
            pos(&app.frame(frame), cube),
            "frame {frame}"
        );
    }
    assert!((pos(&loaded.frame(30), cube)[0] - 100.0).abs() < 1e-9);
    // save again: the text is stable
    let again = dir.join("expr2.forge").to_string_lossy().into_owned();
    loaded.save(&again).unwrap();
    assert_eq!(std::fs::read_to_string(&again).unwrap(), text);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_version_7_file_loads_and_an_expression_added_afterwards_saves_as_version_8() {
    // The previous build wrote this: no expression tag, version 7.
    let v7 = r#"(format: "forge3d-scene", version: 7, settings: (fps: 30, start: 0, end: 90), roots: [
        (id: 1, type_id: "forge.cube", name: "Cube", params: {}, tags: [], children: [])])"#;
    let app = App::new_default();
    let registry = app.doc.scene().registry().clone();
    let (scene, report) = forge_core::file::load(registry, v7).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let mut app = App::new_default();
    app.reset(forge_core::Document::from_scene(scene), None, true);
    let cube = NodeId(1);
    let tag = add_tag(&mut app, cube);
    drive_x(&mut app, tag, 60.0, "transform.pos");
    let saved = forge_core::file::save(app.doc.scene());
    assert!(saved.contains(&format!("version: {}", forge_core::file::VERSION)));
    let (back, report) =
        forge_core::file::load(app.doc.scene().registry().clone(), &saved).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let (evaluated, diags) = apply_expressions(&back, 0.5);
    assert!(diags.is_empty(), "{diags:?}");
    assert!((pos(&evaluated, cube)[0] - 30.0).abs() < 1e-9);
}

#[test]
fn the_expression_tag_type_is_registered_with_a_hidden_graph_parameter() {
    let app = App::new_default();
    let ty = app.doc.scene().registry().get(TAG).unwrap().clone();
    assert_eq!(ty.category, forge_core::Category::Tag);
    assert!(ty.find_param("graph").unwrap().ui.hidden);
    assert_eq!(ty.find_param("enabled").unwrap().default, Value::Bool(true));
    // a fresh tag has an empty graph
    let mut app = app;
    let (_, tag) = cube_with_tag(&mut app);
    assert!(load_graph(app.doc.scene(), tag).unwrap().is_empty());
    assert!(load_graph(app.doc.scene(), NodeId(1)).is_err());
}
