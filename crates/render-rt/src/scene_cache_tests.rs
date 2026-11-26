use super::*;
use forge_material::ir::{Diffuse, DiffuseModel, IrNode, Outputs};
use forge_render_api::{Instance, Light, LightKind, MaterialData, MeshData, ShadowMode};
use glam::DMat4;

#[test]
fn raster_geometry_is_resident_until_its_inputs_change() {
    let (device, queue) = match crate::request_default_device() {
        Ok(gpu) => gpu,
        Err(err) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU: {err}");
        }
        Err(err) => {
            eprintln!("skipping GPU test: {err}");
            return;
        }
    };
    let new_engine = || RtEngine::with_device(device.clone(), queue.clone());
    let mut light = Light::new(LightKind::Infinite { angle: 0.0 }, DMat4::IDENTITY);
    light.shadow = ShadowMode::Hard;
    let mut scene = Snapshot {
        meshes: vec![MeshData::new(
            vec![[-100.0, -90.0, 0.0], [100.0, -90.0, 0.0], [0.0, 100.0, 0.0]],
            vec![[0, 1, 2]],
        )],
        instances: vec![Instance::new(0, DMat4::IDENTITY, 0, 7)],
        materials: vec![MaterialData::new(
            "test surface",
            ShadingGraph {
                nodes: vec![IrNode::Color([0.1, 0.2, 0.3])],
                outputs: Outputs {
                    diffuse: Some(Diffuse {
                        color: 0,
                        level: 1.0,
                        falloff: 0.0,
                        model: DiffuseModel::Lambertian,
                        roughness: 0.0,
                    }),
                    ..Outputs::default()
                },
            },
        )],
        lights: vec![light],
        ..Snapshot::default()
    };
    let settings = RenderSettings {
        aovs: vec![Aov::Depth, Aov::Normal, Aov::ObjectId, Aov::MaterialId],
        ..RenderSettings::new(32, 32, 1)
    };
    let engine = new_engine();
    let render = |engine: &RtEngine, scene: &Snapshot| {
        engine
            .render_with_backend(scene, &settings, &Control::new())
            .map(|(out, _)| out)
    };
    let resident = || {
        engine
            .raster_scene
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .clone()
    };
    let check = |scene: &Snapshot| {
        let warm = render(&engine, scene).unwrap();
        let cold = render(&new_engine(), scene).unwrap();
        assert_eq!(warm.beauty.data, cold.beauty.data);
        for aov in &settings.aovs {
            assert_eq!(warm.aovs[aov].data, cold.aovs[aov].data, "{aov:?}");
        }
        warm
    };

    let first = check(&scene);
    assert!(first.aovs[&Aov::ObjectId].data.contains(&8.0));
    let initial = resident();
    scene.camera.transform[12] += 35.0;
    let moved_camera = check(&scene);
    assert_ne!(
        first.aovs[&Aov::Depth].data,
        moved_camera.aovs[&Aov::Depth].data
    );
    assert!(Arc::ptr_eq(&initial, &resident()));
    scene.lights[0].intensity *= 2.0;
    let brighter = check(&scene);
    assert_ne!(moved_camera.beauty.data, brighter.beauty.data);
    assert!(Arc::ptr_eq(&initial, &resident()));
    scene.materials[0].graph.nodes[0] = IrNode::Color([0.7, 0.6, 0.5]);
    let recolored = check(&scene);
    assert_ne!(brighter.beauty.data, recolored.beauty.data);
    assert!(Arc::ptr_eq(&initial, &resident()));
    let materials = std::mem::take(&mut scene.materials);
    assert!(matches!(
        render(&engine, &scene),
        Err(RenderError::Snapshot(_))
    ));
    assert!(Arc::ptr_eq(&initial, &resident()));
    scene.materials = materials;
    let projection = scene.camera.projection;
    scene.camera.projection = Projection::Orthographic { width: 0.0 };
    assert!(matches!(
        render(&engine, &scene),
        Err(RenderError::Snapshot(_))
    ));
    scene.camera.projection = projection;
    scene.meshes[0].triangles[0][0] = 3;
    assert!(matches!(
        render(&engine, &scene),
        Err(RenderError::Snapshot(_))
    ));
    scene.meshes[0].triangles[0][0] = 0;
    assert!(Arc::ptr_eq(&initial, &resident()));

    scene.instances[0].transform[12] += 30.0;
    let moved = check(&scene);
    assert_ne!(
        recolored.aovs[&Aov::Depth].data,
        moved.aovs[&Aov::Depth].data
    );
    let transformed = resident();
    assert!(!Arc::ptr_eq(&initial, &transformed));
    scene.meshes[0].positions[0][1] -= 20.0;
    check(&scene);
    let edited = resident();
    assert!(!Arc::ptr_eq(&transformed, &edited));
    scene.instances[0].object_id = 42;
    assert!(check(&scene).aovs[&Aov::ObjectId].data.contains(&43.0));
    assert!(!Arc::ptr_eq(&edited, &resident()));
}
