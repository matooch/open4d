//! U7: the application applies and undoes UV commands through the registry.

use std::sync::Arc;

use forge_app::{App, uv_commands};
use forge_core::Value;
use forge_geometry::PolyMesh;
use glam::DVec3;
use serde_json::json;

#[test]
fn commands_edit_one_uvw_tag_and_transform_is_undoable() {
    let mut app = App::new_default();
    let mesh = PolyMesh {
        points: vec![DVec3::ZERO, DVec3::X, DVec3::X + DVec3::Y, DVec3::Y],
        polys: vec![vec![0, 1, 2, 3]],
    };
    let obj = app
        .doc
        .transact("mesh", |tx| {
            let id = tx.create(forge_generators::POLYGON, None, None)?;
            tx.set_param(
                id,
                "mesh",
                Value::Mesh(Arc::new(forge_generators::from_poly_mesh(&mesh))),
            )?;
            Ok(id)
        })
        .unwrap();
    let initial = uv_commands::read(app.doc.scene(), obj).unwrap().uv;
    app.run("uv.seam", json!({"object":obj.0, "edge":0, "marked":true}))
        .unwrap();
    let edit_depth = app.doc.undo_depth();
    app.run(
        "uv.transform",
        json!({"object":obj.0, "corners":[0], "offset":[0.25,-0.125]}),
    )
    .unwrap();
    let changed = uv_commands::read(app.doc.scene(), obj).unwrap();
    assert_eq!(
        changed.uv.corners[0][0],
        [
            initial.corners[0][0][0] + 0.25,
            initial.corners[0][0][1] - 0.125
        ]
    );
    assert!(changed.seams.contains(&0));
    assert_eq!(app.doc.undo_depth(), edit_depth + 1);
    assert!(app.doc.undo());
    assert_eq!(uv_commands::read(app.doc.scene(), obj).unwrap().uv, initial);
    app.run("uv.relax", json!({"object":obj.0})).unwrap();
    app.run("uv.pack", json!({"object":obj.0, "padding":0.05}))
        .unwrap();
    let view = uv_commands::read(app.doc.scene(), obj).unwrap();
    assert!(view.seams.contains(&0));
    assert!(
        view.uv
            .corners
            .iter()
            .flatten()
            .flatten()
            .all(|c| (0.05..=0.95).contains(c))
    );
    assert_eq!(app.doc.scene().node(obj).unwrap().tags.len(), 1);
}
