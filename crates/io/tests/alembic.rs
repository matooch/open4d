//! O44 Alembic imports and independent authored round trips (IO.md §7).

use std::{path::Path, sync::Arc};

use forge_core::{Document, Interp, Key, MeshData, NodeId, Registry, Track, Value};
use forge_generators::Primitives;
use forge_io::{
    export_scene,
    formats::{Options, export_bytes, read_scene},
};
use forge_parity_harness::{Mesh, compare_geometric, discover_cases};

fn registry() -> Arc<Registry> {
    let mut registry = Registry::with_builtins();
    registry.install(&Primitives).unwrap();
    registry.install(&forge_material::MaterialPlugin).unwrap();
    Arc::new(registry)
}

fn world_mesh(scene: &forge_core::Scene) -> Mesh {
    let mut mesh = Mesh::default();
    for object in export_scene(scene, None) {
        let offset = mesh.positions.len() as u32;
        mesh.positions.extend(
            object
                .mesh
                .points
                .iter()
                .map(|p| object.world.transform_point3(*p).to_array()),
        );
        mesh.faces.extend(
            object
                .mesh
                .polys
                .iter()
                .map(|p| p.iter().map(|i| i + offset).collect()),
        );
    }
    mesh
}

fn import(bytes: &[u8]) -> Document {
    let mut doc = Document::new(registry());
    let scene = read_scene("abc", bytes, &Options::new()).unwrap();
    doc.transact("Import Alembic", |tx| scene.apply(tx))
        .unwrap();
    doc
}

#[test]
fn o44_alembic_meshes_and_animated_samples() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let cases = discover_cases(&root).unwrap();
    let cases: Vec<_> = cases
        .into_iter()
        .filter(|c| c.feature == "exchange")
        .collect();
    assert_eq!(cases.len(), 6);
    for case in cases {
        let m = case.manifest().unwrap();
        let bytes = std::fs::read(case.dir.join("scene.abc")).unwrap();
        let doc = import(&bytes);
        let start = forge_anim::scene_at(doc.scene(), 0.0);
        let expected = case.expected_mesh().unwrap().unwrap();
        let mesh = world_mesh(&start);
        let comparison = compare_geometric(&expected, &mesh, &m.tolerance);
        assert!(
            comparison.passed(),
            "{}: {:?}",
            case.name,
            comparison.failures
        );
        let scene = doc.scene();
        let primary = scene.roots()[0];
        match case.name.as_str() {
            "hierarchy" => {
                assert_eq!(scene.node(primary).unwrap().name, "Root");
                assert_eq!(scene.node(primary).unwrap().children.len(), 1);
                assert_eq!(scene.local_transform(primary).unwrap().pos.x, 100.0);
            }
            "anim-transform" => {
                let at = forge_anim::scene_at(scene, 1.0);
                let pose = at.local_transform(primary).unwrap();
                assert_eq!(pose.pos.to_array(), [200.0, 100.0, 0.0]);
                assert!((pose.rot.x.to_degrees() - 90.0).abs() < 1e-6);
                assert_eq!(pose.scale.to_array(), [1.5; 3]);
            }
            "camera-light" => {
                assert!(mesh.positions.is_empty());
                assert_eq!(scene.roots().len(), 1);
                assert_eq!(scene.node(primary).unwrap().type_id, "forge.camera");
                assert_eq!(
                    scene.local_transform(primary).unwrap().pos.to_array(),
                    [0.0, 200.0, -600.0]
                );
                assert_eq!(
                    scene.param(primary, "sensor_size"),
                    Some(&Value::Float(36.0))
                );
            }
            "skinned-arm" => {
                assert!(
                    !scene.node(primary).unwrap().tracks["mesh.points"]
                        .points
                        .is_empty()
                );
                for (t, key) in [(0.5, "f15_bbox"), (1.0, "f30_bbox")] {
                    let posed = world_mesh(&forge_anim::scene_at(scene, t));
                    let bounds = &m.params["posed_samples"][key];
                    for axis in 0..3 {
                        let values = posed.positions.iter().map(|p| p[axis]);
                        for (i, got) in [
                            values.clone().fold(f64::INFINITY, f64::min),
                            values.fold(f64::NEG_INFINITY, f64::max),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let want = bounds.as_array().unwrap()[i].as_array().unwrap()[axis]
                                .as_float()
                                .unwrap();
                            assert!(
                                (got - want).abs() <= m.tolerance.position,
                                "{key} axis {axis}: {got} vs {want}"
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn round_trip_source() -> (Document, NodeId, NodeId) {
    let mut doc = Document::new(registry());
    let (parent, mesh) = doc
        .transact("Sampled hierarchy", |tx| {
            let parent = tx.create("forge.null", None, None)?;
            tx.rename(parent, "Parent")?;
            tx.set_param(parent, "transform.pos", Value::Vec3([1.0, 2.0, 17.0]))?;
            let mesh = tx.create("forge.polygon", Some(parent), None)?;
            tx.rename(mesh, "Quad")?;
            let points = vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ];
            let data = MeshData {
                points: points.clone(),
                polys: vec![vec![0, 1, 2, 3]],
            };
            tx.set_param(mesh, "mesh", Value::Mesh(Arc::new(data)))?;
            let uv = tx.attach_tag("forge.uvw", mesh, None)?;
            tx.set_param(
                uv,
                "uvw",
                Value::FloatList(vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]),
            )?;
            let mut deform = Track::default();
            let mut end = points.clone();
            end[2][2] = 8.0;
            assert!(deform.set_points_key(0.0, Interp::Linear, points));
            assert!(deform.set_points_key(1.0, Interp::Linear, end));
            tx.set_track(mesh, "mesh.points", Some(deform))?;
            tx.set_track(
                mesh,
                "transform.rot.y",
                Some(Track::new(vec![
                    Key::new(0.0, 0.0).with_interp(Interp::Linear),
                    Key::new(1.0, 0.6),
                ])),
            )?;
            Ok((parent, mesh))
        })
        .unwrap();
    (doc, parent, mesh)
}

#[test]
fn authored_hierarchy_polygon_uvs_and_animation_round_trip() {
    let (source, _, _) = round_trip_source();
    let (bytes, count) = export_bytes("abc", source.scene(), None, &Options::new()).unwrap();
    assert_eq!(count, 1);
    assert!(bytes.starts_with(b"Ogawa\xff\0\x01"));
    assert_eq!(
        bytes,
        export_bytes("abc", source.scene(), None, &Options::new())
            .unwrap()
            .0
    );
    let decoded = import(&bytes);
    let root = decoded.scene().roots()[0];
    assert_eq!(decoded.scene().node(root).unwrap().name, "Parent");
    let child = decoded.scene().node(root).unwrap().children[0];
    assert_eq!(decoded.scene().node(child).unwrap().name, "Quad");
    let obj = export_scene(decoded.scene(), None).remove(0);
    assert_eq!(
        obj.uvs.unwrap().corners,
        vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]]
    );
    for t in [0.0, 0.5, 1.0] {
        let before = world_mesh(&forge_anim::scene_at(source.scene(), t));
        let after = world_mesh(&forge_anim::scene_at(decoded.scene(), t));
        assert_eq!(before.faces, after.faces);
        for (a, b) in before.positions.iter().zip(&after.positions) {
            assert!(
                a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-4),
                "{t}: {a:?} vs {b:?}"
            );
        }
    }
}

#[test]
fn ogawa_rejects_unfrozen_truncated_or_invalid_pointers() {
    let (source, _, _) = round_trip_source();
    let (bytes, _) = export_bytes("abc", source.scene(), None, &Options::new()).unwrap();
    for limit in [0, 15, bytes.len() - 1] {
        assert!(read_scene("abc", &bytes[..limit], &Options::new()).is_err());
    }
    let mut unfrozen = bytes.clone();
    unfrozen[5] = 0;
    assert!(read_scene("abc", &unfrozen, &Options::new()).is_err());
    unfrozen[5] = 0xff;
    unfrozen[8..16].fill(0xff);
    assert!(read_scene("abc", &unfrozen, &Options::new()).is_err());
}

#[test]
fn detached_selection_bakes_world_transform_and_applies_scale_to_both_spaces() {
    let (source, _, mesh) = round_trip_source();
    let opts = Options::from([("scale".into(), Value::Float(2.0))]);
    let (bytes, count) = export_bytes("abc", source.scene(), Some(&[mesh]), &opts).unwrap();
    assert_eq!(count, 1);
    let decoded = import(&bytes);
    let root = decoded.scene().roots()[0];
    assert_eq!(decoded.scene().node(root).unwrap().name, "Quad");
    assert_eq!(
        decoded
            .scene()
            .local_transform(root)
            .unwrap()
            .pos
            .to_array(),
        [2.0, 4.0, 34.0]
    );
    let at = world_mesh(&forge_anim::scene_at(decoded.scene(), 0.0));
    assert_eq!(at.positions[2], [4.0, 6.0, 34.0]);
}

#[test]
fn perspective_camera_round_trips_without_a_mesh() {
    let mut doc = Document::new(registry());
    doc.transact("Camera", |tx| {
        let camera = tx.create("forge.camera", None, None)?;
        tx.set_param(camera, "transform.pos", Value::Vec3([10.0, 5.0, -70.0]))?;
        tx.set_param(camera, "sensor_size", Value::Float(28.0))?;
        tx.set_param(camera, "fov_horizontal", Value::Float(0.85))?;
        tx.set_param(camera, "focus_distance", Value::Float(220.0))
    })
    .unwrap();
    let (bytes, count) = export_bytes("abc", doc.scene(), None, &Options::new()).unwrap();
    assert_eq!(count, 1);
    let decoded = import(&bytes);
    let id = decoded.scene().roots()[0];
    assert_eq!(decoded.scene().node(id).unwrap().type_id, "forge.camera");
    for field in ["sensor_size", "fov_horizontal", "focus_distance"] {
        let Some(Value::Float(before)) = doc.scene().param(doc.scene().roots()[0], field) else {
            panic!()
        };
        let Some(Value::Float(after)) = decoded.scene().param(id, field) else {
            panic!()
        };
        assert!((before - after).abs() < 1e-12, "{field}");
    }
    assert!(world_mesh(decoded.scene()).positions.is_empty());
}
