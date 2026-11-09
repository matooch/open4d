//! docs/specs/ANIM.md section 2b.3: path and ghost evaluation, the key, and the async cache.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use forge_anim::motion::{PathRange, onion_frames};
use forge_app::App;
use forge_app::motion::{AsyncCache, compute_ghosts, compute_paths, scene_key};
use forge_core::{Interp, Key, NodeId};
use serde_json::json;

fn cube(app: &mut App) -> NodeId {
    let id = NodeId(
        app.run("object.create", json!({"type": "forge.cube"}))
            .unwrap()["node"]
            .as_u64()
            .unwrap(),
    );
    app.doc
        .transact("keys", |tx| {
            tx.set_key(
                id,
                "transform.pos.x",
                Key::new(0.0, 0.0).with_interp(Interp::Linear),
            )?;
            tx.set_key(id, "transform.pos.x", Key::new(1.0, 300.0))
        })
        .unwrap();
    id
}

#[test]
fn paths_and_ghosts_follow_the_animation() {
    let mut app = App::new_default();
    let c = cube(&mut app);
    let scene = app.doc.scene();
    let range = PathRange::new(0, 30, 10).unwrap();
    let paths = compute_paths(scene, &[c], &range);
    assert_eq!(paths.len(), 1);
    let xs: Vec<f64> = paths[0].samples.iter().map(|s| s.pos.x).collect();
    assert_eq!(xs.len(), 4);
    // frame times are rounded to a whole millisecond (ANIM.md 1.4): 99.9 at frame 10
    assert!(
        (xs[1] - 100.0).abs() < 0.2 && (xs[3] - 300.0).abs() < 1e-6,
        "{xs:?}"
    );

    let frames = onion_frames(15, 1, 1, 5, 0, 90);
    let ghosts = compute_ghosts(scene, &[c], &frames);
    assert_eq!(ghosts.len(), 2);
    let cx = |g: &forge_app::motion::GhostMesh| {
        g.mesh.points.iter().map(|p| p.x).sum::<f64>() / g.mesh.points.len() as f64
    };
    assert!(
        (cx(&ghosts[0]) - 100.0).abs() < 0.2,
        "frame 10: {}",
        cx(&ghosts[0])
    );
    assert!(
        (cx(&ghosts[1]) - 200.0).abs() < 0.2,
        "frame 20: {}",
        cx(&ghosts[1])
    );
    assert!(!ghosts[0].forward && ghosts[1].forward);
    assert_eq!((ghosts[0].of, ghosts[1].of), (1, 1));
    // stored values stay put
    assert_eq!(app.doc.scene().revision(), scene.revision());
}

#[test]
fn the_key_changes_with_every_edit_undo_and_redo() {
    let mut app = App::new_default();
    let c = cube(&mut app);
    let mut keys = vec![scene_key(app.doc.scene())];
    app.doc
        .transact("move key", |tx| {
            tx.set_key(c, "transform.pos.x", Key::new(2.0, 900.0))
        })
        .unwrap();
    keys.push(scene_key(app.doc.scene()));
    assert!(app.doc.undo());
    keys.push(scene_key(app.doc.scene()));
    assert!(app.doc.redo());
    keys.push(scene_key(app.doc.scene()));
    for i in 0..keys.len() {
        for j in i + 1..keys.len() {
            assert_ne!(keys[i], keys[j], "states {i} and {j} share a key");
        }
    }
    // and the path really differs after the edit and returns after undo
    let range = PathRange::new(0, 60, 30).unwrap();
    let last = |app: &App| {
        compute_paths(app.doc.scene(), &[c], &range)[0].samples[2]
            .pos
            .x
    };
    assert!((last(&app) - 900.0).abs() < 1e-6);
    app.doc.undo();
    assert!((last(&app) - 300.0).abs() < 1e-6);
}

#[test]
fn synchronous_cache_computes_once_per_key() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut cache: AsyncCache<u32, u32> = AsyncCache::default();
    cache.synchronous = true;
    let run = |cache: &mut AsyncCache<u32, u32>, key: u32| {
        let n = calls.clone();
        cache.get(key, move || {
            n.fetch_add(1, Ordering::SeqCst);
            key * 10
        })
    };
    assert_eq!(run(&mut cache, 1), (Some(Arc::new(10)), false));
    assert_eq!(run(&mut cache, 1).0.as_deref(), Some(&10));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(run(&mut cache, 2).0.as_deref(), Some(&20));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn background_job_gives_the_same_result_and_keeps_the_stale_one() {
    let mut cache: AsyncCache<u32, Vec<u32>> = AsyncCache::default();
    let (first, pending) = cache.get(1, || vec![1, 2, 3]);
    assert!(first.is_none() && pending, "nothing to show yet");
    cache.wait(&1);
    let (r, pending) = cache.get(1, || unreachable!("cached"));
    assert_eq!((r.as_deref(), pending), (Some(&vec![1, 2, 3]), false));
    // a new key shows the old result until the new one is ready
    let (r, pending) = cache.get(2, || vec![9]);
    assert_eq!((r.as_deref(), pending), (Some(&vec![1, 2, 3]), true));
    cache.wait(&2);
    let (r, pending) = cache.get(2, || unreachable!("cached"));
    assert_eq!((r.as_deref(), pending), (Some(&vec![9]), false));
}

#[test]
fn newest_request_wins_and_a_panicking_job_is_not_retried() {
    let mut cache: AsyncCache<u32, u32> = AsyncCache::default();
    let _ = cache.get(1, || {
        std::thread::sleep(std::time::Duration::from_millis(30));
        1
    });
    let _ = cache.get(2, || 2); // queued
    let _ = cache.get(3, || 3); // replaces the queued one
    cache.wait(&3);
    let (r, pending) = cache.get(3, || unreachable!("cached"));
    assert_eq!((r.as_deref(), pending), (Some(&3), false));

    let mut bad: AsyncCache<u32, u32> = AsyncCache::default();
    let _ = bad.get(7, || panic!("job failed"));
    bad.wait(&7);
    let runs = Arc::new(AtomicUsize::new(0));
    let n = runs.clone();
    let (r, pending) = bad.get(7, move || {
        n.fetch_add(1, Ordering::SeqCst);
        0
    });
    assert!(r.is_none() && !pending);
    assert_eq!(runs.load(Ordering::SeqCst), 0, "not retried every frame");
}
