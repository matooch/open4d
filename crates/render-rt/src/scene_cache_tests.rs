use super::*;
use forge_material::ir::{Diffuse, DiffuseModel, IrNode, Outputs, Specular};
use forge_render_api::{Instance, Light, LightKind, Lighting, MaterialData, MeshData, ShadowMode};
use forge_shaders::ShaderSet;
use forge_shaders::shader::{Bitmap, Shader};
use glam::{DMat4, DVec3};

#[test]
fn raster_geometry_is_resident_until_its_inputs_change() {
    let _serial = crate::gpu_test_serial();
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
    light.specular = false;
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
                    specular: Some(Specular {
                        kind: "plastic".into(),
                        width: 0.3,
                        falloff: 0.0,
                        inner_width: 0.0,
                        strength: 1.0,
                        color: 0,
                        metal: false,
                        colored: true,
                    }),
                    ..Outputs::default()
                },
            },
        )],
        lights: vec![light],
        ..Snapshot::default()
    };
    scene.environment.lighting = Lighting::Constant {
        color: [0.8, 0.6, 0.5],
        intensity: 1.0,
    };
    let mut settings = RenderSettings {
        default_light: false,
        region: Some([2, 1, 30, 29]),
        aovs: vec![
            Aov::Depth,
            Aov::Normal,
            Aov::ObjectId,
            Aov::MaterialId,
            Aov::Diffuse,
            Aov::Specular,
        ],
        ..RenderSettings::new(32, 32, 1)
    };
    settings.hybrid.enabled = true;
    settings.hybrid.shadows = true;
    settings.hybrid.reflections = true;
    settings.hybrid.gi = true;
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
    let program = || {
        engine
            .raster_program
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .clone()
    };
    let shadow = || engine.hybrid.lock().unwrap().shadow_residency().unwrap();
    let reflection = || {
        engine
            .hybrid
            .lock()
            .unwrap()
            .reflection_residency()
            .unwrap()
    };
    let diffuse = || engine.hybrid.lock().unwrap().diffuse_residency().unwrap();
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
    let initial_program = program();
    let initial_shadow = shadow();
    let initial_reflection = reflection();
    let initial_diffuse = diffuse();
    assert_eq!(initial_shadow.0, initial_reflection.0);
    assert_eq!(initial_shadow.0, initial_diffuse.0);
    assert_ne!(initial_reflection.1, initial_diffuse.1);
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
    assert!(Arc::ptr_eq(&initial_program, &program()));
    assert_eq!(initial_shadow, shadow());
    assert_eq!(initial_reflection, reflection());
    assert_eq!(initial_diffuse, diffuse());
    render(&engine.clone(), &scene).unwrap();
    assert!(Arc::ptr_eq(&initial_program, &program()));
    scene.materials[0].graph.nodes[0] = IrNode::Color([0.7, 0.6, 0.5]);
    let recolored = check(&scene);
    assert_ne!(brighter.beauty.data, recolored.beauty.data);
    assert!(Arc::ptr_eq(&initial, &resident()));
    let recolored_program = program();
    assert!(!Arc::ptr_eq(&initial_program, &recolored_program));

    let bitmap = |rgb: [f32; 3]| {
        let mut set = ShaderSet::default();
        let mut image = Image::new(1, 1, AlphaMode::Straight);
        image.set_pixel(0, 0, [rgb[0], rgb[1], rgb[2], 1.0]);
        set.shaders.insert(
            "color".into(),
            Shader::Bitmap(Bitmap {
                image: Arc::new(image),
                repeat: true,
                interpolate: false,
            }),
        );
        Arc::new(set)
    };
    scene.materials[0].graph.nodes[0] = IrNode::Texture("color".into());
    scene.materials[0].sampler = bitmap([0.6, 0.2, 0.1]);
    let textured = check(&scene);
    assert_ne!(recolored.beauty.data, textured.beauty.data);
    let textured_program = program();
    let textured_shadow = shadow();
    let textured_reflection = reflection();
    let textured_diffuse = diffuse();
    assert_eq!(textured_shadow.0, textured_reflection.0);
    assert_ne!(initial_reflection.1, textured_reflection.1);
    assert_eq!(textured_shadow.0, textured_diffuse.0);
    assert_ne!(initial_diffuse.1, textured_diffuse.1);
    assert_eq!(initial_shadow.0, textured_shadow.0);
    assert_ne!(initial_shadow.1, textured_shadow.1);
    assert!(!Arc::ptr_eq(&recolored_program, &textured_program));
    assert_eq!(textured_program.bitmap_count, 1);
    scene.materials[0].sampler = bitmap([0.1, 0.2, 0.6]);
    let retextured = check(&scene);
    assert_ne!(textured.beauty.data, retextured.beauty.data);
    assert_ne!(
        textured.aovs[&Aov::Specular].data,
        retextured.aovs[&Aov::Specular].data
    );
    assert!(Arc::ptr_eq(&textured_program, &program()));
    assert_eq!(textured_shadow, shadow());
    assert_eq!(textured_reflection, reflection());
    assert_eq!(textured_diffuse, diffuse());
    let lighting = std::mem::replace(&mut scene.environment.lighting, Lighting::None);
    let unlit = check(&scene);
    assert!(
        unlit.aovs[&Aov::Specular]
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[..3] == [0.0; 3])
    );
    assert_eq!(textured_reflection, reflection());
    assert_eq!(textured_diffuse, diffuse());
    scene.environment.lighting = lighting;
    scene.materials[0].graph.nodes[0] = IrNode::Color([0.7, 0.6, 0.5]);
    assert_eq!(recolored.beauty.data, check(&scene).beauty.data);
    assert!(!Arc::ptr_eq(&textured_program, &program()));
    assert_eq!(program().bitmap_count, 0);
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
    assert_ne!(initial_shadow.0, shadow().0);
    let transformed_shadow = shadow();
    let transformed_reflection = reflection();
    let transformed_diffuse = diffuse();
    assert_eq!(transformed_shadow.0, transformed_reflection.0);
    assert_ne!(initial_reflection.0, transformed_reflection.0);
    assert_eq!(transformed_shadow.0, transformed_diffuse.0);
    assert_ne!(initial_diffuse.0, transformed_diffuse.0);
    scene.meshes[0].positions[0][1] -= 20.0;
    check(&scene);
    let edited = resident();
    assert!(!Arc::ptr_eq(&transformed, &edited));
    assert_ne!(transformed_shadow.0, shadow().0);
    assert_eq!(transformed_shadow.1, shadow().1);
    assert_ne!(transformed_reflection.0, reflection().0);
    assert_eq!(transformed_reflection.1, reflection().1);
    assert_ne!(transformed_diffuse.0, diffuse().0);
    assert_eq!(transformed_diffuse.1, diffuse().1);
    scene.instances[0].object_id = 42;
    assert!(check(&scene).aovs[&Aov::ObjectId].data.contains(&43.0));
    assert!(!Arc::ptr_eq(&edited, &resident()));

    // An emitter behind the camera supplies the only light, so any diffuse
    // signal on the visible receiver must come from GI.
    scene.lights.clear();
    scene.environment.lighting = Lighting::None;
    scene.meshes.push(MeshData::new(
        vec![
            [-5000.0, -5000.0, 0.0],
            [5000.0, -5000.0, 0.0],
            [0.0, 5000.0, 0.0],
        ],
        vec![[0, 1, 2]],
    ));
    scene.instances.push(Instance::new(
        1,
        DMat4::from_translation(DVec3::new(0.0, 0.0, -600.0)),
        1,
        3,
    ));
    scene.materials.push(MaterialData::new(
        "indirect emitter",
        ShadingGraph {
            nodes: vec![IrNode::Color([1.0; 3])],
            outputs: Outputs {
                emission: Some(0),
                ..Outputs::default()
            },
        },
    ));
    scene.materials[0].graph.nodes[0] = IrNode::Texture("color".into());
    scene.materials[0].sampler = bitmap([0.6, 0.2, 0.1]);
    let red = check(&scene);
    let emitting_diffuse = diffuse();
    scene.materials[0].sampler = bitmap([0.1, 0.2, 0.6]);
    let blue = check(&scene);
    assert_eq!(emitting_diffuse, diffuse());
    assert!(
        red.aovs[&Aov::Diffuse]
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .zip(blue.aovs[&Aov::Diffuse].data.as_chunks::<4>().0)
            .any(|(red, blue)| red[0] > 0.1 && red[0] > red[2] && blue[2] > blue[0])
    );
}
