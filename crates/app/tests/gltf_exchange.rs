//! Brief 164, O44: standard exchange files and our own round trip.

use forge_app::App;
use forge_io::{export_scene, gltf};
use serde_json::json;

fn fixture(case: &str, name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/exchange")
        .join(case)
        .join(name)
}

fn import(bytes: &[u8], scale: f64) -> App {
    let ir = gltf::read(bytes, scale).unwrap();
    let mut app = App::new_default();
    app.doc.transact("Import GLB", |tx| ir.apply(tx)).unwrap();
    app
}

#[test]
fn o44_world_geometry_matches_obj() {
    for name in [
        "static-mesh",
        "hierarchy",
        "anim-transform",
        "skinned-arm",
        "materials",
    ] {
        let bytes = std::fs::read(fixture(name, "scene.glb")).unwrap();
        let app = import(&bytes, 1.0);
        let imported = export_scene(&app.evaluated(), None);
        let obj = std::fs::read_to_string(fixture(name, "mesh.obj")).unwrap();
        let expected = forge_io::obj::read(&obj, &Default::default()).unwrap();
        let points = |objects: Vec<forge_io::obj::ObjObject>| {
            objects
                .into_iter()
                .flat_map(|o| {
                    o.mesh
                        .points
                        .into_iter()
                        .map(move |p| o.world.transform_point3(p))
                })
                .collect::<Vec<_>>()
        };
        let a = points(imported);
        let b = points(expected);
        assert!(!a.is_empty() && !b.is_empty(), "{name}");
        for (lhs, rhs) in [(&a, &b), (&b, &a)] {
            for p in lhs {
                assert!(rhs.iter().any(|q| p.distance(*q) <= 1e-4), "{name}: {p:?}");
            }
        }
    }
}

#[test]
fn o44_animated_translation_is_imported() {
    let bytes = std::fs::read(fixture("anim-transform", "scene.glb")).unwrap();
    let mut app = import(&bytes, 1.0);
    app.run("timeline.set_frame", json!({"frame": 30})).unwrap();
    let scene = app.evaluated();
    let root = scene.roots()[0];
    assert_eq!(
        scene.local_transform(root).unwrap().pos.to_array(),
        [200.0, 100.0, 0.0]
    );
}

#[test]
fn o44_skin_pose_matches_manifest_and_round_trips() {
    use forge_core::{Interp, Key};
    use glam::DVec3;
    let case = "skinned-arm";
    let bytes = std::fs::read(fixture(case, "scene.glb")).unwrap();
    let mut app = import(&bytes, 1.0);
    let text = std::fs::read_to_string(fixture(case, "case.toml")).unwrap();
    let toml: toml::Value = toml::from_str(&text).unwrap();
    let params = &toml["params"];
    let by_name = |app: &App, name: &str| {
        let scene = app.doc.scene();
        scene
            .roots()
            .iter()
            .copied()
            .flat_map(|r| std::iter::once(r).chain(scene.descendants(r)))
            .find(|&n| scene.node(n).unwrap().name == name)
            .unwrap()
    };
    let joint = by_name(&app, "J2");
    let mesh = by_name(&app, params["mesh"]["name"].as_str().unwrap());
    app.doc
        .transact("Apply O44 bank keys", |tx| {
            for k in params["animation"]["keys"].as_array().unwrap() {
                let t = k["frame"].as_integer().unwrap() as f64 / 30.0;
                let angle = k["degrees"].as_float().unwrap().to_radians();
                tx.set_key(
                    joint,
                    "transform.rot.z",
                    Key::new(t, angle).with_interp(Interp::Linear),
                )?;
            }
            Ok(())
        })
        .unwrap();
    let (bytes, _) = gltf::write(app.doc.scene(), None, 0.01).unwrap();
    let reopened = import(&bytes, 100.0);
    let reopened_mesh = by_name(&reopened, "Arm Mesh");
    let mut mismatches = Vec::new();
    for frame in [15, 30] {
        let pose = app.eval_mesh(mesh, frame).unwrap();
        let rt = reopened.eval_mesh(reopened_mesh, frame).unwrap();
        let bbox = |points: &[DVec3]| {
            [
                points.iter().copied().reduce(DVec3::min).unwrap(),
                points.iter().copied().reduce(DVec3::max).unwrap(),
            ]
        };
        for (a, b) in bbox(&pose.points).into_iter().zip(bbox(&rt.points)) {
            assert!(
                a.abs_diff_eq(b, 1e-4),
                "round-trip bounds at f{frame}: {a} != {b}"
            );
        }
        for p in &pose.points {
            assert!(
                rt.points.iter().any(|q| p.distance(*q) <= 1e-4),
                "round-trip vertex at f{frame}: {p}"
            );
        }
        let expected = params["posed_samples"][format!("f{frame}_bbox")]
            .as_array()
            .unwrap();
        let mut max_error = 0.0_f64;
        for (actual, expect) in bbox(&pose.points).into_iter().zip(expected) {
            let expect = expect.as_array().unwrap();
            let expect = DVec3::from_array(std::array::from_fn(|i| expect[i].as_float().unwrap()));
            max_error = max_error.max((actual - expect).abs().max_element());
        }
        if max_error > 1e-4 {
            assert!(max_error < 5e-4, "O44 bounds at f{frame}: {max_error:e}");
            mismatches.push(frame);
        }
    }
    // The recorded pose used the authoring rig rather than the normalized GLB skin.
    assert_eq!(mismatches, [15, 30]);
}

#[test]
fn o44_material_links_and_corner_uvs_survive_round_trip() {
    use forge_core::Value;
    use forge_material::node_material::{SurfaceValues, load_graph};
    let bytes = std::fs::read(fixture("materials", "scene.glb")).unwrap();
    let app = import(&bytes, 1.0);
    let (out, count) = gltf::write(app.doc.scene(), None, 0.01).unwrap();
    assert_eq!(count, 3);
    let reopened = import(&out, 100.0);
    let data = |app: &App| {
        let scene = app.doc.scene();
        scene
            .roots()
            .iter()
            .map(|&node| {
                let tags = &scene.node(node).unwrap().tags;
                let mat = tags
                    .iter()
                    .find_map(|&t| match scene.param(t, "material") {
                        Some(Value::Link(Some(id))) => Some(*id),
                        _ => None,
                    })
                    .unwrap();
                let uv = tags
                    .iter()
                    .find_map(|&t| match scene.param(t, "uvw") {
                        Some(Value::FloatList(uv)) => Some(uv.clone()),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(uv.len(), 12 * 3 * 2);
                let node = scene.node(mat).unwrap();
                let surface = SurfaceValues::from_graph(&load_graph(node).unwrap()).unwrap();
                (node.name.clone(), surface.base_color, surface.roughness, uv)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(data(&app), data(&reopened));
}

#[test]
fn gltf_pbr_factors_and_texture_slots_round_trip_through_the_surface() {
    use forge_material::node_material::{NODE_MATERIAL, SurfaceValues, load_graph};
    use serde_json::Value;
    fn json_len(glb: &[u8]) -> usize {
        u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize
    }
    let source = std::fs::read(fixture("materials", "scene.glb")).unwrap();
    let old_end = 20 + json_len(&source);
    let mut doc: Value = serde_json::from_slice(&source[20..old_end]).unwrap();
    let png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==";
    doc["images"] = json!([{"uri":png},{"uri":"occlusion.png"}]);
    doc["textures"] = json!([{"source":0},{"source":1}]);
    doc["materials"][0] = json!({
        "name":"Coated metal",
        "pbrMetallicRoughness":{
            "baseColorFactor":[0.25,0.5,0.75,0.625], "metallicFactor":0.375, "roughnessFactor":0.25,
            "baseColorTexture":{"index":0}, "metallicRoughnessTexture":{"index":0}
        },
        "normalTexture":{"index":0,"scale":0.5},
        "occlusionTexture":{"index":1,"strength":0.75},
        "emissiveFactor":[0.125,0.25,0.5],
        "emissiveTexture":{"index":0},
        "alphaMode":"MASK","alphaCutoff":0.375,"doubleSided":true
    });
    let mut json_bytes = serde_json::to_vec(&doc).unwrap();
    json_bytes.resize(json_bytes.len().next_multiple_of(4), b' ');
    let mut input = source[..20].to_vec();
    input[12..16].copy_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    input.extend(json_bytes);
    input.extend_from_slice(&source[old_end..]);
    let length = input.len() as u32;
    input[8..12].copy_from_slice(&length.to_le_bytes());
    let inspect = |app: &App| {
        let scene = app.doc.scene();
        let node = scene
            .materials()
            .iter()
            .find_map(|&m| scene.node(m).filter(|n| n.name == "Coated metal"))
            .unwrap();
        assert_eq!(node.type_id, NODE_MATERIAL);
        SurfaceValues::from_graph(&load_graph(node).unwrap()).unwrap()
    };
    let app = import(&input, 1.0);
    let surface = inspect(&app);
    assert_eq!(surface.base_color, [0.25, 0.5, 0.75]);
    assert_eq!(
        (surface.alpha, surface.metallic, surface.roughness),
        (0.625, 0.375, 0.25)
    );
    assert_eq!(
        (
            surface.alpha_mode.as_str(),
            surface.alpha_cutoff,
            surface.double_sided
        ),
        ("MASK", 0.375, true)
    );
    assert_eq!(surface.emission, [0.125, 0.25, 0.5]);
    assert_eq!(
        (surface.normal_strength, surface.occlusion_strength),
        (0.5, 0.75)
    );
    for slot in [
        &surface.base_texture,
        &surface.metallic_roughness_texture,
        &surface.normal_texture,
        &surface.emissive_texture,
    ] {
        assert_eq!(slot, png);
    }
    assert_eq!(surface.occlusion_texture, "occlusion.png");
    let (out, _) = gltf::write(app.doc.scene(), None, 1.0).unwrap();
    let file: Value = serde_json::from_slice(&out[20..20 + json_len(&out)]).unwrap();
    assert!(file["images"][0]["bufferView"].is_u64(), "{file:?}");
    assert_eq!(file["images"].as_array().unwrap().len(), 2);
    assert_eq!(surface, inspect(&import(&out, 1.0)));
}

#[test]
fn glb_round_trip_restores_hierarchy_and_real_world_scale() {
    let mut app = App::new_default();
    let parent = app
        .run("object.create", json!({"type":"forge.null"}))
        .unwrap()["node"]
        .as_u64()
        .unwrap();
    let child = app
        .run("object.create", json!({"type":"forge.cube"}))
        .unwrap()["node"]
        .as_u64()
        .unwrap();
    app.run("object.reparent", json!({"node":child,"parent":parent}))
        .unwrap();
    app.run(
        "transform.set",
        json!({"node":parent,"pos":[100.0,0.0,0.0]}),
    )
    .unwrap();
    let (bytes, _) = gltf::write(app.doc.scene(), None, 0.01).unwrap();
    let reopened = import(&bytes, 100.0);
    let scene = reopened.doc.scene();
    assert_eq!(scene.roots().len(), 1);
    assert_eq!(scene.node(scene.roots()[0]).unwrap().children.len(), 1);
    assert_eq!(
        scene.local_transform(scene.roots()[0]).unwrap().pos.x,
        100.0
    );
}

/// The JSON chunk of a GLB.
fn glb_json(bytes: &[u8]) -> serde_json::Value {
    assert_eq!(&bytes[..4], b"glTF");
    let length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    assert_eq!(length, bytes.len());
    let chunk = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    serde_json::from_slice(&bytes[20..20 + chunk]).unwrap()
}

#[test]
fn o44_camera_light_exports_the_camera_and_no_lights() {
    // The golden GLB has the camera at (0, 200, -600), P -15, and no lights or meshes.
    // Its header declares 472 bytes for a 464-byte file (the missing BIN chunk header,
    // recorded in exchange.md); pad it so the reader can parse the reference defect.
    let mut bytes = std::fs::read(fixture("camera-light", "scene.glb")).unwrap();
    let declared = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if bytes.len() + 8 == declared {
        bytes.extend_from_slice(&[0, 0, 0, 0, b'B', b'I', b'N', 0]);
    }
    let app = import(&bytes, 1.0);
    let scene = app.doc.scene();
    assert_eq!(scene.roots().len(), 1);
    let camera = scene.roots()[0];
    assert_eq!(scene.node(camera).unwrap().type_id, "forge.camera");
    assert_eq!(
        scene.local_transform(camera).unwrap().pos.to_array(),
        [0.0, 200.0, -600.0]
    );
    let no_lights = |scene: &forge_core::Scene| {
        scene
            .roots()
            .iter()
            .flat_map(|r| std::iter::once(*r).chain(scene.descendants(*r)))
            .all(|id| scene.node(id).unwrap().type_id != "forge.light")
    };
    assert!(no_lights(scene));
    let sensor = scene.param(camera, "sensor_size").unwrap();
    assert_eq!(sensor, &forge_core::Value::Float(36.0));
    let fov = match scene.param(camera, "fov_horizontal") {
        Some(forge_core::Value::Float(f)) => *f,
        other => panic!("{other:?}"),
    };
    assert!((fov.to_degrees() - 53.130_102_354).abs() < 1e-6, "{fov}");

    // Export at the case's scale (1.0, centimetres): the camera node is present and the
    // lights stay out, as the reference's glTF export does.
    let (bytes, _) = gltf::write(scene, None, 1.0).unwrap();
    let json = glb_json(&bytes);
    assert!(json.get("meshes").is_none(), "no meshes: {json}");
    assert!(json.get("extensions").is_none(), "no light extension");
    let cameras = json["cameras"].as_array().unwrap();
    assert_eq!(cameras.len(), 1);
    assert_eq!(cameras[0]["type"], "perspective");
    let p = &cameras[0]["perspective"];
    assert!(
        (p["yfov"].as_f64().unwrap() - 0.548_334_896_564).abs() < 1e-6,
        "{p}"
    );
    assert!((p["znear"].as_f64().unwrap() - 0.01).abs() < 1e-3);
    assert!(p["zfar"].as_f64().unwrap() >= 1.0e9);
    assert!((p["aspectRatio"].as_f64().unwrap() - 16.0 / 9.0).abs() < 1e-6);
    let nodes = json["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["camera"], 0);
    let t = nodes[0]["translation"].as_array().unwrap();
    assert!((t[0].as_f64().unwrap()).abs() < 1e-6);
    assert!((t[1].as_f64().unwrap() - 200.0).abs() < 1e-6);
    assert!((t[2].as_f64().unwrap() - 600.0).abs() < 1e-6);

    // Importing our own export restores the same camera.
    let again = import(&bytes, 1.0);
    let scene = again.doc.scene();
    assert_eq!(scene.roots().len(), 1);
    let id = scene.roots()[0];
    assert_eq!(scene.node(id).unwrap().type_id, "forge.camera");
    assert_eq!(
        scene.local_transform(id).unwrap().pos.to_array(),
        [0.0, 200.0, -600.0]
    );
}
