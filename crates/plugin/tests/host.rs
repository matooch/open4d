//! Plugin host integration (docs/specs/PLUGINS.md sections 8 and 9): the evaluation hook and
//! the plugin folders. Every folder is a temp dir; the real data dir is never read.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use forge_app::App;
use forge_core::{EvalCtx, NodeId};
use forge_generators::BuildError;
use forge_plugin::{HostOptions, RhaiPlugin, load_host};
use serde_json::json;

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins")
}

fn opts_with(extra: Vec<PathBuf>) -> HostOptions {
    HostOptions {
        extra_dirs: extra,
        ..HostOptions::default()
    }
}

/// Writes a plugin `name` with node type `<name>.shape` and command `<name>.hello`.
fn write_plugin(root: &Path, folder: &str, name: &str, build: &str) -> PathBuf {
    let dir = root.join(folder);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("plugin.toml"),
        format!(
            "name = \"{name}\"\nversion = \"1.2.3\"\nentry = \"main.rhai\"\n\
             [[node_types]]\nid = \"{name}.shape\"\nlabel = \"Shape\"\nbuild = \"build\"\n\
             [[commands]]\nid = \"{name}.hello\"\nlabel = \"Hello\"\nfunction = \"hello\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("main.rhai"),
        format!("{build}\nfn hello(a) {{ 1 }}"),
    )
    .unwrap();
    dir
}

const TRIANGLE: &str = "fn build(p) { #{ points: [[0,0,0],[1,0,0],[0,1,0]], polys: [[0,1,2]] } }";

fn names(report: &forge_plugin::LoadReport) -> Vec<&str> {
    report.plugins.iter().map(|p| p.name.as_str()).collect()
}

#[test]
fn ring_kit_evaluates_through_generators_scheduler_and_export() {
    let host = load_host(&opts_with(vec![examples()]));
    assert!(host.report.skipped.is_empty(), "{:?}", host.report.skipped);
    let mut app = host.new_app();
    let commands = app.commands.clone();
    let id = NodeId(
        commands
            .run(&mut app, "object.create", json!({"type": "ring_kit.ring"}))
            .unwrap()["node"]
            .as_u64()
            .unwrap(),
    );
    commands
        .run(
            &mut app,
            "param.set",
            json!({"node": id.0, "param": "segments", "value": 7}),
        )
        .unwrap();

    let plugin = RhaiPlugin::load(&examples().join("ring-kit")).unwrap();
    let want = plugin.evaluate(app.doc.scene(), id).unwrap().unwrap();
    assert_eq!(want.points.len(), 7 * 8);

    // forge_generators::evaluate
    let got = forge_generators::evaluate(app.doc.scene(), id, &EvalCtx::default())
        .unwrap()
        .unwrap();
    assert_eq!(got, want);

    // the scheduler
    let frame = app.scheduled(0);
    let out = frame.outputs.get(&id).expect("the ring is drawn");
    assert_eq!(out.error, None);
    assert_eq!(out.mesh.as_ref().unwrap(), &want);

    // eval_mesh and OBJ export
    assert_eq!(app.eval_mesh(id, 0).unwrap(), want);
    let dir = tempfile::tempdir().unwrap();
    let obj = dir.path().join("ring.obj");
    app.export_obj(obj.to_str().unwrap(), None).unwrap();
    let text = std::fs::read_to_string(&obj).unwrap();
    assert_eq!(text.lines().filter(|l| l.starts_with("v ")).count(), 7 * 8);
    assert_eq!(text.lines().filter(|l| l.starts_with("f ")).count(), 7 * 8);
}

#[test]
fn a_plugin_less_app_leaves_the_node_without_geometry() {
    // the type is unknown to a plain registry, so creating it fails; nothing panics
    let mut app = App::new_default();
    let commands = app.commands.clone();
    assert!(
        commands
            .run(&mut app, "object.create", json!({"type": "ring_kit.ring"}))
            .is_err()
    );
}

#[test]
fn a_failing_build_function_is_a_node_error_not_a_panic() {
    let root = tempfile::tempdir().unwrap();
    for (folder, name, build) in [
        ("a", "boom", "fn build(p) { throw \"bad things\"; }"),
        (
            "b",
            "badmesh",
            "fn build(p) { #{ points: [[0,0,0]], polys: [[0,1,2]] } }",
        ),
        ("c", "spin", "fn build(p) { loop { } }"),
    ] {
        write_plugin(root.path(), folder, name, build);
    }
    let host = load_host(&opts_with(vec![root.path().to_path_buf()]));
    assert_eq!(host.report.plugins.len(), 3, "{:?}", host.report.skipped);
    let mut app = host.new_app();
    let commands = app.commands.clone();
    for name in ["boom", "badmesh", "spin"] {
        let id = NodeId(
            commands
                .run(
                    &mut app,
                    "object.create",
                    json!({"type": format!("{name}.shape")}),
                )
                .unwrap()["node"]
                .as_u64()
                .unwrap(),
        );
        match forge_generators::evaluate(app.doc.scene(), id, &EvalCtx::default()) {
            Err(BuildError::Failed(m)) => assert!(m.contains(name), "{m}"),
            other => panic!("{name}: expected a node error, got {other:?}"),
        }
        let frame = app.scheduled(0);
        let out = frame.outputs.get(&id).unwrap();
        assert!(out.error.is_some() && out.mesh.is_none());
        assert!(app.eval_mesh(id, 0).is_err());
    }
}

#[test]
fn a_plugin_node_evaluates_without_params_set() {
    let root = tempfile::tempdir().unwrap();
    write_plugin(root.path(), "a", "tri", TRIANGLE);
    let host = load_host(&opts_with(vec![root.path().to_path_buf()]));
    let mut app = host.new_app();
    let commands = app.commands.clone();
    let id = NodeId(
        commands
            .run(&mut app, "object.create", json!({"type": "tri.shape"}))
            .unwrap()["node"]
            .as_u64()
            .unwrap(),
    );
    let mesh = app.eval_mesh(id, 0).unwrap();
    assert_eq!(mesh.points.len(), 3);
    assert_eq!(mesh.polys, vec![vec![0, 1, 2]]);
}

#[test]
fn folder_order_env_extra_and_disable() {
    let data = tempfile::tempdir().unwrap();
    let envdir = tempfile::tempdir().unwrap();
    let envdir2 = tempfile::tempdir().unwrap();
    let extra = tempfile::tempdir().unwrap();
    write_plugin(data.path(), "p", "from_data", TRIANGLE);
    write_plugin(envdir.path(), "p", "from_env", TRIANGLE);
    write_plugin(envdir2.path(), "p", "from_env2", TRIANGLE);
    write_plugin(extra.path(), "p", "from_extra", TRIANGLE);

    let env_value = std::env::join_paths([envdir.path(), Path::new(""), envdir2.path()]).unwrap();
    let env = |k: &str| (k == "FORGE_PLUGIN_PATH").then(|| env_value.clone());
    let options = HostOptions::resolve(
        vec![extra.path().to_path_buf()],
        false,
        &env,
        Some(data.path().to_path_buf()),
    );
    assert_eq!(
        options.folders(),
        vec![
            data.path().to_path_buf(),
            envdir.path().to_path_buf(),
            envdir2.path().to_path_buf(),
            extra.path().to_path_buf()
        ]
    );
    let host = load_host(&options);
    assert_eq!(
        names(&host.report),
        ["from_data", "from_env", "from_env2", "from_extra"]
    );
    assert!(host.registry.get("from_extra.shape").is_some());
    assert!(host.commands.get("from_env.hello").is_some());

    // --no-plugins and FORGE_NO_PLUGINS disable everything
    let flag = HostOptions::resolve(
        vec![extra.path().to_path_buf()],
        true,
        &env,
        Some(data.path().to_path_buf()),
    );
    assert!(flag.folders().is_empty());
    for (value, off) in [("1", true), ("yes", true), ("0", false), ("", false)] {
        let env = |k: &str| (k == "FORGE_NO_PLUGINS").then(|| OsString::from(value));
        let o = HostOptions::resolve(Vec::new(), false, &env, None);
        assert_eq!(o.disabled, off, "FORGE_NO_PLUGINS={value:?}");
    }
    let host = load_host(&flag);
    assert!(host.report.disabled && host.report.plugins.is_empty());
    assert!(host.registry.get("from_data.shape").is_none());
    // plugin.list is still there
    assert!(host.commands.get("plugin.list").is_some());
}

#[test]
fn later_folders_skip_taken_names_and_ids() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    write_plugin(first.path(), "p", "dup", TRIANGLE);
    write_plugin(second.path(), "same_name", "dup", TRIANGLE);
    write_plugin(second.path(), "other", "unique", TRIANGLE);
    let host = load_host(&HostOptions {
        data_dir: Some(first.path().to_path_buf()),
        extra_dirs: vec![second.path().to_path_buf()],
        ..HostOptions::default()
    });
    assert_eq!(names(&host.report), ["dup", "unique"]);
    assert_eq!(host.report.plugins[0].dir, first.path().join("p"));
    assert_eq!(host.report.skipped.len(), 1);
    assert_eq!(host.report.skipped[0].dir, second.path().join("same_name"));
    assert!(host.report.skipped[0].reason.contains("same name"));
}

#[test]
fn a_broken_plugin_is_reported_and_the_rest_load() {
    let root = tempfile::tempdir().unwrap();
    write_plugin(root.path(), "good", "good", TRIANGLE);
    let broken = root.path().join("broken");
    std::fs::create_dir(&broken).unwrap();
    std::fs::write(broken.join("plugin.toml"), "name = ").unwrap();
    let host = load_host(&opts_with(vec![root.path().to_path_buf()]));
    assert_eq!(names(&host.report), ["good"]);
    assert_eq!(host.report.skipped.len(), 1);
    assert_eq!(host.report.skipped[0].dir, broken);
}

#[test]
fn plugin_list_reports_plugins_types_commands_and_skipped() {
    let root = tempfile::tempdir().unwrap();
    write_plugin(root.path(), "a", "alpha", TRIANGLE);
    let broken = root.path().join("b");
    std::fs::create_dir(&broken).unwrap();
    std::fs::write(broken.join("plugin.toml"), "oops").unwrap();
    let host = load_host(&opts_with(vec![root.path().to_path_buf()]));
    let mut app = host.new_app();
    let before = app.doc.undo_depth();
    let list = app.run("plugin.list", json!({})).unwrap();
    assert_eq!(app.doc.undo_depth(), before, "read-only: no undo step");
    assert!(!host.commands.get("plugin.list").unwrap().undoable());
    assert_eq!(list["disabled"], false);
    assert_eq!(list["folders"][0], root.path().display().to_string());
    let p = &list["plugins"][0];
    assert_eq!(p["name"], "alpha");
    assert_eq!(p["version"], "1.2.3");
    assert_eq!(p["id"], "plugin.alpha");
    assert_eq!(p["node_types"], json!(["alpha.shape"]));
    assert_eq!(p["commands"], json!(["alpha.hello"]));
    assert_eq!(list["skipped"].as_array().unwrap().len(), 1);
    assert_eq!(list["skipped"][0]["dir"], broken.display().to_string());
    assert!(!list["skipped"][0]["reason"].as_str().unwrap().is_empty());

    // plugin commands are bound by the host
    assert_eq!(app.run("alpha.hello", json!({})).unwrap(), json!(1));
}
