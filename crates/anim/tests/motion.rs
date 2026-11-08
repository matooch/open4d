//! docs/specs/ANIM.md section 2b: motion path sampling and onion frames.

use std::sync::Arc;

use forge_anim::motion::{MAX_PATH_NODES, MAX_PATH_SAMPLES, PathRange, onion_frames, sample_paths};
use forge_anim::scene_at;
use forge_core::{Document, Interp, Key, NodeId, Registry, Value};
use forge_generators::Primitives;
use glam::DVec3;

const FPS: f64 = 30.0;

fn doc() -> Document {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    Document::new(Arc::new(reg))
}

/// A cube moving linearly along x: 0 at frame 0, 300 at frame 30 (1 s).
fn moving_cube(doc: &mut Document) -> NodeId {
    doc.transact("cube", |tx| {
        let c = tx.create("forge.cube", None, None)?;
        tx.set_key(
            c,
            "transform.pos.x",
            Key::new(0.0, 0.0).with_interp(Interp::Linear),
        )?;
        tx.set_key(c, "transform.pos.x", Key::new(1.0, 300.0))?;
        Ok(c)
    })
    .unwrap()
}

#[test]
fn range_validation_and_frames() {
    assert!(PathRange::new(5, 4, 1).is_none(), "start after end");
    assert!(PathRange::new(0, 10, 0).is_none(), "step below one");
    let r = PathRange::new(0, 10, 4).unwrap();
    assert_eq!(r.frames(), vec![0, 4, 8, 10], "end is appended");
    assert_eq!(PathRange::new(0, 8, 4).unwrap().frames(), vec![0, 4, 8]);
    assert_eq!(PathRange::new(3, 3, 1).unwrap().frames(), vec![3]);
    assert_eq!(
        PathRange::new(-2, 1, 1).unwrap().frames(),
        vec![-2, -1, 0, 1]
    );
    // the step is raised to keep the sample count bounded
    let big = PathRange::new(0, 100_000, 1).unwrap();
    assert!(big.step > 1);
    let n = big.frames().len();
    assert!(n <= MAX_PATH_SAMPLES + 1, "{n}");
    assert_eq!(*big.frames().last().unwrap(), 100_000);
}

#[test]
fn path_of_an_animated_cube_follows_scene_at() {
    let mut d = doc();
    let c = moving_cube(&mut d);
    let scene = d.scene();
    let range = PathRange::new(0, 30, 5).unwrap();
    let eval = |t: f64| scene_at(scene, t);
    let paths = sample_paths(scene, &[c], &range, &eval);
    assert_eq!(paths.len(), 1);
    let p = &paths[0];
    assert_eq!(p.node, c);
    let frames: Vec<i64> = p.samples.iter().map(|s| s.frame).collect();
    assert_eq!(frames, vec![0, 5, 10, 15, 20, 25, 30]);
    for s in &p.samples {
        let expect = scene_at(scene, s.frame as f64 / FPS)
            .world_matrix(c)
            .unwrap()
            .transform_point3(DVec3::ZERO);
        assert_eq!(s.pos, expect, "frame {}", s.frame);
    }
    // linear, so frame 15 is half way (time is rounded to a millisecond: 0.5 s is exact)
    assert!((p.samples[3].pos.x - 150.0).abs() < 1e-9);
    assert_eq!(p.samples[0].pos, DVec3::ZERO);
    // sampling does not touch the document and is deterministic
    let again = sample_paths(scene, &[c], &range, &eval);
    assert_eq!(paths, again);
}

#[test]
fn key_marks_include_ancestors_and_stay_in_range() {
    let mut d = doc();
    let (parent, child) = d
        .transact("tree", |tx| {
            let p = tx.create("forge.null", None, None)?;
            let c = tx.create("forge.cube", Some(p), None)?;
            // child key at frame 10 (1/3 s) and at frame 60 (outside the range below)
            tx.set_key(c, "transform.pos.y", Key::new(10.0 / FPS, 0.0))?;
            tx.set_key(c, "transform.pos.y", Key::new(2.0, 90.0))?;
            // parent keys at frames 0 and 30
            tx.set_key(p, "transform.pos.x", Key::new(0.0, 0.0))?;
            tx.set_key(p, "transform.pos.x", Key::new(1.0, 60.0))?;
            // a track that is not a transform track makes no mark
            tx.set_key(c, "size_x", Key::new(0.5, 10.0))?;
            Ok((p, c))
        })
        .unwrap();
    let scene = d.scene();
    let range = PathRange::new(0, 40, 10).unwrap();
    let eval = |t: f64| scene_at(scene, t);
    let paths = sample_paths(scene, &[child], &range, &eval);
    let p = &paths[0];
    let marks: Vec<f64> = p.keys.iter().map(|k| k.frame).collect();
    assert_eq!(marks.len(), 3, "{marks:?}");
    assert!(
        marks[0].abs() < 1e-6 && (marks[1] - 10.0).abs() < 1e-6 && (marks[2] - 30.0).abs() < 1e-6
    );
    // the mark position is evaluated at its exact time
    let exact = scene_at(scene, 10.0 / FPS)
        .world_matrix(child)
        .unwrap()
        .transform_point3(DVec3::ZERO);
    assert_eq!(p.keys[1].pos, exact);
    let _ = parent;
}

#[test]
fn objects_that_do_not_move_have_no_path_and_the_node_cap_holds() {
    let mut d = doc();
    let moving = moving_cube(&mut d);
    let still = d
        .transact("still", |tx| tx.create("forge.cube", None, None))
        .unwrap();
    // an animated parameter that is not a transform leaves the object still
    let sized = d
        .transact("sized", |tx| {
            let c = tx.create("forge.cube", None, None)?;
            tx.set_key(c, "size_x", Key::new(0.0, 1.0))?;
            tx.set_key(c, "size_x", Key::new(1.0, 50.0))?;
            Ok(c)
        })
        .unwrap();
    let scene = d.scene();
    let range = PathRange::new(0, 30, 10).unwrap();
    let eval = |t: f64| scene_at(scene, t);
    let paths = sample_paths(
        scene,
        &[still, moving, sized, NodeId(99_999)],
        &range,
        &eval,
    );
    assert_eq!(
        paths.iter().map(|p| p.node).collect::<Vec<_>>(),
        vec![moving]
    );

    let many: Vec<NodeId> = (0..MAX_PATH_NODES + 4).map(|_| moving).collect();
    let capped = sample_paths(scene, &many, &range, &eval);
    assert_eq!(capped.len(), MAX_PATH_NODES);
}

#[test]
fn onion_frames_are_clipped_to_the_range_and_ordered() {
    let f = onion_frames(10, 2, 2, 3, 0, 90);
    let got: Vec<(i64, u32, bool)> = f.iter().map(|o| (o.frame, o.rank, o.forward)).collect();
    assert_eq!(
        got,
        vec![(4, 2, false), (7, 1, false), (13, 1, true), (16, 2, true)]
    );
    // clipped at both ends of the project range
    let f = onion_frames(1, 3, 3, 1, 0, 3);
    let frames: Vec<i64> = f.iter().map(|o| o.frame).collect();
    assert_eq!(frames, vec![0, 2, 3]);
    // counts are capped, step 0 is treated as 1, nothing asked gives nothing
    assert!(onion_frames(50, 99, 0, 1, 0, 100).len() <= 10);
    assert_eq!(onion_frames(5, 0, 0, 1, 0, 10), vec![]);
    assert_eq!(onion_frames(5, 1, 0, 0, 0, 10).len(), 1);
}

#[test]
fn stored_values_are_unchanged() {
    let mut d = doc();
    let c = moving_cube(&mut d);
    let before = d.scene().param(c, "transform.pos").cloned();
    let scene = d.scene();
    let range = PathRange::new(0, 30, 1).unwrap();
    let _ = sample_paths(scene, &[c], &range, &|t| scene_at(scene, t));
    assert_eq!(d.scene().param(c, "transform.pos").cloned(), before);
    assert!(matches!(before, Some(Value::Vec3(_))));
}
