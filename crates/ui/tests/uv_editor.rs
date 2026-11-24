//! U7: the UV workspace exposes commands against the active polygon.

use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_app::uv_commands;
use forge_core::Value;
use forge_geometry::PolyMesh;
use forge_ui::{
    app::Editor,
    dock::{Area, Panel},
    state::EditorState,
    workspaces::Workspace,
};
use glam::DVec3;

#[test]
fn uv_workspace_relax_and_pack_update_the_active_mesh() {
    let mut st = EditorState::new(Arc::new(forge_app::default_registry().unwrap()));
    let mesh = PolyMesh {
        points: vec![DVec3::ZERO, DVec3::X, DVec3::X + DVec3::Y, DVec3::Y],
        polys: vec![vec![0, 1, 2, 3]],
    };
    let id = st
        .doc
        .transact("quad", |tx| {
            let id = tx.create(forge_generators::POLYGON, None, None)?;
            tx.set_param(
                id,
                "mesh",
                Value::Mesh(Arc::new(forge_generators::from_poly_mesh(&mesh))),
            )?;
            Ok(id)
        })
        .unwrap();
    st.selection.click(id);
    st.dock = Workspace::Uv.layout().dock;
    assert_eq!(st.dock.active(Area::Center), Some(Panel::UvEditor));
    let initial = uv_commands::read(st.doc.scene(), id).unwrap().uv;
    let mut h = Harness::builder()
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, editor: &mut Editor| editor.show(ui, None),
            Editor::new(st),
        );
    h.get_by_label("Relax").click();
    h.run();
    let relaxed = uv_commands::read(h.state().state.doc.scene(), id)
        .unwrap()
        .uv;
    assert_ne!(relaxed, initial);
    h.get_by_label("Pack").click();
    h.run();
    let packed = uv_commands::read(h.state().state.doc.scene(), id)
        .unwrap()
        .uv;
    assert_ne!(packed, relaxed);
    assert!(
        packed
            .corners
            .iter()
            .flatten()
            .flatten()
            .all(|c| (0.0..=1.0).contains(c))
    );
}
