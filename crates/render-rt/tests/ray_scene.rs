//! CPU reference fixtures for the ray-query scene specified in REALTIME.md §4.

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use forge_render_api::{Instance, MeshData, Snapshot};
use forge_render_cpu::scene::Scene;
use glam::{DMat4, DVec3};

fn snapshot(count: usize) -> Snapshot {
    let mesh = MeshData::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 2]],
    );
    Snapshot {
        meshes: vec![mesh],
        instances: (0..count)
            .map(|i| {
                let x = (i % 100) as f64 * 2.0;
                let y = (i / 100) as f64 * 2.0;
                Instance::new(
                    0,
                    DMat4::from_translation(DVec3::new(x, y, 0.0)),
                    0,
                    i as u32,
                )
            })
            .collect(),
        materials: vec![common::diffuse([1.0; 3])],
        ..Snapshot::default()
    }
}

#[test]
fn cpu_reference_rays_distinguish_instances_misses_and_range() {
    let scene = Scene::build(&snapshot(3)).unwrap();
    for i in 0..3 {
        let origin = DVec3::new(i as f64 * 2.0 + 0.25, 0.25, 2.0);
        let hit = scene.intersect(origin, -DVec3::Z, 3.0).unwrap();
        assert_eq!((hit.inst, hit.tri), (i, 0));
        assert!((hit.t - 2.0).abs() < 1e-12);
        assert!((hit.u - 0.25).abs() < 1e-12);
        assert!((hit.v - 0.25).abs() < 1e-12);
        assert!(scene.intersect(origin, -DVec3::Z, 1.0).is_none());
    }
    assert!(
        scene
            .intersect(DVec3::new(1.5, 0.25, 2.0), -DVec3::Z, 3.0)
            .is_none()
    );
}

#[test]
fn cpu_reference_shares_geometry_across_ten_thousand_instances() {
    let scene = Scene::build(&snapshot(10_000)).unwrap();
    assert_eq!(scene.blases.len(), 1);
    assert_eq!(scene.triangles, 1);
    assert_eq!(scene.insts.len(), 10_000);
    assert!(scene.insts.iter().all(|inst| inst.blas == 0));
    let hit = scene
        .intersect(DVec3::new(198.25, 198.25, 2.0), -DVec3::Z, 3.0)
        .unwrap();
    assert_eq!((hit.inst, hit.tri), (9_999, 0));
}

#[test]
fn cpu_reference_distinguishes_instance_transforms_across_frames() {
    let mut frame = snapshot(2);
    let before = Scene::build(&frame).unwrap();
    let old_origin = DVec3::new(2.25, 0.25, 2.0);
    let new_origin = DVec3::new(6.25, 0.25, 2.0);
    assert_eq!(
        before.intersect(old_origin, -DVec3::Z, 3.0).unwrap().inst,
        1
    );
    assert!(before.intersect(new_origin, -DVec3::Z, 3.0).is_none());

    frame.instances[1] = Instance::new(0, DMat4::from_translation(DVec3::new(6.0, 0.0, 0.0)), 0, 1);
    let after = Scene::build(&frame).unwrap();
    assert_eq!(after.blases.len(), 1);
    assert!(after.intersect(old_origin, -DVec3::Z, 3.0).is_none());
    let hit = after.intersect(new_origin, -DVec3::Z, 3.0).unwrap();
    assert_eq!((hit.inst, hit.tri), (1, 0));
    assert!((hit.t - 2.0).abs() < 1e-12);
    assert!((hit.u - 0.25).abs() < 1e-12);
    assert!((hit.v - 0.25).abs() < 1e-12);
}
