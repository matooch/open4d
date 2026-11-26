//! The Volume Builder reads an emitter's live particles as sphere sources (DYNAMICS.md
//! section 11). The emitter's x position is animated 0 -> 3000 cm over one second at one
//! particle per frame with speed 0: particle n stands still where the emitter was when it was
//! born, about 100 n cm along x (scene_at rounds the time to a millisecond: 99, 201, 300).

use std::sync::Arc;

use forge_core::{Document, EvalCtx, Interp, Key, NodeId, Registry, Value};
use forge_generators::Primitives;
use forge_particles::{EMITTER, ParticleToolset};
use forge_volume::{VolumePlugin, mesh_volume_node};

fn scene() -> (Document, NodeId, NodeId) {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r.install(&ParticleToolset).unwrap();
    r.install(&VolumePlugin).unwrap();
    let mut d = Document::new(Arc::new(r));
    let (mesher, builder) = d
        .transact("s", |tx| {
            let m = tx.create("forge.volume_mesher", None, None)?;
            let b = tx.create("forge.volume_builder", Some(m), None)?;
            let e = tx.create(EMITTER, Some(b), None)?;
            for (k, v) in [
                ("birthrate_editor", Value::Float(1.0)),
                ("size_x", Value::Float(0.0)),
                ("size_y", Value::Float(0.0)),
                ("lifetime", Value::Float(1000.0)),
                ("speed", Value::Float(0.0)),
            ] {
                tx.set_param(e, k, v)?;
            }
            tx.set_key(
                e,
                "transform.pos.x",
                Key::new(0.0, 0.0).with_interp(Interp::Linear),
            )?;
            tx.set_key(
                e,
                "transform.pos.x",
                Key::new(1.0, 3000.0).with_interp(Interp::Linear),
            )?;
            Ok((m, b))
        })
        .unwrap();
    (d, mesher, builder)
}

fn x_range(m: &forge_geometry::PolyMesh) -> (f64, f64) {
    let xs = m.points.iter().map(|p| p.x);
    (
        xs.clone().fold(f64::MAX, f64::min),
        xs.fold(f64::MIN, f64::max),
    )
}

#[test]
fn live_particles_become_spheres_of_the_particle_radius() {
    let (d, mesher, _) = scene();
    // Voxel size 10, Particle Voxel Range 2: radius 0.735 * 2 * 10 = 14.7.
    let mesh_at = |frame| {
        mesh_volume_node(d.scene(), mesher, &EvalCtx::at_frame(frame, 30.0))
            .unwrap()
            .expect("a mesh")
    };
    // Frame 0: one particle at the origin: a sphere of radius 14.7 (the mesher puts the
    // surface within a voxel of the true one).
    let (lo, hi) = x_range(&mesh_at(0));
    assert!(
        (lo + 14.7).abs() < 10.0 && (hi - 14.7).abs() < 10.0,
        "frame 0: {lo} .. {hi}"
    );
    // Frame 3: particles at about x = 0, 99, 201, 300: the cloud reaches 300 + 14.7.
    let m3 = mesh_at(3);
    let (lo, hi) = x_range(&m3);
    assert!((lo + 14.7).abs() < 10.0, "frame 3 low end {lo}");
    assert!((hi - 314.7).abs() < 10.0, "frame 3 high end {hi}");
    // The spheres are 99 cm apart and 29.4 wide: four separate blobs. Every mesh point is
    // within 14.7 + 10 of a particle centre, and there are points near each centre's equator.
    let centres = [0.0, 99.0, 201.0, 300.0];
    assert!(
        m3.points
            .iter()
            .all(|p| centres.iter().any(|c| (p.x - c).abs() < 24.7))
    );
    for c in centres {
        assert!(
            m3.points
                .iter()
                .any(|p| (p.x - c).abs() < 5.0 && p.y.abs() < 5.0),
            "a surface point at the equator of the sphere at x = {c}"
        );
    }
}

#[test]
fn the_particle_voxel_range_sets_the_radius() {
    let (mut d, mesher, builder) = scene();
    d.transact("range", |tx| {
        tx.set_param(builder, "particle_voxel_range", Value::Int(4))
    })
    .unwrap();
    // radius 0.735 * 4 * 10 = 29.4
    let m = mesh_volume_node(d.scene(), mesher, &EvalCtx::at_frame(0, 30.0))
        .unwrap()
        .unwrap();
    let (lo, hi) = x_range(&m);
    assert!(
        (lo + 29.4).abs() < 10.0 && (hi - 29.4).abs() < 10.0,
        "{lo} .. {hi}"
    );
}

#[test]
fn no_live_particle_gives_an_empty_volume() {
    let (d, mesher, _) = scene();
    // the default start frame is 0: before it there is nothing
    let m = mesh_volume_node(d.scene(), mesher, &EvalCtx::at_frame(-5, 30.0)).unwrap();
    assert!(m.is_none_or(|m| m.points.is_empty()));
}
