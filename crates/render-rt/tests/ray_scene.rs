//! CPU reference and GPU backend acceptance for RENDER.md R23.

#[path = "../../render-cpu/tests/common/mod.rs"]
mod common;

use forge_render_api::{Instance, MeshData, Snapshot};
use forge_render_cpu::scene::Scene;
use forge_render_rt::ray::{
    BackendKind, Ray, RayHit, RtBackend, SceneStats, preferred_backend, request_backend,
};
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

#[test]
fn backend_preference_requires_both_the_api_and_ray_queries() {
    use BackendKind::*;
    let rays = wgpu::Features::EXPERIMENTAL_RAY_QUERY;
    for (api, hardware) in [
        (wgpu::Backend::Metal, Metal),
        (wgpu::Backend::Vulkan, Vulkan),
    ] {
        assert_eq!(preferred_backend(api, wgpu::Features::empty()), Compute);
        assert_eq!(preferred_backend(api, rays), hardware);
    }
    assert_eq!(preferred_backend(wgpu::Backend::Dx12, rays), Compute);
}

fn compare(backend: &dyn RtBackend, frame: &Snapshot, rays: &[Ray]) -> Vec<Option<RayHit>> {
    let cpu = Scene::build(frame).unwrap();
    let hits = backend.trace(rays).unwrap();
    assert_eq!(hits.len(), rays.len());
    for (i, (ray, &got)) in rays.iter().zip(&hits).enumerate() {
        let want = cpu.intersect(ray.origin, ray.direction, ray.t_max);
        match (got, want) {
            (Some(got), Some(want)) => {
                assert_eq!(
                    (got.instance, got.triangle),
                    (want.inst, want.tri),
                    "ray {i}"
                );
                for (got, want) in [(got.t, want.t), (got.u, want.u), (got.v, want.v)] {
                    assert!((got - want).abs() <= 1e-4, "ray {i}: {got} != {want}");
                }
            }
            (None, None) => (),
            other => panic!("ray {i}: GPU/CPU hits disagree: {other:?}"),
        }
    }
    hits
}

fn ray(x: f64, y: f64, t_max: f64) -> Ray {
    Ray {
        origin: DVec3::new(x, y, 4.0),
        direction: -DVec3::Z,
        t_max,
    }
}

#[test]
fn available_ray_backends_match_and_update_only_tlas_for_moving_clones() {
    // This is the only GPU test in this file: one adapter, sequential device work.
    let require_vulkan = std::env::var("FORGE_REQUIRE_VULKAN_RT").as_deref() == Ok("1");
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = match pollster::block_on(instance.request_adapter(&Default::default())) {
        Ok(adapter) => adapter,
        Err(reason) => {
            assert!(!require_vulkan, "Vulkan RT required but no GPU: {reason}");
            assert_ne!(
                std::env::var("FORGE_REQUIRE_GPU").as_deref(),
                Ok("1"),
                "{reason}"
            );
            eprintln!("skipping ray queries: {reason}");
            return;
        }
    };
    let expected_kind = preferred_backend(adapter.get_info().backend, adapter.features());
    if require_vulkan {
        assert_eq!(
            expected_kind,
            BackendKind::Vulkan,
            "Vulkan RT required on the selected adapter: {:?}",
            adapter.get_info()
        );
    } else if expected_kind != BackendKind::Vulkan {
        eprintln!("skipping Vulkan hardware comparison: no selected Vulkan RT adapter");
    }
    let (kind, mut compute) = request_backend(&adapter, false).unwrap();
    assert_eq!(kind, BackendKind::Compute);
    let expected = exercise_instanced_queries(compute.as_mut());
    drop(compute);

    let (kind, mut selected) = request_backend(&adapter, true).unwrap();
    assert_eq!(kind, expected_kind);
    eprintln!(
        "comparing {kind:?} on {} against Compute",
        adapter.get_info().name
    );
    let actual = exercise_instanced_queries(selected.as_mut());
    assert_eq!(actual.len(), expected.len());
    for (i, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
        match (actual, expected) {
            (Some(a), Some(b)) => {
                assert_eq!(
                    (a.instance, a.triangle),
                    (b.instance, b.triangle),
                    "ray {i}"
                );
                for (a, b) in [(a.t, b.t), (a.u, b.u), (a.v, b.v)] {
                    assert!((a - b).abs() <= 1e-4, "ray {i}: {a} != {b}");
                }
            }
            (None, None) => (),
            other => panic!("ray {i}: selected/compute hits disagree: {other:?}"),
        }
    }
}

fn exercise_instanced_queries(backend: &mut dyn RtBackend) -> Vec<Option<RayHit>> {
    let mut frame = snapshot(10_000);
    frame.meshes[0] = MeshData::new(
        vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
        vec![[0, 1, 2], [3, 2, 1]],
    );
    // Overlap a nearer, nonuniformly scaled instance with the first clone.
    frame.instances[1].transform = (DMat4::from_translation(DVec3::Z)
        * DMat4::from_scale(DVec3::new(0.5, 0.5, 2.0)))
    .to_cols_array();
    let rays = [
        ray(0.1, 0.15, 6.),
        ray(0.8, 0.9, 6.),
        ray(198.2, 198.3, 6.),
        ray(198.2, 198.3, 3.),
        ray(1.5, 0.25, 6.),
        ray(250.2, 0.3, 6.),
        Ray {
            direction: -2. * DVec3::Z,
            ..ray(0.1, 0.15, 2.0)
        },
        Ray {
            direction: -2. * DVec3::Z,
            ..ray(0.1, 0.15, 1.4)
        },
        Ray {
            origin: DVec3::new(0.1, 0.15, -2.0),
            direction: DVec3::Z,
            t_max: 6.0,
        },
    ];
    backend.update_scene(&frame).unwrap();
    let mut results = compare(backend, &frame, &rays);
    assert_eq!(
        backend.stats(),
        SceneStats {
            blases: 1,
            triangles: 2,
            instances: 10_000,
            blas_builds: 1,
            tlas_updates: 1
        }
    );

    frame.instances[9_999].transform =
        DMat4::from_translation(DVec3::new(250., 0., 0.)).to_cols_array();
    backend.update_scene(&frame).unwrap();
    results.extend(compare(backend, &frame, &rays));
    assert_eq!(
        backend.stats(),
        SceneStats {
            blases: 1,
            triangles: 2,
            instances: 10_000,
            blas_builds: 1,
            tlas_updates: 2
        }
    );
    assert!(backend.trace(&[]).unwrap().is_empty());

    // A second mesh starts at a nonzero BLAS offset and needs interior nodes.
    // Leaving an empty mesh between the two also tests sparse mesh indices.
    frame.meshes.push(MeshData::default());
    let positions = (0..8)
        .flat_map(|i| {
            let x = f64::from(i) * 2.0;
            [[x, 0., 0.], [x + 1., 0., 0.], [x, 1., 0.]]
        })
        .collect();
    let triangles = (0..8).map(|i| [3 * i, 3 * i + 1, 3 * i + 2]).collect();
    frame.meshes.push(MeshData::new(positions, triangles));
    frame
        .instances
        .push(Instance::new(1, DMat4::IDENTITY, 0, 42));
    frame.instances.push(Instance::new(
        2,
        DMat4::from_translation(DVec3::new(5., 0., 2.))
            * DMat4::from_rotation_z(std::f64::consts::FRAC_PI_2),
        0,
        42,
    ));
    backend.update_scene(&frame).unwrap();
    let rays: Vec<_> = (0..129)
        // Avoid the underlying quad's shared triangle edge.
        .map(|i| ray(4.625, f64::from(i % 17) + 0.25, 6.))
        .collect();
    results.extend(compare(backend, &frame, &rays));
    assert_eq!(backend.stats().blases, 2);
    assert_eq!(backend.stats().triangles, 10);
    assert_eq!(backend.stats().blas_builds, 2);
    assert_eq!(backend.stats().tlas_updates, 3);

    let empty = Snapshot::default();
    backend.update_scene(&empty).unwrap();
    results.extend(compare(backend, &empty, &rays));
    results
}
