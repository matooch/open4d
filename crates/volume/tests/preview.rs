//! The viewport preview (docs/specs/VOLUME.md section 11): coarse build, worker, slices.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::Primitives;
use forge_geometry::PolyMesh;
use forge_volume::{
    Preview, PreviewFn, PreviewWorker, VolumePlugin, build_preview, build_volume, slice_range,
    slice_samples,
};
use glam::DVec3;

fn doc() -> Document {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r.install(&VolumePlugin).unwrap();
    Document::new(Arc::new(r))
}

/// A mesher with a builder holding one sphere of radius `r` and the given voxel size.
fn sphere_scene(r: f64, voxel: f64) -> (Document, NodeId, NodeId) {
    let mut d = doc();
    let (m, b) = d
        .transact("s", |tx| {
            let m = tx.create("forge.volume_mesher", None, None)?;
            let b = tx.create("forge.volume_builder", Some(m), None)?;
            tx.set_param(b, "voxel_size", Value::Float(voxel))?;
            let s = tx.create("forge.sphere", Some(b), None)?;
            tx.set_param(s, "radius", Value::Float(r))?;
            Ok((m, b))
        })
        .unwrap();
    (d, m, b)
}

#[test]
fn the_preview_coarsens_the_voxel_size_but_not_the_scene() {
    // 1000 wide at voxel size 1 is 1000 cells: far too many for a preview
    let (d, m, b) = sphere_scene(500.0, 1.0);
    let p = build_preview(d.scene(), m, &EvalCtx::default(), 40)
        .unwrap()
        .unwrap();
    assert!(
        (p.voxel_size - 1000.0 / 40.0).abs() < 1e-9,
        "{}",
        p.voxel_size
    );
    assert!(p.mesh.polys.len() > 100 && p.mesh.polys.len() < 20_000);
    let (lo, hi) = p.bounds.unwrap();
    assert!(lo.x < -480.0 && hi.x > 480.0 && lo.x > -560.0);
    assert!(p.active_voxels > 100);
    // the scene itself is untouched
    assert_eq!(d.scene().param(b, "voxel_size"), Some(&Value::Float(1.0)));
    // a scene already coarser than the limit keeps its own voxel size
    let (d, m, _) = sphere_scene(100.0, 10.0);
    let p = build_preview(d.scene(), m, &EvalCtx::default(), 40)
        .unwrap()
        .unwrap();
    assert_eq!(p.voxel_size, 10.0);
    // a mesher without a builder has no preview
    let mut d2 = doc();
    let m2 = d2
        .transact("m", |tx| tx.create("forge.volume_mesher", None, None))
        .unwrap();
    assert!(
        build_preview(d2.scene(), m2, &EvalCtx::default(), 40)
            .unwrap()
            .is_none()
    );
}

fn fake_preview(tag: usize) -> Preview {
    Preview {
        mesh: PolyMesh {
            points: vec![DVec3::splat(tag as f64); 3],
            polys: vec![vec![0, 1, 2]],
        },
        bounds: None,
        voxel_size: tag as f64,
        active_voxels: tag,
    }
}

#[test]
fn a_slow_preview_does_not_block_and_the_last_result_stays_visible() {
    let (d, m, _) = sphere_scene(100.0, 10.0);
    let calls = Arc::new(AtomicUsize::new(0));
    let gate: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let (c, g) = (Arc::clone(&calls), Arc::clone(&gate));
    let compute: Arc<PreviewFn> = Arc::new(move |_scene, _mesher| {
        let n = c.fetch_add(1, Ordering::SeqCst) + 1;
        // the artificially slow mesher: wait for the gate, then 80 ms of work
        drop(g.lock().unwrap());
        std::thread::sleep(Duration::from_millis(80));
        Ok(Some(fake_preview(n)))
    });
    let w = PreviewWorker::new(compute);
    assert!(w.snapshot().preview.is_none() && !w.snapshot().busy);

    // first result
    let t = Instant::now();
    w.request(d.scene(), m);
    assert!(
        t.elapsed() < Duration::from_millis(50),
        "request must not wait"
    );
    assert!(w.snapshot().busy);
    assert!(w.wait_idle(Duration::from_secs(5)));
    assert_eq!(w.snapshot().preview.unwrap().active_voxels, 1);

    // hold the gate: the second computation cannot finish while we look
    let held = gate.lock().unwrap();
    let t = Instant::now();
    for _ in 0..5 {
        w.request(d.scene(), m);
    }
    let s = w.snapshot();
    assert!(
        t.elapsed() < Duration::from_millis(50),
        "requests and snapshot never wait"
    );
    assert!(s.busy);
    assert_eq!(
        s.preview.as_ref().unwrap().active_voxels,
        1,
        "the last result stays up"
    );
    drop(held);
    assert!(w.wait_idle(Duration::from_secs(5)));
    let s = w.snapshot();
    // five requests while one was running: one running + the newest kept = at most 2 more runs
    assert!(
        calls.load(Ordering::SeqCst) <= 3,
        "{} calls",
        calls.load(Ordering::SeqCst)
    );
    assert_eq!(s.requested, 6);
    assert_eq!(s.finished as usize, calls.load(Ordering::SeqCst));
    assert_eq!(
        s.preview.unwrap().active_voxels,
        calls.load(Ordering::SeqCst)
    );
    assert!(!s.busy);
}

#[test]
fn errors_and_panics_keep_the_last_good_preview() {
    let (d, m, _) = sphere_scene(100.0, 10.0);
    let mode = Arc::new(AtomicUsize::new(0));
    let md = Arc::clone(&mode);
    let compute: Arc<PreviewFn> = Arc::new(move |_s, _m| match md.load(Ordering::SeqCst) {
        0 => Ok(Some(fake_preview(7))),
        1 => Err(forge_volume::VolumeBuildError::Unsupported("a thing")),
        _ => panic!("boom"),
    });
    let w = PreviewWorker::new(compute);
    w.request(d.scene(), m);
    assert!(w.wait_idle(Duration::from_secs(5)));
    assert_eq!(w.snapshot().preview.unwrap().active_voxels, 7);
    for (k, text) in [(1, "not supported"), (2, "panicked")] {
        mode.store(k, Ordering::SeqCst);
        w.request(d.scene(), m);
        assert!(w.wait_idle(Duration::from_secs(5)));
        let s = w.snapshot();
        assert!(s.error.as_deref().unwrap().contains(text), "{:?}", s.error);
        assert_eq!(s.preview.unwrap().active_voxels, 7);
    }
    mode.store(0, Ordering::SeqCst);
    w.request(d.scene(), m);
    assert!(w.wait_idle(Duration::from_secs(5)));
    assert!(w.snapshot().error.is_none());
}

#[test]
fn the_standard_worker_previews_a_sphere() {
    let (d, m, _) = sphere_scene(100.0, 10.0);
    let w = PreviewWorker::standard(48);
    w.request(d.scene(), m);
    assert!(w.wait_idle(Duration::from_secs(30)));
    let s = w.snapshot();
    assert!(s.error.is_none(), "{:?}", s.error);
    let p = s.preview.expect("a preview");
    assert_eq!(p.mesh.polys.len(), 1830);
    assert!(p.bounds.is_some());
}

#[test]
fn slices_select_the_active_voxels_of_one_plane() {
    let (d, _, b) = sphere_scene(100.0, 10.0);
    let v = build_volume(d.scene(), b, &EvalCtx::default()).unwrap();
    let (lo, hi) = slice_range(&v, 1).unwrap();
    assert!(lo < -8 && hi > 8);
    let mid = slice_samples(&v, 1, 0);
    assert!(mid.len() > 20);
    assert!(mid.iter().all(|s| s.position.y == 0.0));
    // the plane through the centre has its band on a circle: values are within the band
    assert!(mid.iter().all(|s| s.value.abs() <= 20.0));
    assert!(slice_samples(&v, 1, hi + 5).is_empty());
}
