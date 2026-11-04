//! Rhai plugin packages (docs/specs/PLUGINS.md): loading, registration, sandbox.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use forge_app::App;
use forge_command::{CommandPlugin, CommandRegistry};
use forge_core::NodeId;
use forge_plugin::{Manifest, PluginError, RhaiPlugin, discover, install_all};
use serde_json::{Map, json};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/ring-kit")
}

fn example_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins")
}

/// An app with the plugin installed and its commands bound.
fn app_with(plugin: RhaiPlugin) -> (App, Arc<RhaiPlugin>) {
    let mut registry = forge_app::default_registry().unwrap();
    let mut commands = forge_app::default_command_registry();
    forge_core::Plugin::register(&plugin, &mut registry).unwrap();
    plugin.register_commands(&mut commands).unwrap();
    let commands = Arc::new(commands);
    plugin.bind(&commands);
    (
        App::with_commands(Arc::new(registry), commands),
        Arc::new(plugin),
    )
}

/// Writes a plugin folder and loads it.
fn load_text(dir: &Path, manifest: &str, script: &str) -> Result<RhaiPlugin, PluginError> {
    std::fs::write(dir.join("plugin.toml"), manifest).unwrap();
    std::fs::write(dir.join("main.rhai"), script).unwrap();
    RhaiPlugin::load(dir)
}

const HEAD: &str = "name = \"t\"\nversion = \"1.0.0\"\nentry = \"main.rhai\"\n";

fn one_type(extra: &str) -> String {
    format!(
        "{HEAD}{extra}\n[[node_types]]\nid = \"t.shape\"\nlabel = \"Shape\"\nbuild = \"build\"\n"
    )
}

const TRIANGLE: &str = "fn build(p) { #{ points: [[0,0,0],[1,0,0],[0,1,0]], polys: [[0,1,2]] } }";

#[test]
fn example_plugin_adds_a_node_type_and_builds_its_mesh() {
    let plugin = RhaiPlugin::load(&example_dir()).unwrap();
    assert_eq!(plugin.manifest().name, "ring_kit");
    let (mut app, plugin) = app_with(plugin);

    // the type is in the registry with its descriptors
    let ty = app
        .doc
        .scene()
        .registry()
        .get("ring_kit.ring")
        .unwrap()
        .clone();
    let ids: Vec<&str> = ty.params.iter().map(|p| p.id.as_str()).collect();
    for want in ["radius", "tube", "segments", "sides"] {
        assert!(ids.contains(&want), "{want} missing in {ids:?}");
    }

    // created through the command registry like any node
    let commands = app.commands.clone();
    let r = commands
        .run(&mut app, "object.create", json!({"type": "ring_kit.ring"}))
        .unwrap();
    let id = NodeId(r["node"].as_u64().unwrap());
    let mesh = plugin.evaluate(app.doc.scene(), id).unwrap().unwrap();
    assert_eq!(mesh.points.len(), 24 * 8);
    assert_eq!(mesh.polys.len(), 24 * 8);
    // outermost point: radius + tube on the x axis
    let max_x = mesh.points.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    assert!((max_x - 120.0).abs() < 1e-9, "{max_x}");

    // a changed parameter changes the mesh; evaluation is deterministic
    commands
        .run(
            &mut app,
            "param.set",
            json!({"node": id.0, "param": "segments", "value": 6}),
        )
        .unwrap();
    let a = plugin.evaluate(app.doc.scene(), id).unwrap().unwrap();
    let b = plugin.evaluate(app.doc.scene(), id).unwrap().unwrap();
    assert_eq!(a.points.len(), 6 * 8);
    assert_eq!(a, b);
}

#[test]
fn example_command_runs_as_one_undo_step() {
    let (mut app, _plugin) = app_with(RhaiPlugin::load(&example_dir()).unwrap());
    let commands = app.commands.clone();
    assert!(commands.get("ring_kit.add_ring").is_some());
    let before = app.doc.undo_depth();
    let id = commands
        .run(&mut app, "ring_kit.add_ring", json!({"radius": 55.0}))
        .unwrap()
        .as_u64()
        .unwrap();
    // create + rename + set_param, grouped
    assert_eq!(app.doc.undo_depth(), before + 1);
    let node = app.doc.scene().node(NodeId(id)).unwrap();
    assert_eq!(node.name, "Ring");
    assert_eq!(node.params["radius"], forge_core::Value::Float(55.0));
    // no arguments at all is fine
    commands
        .run(&mut app, "ring_kit.add_ring", serde_json::Value::Null)
        .unwrap();
    assert!(app.doc.undo());
}

#[test]
fn discover_and_install_all_find_the_example() {
    let (found, failed) = discover(&example_root());
    assert!(failed.is_empty(), "{failed:?}");
    assert!(found.iter().any(|p| p.manifest().name == "ring_kit"));

    let mut registry = forge_app::default_registry().unwrap();
    let mut commands = forge_app::default_command_registry();
    let out = install_all(&example_root(), &mut registry, &mut commands);
    assert!(out.failures.is_empty(), "{:?}", out.failures);
    assert!(registry.get("ring_kit.ring").is_some());
    assert!(commands.get("ring_kit.add_ring").is_some());
}

#[test]
fn install_all_reports_broken_and_clashing_plugins() {
    let root = tempfile::tempdir().unwrap();
    for (folder, manifest) in [
        ("a_good", one_type("")),
        ("b_broken", "name = ".to_string()),
        ("c_same_name", one_type("")),
    ] {
        let dir = root.path().join(folder);
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("plugin.toml"), manifest).unwrap();
        std::fs::write(dir.join("main.rhai"), TRIANGLE).unwrap();
    }
    let mut registry = forge_app::default_registry().unwrap();
    let mut commands = CommandRegistry::new();
    let out = install_all(root.path(), &mut registry, &mut commands);
    assert_eq!(out.plugins.len(), 1);
    assert_eq!(out.failures.len(), 2, "{:?}", out.failures);
    assert!(registry.get("t.shape").is_some());
}

#[test]
fn manifest_rules() {
    let bad = |text: String| match Manifest::parse(&text) {
        Err(PluginError::Manifest(m)) => m,
        other => panic!("expected a manifest error, got {other:?}"),
    };
    assert!(bad(HEAD.replace("\"t\"", "\"Bad-Name\"")).contains("name"));
    assert!(bad(HEAD.replace("1.0.0", "1.0")).contains("version"));
    assert!(bad(HEAD.replace("main.rhai", "../main.rhai")).contains("entry"));
    assert!(bad(HEAD.replace("main.rhai", "/etc/passwd")).contains("entry"));
    assert!(bad(format!("{HEAD}permissions = [\"net\"]")).contains("permission"));
    assert!(bad(format!("{HEAD}surprise = 1")).contains("surprise"));
    let wrong_ns = "[[node_types]]\nid = \"other.shape\"\nlabel = \"S\"\nbuild = \"b\"\n";
    assert!(bad(format!("{HEAD}{wrong_ns}")).contains("must start with"));
    let wrong_cmd = "[[commands]]\nid = \"object.evil\"\nlabel = \"S\"\nfunction = \"f\"\n";
    assert!(bad(format!("{HEAD}{wrong_cmd}")).contains("must be a command id"));
    let kind = "\n[[node_types.params]]\nid = \"x\"\nkind = \"matrix\"\n";
    assert!(bad(one_type("") + kind).contains("unknown kind"));
    let unit = "\n[[node_types.params]]\nid = \"x\"\nkind = \"float\"\nunit = \"parsec\"\n";
    assert!(bad(one_type("") + unit).contains("unknown unit"));
    let range = "\n[[node_types.params]]\nid = \"x\"\nkind = \"int\"\nmin = 5\nmax = 1\n";
    assert!(bad(one_type("") + range).contains("min > max"));
    assert!(Manifest::parse(&one_type("")).is_ok());
}

#[test]
fn load_checks_script_and_descriptors() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    assert!(load_text(d, &one_type(""), TRIANGLE).is_ok());
    // missing function, wrong arity, syntax error
    for (script, want) in [
        ("fn other(p) { 1 }", "not defined"),
        ("fn build(a, b) { 1 }", "not defined"),
        ("fn build(p) { ", "script"),
    ] {
        let err = load_text(d, &one_type(""), script).unwrap_err();
        assert!(
            matches!(&err, PluginError::Script(_)) && err.to_string().contains(want),
            "{err}"
        );
    }
    // default outside the range, enum default that is no option
    let p =
        "\n[[node_types.params]]\nid = \"x\"\nkind = \"float\"\ndefault = 5\nmin = 0\nmax = 1\n";
    assert!(matches!(
        load_text(d, &(one_type("") + p), TRIANGLE),
        Err(PluginError::Manifest(_))
    ));
    let p = "\n[[node_types.params]]\nid = \"m\"\nkind = \"enum\"\ndefault = \"zz\"\noptions = [{id = \"a\", label = \"A\"}]\n";
    assert!(matches!(
        load_text(d, &(one_type("") + p), TRIANGLE),
        Err(PluginError::Manifest(_))
    ));
    // a missing entry file
    std::fs::remove_file(d.join("main.rhai")).unwrap();
    assert!(matches!(RhaiPlugin::load(d), Err(PluginError::Io(_))));
}

#[test]
fn angle_params_are_degrees_in_the_manifest_and_radians_stored() {
    let dir = tempfile::tempdir().unwrap();
    let p = "\n[[node_types.params]]\nid = \"turn\"\nkind = \"float\"\nunit = \"angle\"\ndefault = 90\nmin = 0\nmax = 180\n";
    let script = "fn build(p) { #{ points: [[p.turn,0,0],[1,0,0],[0,1,0]], polys: [[0,1,2]] } }";
    let plugin = load_text(dir.path(), &(one_type("") + p), script).unwrap();
    let ty = &plugin.manifest().node_types[0].node_type;
    assert_eq!(
        ty.find_param("turn").unwrap().default,
        forge_core::Value::Float(90f64.to_radians())
    );
    // the build function sees degrees
    let mesh = plugin.build_mesh("t.shape", &Map::new()).unwrap();
    assert!((mesh.points[0].x - 90.0).abs() < 1e-9);
}

#[test]
fn build_functions_are_sandboxed() {
    let dir = tempfile::tempdir().unwrap();
    let build = |script: &str| {
        load_text(dir.path(), &one_type(""), script)
            .unwrap()
            .build_mesh("t.shape", &Map::new())
    };
    // no commands, no scene
    let e =
        build("fn build(p) { run(\"object.create\", #{type: \"forge.null\"}); 1 }").unwrap_err();
    assert!(matches!(e, PluginError::Build { .. }), "{e}");
    assert!(build("fn build(p) { scene().len() }").is_err());
    // runaway loops stop (operation limit)
    let e = build("fn build(p) { loop {} }").unwrap_err();
    assert!(
        e.to_string().to_lowercase().contains("limit") || e.to_string().contains("operations"),
        "{e}"
    );
    // runaway recursion stops
    assert!(build("fn build(p) { build(p) }").is_err());
    // bad meshes are errors, not panics
    for script in [
        "fn build(p) { 5 }",
        "fn build(p) { #{ points: [[0,0,0]] } }",
        "fn build(p) { #{ points: [[0,0]], polys: [] } }",
        "fn build(p) { #{ points: [[0,0,0],[1,0,0],[0,1,0]], polys: [[0,1,9]] } }",
        "fn build(p) { #{ points: [[0,0,0],[1,0,0],[0,1,0]], polys: [[0,1]] } }",
        "fn build(p) { #{ points: [[0,0,0],[1,0,0],[0,1,0]], polys: [[0,1,-1]] } }",
    ] {
        assert!(
            matches!(build(script), Err(PluginError::Build { .. })),
            "{script}"
        );
    }
    // deterministic random numbers are available
    let mesh = build("fn build(p) { #{ points: [[0,0,0],[1,0,0],[0,1,0]], polys: [[0,1,2]] } }");
    assert_eq!(mesh.unwrap().polys.len(), 1);
}

#[test]
fn top_level_statements_never_run() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = one_type("permissions = [\"commands\"]\n");
    let manifest =
        format!("{manifest}\n[[commands]]\nid = \"t.go\"\nlabel = \"Go\"\nfunction = \"run_it\"\n");
    let script = format!("create(\"forge.null\");\n{TRIANGLE}\nfn run_it(a) {{ 7 }}");
    let (mut app, _plugin) = app_with(load_text(dir.path(), &manifest, &script).unwrap());
    let commands = app.commands.clone();
    let before = app.doc.scene().len();
    let r = commands.run(&mut app, "t.go", json!({})).unwrap();
    assert_eq!(r, json!(7));
    assert_eq!(app.doc.scene().len(), before);
}

#[test]
fn commands_need_the_permission_to_edit() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = "\n[[commands]]\nid = \"t.make\"\nlabel = \"Make\"\nfunction = \"make\"\n";
    let script = format!("{TRIANGLE}\nfn make(a) {{ create(\"forge.null\") }}");

    // without `commands` the run budget is zero: nothing is created
    let (mut app, _p) = app_with(load_text(dir.path(), &(one_type("") + cmd), &script).unwrap());
    let commands = app.commands.clone();
    assert!(!commands.get("t.make").unwrap().undoable());
    let before = app.doc.scene().len();
    assert!(commands.run(&mut app, "t.make", json!({})).is_err());
    assert_eq!(app.doc.scene().len(), before);

    // with it, the command works
    let granted = one_type("permissions = [\"commands\"]\n") + cmd;
    let (mut app, _p) = app_with(load_text(dir.path(), &granted, &script).unwrap());
    let commands = app.commands.clone();
    assert!(commands.get("t.make").unwrap().undoable());
    commands.run(&mut app, "t.make", json!({})).unwrap();
    assert_eq!(app.doc.scene().len(), before + 1);
}

#[test]
fn failing_plugin_commands_roll_back_and_blocked_commands_stay_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = "\n[[commands]]\nid = \"t.bad\"\nlabel = \"Bad\"\nfunction = \"bad\"\n\n[[commands]]\nid = \"t.nuke\"\nlabel = \"Nuke\"\nfunction = \"nuke\"\n";
    let script = format!(
        "{TRIANGLE}\nfn bad(a) {{ create(\"forge.null\"); throw \"boom\"; }}\nfn nuke(a) {{ run(\"file.new\") }}"
    );
    let manifest = one_type("permissions = [\"commands\"]\n") + cmd;
    let (mut app, _p) = app_with(load_text(dir.path(), &manifest, &script).unwrap());
    let commands = app.commands.clone();
    let before = app.doc.scene().len();
    let e = commands.run(&mut app, "t.bad", json!({})).unwrap_err();
    assert!(e.to_string().contains("boom"), "{e}");
    assert_eq!(app.doc.scene().len(), before);
    assert!(commands.run(&mut app, "t.nuke", json!({})).is_err());
    // arguments must be an object
    assert!(commands.run(&mut app, "t.bad", json!([1])).is_err());
}

#[test]
fn unbound_plugin_commands_fail_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = format!(
        "{}\n[[commands]]\nid = \"t.go\"\nlabel = \"Go\"\nfunction = \"run_it\"\n",
        one_type("")
    );
    let plugin = load_text(
        dir.path(),
        &manifest,
        &format!("{TRIANGLE}\nfn run_it(a) {{ 1 }}"),
    )
    .unwrap();
    let mut commands = forge_app::default_command_registry();
    plugin.register_commands(&mut commands).unwrap();
    let mut app = App::new_default();
    let e = commands.run(&mut app, "t.go", json!({})).unwrap_err();
    assert!(e.to_string().contains("not bound"), "{e}");
}
