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

/// Starts a server and serves it from this thread while `f` runs on a client thread.
fn with_editor<T: Send + 'static>(
    state: &mut EditorState,
    f: impl FnOnce(Client) -> T + Send + 'static,
) -> T {
    let (server, inbox) = Server::start(ServerConfig {
        port: 0,
        token: Some("t".into()),
        insecure_local: false,
    })
    .unwrap();
    let port = server.port();
    let client = std::thread::spawn(move || f(Client::connect(port, Some("t")).unwrap()));
    while !client.is_finished() {
        inbox.drain(state);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    client.join().unwrap()
}

#[test]
fn editor_notifies_frame_changes_between_frames() {
    let mut state = EditorState::new(Arc::new(forge_app::default_registry().unwrap()));
    let n = with_editor(&mut state, |mut c| {
        c.version().unwrap();
        c.subscribe(&["frame"]).unwrap();
        c.command_run("timeline.set_frame", json!({"frame": 5}))
            .unwrap();
        c.next_notification(std::time::Duration::from_secs(10))
            .unwrap()
    });
    let n = n.expect("frame.changed");
    assert_eq!(n.method, "frame.changed");
    assert_eq!(n.params, json!({"frame": 5}));
}

/// A missing GPU adapter skips the test, unless FORGE_REQUIRE_GPU=1 (Linux CI has lavapipe).
fn skip_without_adapter(message: &str) -> bool {
    if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") {
        panic!("FORGE_REQUIRE_GPU=1 but the screenshot failed: {message}");
    }
    eprintln!("skipping GPU test: {message}");
    true
}

#[test]
fn screenshot_of_a_cube_is_a_png_of_the_requested_size() {
    let mut state = EditorState::new(Arc::new(forge_app::default_registry().unwrap()));
    let (w, h) = (160u32, 120u32);
    let shot = with_editor(&mut state, move |mut c| {
        c.command_run("object.create", json!({"type": "forge.cube"}))
            .unwrap();
        c.screenshot(w, h)
    });
    let png_bytes = match shot {
        Ok(bytes) => bytes,
        Err(forge_rpc::ClientError::Rpc {
            code,
            message,
            data,
        }) => {
            assert_eq!(code, -32000);
            assert_eq!(data.unwrap()["kind"], "viewport");
            assert!(message.contains("no GPU"), "{message}");
            if skip_without_adapter(&message) {
                return;
            }
            unreachable!();
        }
        Err(other) => panic!("{other}"),
    };
    let mut reader = png::Decoder::new(std::io::Cursor::new(&png_bytes))
        .read_info()
        .expect("valid PNG");
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!((info.width, info.height), (w, h));
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    let at = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2]]
    };
    let corner = at(0, 0);
    let centre = at(w / 2, h / 2);
    assert_ne!(corner, centre, "the cube must cover the centre");
}

#[test]
fn screenshot_sizes_are_checked_without_a_gpu() {
    let mut state = EditorState::new(Arc::new(forge_app::default_registry().unwrap()));
    let codes = with_editor(&mut state, |mut c| {
        [(0, 10), (10, 0), (5000, 10)].map(|(w, h)| match c.screenshot(w, h).unwrap_err() {
            forge_rpc::ClientError::Rpc { code, .. } => code,
            other => panic!("{other}"),
        })
    });
    assert_eq!(codes, [-32602; 3]);
}
