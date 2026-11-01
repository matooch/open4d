//! The editor answers RPC requests with its own context (docs/specs/RPC.md §5, §7).

use std::sync::Arc;

use forge_rpc::{Client, Server, ServerConfig};
use forge_ui::state::EditorState;
use serde_json::json;

#[test]
fn editor_state_serves_commands_including_view_commands() {
    let registry = forge_app::default_registry().unwrap();
    let mut state = EditorState::new(Arc::new(registry));
    let (server, inbox) = Server::start(ServerConfig {
        port: 0,
        token: Some("t".into()),
        insecure_local: false,
    })
    .unwrap();
    let port = server.port();
    // The client runs on another thread; this (the "UI") thread drains between "frames".
    let client = std::thread::spawn(move || {
        let mut c = Client::connect(port, Some("t")).unwrap();
        let made = c
            .command_run("object.create", json!({"type": "forge.cube"}))
            .unwrap();
        let node = made["node"].as_u64().unwrap();
        c.command_run("view.shading", json!({"mode": "flat"}))
            .unwrap();
        let mesh = c.eval_mesh(node, None).unwrap();
        (node, mesh.points.len(), c.scene_get().unwrap())
    });
    while !client.is_finished() {
        inbox.drain(&mut state);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let (node, points, scene) = client.join().unwrap();
    assert_eq!(points, 8);
    assert_eq!(scene["roots"][0]["id"], node);
    assert_eq!(state.shading, forge_viewport::render::Shading::Flat);
    assert!(state.doc.scene().contains(forge_core::NodeId(node)));
}
