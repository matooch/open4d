//! `user_data.add` command (brief 110).

use forge_app::App;
use forge_core::Value;
use serde_json::json;

#[test]
fn add_float_user_data_and_edit_it() {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let id = app.selection.active.unwrap();
    let r = app
        .run(
            "user_data.add",
            json!({
                "node": id.0,
                "name": "Amount",
                "kind": "float",
                "default": 1.0,
                "min": 0.0,
                "max": 10.0,
            }),
        )
        .unwrap();
    assert_eq!(r["param"], "user_data.0");
    assert_eq!(
        app.doc.scene().param(id, "user_data.0"),
        Some(&Value::Float(1.0))
    );

    app.run("param.edit", json!({"param": "user_data.0", "value": 7.5}))
        .unwrap();
    assert_eq!(
        app.doc.scene().param(id, "user_data.0"),
        Some(&Value::Float(7.5))
    );
}

#[test]
fn add_link_user_data_and_target_an_object() {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let holder = app.selection.active.unwrap();
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let target = app.selection.active.unwrap();
    let r = app
        .run(
            "user_data.add",
            json!({
                "node": holder.0,
                "name": "Target",
                "kind": "link",
                "default": target.0,
            }),
        )
        .unwrap();
    assert_eq!(r["param"], "user_data.0");
    assert_eq!(
        app.doc.scene().param(holder, "user_data.0"),
        Some(&Value::Link(Some(target)))
    );
}
