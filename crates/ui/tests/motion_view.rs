//! Motion paths and onion skin in the editor (docs/specs/ANIM.md section 2b.4,
//! VIEWPORT.md section 12): the commands, the draw data of a scene, undo and scrub.

use std::collections::BTreeSet;
use std::sync::Arc;

use forge_command::MotionDisplay;
use forge_core::{Interp, Key, NodeId};
use forge_ui::motion_view::MotionState;
use forge_ui::state::EditorState;
use serde_json::json;

fn state() -> EditorState {
    EditorState::new(Arc::new(forge_app::default_registry().unwrap()))
}

fn run(s: &mut EditorState, id: &str, args: serde_json::Value) -> Result<(), String> {
    s.app
        .commands
        .clone()
        .run(s, id, args)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn animated_cube(s: &mut EditorState) -> NodeId {
    s.app
        .doc
        .transact("cube", |tx| {
            let c = tx.create("forge.cube", None, None)?;
            tx.set_key(
                c,
                "transform.pos.x",
                Key::new(0.0, 0.0).with_interp(Interp::Linear),
            )?;
            tx.set_key(c, "transform.pos.x", Key::new(2.0, 400.0))?;
            Ok(c)
        })
        .unwrap()
}

#[test]
fn paths_command_validates_toggles_and_is_not_an_undo_step() {
    let mut s = state();
    let depth = s.app.doc.undo_depth();
    assert_eq!(s.views.motion, MotionDisplay::default());
    run(&mut s, "view.motion_paths", json!({})).unwrap();
    assert!(s.views.motion.paths, "no field toggles on");
    run(&mut s, "view.motion_paths", json!({})).unwrap();
    assert!(!s.views.motion.paths, "and off again");
    run(
        &mut s,
        "view.motion_paths",
        json!({"enabled": true, "start": 5, "end": 40, "step": 5}),
    )
    .unwrap();
    assert_eq!(
        (
            s.views.motion.paths,
            s.views.motion.range,
            s.views.motion.step
        ),
        (true, Some((5, 40)), 5)
    );
    // a setting alone keeps the enabled state
    run(&mut s, "view.motion_paths", json!({"step": 2})).unwrap();
    assert!(s.views.motion.paths && s.views.motion.step == 2);
    run(&mut s, "view.motion_paths", json!({"reset_range": true})).unwrap();
    assert_eq!(s.views.motion.range, None);
    // bad input changes nothing
    let before = s.views.motion.clone();
    for bad in [
        json!({"start": 1}),
        json!({"start": 9, "end": 3}),
        json!({"step": 0}),
        json!({"step": 1001}),
        json!({"start": 1, "end": 2, "reset_range": true}),
        json!({"bogus": 1}),
    ] {
        assert!(
            run(&mut s, "view.motion_paths", bad.clone()).is_err(),
            "{bad}"
        );
    }
    assert_eq!(s.views.motion, before);
    assert_eq!(s.app.doc.undo_depth(), depth, "view state, no undo step");
}

#[test]
fn onion_command_validates_and_toggles() {
    let mut s = state();
    run(&mut s, "view.onion_skin", json!({})).unwrap();
    assert!(s.views.motion.onion);
    run(
        &mut s,
        "view.onion_skin",
        json!({"before": 4, "after": 0, "step": 3}),
    )
    .unwrap();
    let m = &s.views.motion;
    assert_eq!((m.onion, m.before, m.after, m.onion_step), (true, 4, 0, 3));
    for bad in [
        json!({"before": 11}),
        json!({"after": 11}),
        json!({"step": 0}),
        json!({"step": 101}),
        json!({"before": -1}),
    ] {
        assert!(
            run(&mut s, "view.onion_skin", bad.clone()).is_err(),
            "{bad}"
        );
    }
    assert_eq!(s.views.motion.before, 4);
    run(&mut s, "view.onion_skin", json!({"enabled": false})).unwrap();
    assert!(!s.views.motion.onion);
}

#[test]
fn draw_data_for_the_selected_animated_cube() {
    let mut s = state();
    let c = animated_cube(&mut s);
    let other = s
        .app
        .doc
        .transact("still", |tx| tx.create("forge.cube", None, None))
        .unwrap();
    let mut m = MotionState::synchronous();
    let display = MotionDisplay {
        paths: true,
        onion: true,
        before: 1,
        after: 1,
        onion_step: 5,
        ..MotionDisplay::default()
    };
    let scene = s.app.doc.scene().clone();
    let none = BTreeSet::new();
    let (d, pending) = m.draw(&scene, &scene, &none, &display, 15);
    assert!(d.is_empty() && !pending, "nothing selected, nothing drawn");

    let sel = BTreeSet::from([c, other]);
    let (d, _) = m.draw(&scene, &scene, &sel, &display, 15);
    assert_eq!(d.paths.len(), 1, "the still cube has no path");
    let p = &d.paths[0];
    assert_eq!(p.samples.first().unwrap().0, 0.0);
    assert_eq!(p.samples.last().unwrap().0, 90.0);
    assert_eq!(p.keys.len(), 2, "keys at frames 0 and 60");
    let (frame, pos) = p.current.unwrap();
    assert_eq!(frame, 15.0);
    let expect = forge_anim::scene_at(&scene, 0.5)
        .world_matrix(c)
        .unwrap()
        .transform_point3(glam::DVec3::ZERO);
    let shown = forge_anim::scene_at(&scene, 0.5);
    let (d2, _) = m.draw(&scene, &shown, &sel, &display, 15);
    assert_eq!(d2.paths[0].current.unwrap().1, expect);
    let _ = pos;
    // two nodes with geometry, two ghosts each (before and after)
    assert_eq!(d.ghosts.len(), 4);
    assert_eq!(d.ghosts.iter().filter(|g| g.forward).count(), 2);
    assert!(
        d.ghosts
            .iter()
            .all(|g| g.of == 1 && g.rank == 1 && !g.lines.is_empty())
    );

    // the current frame outside a custom range draws no current marker
    let late = MotionDisplay {
        range: Some((0, 10)),
        ..display.clone()
    };
    let (d, _) = m.draw(&scene, &scene, &sel, &late, 50);
    assert!(d.paths[0].current.is_none());
    assert_eq!(d.paths[0].samples.last().unwrap().0, 10.0);

    // disabled displays draw nothing
    let off = MotionDisplay::default();
    assert!(m.draw(&scene, &scene, &sel, &off, 15).0.is_empty());
}

#[test]
fn edits_undo_and_scrubbing_never_show_a_wrong_path() {
    let mut s = state();
    let c = animated_cube(&mut s);
    let sel = BTreeSet::from([c]);
    let display = MotionDisplay {
        paths: true,
        onion: true,
        ..MotionDisplay::default()
    };
    let mut m = MotionState::synchronous();
    let last_x = |m: &mut MotionState, s: &EditorState, frame: i64| {
        let sc = s.app.doc.scene().clone();
        let (d, _) = m.draw(&sc, &sc, &sel, &display, frame);
        d.paths[0].samples.last().unwrap().1.x
    };
    assert!((last_x(&mut m, &s, 0) - 400.0).abs() < 1e-6);
    s.app
        .doc
        .transact("edit", |tx| {
            tx.set_key(c, "transform.pos.x", Key::new(3.0, 900.0))
        })
        .unwrap();
    // frame 90 is 3 s: the new key is the last value
    assert!(
        (last_x(&mut m, &s, 0) - 900.0).abs() < 1e-6,
        "track edit invalidates"
    );
    assert!(s.app.doc.undo());
    assert!(
        (last_x(&mut m, &s, 0) - 400.0).abs() < 1e-6,
        "undo invalidates"
    );
    assert!(s.app.doc.redo());
    assert!(
        (last_x(&mut m, &s, 0) - 900.0).abs() < 1e-6,
        "redo invalidates"
    );
    // scrubbing leaves the path alone and moves the ghosts
    let sc = s.app.doc.scene().clone();
    let (a, _) = m.draw(&sc, &sc, &sel, &display, 30);
    let (b, _) = m.draw(&sc, &sc, &sel, &display, 60);
    assert_eq!(a.paths[0].samples, b.paths[0].samples);
    assert_ne!(a.ghosts[0].lines, b.ghosts[0].lines);
    // and back: the same frame gives the same ghosts (deterministic, no hidden state)
    let (a2, _) = m.draw(&sc, &sc, &sel, &display, 30);
    assert_eq!(a.ghosts, a2.ghosts);
    assert_eq!(a.paths, a2.paths);
}

#[test]
fn a_background_result_equals_the_inline_one() {
    let mut s = state();
    let c = animated_cube(&mut s);
    let sel = BTreeSet::from([c]);
    let display = MotionDisplay {
        paths: true,
        onion: true,
        ..MotionDisplay::default()
    };
    let sc = s.app.doc.scene().clone();
    let (inline, _) = MotionState::synchronous().draw(&sc, &sc, &sel, &display, 20);
    let mut bg = MotionState::default();
    let mut got = None;
    for _ in 0..5000 {
        let (d, pending) = bg.draw(&sc, &sc, &sel, &display, 20);
        if !pending {
            got = Some(d);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(got.expect("job finished"), inline);
}

#[test]
fn screenshot_contains_the_path() {
    let mut s = state();
    let c = animated_cube(&mut s);
    s.app.selection.click(c);
    let plain = forge_ui::screenshot::render_png(&mut s, 200, 150);
    // no GPU in this environment: nothing to compare
    let Ok(plain) = plain else { return };
    run(&mut s, "view.motion_paths", json!({"enabled": true})).unwrap();
    let with = forge_ui::screenshot::render_png(&mut s, 200, 150).unwrap();
    assert_ne!(plain, with, "the path changes the picture");
}
