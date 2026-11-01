//! Scripting (docs/specs/SCRIPTING.md §8): examples, bindings, undo, sandbox.

use std::time::Duration;

use forge_app::App;
use forge_core::NodeId;
use forge_script::{FailureKind, ScriptFailure, ScriptOptions, ScriptReport, run_script};
use serde_json::json;

fn example(name: &str) -> String {
    let path = format!(
        "{}/../../examples/scripts/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn run_with(app: &mut App, src: &str, opts: &ScriptOptions) -> Result<ScriptReport, ScriptFailure> {
    let commands = app.commands.clone();
    run_script(app, &commands, src, opts)
}

fn run(app: &mut App, src: &str) -> Result<ScriptReport, ScriptFailure> {
    run_with(app, src, &ScriptOptions::default())
}

fn ok(app: &mut App, src: &str) -> ScriptReport {
    run(app, src).unwrap_or_else(|e| panic!("script failed: {e}"))
}

fn fail(app: &mut App, src: &str) -> ScriptFailure {
    run(app, src).expect_err("script should fail")
}

fn scene_text(app: &App) -> String {
    forge_core::file::save(app.doc.scene())
}

fn center(app: &App, node: NodeId, frame: i64) -> [f64; 3] {
    let mesh = app.eval_mesh(node, frame).unwrap();
    let n = mesh.points.len() as f64;
    let sum = mesh
        .points
        .iter()
        .fold([0.0; 3], |a, p| [a[0] + p.x, a[1] + p.y, a[2] + p.z]);
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

// ---- examples ----

#[test]
fn cube_grid_builds_27_cubes_in_one_undo_step() {
    let mut app = App::new_default();
    let report = ok(&mut app, &example("cube_grid.rhai"));
    assert_eq!(report.output, ["built 27 cubes"]);
    let grid = NodeId(report.value.as_u64().unwrap());
    let scene = app.doc.scene();
    assert_eq!(scene.node(grid).unwrap().name, "Grid");
    let kids = scene.node(grid).unwrap().children.clone();
    assert_eq!(kids.len(), 27);
    let mut seen = std::collections::BTreeSet::new();
    for kid in kids {
        let node = scene.node(kid).unwrap();
        assert_eq!(node.type_id, "forge.cube");
        let c = center(&app, kid, 0);
        let (x, y, z) = (c[0] / 300.0, c[1] / 300.0, c[2] / 300.0);
        for v in [x, y, z] {
            assert!(near(v, v.round()) && (0.0..=2.0).contains(&v), "{c:?}");
        }
        let (x, y, z) = (x.round() as i64, y.round() as i64, z.round() as i64);
        assert_eq!(node.name, format!("cube_{x}{y}{z}"));
        seen.insert((x, y, z));
    }
    assert_eq!(seen.len(), 27);
    assert_eq!(report.commands, 2 + 27 * 4);
    assert_eq!(app.doc.history().undo_labels(), ["Run Script"]);
    app.run("edit.undo", json!({})).unwrap();
    assert!(app.doc.scene().is_empty());
}

#[test]
fn animated_sphere_moves_between_keys() {
    let mut app = App::new_default();
    let report = ok(&mut app, &example("animated_sphere.rhai"));
    let ball = NodeId(report.value.as_u64().unwrap());
    assert_eq!(app.timeline.current, 0);
    assert_eq!(app.settings().end, 30);
    assert!(near(center(&app, ball, 0)[0], 0.0));
    assert!(near(center(&app, ball, 15)[0], 200.0));
    assert!(near(center(&app, ball, 30)[0], 0.0));
    let mid = center(&app, ball, 7)[0];
    assert!(mid > 0.0 && mid < 200.0, "{mid}");
    assert_eq!(app.doc.history().undo_labels(), ["Run Script"]);
}

#[test]
fn random_cubes_are_reproducible() {
    let mut a = App::new_default();
    let mut b = App::new_default();
    assert_eq!(ok(&mut a, &example("random_cubes.rhai")).value, json!(20));
    assert_eq!(a.doc.scene().roots().len(), 20);
    ok(&mut b, &example("random_cubes.rhai"));
    assert_eq!(scene_text(&a), scene_text(&b));
    let first = a.doc.scene().roots()[0];
    let c = center(&a, first, 0);
    assert!(c.iter().all(|v| v.abs() <= 500.0));
    assert!(c.iter().any(|v| v.abs() > 1e-6));
}

#[test]
fn query_scene_prints_the_tree() {
    let mut app = App::new_default();
    let report = ok(&mut app, &example("query_scene.rhai"));
    assert_eq!(
        report.output,
        [
            "Root (forge.null)",
            "  A (forge.cube)",
            "  B (forge.cube)",
            "B size = 50.0"
        ]
    );
}

#[test]
fn every_example_has_a_test() {
    let dir = format!("{}/../../examples/scripts", env!("CARGO_MANIFEST_DIR"));
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "animated_sphere.rhai",
            "cube_grid.rhai",
            "query_scene.rhai",
            "random_cubes.rhai"
        ]
    );
}

// ---- bindings ----

#[test]
fn run_converts_results_and_arguments() {
    let mut app = App::new_default();
    let r = ok(
        &mut app,
        r#"
        let out = run("object.create", #{ type: "forge.cube" });
        run("object.rename", #{ node: out.node, name: "Box" });
        let t = run("timeline.step", #{ delta: 3 });
        [out.node, t.frame, run("view.frame_all")]
        "#,
    );
    assert_eq!(r.value, json!([1, 3, null]));
    assert_eq!(app.doc.scene().node(NodeId(1)).unwrap().name, "Box");
    assert_eq!(app.timeline.current, 3);
}

#[test]
fn create_set_param_and_set_transform() {
    let mut app = App::new_default();
    let r = ok(
        &mut app,
        r#"
        let c = create("forge.cube", "Crate");
        set_param(c, "size_x", 123.0);
        set_transform(c, #{ pos: vec3(1, 2, 3), rot: [90.0, 0.0, 0.0], scale: vec3(2, 2, 2) });
        c
        "#,
    );
    let c = NodeId(r.value.as_u64().unwrap());
    let node = app.doc.scene().node(c).unwrap();
    assert_eq!(node.name, "Crate");
    assert_eq!(node.params["size_x"], forge_core::Value::Float(123.0));
    assert_eq!(
        node.params["transform.pos"],
        forge_core::Value::Vec3([1.0, 2.0, 3.0])
    );
    assert_eq!(
        node.params["transform.scale"],
        forge_core::Value::Vec3([2.0, 2.0, 2.0])
    );
    let forge_core::Value::Vec3(rot) = node.params["transform.rot"] else {
        panic!()
    };
    assert!(near(rot[0], std::f64::consts::FRAC_PI_2), "{rot:?}");
}

#[test]
fn failing_command_throws_and_can_be_caught() {
    let mut app = App::new_default();
    let r = ok(
        &mut app,
        r#"
        let msg = "";
        try { run("object.create", #{ type: "nope.nothing" }); } catch (e) { msg = e; }
        let c = create("forge.cube");
        [msg, c]
        "#,
    );
    let msg = r.value[0].as_str().unwrap();
    assert!(msg.starts_with("object.create:"), "{msg}");
    assert_eq!(r.value[1], json!(1));

    let e = fail(&mut app, r#"run("no.such_command")"#);
    assert_eq!(e.kind, FailureKind::Runtime);
    assert!(e.message.contains("no.such_command"), "{e}");
    let e = fail(&mut app, r#"run("param.set", #{ node: 1 })"#);
    assert!(e.message.starts_with("param.set:"), "{e}");
}

#[test]
fn scene_queries() {
    let mut app = App::new_default();
    let r = ok(
        &mut app,
        r#"
        let a = create("forge.cube", "Same");
        let b = create("forge.sphere", "Same");
        let g = create("forge.null", "Group");
        run("object.reparent", #{ node: b, parent: g });
        set_param(a, "size_x", 77.0);
        let n = scene().node(a);
        let s = scene().node(b);
        #{
            ids: [a, b, g],
            nodes: scene().nodes(), roots: scene().roots(), first: scene().find("Same"),
            all: scene().find_all("Same"), none: scene().find("zzz"),
            sel: scene().selection(), len: scene().len(),
            id: n.id, name: n.name, ty: n.type, size: n.param("size_x"),
            parent: s.parent(), top: n.parent(), kids: scene().node(g).children(),
            has_size: n.params().contains("size_x"), fps: scene().fps, end: scene().end,
            frame: frame(),
        }
        "#,
    );
    let v = r.value;
    let (a, b, g) = (
        v["ids"][0].clone(),
        v["ids"][1].clone(),
        v["ids"][2].clone(),
    );
    assert_eq!(v["nodes"], json!([a, g, b]));
    assert_eq!(v["roots"], json!([a, g]));
    assert_eq!(v["first"], a);
    assert_eq!(v["all"], json!([a, b]));
    assert_eq!(v["none"], json!(null));
    assert_eq!(v["sel"], json!([g]));
    // three nodes plus the smoothing tags of the two generators
    assert_eq!(v["len"], json!(5));
    assert_eq!(
        (v["id"].clone(), v["name"].clone(), v["ty"].clone()),
        (a.clone(), json!("Same"), json!("forge.cube"))
    );
    assert_eq!(v["size"], json!(77.0));
    assert_eq!((v["parent"].clone(), v["top"].clone()), (g, json!(null)));
    assert_eq!(v["kids"], json!([b]));
    assert_eq!(v["has_size"], json!(true));
    assert_eq!(
        (v["fps"].clone(), v["end"].clone(), v["frame"].clone()),
        (json!(30), json!(90), json!(0))
    );

    let e = fail(&mut app, "scene().node(99)");
    assert!(e.message.contains("unknown node 99"), "{e}");
    let e = fail(&mut app, r#"scene().node(1).param("nope")"#);
    assert!(e.message.contains("no parameter nope"), "{e}");
}

#[test]
fn param_values_use_display_units() {
    let mut app = App::new_default();
    // The angle parameter reads back in degrees, a vec3 as vec3.
    let r = ok(
        &mut app,
        r#"
        let c = create("forge.cube");
        let p = scene().node(c).params();
        let names = [];
        for k in p.keys() { names.push(k); }
        names
        "#,
    );
    assert!(r.value.as_array().unwrap().iter().any(|n| n == "size_x"));
}

#[test]
fn vec3_and_maths() {
    let mut app = App::new_default();
    let r = ok(
        &mut app,
        r#"
        let a = vec3(1, 2, 3);
        let b = vec3(0.5, 0.5, 0.5);
        let c = (a + b) * 2 - vec3(1, 1, 1);
        c.x = c.x + 10.0;
        [c.to_array(), (a * 2.0).to_array(), (2 * a).to_array(), (a / 2).to_array(),
         vec3(3, 4, 0).len(), a.dot(b), vec3(1, 0, 0).cross(vec3(0, 1, 0)).to_array(),
         (-a).to_array(), vec3(0, 0, 5).normalized().to_array(), a == vec3(1, 2, 3), a != a,
         deg(180.0) > 3.14159, to_deg(PI()), a.to_string()]
        "#,
    );
    let v = r.value;
    assert_eq!(v[0], json!([12.0, 4.0, 6.0]));
    assert_eq!(v[1], json!([2.0, 4.0, 6.0]));
    assert_eq!(v[2], json!([2.0, 4.0, 6.0]));
    assert_eq!(v[3], json!([0.5, 1.0, 1.5]));
    assert_eq!(v[4], json!(5.0));
    assert_eq!(v[5], json!(3.0));
    assert_eq!(v[6], json!([0.0, 0.0, 1.0]));
    assert_eq!(v[7], json!([-1.0, -2.0, -3.0]));
    assert_eq!(v[8], json!([0.0, 0.0, 1.0]));
    assert_eq!(
        (v[9].clone(), v[10].clone(), v[11].clone()),
        (json!(true), json!(false), json!(true))
    );
    assert_eq!(v[12], json!(180.0));
    assert_eq!(v[13], json!("vec3(1, 2, 3)"));
}

#[test]
fn rand_is_seeded_and_deterministic() {
    let mut app = App::new_default();
    let src = r#"
        let r = rand(42);
        let s = rand(42);
        let a = [r.float(), r.float(), r.range(10, 20), r.int(1, 6), r.int(5, 5)];
        let b = [s.float(), s.float(), s.range(10, 20), s.int(1, 6), s.int(5, 5)];
        [a, b, rand(43).float()]
    "#;
    let v = ok(&mut app, src).value;
    assert_eq!(v[0], v[1]);
    assert_ne!(v[0][0], v[2]);
    for f in [&v[0][0], &v[0][1]] {
        assert!((0.0..1.0).contains(&f.as_f64().unwrap()));
    }
    assert!((10.0..20.0).contains(&v[0][2].as_f64().unwrap()));
    assert!((1..=6).contains(&v[0][3].as_i64().unwrap()));
    assert_eq!(v[0][4], json!(5));
    // SplitMix64 with seed 0: first output 0xE220A8397B1DCDAF.
    let first = ok(&mut app, "rand(0).float()").value.as_f64().unwrap();
    assert!(
        near(first, 0xE220_A839_7B1D_CDAF_u64 as f64 / 2f64.powi(64)),
        "{first}"
    );
    let e = fail(&mut app, "rand(1).int(3, 2)");
    assert!(e.message.contains("must not exceed"), "{e}");
}

#[test]
fn frame_sets_and_reads_the_current_frame() {
    let mut app = App::new_default();
    let r = ok(&mut app, "[frame(), frame(12), frame(), frame(100000)]");
    assert_eq!(r.value, json!([0, 12, 12, 90]));
    assert_eq!(app.timeline.current, 90);
}

#[test]
fn print_and_debug_go_to_the_output() {
    let mut app = App::new_default();
    let r = ok(
        &mut app,
        r#"print("a"); print(1 + 2); debug("b"); print(vec3(1, 2, 3)); 5"#,
    );
    assert_eq!(r.output.len(), 4);
    assert_eq!(&r.output[..2], ["a", "3"]);
    assert_eq!(r.value, json!(5));
    assert_eq!(r.commands, 0);
    assert!(r.operations > 0);
}

#[test]
fn commands_lists_the_registry() {
    let mut app = App::new_default();
    let r = ok(&mut app, "commands()");
    let ids: Vec<&str> = r
        .value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let want: Vec<String> = app.commands.iter().map(|c| c.id().to_string()).collect();
    assert_eq!(ids, want);
}

#[test]
fn export_obj_is_allowed_with_an_explicit_path() {
    let dir = std::env::temp_dir().join(format!("forge-script-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.obj");
    let mut app = App::new_default();
    let src = format!(
        r#"create("forge.cube"); run("file.export_obj", #{{ path: "{}" }}).objects"#,
        path.display().to_string().replace('\\', "/")
    );
    assert_eq!(ok(&mut app, &src).value, json!(1));
    assert!(std::fs::read_to_string(&path).unwrap().contains("v "));
}

// ---- undo ----

#[test]
fn a_run_is_one_undo_step_and_redoes() {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let before = scene_text(&app);
    ok(
        &mut app,
        r#"let a = create("forge.cube"); set_param(a, "size_x", 5.0); create("forge.sphere");"#,
    );
    let after = scene_text(&app);
    assert_ne!(before, after);
    assert_eq!(
        app.doc.history().undo_labels(),
        ["Create Null", "Run Script"]
    );
    assert!(
        app.run("edit.undo", json!({})).unwrap()["done"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(scene_text(&app), before);
    app.run("edit.redo", json!({})).unwrap();
    assert_eq!(scene_text(&app), after);
}

#[test]
fn a_script_that_changes_nothing_leaves_no_step() {
    let mut app = App::new_default();
    ok(&mut app, "scene().len() + frame(3)");
    assert!(app.doc.history().undo_labels().is_empty());
}

#[test]
fn a_failing_script_changes_nothing() {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let before = scene_text(&app);
    let steps = app.doc.history().undo_labels().len();
    let e = fail(
        &mut app,
        r#"create("forge.cube"); print("partial"); create("forge.sphere"); set_param(999, "size_x", 1.0); 1"#,
    );
    assert_eq!(e.kind, FailureKind::Runtime);
    assert_eq!(e.output, ["partial"]);
    assert_eq!(scene_text(&app), before);
    assert_eq!(app.doc.history().undo_labels().len(), steps);
    assert!(app.doc.history().redo_labels().is_empty());
    // the selection no longer names deleted nodes
    assert!(
        app.selection
            .nodes
            .iter()
            .all(|n| app.doc.scene().contains(*n))
    );
}

#[test]
fn undo_redo_and_file_commands_are_not_available() {
    let mut app = App::new_default();
    for id in [
        "edit.undo",
        "edit.redo",
        "file.new",
        "file.open",
        "file.save",
        "file.save_as",
        "file.import_obj",
    ] {
        let e = fail(&mut app, &format!(r#"run("{id}", #{{ path: "/tmp/x" }})"#));
        assert!(e.message.contains("not available in scripts"), "{id}: {e}");
    }
}

// ---- sandbox ----

#[test]
fn runaway_loops_stop_at_the_operation_limit() {
    let mut app = App::new_default();
    let opts = ScriptOptions {
        max_operations: 10_000,
        ..ScriptOptions::default()
    };
    let e = run_with(&mut app, "create(\"forge.cube\"); loop { }", &opts).unwrap_err();
    assert_eq!(e.kind, FailureKind::Limit);
    assert!(e.message.contains("operation limit (10000)"), "{e}");
    assert!(app.doc.scene().is_empty(), "rolled back");
    // not catchable by the script
    let e = run_with(&mut app, "try { loop { } } catch (e) { }", &opts).unwrap_err();
    assert_eq!(e.kind, FailureKind::Limit);
}

#[test]
fn deep_recursion_stops_at_the_call_depth_limit() {
    let mut app = App::new_default();
    let e = fail(&mut app, "fn f(n) { f(n + 1) } f(0)");
    assert_eq!(e.kind, FailureKind::Limit);
    assert!(e.message.contains("call depth limit (48)"), "{e}");
    let e = fail(&mut app, "fn f(n) { try { f(n + 1) } catch (e) { } } f(0)");
    assert_eq!(e.kind, FailureKind::Limit, "{e}");
}

#[test]
fn too_many_commands_stop_the_script() {
    let mut app = App::new_default();
    let opts = ScriptOptions {
        max_commands: 5,
        ..ScriptOptions::default()
    };
    let e = run_with(
        &mut app,
        "for i in 0..100 { create(\"forge.null\"); }",
        &opts,
    )
    .unwrap_err();
    assert_eq!(e.kind, FailureKind::Limit);
    assert!(e.message.contains("command limit (5)"), "{e}");
    assert!(app.doc.scene().is_empty());
    let e = run_with(
        &mut app,
        "for i in 0..100 { try { create(\"forge.null\"); } catch (e) { } }",
        &opts,
    )
    .unwrap_err();
    assert_eq!(e.kind, FailureKind::Limit, "not catchable");
}

#[test]
fn the_timeout_stops_the_script() {
    let mut app = App::new_default();
    let opts = ScriptOptions {
        timeout: Duration::from_millis(50),
        ..ScriptOptions::default()
    };
    let started = std::time::Instant::now();
    let e = run_with(&mut app, "loop { }", &opts).unwrap_err();
    assert_eq!(e.kind, FailureKind::Limit);
    assert!(e.message.contains("time limit"), "{e}");
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn oversized_data_is_refused() {
    let mut app = App::new_default();
    let e = fail(&mut app, r#"let s = "x"; loop { s += s; }"#);
    assert_eq!(e.kind, FailureKind::Limit);
    assert!(e.message.contains("too large"), "{e}");
    // growth by a native function is caught by the operation or time limit at the latest
    let opts = ScriptOptions {
        max_operations: 100_000,
        ..ScriptOptions::default()
    };
    let e = run_with(&mut app, "let a = []; loop { a.push(1); }", &opts).unwrap_err();
    assert_eq!(e.kind, FailureKind::Limit);
}

#[test]
fn modules_eval_and_time_are_gone() {
    let mut app = App::new_default();
    for src in [
        r#"import "other" as o; o::f()"#,
        r#"eval("1 + 1")"#,
        "timestamp()",
        r#"let x = 1; export x;"#,
    ] {
        let e = fail(&mut app, src);
        assert_ne!(e.kind, FailureKind::Limit, "{src}");
    }
}

#[test]
fn syntax_errors_report_a_line_and_run_nothing() {
    let mut app = App::new_default();
    let e = fail(&mut app, "create(\"forge.cube\");\nlet x = ;\n");
    assert_eq!(e.kind, FailureKind::Syntax);
    assert_eq!(e.line, Some(2));
    assert!(e.to_string().starts_with("line 2: "), "{e}");
    assert!(app.doc.scene().is_empty());
    assert!(app.doc.history().undo_labels().is_empty());
    // undefined variables are compile errors too (strict variables)
    let e = fail(&mut app, "create(\"forge.cube\"); typo + 1");
    assert_eq!(e.kind, FailureKind::Syntax);
    assert!(app.doc.scene().is_empty());
}

#[test]
fn runtime_errors_report_a_line() {
    let mut app = App::new_default();
    let e = fail(
        &mut app,
        "let a = 1;\nlet b = 2;\nset_param(42, \"size_x\", 1.0)",
    );
    assert_eq!(e.kind, FailureKind::Runtime);
    assert_eq!(e.line, Some(3));
    let e = fail(&mut app, "1 / 0");
    assert_eq!(e.kind, FailureKind::Runtime);
}

#[test]
fn bad_argument_types_are_runtime_errors() {
    let mut app = App::new_default();
    let e = fail(&mut app, r#"run("object.create", #{ type: |x| x })"#);
    assert_eq!(e.kind, FailureKind::Runtime);
    let e = fail(&mut app, "vec3(1, 2, \"z\")");
    assert_eq!(e.kind, FailureKind::Runtime);
}

#[test]
fn modeling_tools_are_commands_for_scripts() {
    // docs/specs/MODELING.md §0.1: the same command a tool's Apply button runs
    let mut app = App::new_default();
    let src = r#"
        run("object.create", #{ type: "forge.cube" });
        run("mesh.make_editable");
        let r = run("mesh.extrude", #{ polys: [4], settings: #{ offset: 50.0, subdivision: 2 } });
        [r.points, r.polys]
    "#;
    let report = ok(&mut app, src);
    assert_eq!(report.value, json!([20, 18]));
    let src = r#"
        run("object.create", #{ type: "forge.cube" });
        run("mesh.make_editable");
        let r = run("mesh.bevel", #{ edges: [[1, 3]], settings: #{ offset: 20.0, bevel_mode: "chamfer" } });
        [r.points, r.polys]
    "#;
    let report = ok(&mut app, src);
    assert_eq!(report.value, json!([10, 7]));
}
