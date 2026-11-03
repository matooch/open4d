//! Registry invariants and headless command runs (docs/specs/COMMANDS.md §7).

use std::sync::Arc;

use forge_command::{
    CommandContext, CommandError, CommandRegistry, ModeArg, NodeSelection, SelectOp, Timeline,
    register_editor_commands, schema, valid_command_id,
};
use forge_core::transform::{POS, ROT};
use forge_core::{Category, Document, NodeId, NodeType, ParamDesc, Registry, Unit, Value};
use serde_json::{Value as Json, json};

/// A context with no window: just the data commands need.
struct Headless {
    doc: Document,
    selection: NodeSelection,
    mode: ModeArg,
    timeline: Timeline,
    autokey: bool,
    status: String,
    path: Option<String>,
    component_settings: forge_modeling::ComponentSettings,
    transform_settings: forge_modeling::snap::TransformSettings,
}

impl Headless {
    fn new() -> Headless {
        let mut r = Registry::with_builtins();
        r.register(
            NodeType::new("test.box", "Box", Category::Generator)
                .param(ParamDesc::float("size", "Size", 2.0).range(0.0, 100.0))
                .param(ParamDesc::float("twist", "Twist", 0.0).unit(Unit::Angle))
                .param(ParamDesc::float("ratio", "Ratio", 0.5).unit(Unit::Percent)),
        )
        .unwrap();
        Headless {
            doc: Document::new(Arc::new(r)),
            selection: NodeSelection::default(),
            mode: ModeArg::Model,
            timeline: Timeline::default(),
            autokey: false,
            status: String::new(),
            path: None,
            component_settings: Default::default(),
            transform_settings: Default::default(),
        }
    }
}

impl CommandContext for Headless {
    fn doc(&self) -> &Document {
        &self.doc
    }
    fn doc_mut(&mut self) -> &mut Document {
        &mut self.doc
    }
    fn selection(&self) -> &NodeSelection {
        &self.selection
    }
    fn selection_mut(&mut self) -> &mut NodeSelection {
        &mut self.selection
    }
    fn edit_mode(&self) -> ModeArg {
        self.mode
    }
    fn set_edit_mode(&mut self, mode: ModeArg) {
        self.mode = mode;
    }
    fn component_settings(&self) -> &forge_modeling::ComponentSettings {
        &self.component_settings
    }
    fn component_settings_mut(&mut self) -> &mut forge_modeling::ComponentSettings {
        &mut self.component_settings
    }
    fn transform_settings(&self) -> &forge_modeling::snap::TransformSettings {
        &self.transform_settings
    }
    fn transform_settings_mut(&mut self) -> &mut forge_modeling::snap::TransformSettings {
        &mut self.transform_settings
    }
    fn timeline(&self) -> &Timeline {
        &self.timeline
    }
    fn timeline_mut(&mut self) -> &mut Timeline {
        &mut self.timeline
    }
    fn autokey(&self) -> bool {
        self.autokey
    }
    fn set_autokey(&mut self, on: bool) {
        self.autokey = on;
    }
    fn set_status(&mut self, text: String) {
        self.status = text;
    }
    fn select_components(
        &mut self,
        _op: SelectOp,
        _nodes: Option<&[NodeId]>,
    ) -> Result<(), String> {
        Err("no components headless".into())
    }
    fn record_keys(&mut self) -> Result<usize, String> {
        Ok(0)
    }
    fn delete_keys(&mut self) -> Result<usize, String> {
        Ok(0)
    }
    fn key_frame_near(&self, _forward: bool) -> Option<i64> {
        None
    }
    fn reset_document(&mut self, doc: Document, path: Option<String>, _full: bool) {
        self.doc = doc;
        self.path = path;
        self.selection.clear();
    }
    fn file_path(&self) -> Option<String> {
        self.path.clone()
    }
    fn mark_saved(&mut self, path: String) {
        self.path = Some(path);
    }
}

fn registry() -> CommandRegistry {
    let mut r = CommandRegistry::new();
    register_editor_commands(&mut r).unwrap();
    r
}

fn steps(c: &Headless) -> usize {
    c.doc.history().undo_labels().len()
}

fn node_of(v: &Json) -> NodeId {
    NodeId(v["node"].as_u64().unwrap())
}

#[test]
fn every_command_is_well_formed() {
    let r = registry();
    assert!(r.len() >= 30);
    let mut seen = std::collections::BTreeSet::new();
    for h in r.iter() {
        assert!(valid_command_id(h.id()), "bad id {}", h.id());
        assert!(seen.insert(h.id().to_string()), "duplicate id {}", h.id());
        assert!(!h.label().trim().is_empty(), "{} has no label", h.id());
        let schema = h.args_schema();
        assert!(
            schema.is_object() && schema["type"] == "object",
            "{}: schema is not an object schema: {schema}",
            h.id()
        );
        let sample = h.sample_args();
        schema::validate(&schema, &sample)
            .unwrap_or_else(|e| panic!("{}: sample {sample} invalid: {e}", h.id()));
    }
}

#[test]
fn brief_command_ids_exist() {
    let r = registry();
    for id in [
        "object.create",
        "object.delete",
        "object.rename",
        "object.reparent",
        "edit.undo",
        "edit.redo",
        "view.shading",
        "view.frame_all",
        "mesh.make_editable",
        "mesh.current_state_to_object",
        "select.mode",
        "select.all",
        "select.none",
        "select.invert",
        "select.grow",
        "select.shrink",
        "file.new",
        "file.open",
        "file.save",
        "file.save_as",
        "file.import_obj",
        "file.export_obj",
        "key.record",
        "key.delete_at_frame",
        "key.set",
        "timeline.set_frame",
        "timeline.play",
        "timeline.toggle_play",
        "view.gizmo_mode",
        "view.gizmo_space",
        "object.begin_rename",
        "file.open_dialog",
        "file.save_dialog",
        "file.save_as_dialog",
        "file.autosave_now",
        "timeline.stop",
        "timeline.settings",
        "transform.set",
        "param.set",
    ] {
        assert!(r.get(id).is_some(), "missing {id}");
    }
}

#[test]
fn registration_rejects_bad_and_duplicate_ids() {
    let mut r = registry();
    let first = r.iter().next().unwrap().id().to_string();
    assert!(!valid_command_id("nodots"));
    assert!(!valid_command_id("Upper.case"));
    assert!(!valid_command_id("a..b"));
    assert!(valid_command_id("a.b_c.d1"));
    // registering the same set again hits the first duplicate
    let err = register_editor_commands(&mut r).unwrap_err();
    assert!(err.to_string().contains(&first), "{err}");
}

#[test]
fn order_is_insertion_order() {
    let ids: Vec<String> = registry().iter().map(|h| h.id().to_string()).collect();
    assert_eq!(ids[0], "object.create");
    assert_eq!(ids[1], "object.delete");
}

#[test]
fn unknown_command_and_bad_arguments() {
    let r = registry();
    let mut c = Headless::new();
    assert!(matches!(
        r.run(&mut c, "nope.nothing", json!({})),
        Err(CommandError::Unknown(_))
    ));
    // unknown key, missing key, wrong type
    for args in [
        json!({"type": "test.box", "x": 1}),
        json!({}),
        json!({"type": 3}),
    ] {
        assert!(
            matches!(
                r.run(&mut c, "object.create", args),
                Err(CommandError::InvalidArgs(_))
            ),
            "object.create accepted bad arguments"
        );
    }
    assert!(matches!(
        r.run(&mut c, "edit.undo", json!({"x": 1})),
        Err(CommandError::InvalidArgs(_))
    ));
    assert_eq!(steps(&c), 0);
}

#[test]
fn ten_commands_run_headless_one_undo_step_each() {
    let r = registry();
    let mut c = Headless::new();
    let run = |c: &mut Headless, id: &str, args: Json| r.run(c, id, args).unwrap();

    // 1. create
    let a = node_of(&run(&mut c, "object.create", json!({"type": "test.box"})));
    assert_eq!(steps(&c), 1);
    assert_eq!(c.selection.active, Some(a));
    let b = node_of(&run(&mut c, "object.create", json!({"type": "forge.null"})));
    assert_eq!(steps(&c), 2);
    // 2. rename
    run(
        &mut c,
        "object.rename",
        json!({"node": a.0, "name": "Crate"}),
    );
    assert_eq!(c.doc.scene().node(a).unwrap().name, "Crate");
    assert_eq!(steps(&c), 3);
    // 3. reparent
    run(
        &mut c,
        "object.reparent",
        json!({"node": b.0, "parent": a.0}),
    );
    assert_eq!(c.doc.scene().node(b).unwrap().parent, Some(a));
    assert_eq!(steps(&c), 4);
    // 4. transform.set (rotation in degrees)
    run(
        &mut c,
        "transform.set",
        json!({"node": b.0, "pos": [1.0, 2.0, 3.0], "rot": [90.0, 0.0, 0.0]}),
    );
    assert_eq!(steps(&c), 5);
    assert_eq!(
        c.doc.scene().param(b, POS),
        Some(&Value::Vec3([1.0, 2.0, 3.0]))
    );
    let Some(Value::Vec3(rot)) = c.doc.scene().param(b, ROT) else {
        panic!("no rotation");
    };
    assert!((rot[0] - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    // 5. param.set: plain, angle in degrees, percent as shown
    run(
        &mut c,
        "param.set",
        json!({"node": a.0, "param": "size", "value": 5.0}),
    );
    run(
        &mut c,
        "param.set",
        json!({"node": a.0, "param": "twist", "value": 180.0}),
    );
    run(
        &mut c,
        "param.set",
        json!({"node": a.0, "param": "ratio", "value": 25.0}),
    );
    assert_eq!(steps(&c), 8);
    assert_eq!(c.doc.scene().param(a, "size"), Some(&Value::Float(5.0)));
    let Some(Value::Float(t)) = c.doc.scene().param(a, "twist") else {
        panic!("no twist");
    };
    assert!((t - std::f64::consts::PI).abs() < 1e-12);
    assert_eq!(c.doc.scene().param(a, "ratio"), Some(&Value::Float(0.25)));
    // a value of the wrong type fails and records nothing; numbers clamp into the range
    assert!(
        r.run(
            &mut c,
            "param.set",
            json!({"node": a.0, "param": "size", "value": "big"})
        )
        .is_err()
    );
    assert!(
        r.run(
            &mut c,
            "param.set",
            json!({"node": a.0, "param": "nope", "value": 1})
        )
        .is_err()
    );
    assert_eq!(steps(&c), 8);
    // 6. undo / 7. redo
    assert_eq!(run(&mut c, "edit.undo", json!({}))["done"], true);
    assert_eq!(c.doc.scene().param(a, "ratio"), Some(&Value::Float(0.5)));
    assert_eq!(run(&mut c, "edit.redo", json!({}))["done"], true);
    assert_eq!(c.doc.scene().param(a, "ratio"), Some(&Value::Float(0.25)));
    // 8. timeline
    run(&mut c, "timeline.set_frame", json!({"frame": 12}));
    assert_eq!(c.timeline.current, 12);
    assert_eq!(
        run(&mut c, "timeline.set_frame", json!({"frame": 9999}))["frame"],
        90
    );
    run(&mut c, "timeline.step", json!({"delta": -1}));
    assert_eq!(c.timeline.current, 89);
    run(
        &mut c,
        "timeline.settings",
        json!({"fps": 24, "start": 0, "end": 48}),
    );
    assert_eq!(c.timeline.current, 48);
    assert_eq!(steps(&c), 9);
    assert!(
        r.run(
            &mut c,
            "timeline.settings",
            json!({"fps": 0, "start": 0, "end": 48})
        )
        .is_err()
    );
    run(&mut c, "timeline.play", json!({}));
    assert!(c.timeline.playing);
    run(&mut c, "timeline.stop", json!({}));
    assert!(!c.timeline.playing);
    // 9. select.mode
    run(&mut c, "select.mode", json!({"mode": "polys"}));
    assert_eq!(c.mode, ModeArg::Polys);
    run(&mut c, "select.mode", json!({"mode": "model"}));
    // 10. delete: one step, children go with the parent, selection cleared
    run(&mut c, "object.delete", json!({"nodes": [a.0, b.0]}));
    assert_eq!(steps(&c), 10);
    assert!(!c.doc.scene().contains(a) && !c.doc.scene().contains(b));
    assert!(c.selection.nodes.is_empty());
    run(&mut c, "edit.undo", json!({}));
    assert!(c.doc.scene().contains(a) && c.doc.scene().contains(b));
    // key.autokey flips without arguments
    assert_eq!(run(&mut c, "key.autokey", json!({}))["enabled"], true);
    assert_eq!(
        run(&mut c, "key.autokey", json!({"enabled": false}))["enabled"],
        false
    );
}

#[test]
fn file_commands_round_trip() {
    let r = registry();
    let mut c = Headless::new();
    let dir = std::env::temp_dir().join(format!("forge-command-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("scene.forge").to_string_lossy().into_owned();
    let a = node_of(
        &r.run(&mut c, "object.create", json!({"type": "test.box"}))
            .unwrap(),
    );
    assert!(r.run(&mut c, "file.save", json!({})).is_err()); // no file name yet
    r.run(&mut c, "file.save_as", json!({"path": path}))
        .unwrap();
    assert_eq!(c.path.as_deref(), Some(path.as_str()));
    r.run(&mut c, "file.new", json!({})).unwrap();
    assert!(c.doc.scene().is_empty() && c.path.is_none());
    r.run(&mut c, "file.open", json!({"path": path})).unwrap();
    assert!(c.doc.scene().contains(a));
    let missing = dir.join("none.forge").to_string_lossy().into_owned();
    let err = r
        .run(&mut c, "file.open", json!({"path": missing}))
        .unwrap_err();
    assert!(err.to_string().starts_with("Open failed"), "{err}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn window_commands_toggle_play_and_refuse_headless() {
    let r = registry();
    let mut c = Headless::new();
    r.run(&mut c, "timeline.toggle_play", json!({})).unwrap();
    assert!(c.timeline.playing);
    r.run(&mut c, "timeline.toggle_play", json!({})).unwrap();
    assert!(!c.timeline.playing);
    // view-only commands are no-ops without a window
    r.run(&mut c, "view.gizmo_mode", json!({"mode": "rotate"}))
        .unwrap();
    r.run(&mut c, "view.gizmo_space", json!({})).unwrap();
    r.run(&mut c, "view.gizmo_space", json!({"space": "world"}))
        .unwrap();
    assert!(matches!(
        r.run(&mut c, "view.gizmo_mode", json!({"mode": "tilt"})),
        Err(CommandError::InvalidArgs(_))
    ));
    // dialogs and inline rename need the editor window
    for id in [
        "file.open_dialog",
        "file.save_dialog",
        "file.save_as_dialog",
        "file.autosave_now",
        "object.begin_rename",
    ] {
        match r.run(&mut c, id, json!({})) {
            Err(CommandError::Failed(m)) => assert_eq!(m, "needs the editor window", "{id}"),
            other => panic!("{id}: {other:?}"),
        }
        assert!(matches!(
            r.run(&mut c, id, json!({"x": 1})),
            Err(CommandError::InvalidArgs(_))
        ));
    }
    assert_eq!(steps(&c), 0);
}
