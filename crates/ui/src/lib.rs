//! forge-ui: the editor shell. Spec: docs/specs/UI.md.
//!
//! UI-free models (`state`, `dock`, `tree`, `attributes`, `commands`, `timeline`,
//! `theme`, navigation in `viewport_panel`) are unit-tested; `app` draws them with egui.

pub mod animate;
pub mod app;
pub mod attributes;
pub mod commands;
pub mod component_edit;
pub mod components;
pub mod coordinates;
pub mod dock;
pub mod manipulate;
pub mod state;
pub mod theme;
pub mod timeline;
pub mod tree;
pub mod viewport_panel;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use forge_core::Registry;

/// Screenshot mode (`FORGE_SCREENSHOT=<png path>`): adds a small sample hierarchy,
/// captures the window after a few frames, writes the PNG and closes.
struct ScreenshotApp {
    editor: app::Editor,
    path: String,
    frames: u32,
    done: Arc<AtomicBool>,
}

impl eframe::App for ScreenshotApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.editor.show(ui, frame.wgpu_render_state());
        self.frames += 1;
        let ctx = ui.ctx().clone();
        if self.frames == 30 {
            let (path, done, repaint) = (self.path.clone(), self.done.clone(), ctx.clone());
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                egui::ScreenshotCallback::new(move |img| {
                    if let Err(e) = write_png(&path, &img) {
                        eprintln!("screenshot failed: {e}");
                    }
                    done.store(true, Ordering::SeqCst);
                    repaint.request_repaint();
                }),
            ));
        }
        if self.done.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

fn write_png(path: &str, img: &egui::ColorImage) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, img.size[0] as u32, img.size[1] as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
    enc.write_header()?.write_image_data(&bytes)?;
    Ok(())
}

fn sample_hierarchy(state: &mut state::EditorState) {
    use commands::{Command, execute};
    let make = |s: &mut state::EditorState, name: &str| {
        let _ = execute(s, Command::Create("forge.null".into()));
        let id = s.selection.active.expect("just created");
        let _ = execute(s, Command::Rename(id, name.into()));
        id
    };
    let rig = make(state, "Rig");
    let arm = make(state, "Arm");
    let hand = make(state, "Hand");
    make(state, "Camera target");
    let _ = execute(state, Command::Reparent(arm, Some(rig)));
    let _ = execute(state, Command::Reparent(hand, Some(arm)));
    state.expanded.extend([rig, arm]);
    state.selection.click(arm);

    // Primitives, when the registry has them: a floor plane and two objects on it.
    let place = |s: &mut state::EditorState, ty: &str, pos: [f64; 3]| {
        s.doc.scene().registry().get(ty)?;
        let _ = execute(s, Command::Create(ty.into()));
        let id = s.selection.active?;
        let _ = attributes::apply_edit(
            &mut s.doc,
            id,
            "transform.pos",
            forge_core::Value::Vec3(pos),
            attributes::Gesture::Commit,
        );
        Some(id)
    };
    place(state, "forge.plane", [0.0, 0.0, 0.0]);
    let cube = place(state, "forge.cube", [-150.0, 100.0, 0.0]);
    place(state, "forge.sphere", [150.0, 100.0, 50.0]);
    if let Some(cube) = cube {
        state.selection.click(cube);
        state
            .camera
            .frame([-400.0, 0.0, -400.0].into(), [400.0, 250.0, 400.0].into());
    }
}

/// Opens the editor window and runs until it is closed.
pub fn run(registry: Arc<Registry>) -> eframe::Result {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("Forge3D")
            .with_inner_size([1440.0, 900.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Forge3D",
        options,
        Box::new(move |cc| {
            theme::Theme::default().apply(&cc.egui_ctx);
            let mut state = state::EditorState::new(registry);
            let app: Box<dyn eframe::App> = match std::env::var("FORGE_SCREENSHOT") {
                Ok(path) => {
                    sample_hierarchy(&mut state);
                    Box::new(ScreenshotApp {
                        editor: app::Editor::new(state),
                        path,
                        frames: 0,
                        done: Arc::new(AtomicBool::new(false)),
                    })
                }
                Err(_) => Box::new(app::Editor::new(state)),
            };
            Ok(app)
        }),
    )
}
