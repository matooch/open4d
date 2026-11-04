//! docs/specs/UI.md §26 and COMMANDS.md §6 (brief 154): the preset library and the recent
//! file list. Everything runs against temporary folders; no real user folder is touched.

use std::path::{Path, PathBuf};

use forge_app::App;
use forge_core::{NodeId, Value};
use serde_json::{Value as Json, json};

fn app_with(dir: &Path) -> App {
    let mut app = App::new_default();
    app.browser
        .use_files(Some(dir.join("settings.json")), Some(dir.join("presets")));
    app
}

fn make(app: &mut App, ty: &str) -> NodeId {
    NodeId(
        app.run("object.create", json!({"type": ty})).unwrap()["node"]
            .as_u64()
            .unwrap(),
    )
}

fn set(app: &mut App, node: NodeId, param: &str, value: Json) {
    app.run(
        "param.set",
        json!({"node": node.0, "param": param, "value": value}),
    )
    .unwrap();
}

fn at(app: &mut App, node: NodeId, pos: [f64; 3]) {
    app.run("transform.set", json!({"node": node.0, "pos": pos}))
        .unwrap();
}

fn child(app: &mut App, node: NodeId, parent: NodeId) {
    app.run(
        "object.reparent",
        json!({"node": node.0, "parent": parent.0}),
    )
    .unwrap();
}

fn named(app: &mut App, ty: &str, name: &str, parent: Option<NodeId>, pos: [f64; 3]) -> NodeId {
    let n = make(app, ty);
    app.run("object.rename", json!({"node": n.0, "name": name}))
        .unwrap();
    at(app, n, pos);
    if let Some(p) = parent {
        child(app, n, p);
    }
    n
}

fn count(app: &App) -> usize {
    let sc = app.doc.scene();
    sc.roots()
        .iter()
        .map(|r| 1 + sc.descendants(*r).len())
        .sum()
}

/// The three built-in presets, as written by `regenerate_builtin_presets`.
fn build_preset(app: &mut App, name: &str) {
    match name {
        "table" => {
            let t = named(app, "forge.null", "Table", None, [0.0; 3]);
            let top = named(app, "forge.cube", "Top", Some(t), [0.0, 100.0, 0.0]);
            for (k, v) in [("size_x", 300.0), ("size_y", 20.0), ("size_z", 180.0)] {
                set(app, top, k, json!(v));
            }
            for (i, (x, z)) in [
                (-130.0, -70.0),
                (130.0, -70.0),
                (-130.0, 70.0),
                (130.0, 70.0),
            ]
            .into_iter()
            .enumerate()
            {
                let leg = named(
                    app,
                    "forge.cylinder",
                    &format!("Leg {}", i + 1),
                    Some(t),
                    [x, 45.0, z],
                );
                set(app, leg, "radius", json!(10.0));
                set(app, leg, "height", json!(90.0));
            }
        }
        "pedestal" => {
            let p = named(app, "forge.null", "Pedestal", None, [0.0; 3]);
            let base = named(app, "forge.cube", "Base", Some(p), [0.0, 10.0, 0.0]);
            for (k, v) in [("size_x", 160.0), ("size_y", 20.0), ("size_z", 160.0)] {
                set(app, base, k, json!(v));
            }
            let col = named(app, "forge.cylinder", "Column", Some(p), [0.0, 80.0, 0.0]);
            set(app, col, "radius", json!(30.0));
            set(app, col, "height", json!(120.0));
            let cap = named(app, "forge.cube", "Cap", Some(p), [0.0, 148.0, 0.0]);
            for (k, v) in [("size_x", 120.0), ("size_y", 16.0), ("size_z", 120.0)] {
                set(app, cap, k, json!(v));
            }
            let ball = named(app, "forge.sphere", "Ball", Some(p), [0.0, 196.0, 0.0]);
            set(app, ball, "radius", json!(40.0));
        }
        "snowman" => {
            let s = named(app, "forge.null", "Snowman", None, [0.0; 3]);
            for (name, r, y) in [
                ("Body", 60.0, 60.0),
                ("Torso", 45.0, 150.0),
                ("Head", 30.0, 220.0),
            ] {
                let b = named(app, "forge.sphere", name, Some(s), [0.0, y, 0.0]);
                set(app, b, "radius", json!(r));
            }
            let hat = named(app, "forge.cylinder", "Hat", Some(s), [0.0, 265.0, 0.0]);
            set(app, hat, "radius", json!(22.0));
            set(app, hat, "height", json!(40.0));
        }
        other => panic!("no builder for {other}"),
    }
}

/// Rewrites `crates/command/presets/objects/*.forge`. Run by hand after changing a builder:
/// `cargo test -p forge-app --test browser regenerate_builtin_presets -- --ignored`.
#[test]
#[ignore = "writes the built-in preset files"]
fn regenerate_builtin_presets() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../command/presets/objects");
    for name in ["pedestal", "snowman", "table"] {
        let mut app = App::new_default();
        build_preset(&mut app, name);
        let bytes = forge_core::file::save(app.doc.scene());
        std::fs::write(out.join(format!("{name}.forge")), bytes).unwrap();
    }
}

#[test]
fn builtin_files_match_their_builders() {
    for name in ["pedestal", "snowman", "table"] {
        let mut built = App::new_default();
        build_preset(&mut built, name);
        let mut loaded = App::new_default();
        loaded
            .run("browser.load", json!({"id": format!("builtin:{name}")}))
            .unwrap();
        assert_eq!(count(&loaded), count(&built), "{name}");
        let names = |a: &App| {
            let sc = a.doc.scene();
            let mut v: Vec<(String, String)> = sc
                .roots()
                .iter()
                .flat_map(|r| std::iter::once(*r).chain(sc.descendants(*r)))
                .map(|i| {
                    let n = sc.node(i).unwrap();
                    (n.type_id.clone(), n.name.clone())
                })
                .collect();
            v.sort();
            v
        };
        assert_eq!(names(&loaded), names(&built), "{name}");
    }
}

#[test]
fn list_returns_builtin_then_user_presets_and_filters() {
    let tmp = tempfile::tempdir().unwrap();
    let mut app = app_with(tmp.path());
    let ids = |v: &Json| -> Vec<String> {
        v["presets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].as_str().unwrap().to_string())
            .collect()
    };
    let all = app.run("browser.list", json!({})).unwrap();
    assert_eq!(
        ids(&all),
        ["builtin:pedestal", "builtin:snowman", "builtin:table"]
    );
    assert_eq!(all["presets"][0]["name"], "Pedestal");
    assert_eq!(all["presets"][0]["source"], "builtin");
    assert_eq!(all["presets"][0]["kind"], "objects");
    make(&mut app, "forge.cube");
    app.run("browser.save_preset", json!({"name": "A Cube"}))
        .unwrap();
    let all = app.run("browser.list", json!({"kind": "objects"})).unwrap();
    assert_eq!(ids(&all).last().unwrap(), "user:a-cube");
    assert_eq!(all["presets"][3]["name"], "A Cube");
    let found = app.run("browser.list", json!({"query": "CUBE"})).unwrap();
    assert_eq!(ids(&found), ["user:a-cube"]);
    // a stray file in the folder is not a preset
    std::fs::write(tmp.path().join("presets/objects/notes.txt"), "x").unwrap();
    std::fs::write(tmp.path().join("presets/objects/Bad Name.forge"), "x").unwrap();
    assert_eq!(ids(&app.run("browser.list", json!({})).unwrap()).len(), 4);
    assert!(
        app.run("browser.list", json!({"kind": "materials"}))
            .is_err()
    );
}

#[test]
fn every_builtin_preset_loads_in_one_undo_step() {
    for id in ["pedestal", "snowman", "table"] {
        let mut app = App::new_default();
        let marker = make(&mut app, "forge.null");
        let depth = app.doc.undo_depth();
        let out = app
            .run("browser.load", json!({"id": format!("builtin:{id}")}))
            .unwrap();
        assert_eq!(app.doc.undo_depth(), depth + 1, "{id}: one undo step");
        let nodes = out["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 1);
        let root = NodeId(nodes[0].as_u64().unwrap());
        assert_eq!(app.selection.active, Some(root));
        assert_eq!(app.selection.nodes.len(), 1);
        assert!(app.doc.scene().node(root).unwrap().children.len() >= 3);
        // the existing object stays first, the preset comes after it
        assert_eq!(app.doc.scene().roots()[0], marker);
        assert_eq!(app.doc.scene().roots()[1], root);
        let loaded = count(&app);
        app.run("edit.undo", json!({})).unwrap();
        assert_eq!(count(&app), 1);
        app.run("edit.redo", json!({})).unwrap();
        assert_eq!(count(&app), loaded);
    }
}

#[test]
fn a_preset_loads_under_a_parent_and_keeps_params_and_tags() {
    let mut app = App::new_default();
    let group = make(&mut app, "forge.null");
    app.run(
        "browser.load",
        json!({"id": "builtin:pedestal", "parent": group.0}),
    )
    .unwrap();
    assert!(app.doc.scene().roots().len() == 1);
    let root = app.doc.scene().node(group).unwrap().children[0];
    let sc = app.doc.scene();
    let ball = sc
        .descendants(root)
        .into_iter()
        .find(|i| sc.node(*i).unwrap().name == "Ball")
        .unwrap();
    assert_eq!(sc.param(ball, "radius"), Some(&Value::Float(40.0)));
    // generators come with a smoothing tag; it is carried over
    assert!(!sc.node(ball).unwrap().tags.is_empty());
    // a tag or a missing node is no parent
    let tag = sc.node(ball).unwrap().tags[0];
    for bad in [tag.0, 9999] {
        assert!(
            app.run(
                "browser.load",
                json!({"id": "builtin:table", "parent": bad})
            )
            .is_err()
        );
    }
}

#[test]
fn save_load_round_trip_keeps_tracks_and_remaps_links() {
    let tmp = tempfile::tempdir().unwrap();
    let mut app = app_with(tmp.path());
    let cloner = make(&mut app, "forge.cloner");
    let eff = make(&mut app, "forge.effector.random");
    let cube = make(&mut app, "forge.cube");
    // a surrounding object the cloner must not link to after loading
    let other = make(&mut app, "forge.effector.plain");
    set(&mut app, cloner, "effectors", json!([eff.0, other.0]));
    app.run(
        "key.set",
        json!({"node": cube.0, "track": "transform.pos.x", "frame": 4, "value": 25.0}),
    )
    .unwrap();
    app.run(
        "browser.save_preset",
        json!({"name": "Rig", "nodes": [cloner.0, eff.0, cube.0]}),
    )
    .unwrap();

    let mut fresh = app_with(tmp.path());
    // burn some ids so the copies cannot match the originals by accident
    for _ in 0..3 {
        make(&mut fresh, "forge.null");
    }
    let out = fresh
        .run("browser.load", json!({"id": "user:rig"}))
        .unwrap();
    let ids: Vec<NodeId> = out["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| NodeId(n.as_u64().unwrap()))
        .collect();
    assert_eq!(ids.len(), 3);
    let sc = fresh.doc.scene();
    let by_type = |t: &str| {
        *ids.iter()
            .find(|i| sc.node(**i).unwrap().type_id == t)
            .unwrap()
    };
    let (c, e, q) = (
        by_type("forge.cloner"),
        by_type("forge.effector.random"),
        by_type("forge.cube"),
    );
    assert_eq!(sc.param(c, "effectors"), Some(&Value::LinkList(vec![e])));
    assert!(sc.node(q).unwrap().tracks.contains_key("transform.pos.x"));
}

#[test]
fn save_rules_overwrite_names_selection_and_delete() {
    let tmp = tempfile::tempdir().unwrap();
    let mut app = app_with(tmp.path());
    // nothing selected
    assert!(
        app.run("browser.save_preset", json!({"name": "x"}))
            .is_err()
    );
    let g = make(&mut app, "forge.null");
    let c = make(&mut app, "forge.cube");
    child(&mut app, c, g);
    app.selection.nodes = [g, c].into_iter().collect();
    let out = app
        .run("browser.save_preset", json!({"name": "  Two  Parts!! "}))
        .unwrap();
    assert_eq!(out["id"], "user:two-parts");
    assert!(Path::new(out["path"].as_str().unwrap()).starts_with(tmp.path()));
    // only the group is a root; the cube inside it is not saved twice
    let mut probe = app_with(tmp.path());
    let loaded = probe
        .run("browser.load", json!({"id": "user:two-parts"}))
        .unwrap();
    assert_eq!(loaded["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(count(&probe), 2);
    // exists
    assert!(
        app.run("browser.save_preset", json!({"name": "two parts"}))
            .is_err()
    );
    app.run(
        "browser.save_preset",
        json!({"name": "two parts", "overwrite": true}),
    )
    .unwrap();
    // bad names and nodes
    assert!(
        app.run("browser.save_preset", json!({"name": "!!!"}))
            .is_err()
    );
    assert!(
        app.run("browser.save_preset", json!({"name": "a", "nodes": [9999]}))
            .is_err()
    );
    // ids that try to leave the folder
    for bad in [
        "user:../settings",
        "user:",
        "table",
        "builtin:nope",
        "user:nope",
    ] {
        assert!(
            app.run("browser.load", json!({"id": bad})).is_err(),
            "{bad}"
        );
    }
    assert!(
        app.run("browser.delete_preset", json!({"id": "builtin:table"}))
            .is_err()
    );
    app.run("browser.delete_preset", json!({"id": "user:two-parts"}))
        .unwrap();
    assert!(
        app.run("browser.delete_preset", json!({"id": "user:two-parts"}))
            .is_err()
    );
    // the failed load left the document alone
    assert_eq!(count(&app), 2);
}

#[test]
fn without_a_library_folder_saving_fails_and_builtins_still_load() {
    let mut app = App::new_default();
    make(&mut app, "forge.cube");
    let err = app
        .run("browser.save_preset", json!({"name": "x"}))
        .unwrap_err();
    assert!(err.to_string().contains("no user preset folder"), "{err}");
    app.run("browser.load", json!({"id": "builtin:snowman"}))
        .unwrap();
}

#[test]
fn a_damaged_preset_is_an_error_and_leaves_the_document_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let mut app = app_with(tmp.path());
    let dir = tmp.path().join("presets/objects");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("broken.forge"), "not a scene").unwrap();
    let empty = forge_core::file::save(&forge_core::Scene::new(app.doc.scene().registry().clone()));
    std::fs::write(dir.join("empty.forge"), empty).unwrap();
    make(&mut app, "forge.cube");
    let depth = app.doc.undo_depth();
    for id in ["user:broken", "user:empty"] {
        assert!(app.run("browser.load", json!({"id": id})).is_err(), "{id}");
    }
    assert_eq!(app.doc.undo_depth(), depth);
    assert_eq!(count(&app), 1);
}

// ---- recent files -------------------------------------------------------------------

fn save_scene(app: &mut App, path: &Path) {
    app.run("file.save_as", json!({"path": path.to_string_lossy()}))
        .unwrap();
}

fn recent(app: &mut App) -> Vec<String> {
    app.run("browser.recent", json!({})).unwrap()["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn open_and_save_record_recent_files_newest_first_without_duplicates() {
    let tmp = tempfile::tempdir().unwrap();
    let mut app = app_with(tmp.path());
    assert!(recent(&mut app).is_empty());
    let (a, b) = (tmp.path().join("a.forge"), tmp.path().join("b.forge"));
    save_scene(&mut app, &a);
    save_scene(&mut app, &b);
    app.run("file.open", json!({"path": a.to_string_lossy()}))
        .unwrap();
    let list = recent(&mut app);
    assert_eq!(list, [a.to_string_lossy(), b.to_string_lossy()]);
    // file.save to the current file keeps one entry
    app.run("file.save", json!({})).unwrap();
    assert_eq!(recent(&mut app).len(), 2);
    // a failed open records nothing
    assert!(
        app.run(
            "file.open",
            json!({"path": tmp.path().join("no.forge").to_string_lossy()})
        )
        .is_err()
    );
    assert_eq!(recent(&mut app).len(), 2);
    let first = &app.run("browser.recent", json!({})).unwrap()["files"][0];
    assert_eq!(first["name"], "a.forge");
    assert_eq!(first["exists"], true);
    assert_eq!(first["index"], 0);
}

#[test]
fn the_recent_list_is_limited_and_stored_beside_other_settings() {
    let tmp = tempfile::tempdir().unwrap();
    let settings = tmp.path().join("settings.json");
    std::fs::write(&settings, r#"{"ui":{"theme":"dark"},"recent":{"other":1}}"#).unwrap();
    let mut app = app_with(tmp.path());
    for i in 0..12 {
        save_scene(&mut app, &tmp.path().join(format!("s{i}.forge")));
    }
    let list = recent(&mut app);
    assert_eq!(list.len(), forge_command::browser::MAX_RECENT_FILES);
    assert!(list[0].ends_with("s11.forge"));
    let text = std::fs::read_to_string(&settings).unwrap();
    let json: Json = serde_json::from_str(&text).unwrap();
    assert_eq!(json["ui"]["theme"], "dark");
    assert_eq!(json["recent"]["files"].as_array().unwrap().len(), 10);
    assert_eq!(json["recent"]["other"], 1);
    // a new application reads it back; File > New keeps it
    let mut again = app_with(tmp.path());
    assert_eq!(recent(&mut again), list);
    again.run("file.new", json!({})).unwrap();
    assert_eq!(recent(&mut again), list);
}

#[test]
fn a_malformed_recent_key_is_an_empty_list() {
    let tmp = tempfile::tempdir().unwrap();
    for text in [
        "{",
        r#"{"recent":5}"#,
        r#"{"recent":{"files":"x"}}"#,
        r#"{"recent":{"files":[1,null]}}"#,
    ] {
        std::fs::write(tmp.path().join("settings.json"), text).unwrap();
        let mut app = app_with(tmp.path());
        assert!(recent(&mut app).is_empty(), "{text}");
    }
}

#[test]
fn open_recent_opens_the_entry_and_drops_missing_files() {
    let tmp = tempfile::tempdir().unwrap();
    let mut app = app_with(tmp.path());
    let (a, b) = (tmp.path().join("a.forge"), tmp.path().join("b.forge"));
    make(&mut app, "forge.cube");
    save_scene(&mut app, &a);
    app.run("file.new", json!({})).unwrap();
    save_scene(&mut app, &b);
    assert_eq!(count(&app), 0);
    // index 1 is a.forge (one cube)
    app.run("browser.open_recent", json!({"index": 1})).unwrap();
    assert_eq!(count(&app), 1);
    assert_eq!(app.file_path.as_deref(), Some(&*a.to_string_lossy()));
    assert_eq!(recent(&mut app)[0], a.to_string_lossy());
    // b disappears: the entry goes and the command fails, the document stays
    std::fs::remove_file(&b).unwrap();
    let list = app.run("browser.recent", json!({})).unwrap();
    assert_eq!(list["files"][1]["exists"], false);
    let err = app
        .run("browser.open_recent", json!({"index": 1}))
        .unwrap_err();
    assert!(err.to_string().contains("no longer exists"), "{err}");
    assert_eq!(recent(&mut app), [a.to_string_lossy()]);
    assert_eq!(count(&app), 1);
    assert!(app.run("browser.open_recent", json!({"index": 5})).is_err());
    assert!(app.run("browser.open_recent", json!({})).is_err());
}

#[test]
fn clear_recent_empties_the_list_and_the_file() {
    let tmp = tempfile::tempdir().unwrap();
    let mut app = app_with(tmp.path());
    save_scene(&mut app, &tmp.path().join("a.forge"));
    let out = app.run("browser.clear_recent", json!({})).unwrap();
    assert_eq!(out["cleared"], 1);
    assert!(recent(&mut app).is_empty());
    let mut again = app_with(tmp.path());
    assert!(recent(&mut again).is_empty());
    assert!(app.run("browser.clear_recent", json!({"x": 1})).is_err());
}

#[test]
fn the_default_application_never_touches_a_settings_file() {
    // no `use_files`: recents stay in memory
    let tmp = tempfile::tempdir().unwrap();
    let mut app = App::new_default();
    save_scene(&mut app, &tmp.path().join("a.forge"));
    assert_eq!(recent(&mut app).len(), 1);
    assert!(!tmp.path().join("settings.json").exists());
}
