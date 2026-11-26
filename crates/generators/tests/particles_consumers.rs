//! Particles as a point source (DYNAMICS.md section 11): the Tracer and the cloner's Object mode.
//! One emitter at the origin emits one particle per frame along +Z at 150 cm/s (5 cm per frame
//! at 30 fps), no spread, no variation: particle n is born in frame n at the origin and stands at
//! z = 5 (f - n) in frame f, so every expected number below is exact.

use std::sync::Arc;

use forge_clone_engine::CloneToolset;
use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::{Primitives, clone_data, tracer};
use forge_particles::{EMITTER, ParticleToolset};
use glam::DVec3;

fn doc() -> Document {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r.install(&CloneToolset).unwrap();
    r.install(&ParticleToolset).unwrap();
    Document::new(Arc::new(r))
}

fn emitter(tx: &mut forge_core::Tx) -> NodeId {
    let e = tx.create(EMITTER, None, None).unwrap();
    for (k, v) in [
        ("birthrate_editor", Value::Float(1.0)),
        ("size_x", Value::Float(0.0)),
        ("size_y", Value::Float(0.0)),
        ("lifetime", Value::Float(1000.0)),
        ("speed", Value::Float(150.0)),
    ] {
        tx.set_param(e, k, v).unwrap();
    }
    e
}

#[test]
fn the_tracer_draws_one_path_per_particle() {
    let mut d = doc();
    let t = d
        .transact("tracer", |tx| {
            let e = emitter(tx);
            let t = tx.create("forge.tracer", None, None)?;
            tx.set_param(t, "trace_links", Value::LinkList(vec![e]))?;
            Ok(t)
        })
        .unwrap();
    // Trace Paths at frame 10 samples frames 1..=10. Particle n is alive from frame max(n, 1):
    // ids 0..=9 have two points or more (10 paths), id 10 has one point and makes no curve.
    let curve = tracer::curve_of(d.scene(), t, &EvalCtx::at_frame(10, 30.0))
        .unwrap()
        .expect("a curve");
    assert_eq!(
        curve.extra.len(),
        9,
        "10 paths in all: the main curve and 9 more"
    );
    let main = curve.points();
    let zs: Vec<f64> = main.iter().map(|p| p.z).collect();
    let expect: Vec<f64> = (1..=10).map(|f| 5.0 * f as f64).collect();
    assert_eq!(zs.len(), 10, "id 0 is alive in the 10 sampled frames");
    for (a, b) in zs.iter().zip(&expect) {
        assert!((a - b).abs() < 1e-9, "{zs:?}");
    }
    // path of id 5: frames 5..=10: z = 0, 5, ..., 25 (extras are ids 1, 2, ..., 9 in order)
    let five = curve.extra[4].points();
    let zs: Vec<f64> = five.iter().map(|p| p.z).collect();
    assert_eq!(zs.len(), 6);
    for (k, z) in zs.iter().enumerate() {
        assert!((z - 5.0 * k as f64).abs() < 1e-9, "{zs:?}");
    }
    // Sample Step 5 samples frames 1, 5, 10: id 0 has 3 points
    d.transact("step", |tx| tx.set_param(t, "sample_step", Value::Int(5)))
        .unwrap();
    let curve = tracer::curve_of(d.scene(), t, &EvalCtx::at_frame(10, 30.0))
        .unwrap()
        .unwrap();
    assert_eq!(curve.points().len(), 3);
    // before any particle has two samples: nothing to draw
    assert!(
        tracer::curve_of(d.scene(), t, &EvalCtx::at_frame(1, 30.0))
            .unwrap()
            .is_none()
    );
}

#[test]
fn connect_all_joins_the_live_particles_by_id() {
    let mut d = doc();
    let t = d
        .transact("tracer", |tx| {
            let e = emitter(tx);
            let t = tx.create("forge.tracer", None, None)?;
            tx.set_param(t, "tracing_mode", Value::Enum("connect_all".into()))?;
            tx.set_param(t, "trace_links", Value::LinkList(vec![e]))?;
            Ok(t)
        })
        .unwrap();
    let curve = tracer::curve_of(d.scene(), t, &EvalCtx::at_frame(10, 30.0))
        .unwrap()
        .unwrap();
    let zs: Vec<f64> = curve.points().iter().map(|p| p.z).collect();
    assert_eq!(zs.len(), 11, "ids 0..=10 are alive in frame 10");
    for (id, z) in zs.iter().enumerate() {
        assert!((z - 5.0 * (10 - id) as f64).abs() < 1e-9, "{zs:?}");
    }
}

fn cloner_scene() -> (Document, NodeId, NodeId) {
    let mut d = doc();
    let (c, e) = d
        .transact("cloner", |tx| {
            let e = emitter(tx);
            let c = tx.create("forge.cloner", None, None)?;
            tx.create("forge.cube", Some(c), None)?;
            tx.set_param(c, "mode", Value::Enum("object".into()))?;
            tx.set_param(c, "object", Value::Link(Some(e)))?;
            Ok((c, e))
        })
        .unwrap();
    (d, c, e)
}

#[test]
fn the_cloner_makes_one_clone_per_live_particle() {
    let (mut d, c, e) = cloner_scene();
    let at = |d: &Document, frame| {
        clone_data(d.scene(), c, &EvalCtx::at_frame(frame, 30.0))
            .unwrap()
            .unwrap()
    };
    assert_eq!(at(&d, 0).len(), 1);
    let data = at(&d, 10);
    assert_eq!(data.len(), 11, "particles 0..=10 are alive");
    for k in 0..11 {
        let m = data.matrices[k];
        assert!(
            (m.w_axis.truncate() - DVec3::new(0.0, 0.0, 5.0 * (10 - k) as f64)).length() < 1e-9
        );
        // aligned: +Z along the velocity (+Z), which is the identity frame; scale 1
        assert!((m.z_axis.truncate() - DVec3::Z).length() < 1e-9);
        assert!((m.x_axis.truncate() - DVec3::X).length() < 1e-9);
    }
    // the emitter turned (H = +90 degrees sends +Z to -X): the clones turn with their velocity
    d.transact("turn", |tx| {
        tx.set_param(
            e,
            "transform.rot",
            Value::Vec3([std::f64::consts::FRAC_PI_2, 0.0, 0.0]),
        )
    })
    .unwrap();
    let turned = at(&d, 10);
    assert!((turned.matrices[0].z_axis.truncate() - DVec3::NEG_X).length() < 1e-9);
    assert!((turned.matrices[0].w_axis.truncate() - DVec3::new(-50.0, 0.0, 0.0)).length() < 1e-9);
    // Align Clone off: translation only
    d.transact("align", |tx| {
        tx.set_param(c, "align_clone", Value::Bool(false))
    })
    .unwrap();
    let plain = at(&d, 10);
    assert!((plain.matrices[0].z_axis.truncate() - DVec3::Z).length() < 1e-9);
    // and the clones live in the cloner's space: moving the cloner shifts them back
    d.transact("move", |tx| {
        tx.set_param(c, "transform.pos", Value::Vec3([10.0, 20.0, 30.0]))
    })
    .unwrap();
    let moved = at(&d, 10);
    assert!(
        (moved.matrices[0].w_axis.truncate() - DVec3::new(-60.0, -20.0, -30.0)).length() < 1e-9
    );
}

#[test]
fn the_cloner_geometry_has_a_cube_per_particle() {
    let (d, c, _) = cloner_scene();
    let mesh = forge_generators::evaluate(d.scene(), c, &EvalCtx::at_frame(10, 30.0))
        .unwrap()
        .unwrap();
    assert_eq!(mesh.polys.len(), 11 * 6, "11 cubes of 6 faces");
}
