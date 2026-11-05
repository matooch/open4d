//! Viewport HUD and overlays (docs/specs/UI.md §23; brief 157): pure draw list, safe frames,
//! counts, frame timer, settings and layout persistence, the `view.hud` command, a rasterised
//! baseline image of the draw list, and egui_kittest interactions with the View menu.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::TimeDisplay;
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::hud::{
    Anchor, Counts, FrameTimer, HudContext, HudFlags, HudItem, HudPrim, HudSettings, LINE, MARGIN,
    TOP, Tone, counts, draw_list, format_count, safe_frames,
};
use forge_ui::layout::Layout;
use forge_ui::workspaces::Workspace;
use forge_viewport::panes::Pane;
use serde_json::json;

fn sample() -> HudContext {
    HudContext {
        fps: Some((59.94, 16.68)),
        scene: Counts {
            objects: 12,
            points: 1_234_567,
            polys: 1200,
        },
        selection: Counts {
            objects: 1,
            points: 8,
            polys: 6,
        },
        tool: "Move".into(),
        axis_mode: false,
        snap: None,
        frame: 12,
        end: 90,
        scene_fps: 30,
        display: TimeDisplay::Frames,
        pane: "Perspective".into(),
        projection: "Perspective, fov 45\u{b0}".into(),
        render_size: None,
    }
}

fn all_on() -> HudFlags {
    HudFlags {
        fps: true,
        counts: true,
        tool: true,
        frame: true,
        camera: true,
        safe_frames: true,
    }
}

fn texts(list: &[HudPrim]) -> Vec<String> {
    list.iter()
        .filter_map(|p| match p {
            HudPrim::Text { text, .. } => Some(text.clone()),
            HudPrim::Rect { .. } => None,
        })
        .collect()
}

// ---- pure ---------------------------------------------------------------------------

#[test]
fn counts_group_thousands() {
    assert_eq!(format_count(0), "0");
    assert_eq!(format_count(999), "999");
    assert_eq!(format_count(1000), "1,000");
    assert_eq!(format_count(1_234_567), "1,234,567");
    assert_eq!(format_count(12_345), "12,345");
}

#[test]
fn safe_frames_without_a_render_size_use_the_pane() {
    let s = safe_frames([400.0, 200.0], None);
    assert_eq!(s.frame, [0.0, 0.0, 400.0, 200.0]);
    assert_eq!(s.action, [20.0, 10.0, 380.0, 190.0]);
    assert_eq!(s.title, [40.0, 20.0, 360.0, 180.0]);
    // a zero size counts as absent
    assert_eq!(safe_frames([400.0, 200.0], Some([0, 1080])), s);
    assert_eq!(safe_frames([400.0, 200.0], Some([1920, 0])), s);
}

#[test]
fn safe_frames_fit_the_render_aspect_into_the_pane() {
    // a wide render in a square pane: full width, letterboxed
    let s = safe_frames([400.0, 400.0], Some([1920, 1080]));
    assert_eq!(s.frame, [0.0, 87.5, 400.0, 312.5]);
    // a tall pane and a wide render, and the reverse
    let s = safe_frames([300.0, 800.0], Some([1000, 500]));
    assert_eq!(s.frame, [0.0, 325.0, 300.0, 475.0]);
    let s = safe_frames([800.0, 300.0], Some([400, 400]));
    assert_eq!(s.frame, [250.0, 0.0, 550.0, 300.0]);
    // the render aspect equal to the pane's: the whole pane
    let s = safe_frames([400.0, 225.0], Some([1920, 1080]));
    assert!(
        s.frame
            .iter()
            .zip([0.0, 0.0, 400.0, 225.0])
            .all(|(a, b)| (a - b).abs() < 1e-3)
    );
    // the safe areas are centred on the frame and 90 % and 80 % of its size
    let s = safe_frames([800.0, 300.0], Some([400, 400]));
    assert_eq!(s.action, [265.0, 15.0, 535.0, 285.0]);
    assert_eq!(s.title, [280.0, 30.0, 520.0, 270.0]);
}

#[test]
fn nothing_enabled_draws_nothing_and_each_item_only_its_own_primitives() {
    let c = sample();
    assert!(draw_list(&c, &HudFlags::default(), [400.0, 300.0]).is_empty());
    for item in HudItem::ALL {
        let mut f = HudFlags::default();
        f.set(item, true);
        assert!(f.any() && f.get(item));
        let list = draw_list(&c, &f, [400.0, 300.0]);
        assert!(!list.is_empty(), "{item:?}");
        // the other items are untouched: the same list as with that one flag alone
        let mut g = all_on();
        for other in HudItem::ALL {
            if other != item {
                g.set(other, false);
            }
        }
        assert_eq!(list, draw_list(&c, &g, [400.0, 300.0]), "{item:?}");
    }
}

#[test]
fn item_texts() {
    let c = sample();
    let t = |item: HudItem, c: &HudContext| {
        let mut f = HudFlags::default();
        f.set(item, true);
        texts(&draw_list(c, &f, [400.0, 300.0]))
    };
    assert_eq!(t(HudItem::Fps, &c), ["59.9 fps  16.7 ms"]);
    assert_eq!(
        t(HudItem::Counts, &c),
        [
            "Objects  12  (selected 1)",
            "Points  1,234,567  (selected 8)",
            "Polygons  1,200  (selected 6)"
        ]
    );
    assert_eq!(t(HudItem::Tool, &c), ["Tool  Move", "Snap  off"]);
    assert_eq!(t(HudItem::Frame, &c), ["Frame  12  /  90"]);
    assert_eq!(
        t(HudItem::Camera, &c),
        ["Camera  Perspective (Perspective, fov 45\u{b0})"]
    );
    assert!(t(HudItem::SafeFrames, &c).is_empty());
    // before the first measurement
    let early = HudContext {
        fps: None,
        ..sample()
    };
    assert_eq!(t(HudItem::Fps, &early), ["-- fps"]);
    // snap and axis mode
    let snapping = HudContext {
        snap: Some("3D".into()),
        axis_mode: true,
        tool: "Bevel".into(),
        ..sample()
    };
    assert_eq!(
        t(HudItem::Tool, &snapping),
        ["Tool  Bevel  Axis mode", "Snap  on  3D"]
    );
}

#[test]
fn the_frame_item_follows_the_time_display_format() {
    let f = HudFlags {
        frame: true,
        ..Default::default()
    };
    let at = |display| {
        let c = HudContext {
            display,
            frame: 45,
            end: 90,
            ..sample()
        };
        texts(&draw_list(&c, &f, [400.0, 300.0]))
    };
    assert_eq!(at(TimeDisplay::Frames), ["Frame  45  /  90"]);
    assert_eq!(at(TimeDisplay::Seconds), ["Frame  1.500 s  /  3.000 s"]);
    assert_eq!(
        at(TimeDisplay::Smpte),
        ["Frame  00:00:01:15  /  00:00:03:00"]
    );
}

#[test]
fn placement_of_each_group() {
    let c = sample();
    let list = draw_list(&c, &all_on(), [400.0, 300.0]);
    let pos = |text: &str| {
        list.iter()
            .find_map(|p| match p {
                HudPrim::Text {
                    pos,
                    anchor,
                    text: t,
                    ..
                } if t.starts_with(text) => Some((*pos, *anchor)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{text}"))
    };
    // the left column under the pane label, in order: camera, tool, snap, three counts
    let column = ["Camera", "Tool", "Snap", "Objects", "Points", "Polygons"];
    for (i, label) in column.iter().enumerate() {
        assert_eq!(
            pos(label),
            ([MARGIN, TOP + LINE * i as f32], Anchor::LeftTop),
            "{label}"
        );
    }
    assert_eq!(pos("59.9"), ([392.0, 6.0], Anchor::RightTop));
    assert_eq!(pos("Frame"), ([392.0, 294.0], Anchor::RightBottom));
    // the outlines come first so text is drawn over them
    assert!(matches!(list[0], HudPrim::Rect { .. }));
    assert_eq!(
        list.iter()
            .filter(|p| matches!(p, HudPrim::Rect { .. }))
            .count(),
        2
    );
}

#[test]
fn safe_frame_outlines_follow_the_render_size() {
    let f = HudFlags {
        safe_frames: true,
        ..Default::default()
    };
    let none = draw_list(&sample(), &f, [400.0, 200.0]);
    assert_eq!(
        none,
        [
            HudPrim::Rect {
                min: [20.0, 10.0],
                max: [380.0, 190.0],
                tone: Tone::Guide
            },
            HudPrim::Rect {
                min: [40.0, 20.0],
                max: [360.0, 180.0],
                tone: Tone::Guide
            },
        ]
    );
    let c = HudContext {
        render_size: Some([400, 400]),
        ..sample()
    };
    let with = draw_list(&c, &f, [400.0, 200.0]);
    assert_eq!(with.len(), 3);
    assert_eq!(
        with[0],
        HudPrim::Rect {
            min: [100.0, 0.0],
            max: [300.0, 200.0],
            tone: Tone::Weak
        }
    );
}

#[test]
fn the_frame_timer_smooths_and_ignores_bad_samples() {
    let mut t = FrameTimer::default();
    assert_eq!((t.fps(), t.ms()), (None, None));
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        t.push(bad);
    }
    assert_eq!(t.fps(), None);
    t.push(0.02);
    assert_eq!(t.fps(), Some(50.0));
    assert!((t.ms().unwrap() - 20.0).abs() < 1e-9);
    t.push(0.04);
    // 0.02 + (0.04 - 0.02) * 0.1
    assert!((t.ms().unwrap() - 22.0).abs() < 1e-9);
    t.push(f64::NAN);
    assert!((t.ms().unwrap() - 22.0).abs() < 1e-9);
}

#[test]
fn settings_default_off_round_trip_and_read_leniently() {
    let d = HudSettings::default();
    for p in Pane::ALL {
        assert!(!d.flags(p).any());
    }
    let mut s = HudSettings::default();
    s.set(Pane::Top, HudItem::Counts, true);
    s.set(Pane::Right, HudItem::SafeFrames, true);
    assert!(s.top.counts && s.right.safe_frames && !s.perspective.counts);
    let text = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<HudSettings>(&text).unwrap(), s);
    // missing and unknown fields
    let l: HudSettings =
        serde_json::from_str(r#"{"top": {"fps": true, "future": 1}, "elsewhere": {}}"#).unwrap();
    assert!(l.top.fps && !l.top.counts && !l.perspective.any());
    s.set_all(HudItem::Fps, true);
    assert!(Pane::ALL.iter().all(|p| s.flags(*p).fps));
}

#[test]
fn counts_of_the_drawn_objects_and_the_selection() {
    let mut s = state();
    let cube = create(&mut s, "forge.cube");
    let _sphere = create(&mut s, "forge.sphere");
    let null = create(&mut s, "forge.null");
    let frame = s.app.scheduled_draw(0);
    let mesh = |id| frame.outputs[&id].mesh.as_ref().unwrap();
    let (cp, cq) = (mesh(cube).points.len(), mesh(cube).polys.len());
    assert!(cp > 0 && cq > 0 && frame.outputs[&null].mesh.is_none());
    let (all, sel) = counts(&frame, &BTreeSet::from([cube]));
    assert_eq!(all.objects, 3, "a null is an object too");
    assert_eq!(
        all.points,
        frame
            .outputs
            .values()
            .filter_map(|o| o.mesh.as_ref())
            .map(|m| m.points.len())
            .sum::<usize>()
    );
    assert_eq!(
        sel,
        Counts {
            objects: 1,
            points: cp,
            polys: cq
        }
    );
    let (_, none) = counts(&frame, &BTreeSet::new());
    assert_eq!(none, Counts::default());
}

#[test]
fn hidden_objects_are_not_counted_and_instances_count_their_clones() {
    let mut s = forge_ui::state::EditorState::new(std::sync::Arc::new(
        forge_app::default_registry().unwrap(),
    ));
    let cube = create(&mut s, "forge.cube");
    let hidden = create(&mut s, "forge.sphere");
    execute(
        &mut s,
        "object.visibility",
        json!({"nodes": [hidden.0], "editor": "off"}),
    )
    .unwrap();
    let frame = s.app.scheduled_draw(0);
    let (all, _) = counts(&frame, &BTreeSet::new());
    assert_eq!(all.objects, 1);
    // a cloner drawn by instancing: prototype geometry times visible clones
    s.selection.click(cube);
    let r = execute(&mut s, "object.create_cloner", json!({})).unwrap();
    let cloner = forge_core::NodeId(r["node"].as_u64().unwrap());
    let frame = s.app.scheduled_draw(0);
    let out = &frame.outputs[&cloner];
    let (all, sel) = counts(&frame, &BTreeSet::from([cloner]));
    let per_clone = {
        let mut f2 = forge_ui::state::EditorState::new(std::sync::Arc::new(
            forge_app::default_registry().unwrap(),
        ));
        let c = create(&mut f2, "forge.cube");
        f2.app.scheduled_draw(0).outputs[&c]
            .mesh
            .as_ref()
            .unwrap()
            .points
            .len()
    };
    let n = match (&out.mesh, &out.instances) {
        (Some(m), None) => m.points.len() / per_clone,
        (None, Some(i)) => i.clones.visible.iter().filter(|v| **v).count(),
        _ => panic!("a cloner has an output"),
    };
    assert!(n > 1);
    assert_eq!(all.points, n * per_clone);
    assert_eq!(sel.points, n * per_clone);
}

#[test]
fn the_context_reads_tool_snap_camera_and_time_from_the_editor_state() {
    let mut s = state();
    create(&mut s, "forge.cube");
    execute(&mut s, "view.gizmo_mode", json!({"mode": "rotate"})).unwrap();
    let frame = s.app.scheduled_draw(0);
    let mut timer = FrameTimer::default();
    timer.push(0.016);
    let c = forge_ui::hud::context(&s, Pane::Top, &frame, &timer);
    assert_eq!(c.tool, "Rotate");
    assert_eq!(c.snap, None);
    assert_eq!(c.pane, "Top");
    assert_eq!(c.projection, "Orthographic");
    assert_eq!((c.frame, c.end, c.scene_fps), (0, 90, 30));
    assert_eq!(c.display, TimeDisplay::Frames);
    assert_eq!(c.render_size, None);
    assert!((c.fps.unwrap().0 - 62.5).abs() < 1e-9);
    s.app.transform_settings.snap.enabled = true;
    s.app.transform_settings.axis_mode = true;
    let c = forge_ui::hud::context(&s, Pane::Perspective, &frame, &timer);
    assert_eq!(c.snap.as_deref(), Some("3D"));
    assert!(c.axis_mode);
    assert!(
        c.projection.starts_with("Perspective, fov "),
        "{}",
        c.projection
    );
}

#[test]
fn layouts_carry_the_hud_only_when_they_have_one() {
    let std_layout = Layout::standard();
    assert!(std_layout.hud.is_none());
    assert!(!std_layout.to_json().contains("hud"));
    // an old layout file (no hud field) still loads
    assert!(
        Layout::from_json(&std_layout.to_json())
            .unwrap()
            .hud
            .is_none()
    );
    let mut with = Layout::standard();
    let mut h = HudSettings::default();
    h.set(Pane::Front, HudItem::Tool, true);
    with.hud = Some(h);
    let back = Layout::from_json(&with.to_json()).unwrap();
    assert_eq!(back, with);
    assert!(back.hud.unwrap().front.tool);
    for w in Workspace::ALL {
        assert!(w.layout().hud.is_none(), "{}", w.name());
    }
}

// ---- the command ----------------------------------------------------------------------

#[test]
fn view_hud_toggles_one_pane_all_panes_or_sets() {
    let mut s = state();
    let depth = s.doc.undo_depth();
    // the active pane, toggled
    execute(&mut s, "view.hud", json!({"item": "counts"})).unwrap();
    assert!(s.hud.perspective.counts && !s.hud.top.counts);
    execute(&mut s, "view.hud", json!({"item": "counts"})).unwrap();
    assert!(!s.hud.perspective.counts);
    // a named pane, set explicitly (idempotent)
    for _ in 0..2 {
        execute(
            &mut s,
            "view.hud",
            json!({"item": "fps", "pane": "front", "enabled": true}),
        )
        .unwrap();
        assert!(s.hud.front.fps && !s.hud.perspective.fps);
    }
    execute(
        &mut s,
        "view.hud",
        json!({"item": "fps", "pane": "front", "enabled": false}),
    )
    .unwrap();
    assert!(!s.hud.front.fps);
    // all panes
    execute(
        &mut s,
        "view.hud",
        json!({"item": "safe_frames", "all_panes": true, "enabled": true}),
    )
    .unwrap();
    assert!(Pane::ALL.iter().all(|p| s.hud.flags(*p).safe_frames));
    execute(
        &mut s,
        "view.hud",
        json!({"item": "safe_frames", "all_panes": true}),
    )
    .unwrap();
    assert!(Pane::ALL.iter().all(|p| !s.hud.flags(*p).safe_frames));
    // view state: no undo steps, document untouched
    assert_eq!(s.doc.undo_depth(), depth);
    // the active pane follows view.set_view's pane switching
    s.activate(Pane::Right);
    execute(&mut s, "view.hud", json!({"item": "camera"})).unwrap();
    assert!(s.hud.right.camera && !s.hud.perspective.camera);
}

#[test]
fn view_hud_rejects_bad_arguments() {
    let mut s = state();
    assert!(execute(&mut s, "view.hud", json!({"item": "nope"})).is_err());
    assert!(execute(&mut s, "view.hud", json!({})).is_err());
    assert!(execute(&mut s, "view.hud", json!({"item": "fps", "pane": "back"})).is_err());
    assert!(execute(&mut s, "view.hud", json!({"item": "fps", "extra": 1})).is_err());
    assert_eq!(s.hud, HudSettings::default());
}

// ---- baseline image ---------------------------------------------------------------------

const W: u32 = 360;
const H: u32 = 220;

fn tone_rgb(t: Tone) -> [u8; 3] {
    match t {
        Tone::Normal => [214, 217, 222],
        Tone::Weak => [154, 160, 168],
        Tone::Guide => [224, 138, 46],
    }
}

/// Rasterises a draw list: outlines as 1 pixel lines, text as a 6 x 10 box per character
/// (no fonts involved, so the image is exact everywhere).
fn raster(list: &[HudPrim]) -> Vec<u8> {
    let mut px = vec![0u8; (W * H * 4) as usize];
    for p in px.chunks_mut(4) {
        p.copy_from_slice(&[30, 32, 36, 255]);
    }
    let mut put = |x: i64, y: i64, c: [u8; 3]| {
        if (0..W as i64).contains(&x) && (0..H as i64).contains(&y) {
            let i = ((y as u32 * W + x as u32) * 4) as usize;
            px[i..i + 3].copy_from_slice(&c);
        }
    };
    for prim in list {
        match prim {
            HudPrim::Rect { min, max, tone } => {
                let c = tone_rgb(*tone);
                let (x0, y0, x1, y1) = (
                    min[0].round() as i64,
                    min[1].round() as i64,
                    max[0].round() as i64 - 1,
                    max[1].round() as i64 - 1,
                );
                for x in x0..=x1 {
                    put(x, y0, c);
                    put(x, y1, c);
                }
                for y in y0..=y1 {
                    put(x0, y, c);
                    put(x1, y, c);
                }
            }
            HudPrim::Text {
                pos,
                anchor,
                text,
                tone,
            } => {
                let w = text.chars().count() as i64 * 6;
                let (x, y) = (pos[0].round() as i64, pos[1].round() as i64);
                let (x0, y0) = match anchor {
                    Anchor::LeftTop => (x, y),
                    Anchor::RightTop => (x - w, y),
                    Anchor::RightBottom => (x - w, y - 10),
                };
                let c = tone_rgb(*tone);
                for (k, ch) in text.chars().enumerate() {
                    if ch == ' ' {
                        continue;
                    }
                    for dy in 1..9 {
                        for dx in 1..5 {
                            put(x0 + k as i64 * 6 + dx, y0 + dy, c);
                        }
                    }
                }
            }
        }
    }
    px
}

fn baseline_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/baselines")
        .join(format!("{name}.png"))
}

fn check_baseline(name: &str, rgba: &[u8]) {
    let path = baseline_path(name);
    if std::env::var("FORGE_UPDATE_BASELINES").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = std::fs::File::create(&path).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), W, H);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(rgba).unwrap();
        return;
    }
    let file = std::fs::File::open(&path).unwrap_or_else(|_| {
        panic!(
            "missing baseline {}: run FORGE_UPDATE_BASELINES=1 cargo test -p forge-ui --test hud",
            path.display()
        )
    });
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (W, H), "baseline size");
    buf.truncate(info.buffer_size());
    let differing = buf
        .chunks(4)
        .zip(rgba.chunks(4))
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        differing, 0,
        "{name}: {differing} pixels differ from the baseline"
    );
}

#[test]
fn baseline_image_every_item() {
    let list = draw_list(&sample(), &all_on(), [W as f32, H as f32]);
    check_baseline("hud_all", &raster(&list));
}

#[test]
fn baseline_image_safe_frames_with_a_render_size() {
    let c = HudContext {
        render_size: Some([1920, 1080]),
        snap: Some("2D".into()),
        axis_mode: true,
        display: TimeDisplay::Smpte,
        ..sample()
    };
    let mut f = all_on();
    f.counts = false;
    let list = draw_list(&c, &f, [W as f32, H as f32]);
    check_baseline("hud_safe_render", &raster(&list));
    // the baseline is not blank, and differs from the other one
    let other = raster(&draw_list(&sample(), &all_on(), [W as f32, H as f32]));
    assert_ne!(raster(&list), other);
}

// ---- egui_kittest -----------------------------------------------------------------------

fn harness_with(mut editor: Editor) -> Harness<'static, Editor> {
    editor.state.selection.clear();
    let mut h = Harness::builder()
        .with_size([1200.0, 800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    h.run();
    h
}

fn harness() -> Harness<'static, Editor> {
    let mut st = state();
    create(&mut st, "forge.cube");
    create(&mut st, "forge.sphere");
    harness_with(Editor::new(st))
}

fn click(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_by_label(label).click();
    h.run();
}

fn last(h: &Harness<'_, Editor>) -> Vec<(Pane, Vec<String>)> {
    h.state()
        .state
        .hud_last
        .iter()
        .map(|(p, l)| (*p, texts(l)))
        .collect()
}

#[test]
fn the_hud_is_off_by_default() {
    let h = harness();
    assert!(h.state().state.hud_last.is_empty());
}

#[test]
fn the_view_menu_toggles_items_per_pane() {
    let mut h = harness();
    click(&mut h, "View");
    click(&mut h, "HUD");
    click(&mut h, "Perspective");
    click(&mut h, "Counts");
    assert!(h.state().state.hud.perspective.counts);
    assert!(!h.state().state.hud.top.counts);
    let got = last(&h);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].0, Pane::Perspective);
    assert_eq!(got[0].1[0], "Objects  2  (selected 0)");
    // frame item too
    click(&mut h, "View");
    click(&mut h, "HUD");
    click(&mut h, "Perspective");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Frame")
        .click();
    h.run();
    let got = last(&h);
    assert_eq!(got[0].1.last().unwrap(), "Frame  0  /  90");
    // switch the counts off again through the same check box
    click(&mut h, "View");
    click(&mut h, "HUD");
    click(&mut h, "Perspective");
    click(&mut h, "Counts");
    assert!(!h.state().state.hud.perspective.counts);
    assert_eq!(last(&h)[0].1, ["Frame  0  /  90"]);
}

#[test]
fn only_the_visible_panes_with_items_paint() {
    let mut h = harness();
    let st = &mut h.state_mut().state;
    execute(
        st,
        "view.hud",
        json!({"item": "camera", "all_panes": true, "enabled": true}),
    )
    .unwrap();
    h.run();
    // single layout: one pane
    assert_eq!(last(&h).len(), 1);
    execute(
        &mut h.state_mut().state,
        "view.layout",
        json!({"layout": "four"}),
    )
    .unwrap();
    h.run();
    let got = last(&h);
    assert_eq!(got.len(), 4);
    let camera: Vec<&str> = got.iter().map(|(_, t)| t[0].as_str()).collect();
    assert!(camera.contains(&"Camera  Top (Orthographic)"), "{camera:?}");
    assert!(
        camera
            .iter()
            .any(|c| c.starts_with("Camera  Perspective (Perspective, fov"))
    );
    // one pane only: the others stay clean
    execute(
        &mut h.state_mut().state,
        "view.hud",
        json!({"item": "camera", "all_panes": true, "enabled": false}),
    )
    .unwrap();
    execute(
        &mut h.state_mut().state,
        "view.hud",
        json!({"item": "tool", "pane": "front", "enabled": true}),
    )
    .unwrap();
    h.run();
    let got = last(&h);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].0, Pane::Front);
    assert_eq!(got[0].1, ["Tool  Move", "Snap  off"]);
}

#[test]
fn counts_follow_the_selection_and_the_scene() {
    let mut h = harness();
    execute(
        &mut h.state_mut().state,
        "view.hud",
        json!({"item": "counts", "enabled": true}),
    )
    .unwrap();
    h.run();
    assert_eq!(last(&h)[0].1[0], "Objects  2  (selected 0)");
    let first = h.state().state.doc.scene().roots()[0];
    h.state_mut().state.selection.click(first);
    h.run();
    assert_eq!(last(&h)[0].1[0], "Objects  2  (selected 1)");
    execute(
        &mut h.state_mut().state,
        "object.delete",
        json!({"nodes": [first.0]}),
    )
    .unwrap();
    h.run();
    assert_eq!(last(&h)[0].1[0], "Objects  1  (selected 0)");
}

#[test]
fn the_frame_item_follows_the_timeline_and_display() {
    let mut h = harness();
    execute(
        &mut h.state_mut().state,
        "view.hud",
        json!({"item": "frame", "enabled": true}),
    )
    .unwrap();
    execute(
        &mut h.state_mut().state,
        "timeline.set_frame",
        json!({"frame": 45}),
    )
    .unwrap();
    h.run();
    assert_eq!(last(&h)[0].1, ["Frame  45  /  90"]);
    execute(
        &mut h.state_mut().state,
        "timeline.set_display",
        json!({"format": "smpte"}),
    )
    .unwrap();
    h.run();
    assert_eq!(last(&h)[0].1, ["Frame  00:00:01:15  /  00:00:03:00"]);
}

#[test]
fn layouts_save_and_restore_the_hud_and_workspaces_keep_it() {
    let mut h = harness();
    execute(
        &mut h.state_mut().state,
        "view.hud",
        json!({"item": "fps", "pane": "top", "enabled": true}),
    )
    .unwrap();
    let layout = h.state().layout();
    assert!(layout.hud.unwrap().top.fps);
    // a session without a HUD has no block
    assert!(harness().state().layout().hud.is_none());
    let json = layout.to_json();
    // a fresh editor applies the saved layout
    let mut fresh = harness();
    assert!(!fresh.state().state.hud.top.fps);
    fresh
        .state_mut()
        .set_layout(Layout::from_json(&json).unwrap());
    assert!(fresh.state().state.hud.top.fps);
    // a workspace switch does not touch it
    fresh.state_mut().set_layout(Workspace::Modeling.layout());
    assert!(fresh.state().state.hud.top.fps);
    // a saved layout without a hud block keeps the current one too
    let mut no_hud = Layout::standard();
    no_hud.hud = None;
    fresh.state_mut().set_layout(no_hud);
    assert!(fresh.state().state.hud.top.fps);
    // and one with an empty block clears it
    let mut empty = Layout::standard();
    empty.hud = Some(HudSettings::default());
    fresh.state_mut().set_layout(empty);
    assert!(!fresh.state().state.hud.top.fps);
}

#[test]
fn the_frame_rate_item_shows_a_measurement_after_frames() {
    let mut h = harness();
    execute(
        &mut h.state_mut().state,
        "view.hud",
        json!({"item": "fps", "enabled": true}),
    )
    .unwrap();
    for _ in 0..3 {
        h.step();
    }
    let t = last(&h)[0].1[0].clone();
    assert!(t.ends_with(" ms") && t.contains(" fps"), "{t}");
}
