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
                (
                    scene.node(mat).unwrap().name.clone(),
                    scene.param(mat, "color.color").unwrap().clone(),
                    scene.param(mat, "color.roughness").unwrap().clone(),
                    uv,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(data(&app), data(&reopened));
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
