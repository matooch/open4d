//! Brief 166 (IO.md §8.8): our ASCII FBX writer, checked by importing what it writes with the
//! brief 165 importer. The six O44 scenes go import, export, import; scenes built in code
//! cover hierarchy, animation, cameras and lights, materials and skins.

use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, Interp, Key, MeshData, NodeId, Registry, Scene, Track, Value};
use forge_generators::{Primitives, evaluate};
use forge_io::export_scene;
use forge_io::formats::{
    IoError, Options, export_bytes, export_options, export_text, format_by_id, read_scene,
};
use forge_material::node_material::{SurfaceValues, load_graph};
use forge_modifiers::Deformers;
use forge_parity_harness::{Mesh, compare_geometric, discover_cases};
use forge_rig::Rig;
use glam::{DMat4, DVec3};

fn registry() -> Arc<Registry> {
    let mut registry = Registry::with_builtins();
    registry.install(&Primitives).unwrap();
    registry.install(&Deformers).unwrap();
    registry.install(&Rig).unwrap();
    registry.install(&forge_material::MaterialPlugin).unwrap();
    Arc::new(registry)
}

fn options(scale: f64) -> Options {
    Options::from([("scale".into(), Value::Float(scale))])
}

fn import_with(bytes: &[u8], scale: f64) -> Document {
    let mut doc = Document::new(registry());
    let scene = read_scene("fbx", bytes, &options(scale)).unwrap();
    doc.transact("Import FBX", |tx| scene.apply(tx)).unwrap();
    doc
}

fn export(scene: &Scene, roots: Option<&[NodeId]>, scale: f64) -> Vec<u8> {
    export_bytes("fbx", scene, roots, &options(scale))
        .unwrap()
        .0
}

fn find(scene: &Scene, name: &str) -> NodeId {
    preorder(scene)
        .into_iter()
        .find(|&n| scene.node(n).unwrap().name == name)
        .unwrap_or_else(|| panic!("no object named {name}"))
}

/// Every object, parents before children, in sibling order.
fn preorder(scene: &Scene) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack: Vec<NodeId> = scene.roots().iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        out.push(id);
        stack.extend(scene.node(id).unwrap().children.iter().rev());
    }
    out
}

fn kind(scene: &Scene, id: NodeId) -> String {
    let type_id = &scene.node(id).unwrap().type_id;
    match type_id.as_str() {
        "forge.camera" | "forge.light" | "forge.joint" => type_id.clone(),
        _ => match evaluate(scene, id, &EvalCtx::default()) {
            Ok(Some(m)) if !m.polys.is_empty() => "mesh".into(),
            _ => type_id.clone(),
        },
    }
}

fn matrices_close(a: DMat4, b: DMat4, tolerance: f64, what: &str) {
    for (x, y) in a.to_cols_array().iter().zip(b.to_cols_array()) {
        assert!((x - y).abs() <= tolerance, "{what}: {a:?} vs {b:?}");
    }
}

fn float(scene: &Scene, id: NodeId, param: &str) -> Option<f64> {
    match scene.param(id, param) {
        Some(Value::Float(f)) => Some(*f),
        _ => None,
    }
}

type Surface = (String, [f64; 3], f64, f64, [f64; 3]);

fn surfaces(scene: &Scene) -> Vec<Surface> {
    let mut v: Vec<_> = scene
        .materials()
        .iter()
        .map(|&m| {
            let node = scene.node(m).unwrap();
            let s = SurfaceValues::from_graph(&load_graph(node).unwrap()).unwrap();
            (
                node.name.clone(),
                s.base_color,
                s.roughness,
                s.alpha,
                s.emission,
            )
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

/// (material name, selection name) of every texture tag of an object, in tag order, and the
/// polygons of every selection tag.
type Assignments = (Vec<(String, String)>, Vec<(String, Vec<u32>)>);

fn assignments(scene: &Scene, id: NodeId) -> Assignments {
    let node = scene.node(id).unwrap();
    let mut textures = Vec::new();
    let mut selections = Vec::new();
    for &tag in &node.tags {
        let t = scene.node(tag).unwrap();
        match t.type_id.as_str() {
            "forge.tag.texture" => {
                let Some(Value::Link(Some(m))) = scene.param(tag, "material") else {
                    continue;
                };
                let selection = match scene.param(tag, "selection") {
                    Some(Value::String(s)) => s.clone(),
                    _ => String::new(),
                };
                textures.push((scene.node(*m).unwrap().name.clone(), selection));
            }
            "forge.polygon_selection" => {
                if let Some(Value::IndexList(l)) = scene.param(tag, "indices") {
                    selections.push((t.name.clone(), l.clone()));
                }
            }
            _ => {}
        }
    }
    (textures, selections)
}

/// `a` and `b` are the same scene: objects, kinds, world matrices at `times`, meshes (points
/// in order, faces, UVs), camera and light settings, materials and their assignment.
/// The scene with its tracks evaluated at `time` exactly (`scene_at` rounds to milliseconds).
fn at(scene: &Scene, time: f64) -> Scene {
    scene
        .with_params(forge_anim::overrides_exact(
            scene,
            &forge_anim::track_nodes(scene),
            time,
        ))
        .0
}

fn assert_same_scene(a: &Scene, b: &Scene, times: &[f64], tolerance: f64) {
    let (na, nb) = (preorder(a), preorder(b));
    assert_eq!(na.len(), nb.len(), "object count");
    for (&x, &y) in na.iter().zip(&nb) {
        let (nx, ny) = (a.node(x).unwrap(), b.node(y).unwrap());
        assert_eq!(nx.name, ny.name);
        assert_eq!(
            nx.children.len(),
            ny.children.len(),
            "{}: children",
            nx.name
        );
        assert_eq!(kind(a, x), kind(b, y), "{}: kind", nx.name);
        for &t in times {
            let (sa, sb) = (at(a, t), at(b, t));
            matrices_close(
                sa.world_matrix(x).unwrap(),
                sb.world_matrix(y).unwrap(),
                tolerance,
                &format!("{} at {t}", nx.name),
            );
        }
        match nx.type_id.as_str() {
            "forge.camera" => {
                for p in [
                    "sensor_size",
                    "fov_horizontal",
                    "focus_distance",
                    "near_clipping",
                    "far_clipping",
                    "zoom",
                ] {
                    let (u, v) = (float(a, x, p), float(b, y, p));
                    assert!(
                        u.zip(v)
                            .is_none_or(|(u, v)| (u - v).abs() <= 1e-9 * u.abs().max(1.0)),
                        "{}: {p} {u:?} vs {v:?}",
                        nx.name
                    );
                }
                assert_eq!(a.param(x, "projection"), b.param(y, "projection"));
            }
            "forge.light" => {
                for p in ["intensity", "inner_angle", "outer_angle"] {
                    let (u, v) = (float(a, x, p), float(b, y, p));
                    assert!(
                        u.zip(v).is_none_or(|(u, v)| (u - v).abs() <= 1e-9),
                        "{}: {p} {u:?} vs {v:?}",
                        nx.name
                    );
                }
                assert_eq!(a.param(x, "type"), b.param(y, "type"), "{}", nx.name);
                assert_eq!(a.param(x, "color"), b.param(y, "color"), "{}", nx.name);
            }
            _ => {}
        }
        assert_eq!(
            assignments(a, x),
            assignments(b, y),
            "{}: materials",
            nx.name
        );
    }
    let (sa, sb) = (surfaces(a), surfaces(b));
    assert_eq!(sa.len(), sb.len(), "materials");
    for (x, y) in sa.iter().zip(&sb) {
        assert_eq!(x.0, y.0);
        let close = |u: &[f64], v: &[f64]| u.iter().zip(v).all(|(u, v)| (u - v).abs() < 1e-9);
        assert!(close(&x.1, &y.1), "{}: base colour {x:?} vs {y:?}", x.0);
        assert!(close(&[x.2, x.3], &[y.2, y.3]), "{}: {x:?} vs {y:?}", x.0);
        assert!(close(&x.4, &y.4), "{}: emission {x:?} vs {y:?}", x.0);
    }
    for &t in times {
        let (ma, mb) = (export_scene(&at(a, t), None), export_scene(&at(b, t), None));
        assert_eq!(ma.len(), mb.len(), "mesh objects at {t}");
        for (oa, ob) in ma.iter().zip(&mb) {
            assert_eq!(oa.name, ob.name);
            assert_eq!(oa.mesh.polys, ob.mesh.polys, "{}: faces", oa.name);
            assert_eq!(oa.mesh.points.len(), ob.mesh.points.len());
            for (p, q) in oa.mesh.points.iter().zip(&ob.mesh.points) {
                let (p, q) = (oa.world.transform_point3(*p), ob.world.transform_point3(*q));
                assert!(
                    p.distance(q) <= tolerance,
                    "{} at {t}: {p:?} vs {q:?}",
                    oa.name
                );
            }
            match (&oa.uvs, &ob.uvs) {
                (None, None) => {}
                (Some(u), Some(v)) => {
                    for (fu, fv) in u.corners.iter().zip(&v.corners) {
                        for (cu, cv) in fu.iter().zip(fv) {
                            assert!((cu[0] - cv[0]).abs() < 1e-12 && (cu[1] - cv[1]).abs() < 1e-12);
                        }
                    }
                }
                _ => panic!("{}: UVs lost or invented", oa.name),
            }
        }
    }
}

fn world_mesh(scene: &Scene) -> Mesh {
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

#[test]
fn the_o44_scenes_survive_import_export_import() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let cases: Vec<_> = discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "exchange")
        .collect();
    assert_eq!(cases.len(), 6);
    for case in cases {
        let m = case.manifest().unwrap();
        let original = import_with(&std::fs::read(case.dir.join("scene.fbx")).unwrap(), 1.0);
        let bytes = export(original.scene(), None, 1.0);
        assert!(
            bytes.starts_with(b"; FBX 7.4.0 project file"),
            "{}: header",
            case.name
        );
        let again = import_with(&bytes, 1.0);
        let times = [0.0, 0.25, 0.5, 1.0];
        assert_same_scene(original.scene(), again.scene(), &times, 1e-4);
        // and against the reference geometry itself
        let expected = case.expected_mesh().unwrap().unwrap();
        let ours = world_mesh(&forge_anim::scene_at(again.scene(), 0.0));
        let r = compare_geometric(&expected, &ours, &m.tolerance);
        assert!(r.passed(), "{}: {:?}", case.name, r.failures);
        // our own file imports without anything skipped
        let scene = forge_io::fbx::read(&bytes, 1.0).unwrap();
        assert!(
            scene.warnings().is_empty(),
            "{}: {:?}",
            case.name,
            scene.warnings()
        );
        // a second round changes nothing
        let third = import_with(&export(again.scene(), None, 1.0), 1.0);
        assert_same_scene(again.scene(), third.scene(), &times, 1e-6);
    }
}

#[test]
fn the_skinned_arm_keeps_its_recorded_poses() {
    let path = "../../tests/golden/exchange/skinned-arm";
    let original = import_with(
        &std::fs::read(Path::new(path).join("scene.fbx")).unwrap(),
        1.0,
    );
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(Path::new(path).join("case.toml")).unwrap())
            .unwrap();
    // centimetres and metres: the file declares its unit, the importer converts it back
    for write in [1.0, 0.01] {
        let again = import_with(&export(original.scene(), None, write), 1.0);
        for (t, key) in [(0.5, "f15_bbox"), (1.0, "f30_bbox")] {
            let scene = forge_anim::scene_at(again.scene(), t);
            let mesh = find(&scene, "Arm_Mesh");
            let points = evaluate(&scene, mesh, &EvalCtx::default())
                .unwrap()
                .unwrap()
                .points;
            let lo = points.iter().copied().reduce(DVec3::min).unwrap();
            let hi = points.iter().copied().reduce(DVec3::max).unwrap();
            let want = manifest["params"]["posed_samples"][key].as_array().unwrap();
            for (got, want) in [lo, hi].into_iter().zip(want) {
                let want = want.as_array().unwrap();
                for (axis, w) in want.iter().enumerate() {
                    let w = w.as_float().unwrap();
                    assert!(
                        (got[axis] - w).abs() <= 1e-4,
                        "scale {write}, {key}: {got:?} vs {want:?}"
                    );
                }
            }
        }
        assert_same_scene(original.scene(), again.scene(), &[0.0, 0.5, 1.0], 1e-4);
    }
}

fn set(tx: &mut forge_core::Tx<'_>, id: NodeId, param: &str, v: Value) {
    tx.set_param(id, param, v).unwrap();
}

fn quad_mesh(z: f64) -> MeshData {
    MeshData {
        points: vec![
            [0.0, 0.0, z],
            [10.0, 0.0, z],
            [10.0, 10.0, z],
            [0.0, 10.0, z],
        ],
        polys: vec![vec![0, 1, 2, 3]],
    }
}

/// A parent turned about all three axes with a scale, a child with its own, a polygon with
/// UVs under it, and a null below.
fn hierarchy() -> Document {
    hierarchy_scaled([1.0, 2.0, 3.0])
}

fn hierarchy_scaled(parent_scale: [f64; 3]) -> Document {
    let mut doc = Document::new(registry());
    doc.transact("hierarchy", |tx| {
        let parent = tx.create("forge.null", None, None)?;
        tx.rename(parent, "Parent")?;
        set(
            tx,
            parent,
            "transform.pos",
            Value::Vec3([10.0, -20.0, 30.0]),
        );
        set(
            tx,
            parent,
            "transform.rot",
            Value::Vec3([
                30f64.to_radians(),
                20f64.to_radians(),
                (-40f64).to_radians(),
            ]),
        );
        set(tx, parent, "transform.scale", Value::Vec3(parent_scale));
        let quad = tx.create("forge.polygon", Some(parent), None)?;
        tx.rename(quad, "Quad")?;
        set(tx, quad, "mesh", Value::Mesh(Arc::new(quad_mesh(5.0))));
        set(tx, quad, "transform.pos", Value::Vec3([1.0, 2.0, 3.0]));
        set(tx, quad, "transform.rot", Value::Vec3([-1.0, 0.5, 0.25]));
        let uv = tx.attach_tag("forge.uvw", quad, None)?;
        set(
            tx,
            uv,
            "uvw",
            Value::FloatList(vec![0.0, 0.0, 1.0, 0.0, 1.0, 0.5, 0.0, 0.5]),
        );
        let leaf = tx.create("forge.null", Some(quad), None)?;
        tx.rename(leaf, "Leaf")?;
        set(tx, leaf, "transform.pos", Value::Vec3([0.0, 0.0, -7.0]));
        let other = tx.create("forge.cube", None, None)?;
        tx.rename(other, "Cube")?;
        set(tx, other, "transform.pos", Value::Vec3([100.0, 0.0, 0.0]));
        Ok(())
    })
    .unwrap();
    doc
}

#[test]
fn a_hierarchy_with_rotations_about_three_axes_round_trips() {
    let doc = hierarchy();
    let (bytes, count) = export_bytes("fbx", doc.scene(), None, &Options::new()).unwrap();
    assert_eq!(count, 4);
    let again = import_with(&bytes, 1.0);
    assert_same_scene(doc.scene(), again.scene(), &[0.0], 1e-9);
    // the file is in our own writer's layout: ASCII 7.4, Y-up right-handed, centimetres
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.starts_with("; FBX 7.4.0 project file\n"));
    for needle in [
        "FBXVersion: 7400",
        "\"UnitScaleFactor\", \"double\", \"Number\", \"\",1\n",
        "\"UpAxis\", \"int\", \"Integer\", \"\",1\n",
        "Model: ",
        "C: \"OO\",",
    ] {
        assert!(text.contains(needle), "missing {needle:?}");
    }
    // the generator's evaluated mesh was written
    assert!(text.contains("Geometry: "));
    // determinism
    let (again_bytes, _) = export_bytes("fbx", doc.scene(), None, &Options::new()).unwrap();
    assert_eq!(text.as_bytes(), again_bytes);
}

#[test]
fn ufbx_reads_the_file_as_ascii_7_4() {
    let doc = hierarchy();
    let bytes = export(doc.scene(), None, 1.0);
    let root = ufbx::load_memory(&bytes, ufbx::LoadOpts::default()).unwrap();
    let s = &*root;
    assert!(s.metadata.ascii);
    assert_eq!(s.metadata.version, 7400);
    assert_eq!(s.metadata.creator.to_string(), "Forge3D");
    assert_eq!(s.settings.unit_meters, 0.01);
    assert_eq!(s.settings.axes.up, ufbx::CoordinateAxis::PositiveY);
    assert_eq!(s.settings.axes.front, ufbx::CoordinateAxis::PositiveZ);
    assert_eq!(s.metadata.warnings.len(), 0);
    // four objects below the scene's root node
    assert_eq!(s.nodes.len(), 5);
}

/// Keys of every kind on one object: linear and spline positions, Euler rotation (baked per
/// frame), stepped scale.
fn animated() -> Document {
    let mut doc = Document::new(registry());
    doc.transact("animated", |tx| {
        let a = tx.create("forge.null", None, None)?;
        tx.rename(a, "Mover")?;
        let keys = |v: [f64; 3], interp: Interp| {
            Track::new(
                [0.0, 0.5, 1.0]
                    .iter()
                    .zip(v)
                    .map(|(&t, x)| Key::new(t, x).with_interp(interp))
                    .collect(),
            )
        };
        tx.set_track(
            a,
            "transform.pos.x",
            Some(keys([0.0, 50.0, 80.0], Interp::Linear)),
        )?;
        tx.set_track(
            a,
            "transform.pos.y",
            Some(keys([0.0, 10.0, -30.0], Interp::Spline)),
        )?;
        tx.set_track(
            a,
            "transform.pos.z",
            Some(keys([5.0, 5.0, -5.0], Interp::Linear)),
        )?;
        for (axis, vals) in [
            ("x", [0.0, 0.4, 0.9]),
            ("y", [0.0, -0.3, 0.2]),
            ("z", [0.1, 0.1, 1.0]),
        ] {
            tx.set_track(
                a,
                &format!("transform.rot.{axis}"),
                Some(keys(vals, Interp::Linear)),
            )?;
        }
        tx.set_track(
            a,
            "transform.scale.x",
            Some(keys([1.0, 2.0, 3.0], Interp::Step)),
        )?;
        tx.set_track(
            a,
            "transform.scale.y",
            Some(keys([1.0, 2.0, 3.0], Interp::Step)),
        )?;
        tx.set_track(
            a,
            "transform.scale.z",
            Some(keys([1.0, 2.0, 3.0], Interp::Step)),
        )?;
        let child = tx.create("forge.null", Some(a), None)?;
        tx.rename(child, "Child")?;
        set(tx, child, "transform.pos", Value::Vec3([0.0, 10.0, 0.0]));
        Ok(())
    })
    .unwrap();
    doc
}

#[test]
fn animation_is_written_as_curves_and_read_back() {
    let doc = animated();
    let bytes = export(doc.scene(), None, 1.0);
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("AnimationStack: "));
    assert!(text.contains("AnimationCurveNode: "));
    assert!(text.contains("KeyTime: "));
    let again = import_with(&bytes, 1.0);
    // at the document frames the baked curves equal the tracks (the spline position and the
    // Euler rotation are sampled there), the stepped scale holds between its keys
    let frames: Vec<f64> = (0..=30).map(|f| f64::from(f) / 30.0).collect();
    // the step is a one millisecond ramp in the file: leave out the frame of the step itself
    let skip_steps: Vec<f64> = frames
        .iter()
        .copied()
        .filter(|t| (t - 0.5).abs() > 0.01)
        .collect();
    assert_same_scene(doc.scene(), again.scene(), &skip_steps, 1e-4);
    let mover = find(again.scene(), "Mover");
    let local = |t: f64| at(again.scene(), t).local_transform(mover).unwrap();
    assert_eq!(local(0.25).scale, DVec3::ONE);
    assert_eq!(local(0.75).scale, DVec3::splat(2.0));
    assert_eq!(local(1.0).scale, DVec3::splat(3.0));
    // the key range survives
    let track = &again.scene().node(mover).unwrap().tracks["transform.pos.x"];
    assert!((track.keys[0].time).abs() < 1e-9);
    assert!((track.keys.last().unwrap().time - 1.0).abs() < 1e-9);
}

#[test]
fn quaternion_rotation_tracks_are_sampled_as_orientations() {
    let mut doc = Document::new(registry());
    doc.transact("turn", |tx| {
        let a = tx.create("forge.null", None, None)?;
        tx.rename(a, "Turner")?;
        for (axis, vals) in [("x", [0.0, 1.0]), ("y", [0.0, 2.0]), ("z", [0.0, -1.5])] {
            let mut track = Track::new(
                [0.0, 1.0]
                    .iter()
                    .zip(vals)
                    .map(|(&t, v)| Key::new(t, v).with_interp(Interp::Linear))
                    .collect(),
            );
            track.quaternion = true;
            tx.set_track(a, &format!("transform.rot.{axis}"), Some(track))?;
        }
        Ok(())
    })
    .unwrap();
    let again = import_with(&export(doc.scene(), None, 1.0), 1.0);
    let times: Vec<f64> = (0..=30).map(|f| f64::from(f) / 30.0).collect();
    assert_same_scene(doc.scene(), again.scene(), &times, 1e-4);
}

#[test]
fn mixed_step_and_continuous_keys_of_one_parameter_are_an_error() {
    let mut doc = Document::new(registry());
    doc.transact("mixed", |tx| {
        let a = tx.create("forge.null", None, None)?;
        tx.set_track(
            a,
            "transform.pos.x",
            Some(Track::new(vec![
                Key::new(0.0, 0.0).with_interp(Interp::Step),
                Key::new(1.0, 1.0),
            ])),
        )?;
        tx.set_track(
            a,
            "transform.pos.y",
            Some(Track::new(vec![
                Key::new(0.0, 0.0).with_interp(Interp::Linear),
                Key::new(1.0, 1.0),
            ])),
        )?;
        Ok(())
    })
    .unwrap();
    let err = export_bytes("fbx", doc.scene(), None, &Options::new()).unwrap_err();
    assert!(err.to_string().contains("step"), "{err}");
}

#[test]
fn cameras_and_lights_round_trip_with_every_exported_setting() {
    let mut doc = Document::new(registry());
    doc.transact("camera and lights", |tx| {
        let cam = tx.create("forge.camera", None, None)?;
        tx.rename(cam, "Cam")?;
        set(tx, cam, "transform.pos", Value::Vec3([10.0, 200.0, -600.0]));
        set(
            tx,
            cam,
            "transform.rot",
            Value::Vec3([0.3, -15f64.to_radians(), 0.05]),
        );
        set(tx, cam, "sensor_size", Value::Float(24.0));
        set(tx, cam, "fov_horizontal", Value::Float(0.7));
        set(tx, cam, "focus_distance", Value::Float(321.0));
        set(tx, cam, "enable_near_clipping", Value::Bool(true));
        set(tx, cam, "near_clipping", Value::Float(2.5));
        set(tx, cam, "enable_far_clipping", Value::Bool(true));
        set(tx, cam, "far_clipping", Value::Float(5000.0));
        let ortho = tx.create("forge.camera", None, None)?;
        tx.rename(ortho, "Ortho")?;
        set(tx, ortho, "projection", Value::Enum("parallel".into()));
        set(tx, ortho, "zoom", Value::Float(2.0));
        for (name, kind, pos) in [
            ("Omni", "omni", [300.0, 400.0, -300.0]),
            ("Spot", "spot", [0.0, 100.0, 0.0]),
            ("Sun", "infinite", [0.0, 500.0, 0.0]),
            ("Panel", "area", [50.0, 50.0, 50.0]),
        ] {
            let l = tx.create("forge.light", None, None)?;
            tx.rename(l, name)?;
            set(tx, l, "type", Value::Enum(kind.into()));
            set(tx, l, "transform.pos", Value::Vec3(pos));
            set(tx, l, "transform.rot", Value::Vec3([0.4, -0.7, 0.0]));
            set(tx, l, "color", Value::Color([0.2, 0.4, 0.6]));
            set(tx, l, "intensity", Value::Float(0.75));
            if kind == "spot" {
                set(tx, l, "inner_angle", Value::Float(10f64.to_radians()));
                set(tx, l, "outer_angle", Value::Float(40f64.to_radians()));
            }
        }
        Ok(())
    })
    .unwrap();
    let bytes = export(doc.scene(), None, 1.0);
    let again = import_with(&bytes, 1.0);
    assert_same_scene(doc.scene(), again.scene(), &[0.0], 1e-9);
    // a file in metres gives back the centimetres of the scene
    let metres = import_with(&export(doc.scene(), None, 0.01), 1.0);
    assert_same_scene(doc.scene(), metres.scene(), &[0.0], 1e-9);
    let cam = find(metres.scene(), "Cam");
    assert_eq!(float(metres.scene(), cam, "far_clipping"), Some(5000.0));
    assert_eq!(float(metres.scene(), cam, "focus_distance"), Some(321.0));
}

fn standard_material(
    tx: &mut forge_core::Tx<'_>,
    name: &str,
    color: [f64; 3],
    rough: f64,
) -> NodeId {
    use forge_material::node_material::{GRAPH, NODE_MATERIAL};
    let m = tx.create_material(NODE_MATERIAL, None).unwrap();
    tx.rename(m, name).unwrap();
    let s = SurfaceValues {
        base_color: color,
        roughness: rough,
        alpha: 0.75,
        emission: [0.1, 0.0, 0.2],
        ..SurfaceValues::default()
    };
    tx.set_param(m, GRAPH, Value::String(s.to_graph().unwrap().to_json()))
        .unwrap();
    m
}

#[test]
fn materials_and_their_polygon_assignment_round_trip() {
    let mut doc = Document::new(registry());
    doc.transact("materials", |tx| {
        let red = standard_material(tx, "Red", [1.0, 0.0, 0.0], 1.0);
        let green = standard_material(tx, "Green", [0.0, 1.0, 0.0], 0.25);
        // three quads: Red on the first, Green on the second, nothing on the third
        let strip = tx.create("forge.polygon", None, None)?;
        tx.rename(strip, "Strip")?;
        let mesh = MeshData {
            points: (0..8)
                .map(|i| [f64::from(i / 2) * 10.0, f64::from(i % 2) * 10.0, 0.0])
                .collect(),
            polys: vec![vec![0, 2, 3, 1], vec![2, 4, 5, 3], vec![4, 6, 7, 5]],
        };
        set(tx, strip, "mesh", Value::Mesh(Arc::new(mesh)));
        for (name, material, polys) in [("Red", red, vec![0]), ("Green", green, vec![1])] {
            let sel = tx.attach_tag("forge.polygon_selection", strip, None)?;
            tx.rename(sel, name)?;
            set(tx, sel, "indices", Value::IndexList(polys));
            let tag = tx.attach_tag("forge.tag.texture", strip, None)?;
            set(tx, tag, "material", Value::Link(Some(material)));
            set(tx, tag, "selection", Value::String(name.into()));
        }
        // a second object with a single material and no selection
        let single = tx.create("forge.polygon", None, None)?;
        tx.rename(single, "Single")?;
        set(tx, single, "mesh", Value::Mesh(Arc::new(quad_mesh(0.0))));
        let tag = tx.attach_tag("forge.tag.texture", single, None)?;
        set(tx, tag, "material", Value::Link(Some(green)));
        Ok(())
    })
    .unwrap();
    let bytes = export(doc.scene(), None, 1.0);
    let again = import_with(&bytes, 1.0);
    let scene = again.scene();
    let (textures, selections) = assignments(scene, find(scene, "Strip"));
    // the uncovered third polygon got the default surface
    assert_eq!(
        textures,
        [
            ("Red".to_string(), "Red".to_string()),
            ("Green".to_string(), "Green".to_string()),
            ("Default".to_string(), "Default".to_string()),
        ]
    );
    assert_eq!(
        selections,
        [
            ("Red".to_string(), vec![0]),
            ("Green".to_string(), vec![1]),
            ("Default".to_string(), vec![2]),
        ]
    );
    let (textures, selections) = assignments(scene, find(scene, "Single"));
    assert_eq!(textures, [("Green".to_string(), String::new())]);
    assert!(selections.is_empty());
    // values: lambert for roughness 1, phong with the matching exponent otherwise
    let by_name = |n: &str| surfaces(scene).into_iter().find(|s| s.0 == n).unwrap();
    let red = by_name("Red");
    assert_eq!(
        (red.1, red.2, red.3, red.4),
        ([1.0, 0.0, 0.0], 1.0, 0.75, [0.1, 0.0, 0.2])
    );
    let green = by_name("Green");
    assert!((green.2 - 0.25).abs() < 1e-12, "{green:?}");
    assert_eq!(by_name("Default").1, SurfaceValues::default().base_color);
    // shared materials are written once
    assert_eq!(
        String::from_utf8(bytes)
            .unwrap()
            .matches("\n\tMaterial: ")
            .count(),
        3
    );
}

/// A skinned tube (polygon object, three joints, weights by height, J2 keyed).
fn skinned() -> Document {
    let mut doc = Document::new(registry());
    doc.transact("rig", |tx| {
        let tube = tx.create("forge.polygon", None, None)?;
        tx.rename(tube, "Tube")?;
        let ring =
            |y: f64| [(10.0, 0.0), (0.0, 10.0), (-10.0, 0.0), (0.0, -10.0)].map(|(x, z)| [x, y, z]);
        let mut points = Vec::new();
        for y in [-100.0, 0.0, 100.0] {
            points.extend(ring(y));
        }
        let mut polys = Vec::new();
        for r in 0..2u32 {
            for k in 0..4u32 {
                polys.push(vec![
                    r * 4 + k,
                    r * 4 + (k + 1) % 4,
                    (r + 1) * 4 + (k + 1) % 4,
                    (r + 1) * 4 + k,
                ]);
            }
        }
        set(
            tx,
            tube,
            "mesh",
            Value::Mesh(Arc::new(MeshData { points, polys })),
        );
        let joints = forge_rig::tools::create_chain(
            tx,
            None,
            &[
                DVec3::new(0.0, -100.0, 0.0),
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(0.0, 100.0, 0.0),
            ],
            "J",
            None,
        )?;
        let tag = tx.attach_tag(forge_rig::WEIGHT_TAG, tube, None)?;
        tx.set_param(tag, "joints", Value::LinkList(joints.clone()))?;
        let mut rows = Vec::new();
        for ring in 0..3 {
            for _ in 0..4 {
                rows.push(match ring {
                    0 => vec![1.0, 0.0, 0.0],
                    1 => vec![0.5, 0.5, 0.0],
                    _ => vec![0.0, 0.25, 0.75],
                });
            }
        }
        tx.set_param(
            tag,
            "weights",
            forge_rig::WeightMap::from_dense(&rows).to_value(),
        )?;
        forge_rig::weights::bind(tx, tag)?;
        tx.create(forge_rig::SKIN, Some(tube), None)?;
        for (t, v) in [(0.0, 0.0), (1.0, 1.2)] {
            tx.set_key(
                joints[1],
                "transform.rot.z",
                Key::new(t, v).with_interp(Interp::Linear),
            )?;
        }
        Ok(())
    })
    .unwrap();
    doc
}

#[test]
fn a_skin_round_trips_in_centimetres_and_metres() {
    let doc = skinned();
    let posed = |scene: &Scene, t: f64| {
        let at = forge_anim::scene_at(scene, t);
        let tube = find(&at, "Tube");
        evaluate(&at, tube, &EvalCtx::default())
            .unwrap()
            .unwrap()
            .points
    };
    for write in [1.0, 0.01, 0.5] {
        let again = import_with(&export(doc.scene(), None, write), 1.0);
        let scene = again.scene();
        assert_eq!(
            scene.node(find(scene, "J2")).unwrap().type_id,
            "forge.joint"
        );
        for t in [0.0, 0.5, 1.0] {
            let (a, b) = (posed(doc.scene(), t), posed(scene, t));
            assert_eq!(a.len(), b.len());
            for (p, q) in a.iter().zip(&b) {
                assert!(
                    p.distance(*q) < 1e-4,
                    "scale {write}, t {t}: {p:?} vs {q:?}"
                );
            }
        }
        // the posed mesh really moved
        assert!(posed(scene, 1.0)[4].distance(posed(scene, 0.0)[4]) > 1.0);
        assert_same_scene(doc.scene(), scene, &[0.0, 0.5, 1.0], 1e-4);
    }
}

#[test]
fn selected_roots_and_detached_objects() {
    // a uniform parent scale: a detached child's world matrix is then a plain TRS
    let doc = hierarchy_scaled([2.0, 2.0, 2.0]);
    let scene = doc.scene();
    let parent = find(scene, "Parent");
    let quad = find(scene, "Quad");
    // a selected parent brings its descendants
    let (bytes, count) = export_bytes("fbx", scene, Some(&[parent]), &Options::new()).unwrap();
    assert_eq!(count, 3);
    let again = import_with(&bytes, 1.0);
    assert_eq!(again.scene().roots().len(), 1);
    // a selected child without its parent keeps its world transform
    let (bytes, count) = export_bytes("fbx", scene, Some(&[quad]), &Options::new()).unwrap();
    assert_eq!(count, 2);
    let alone = import_with(&bytes, 1.0);
    assert_eq!(alone.scene().roots().len(), 1);
    matrices_close(
        alone
            .scene()
            .world_matrix(find(alone.scene(), "Quad"))
            .unwrap(),
        scene.world_matrix(quad).unwrap(),
        1e-9,
        "detached Quad",
    );
    // an animated object whose parent is not exported cannot be written
    let mut moving = Document::new(registry());
    let child = moving
        .transact("animated child", |tx| {
            let parent = tx.create("forge.null", None, None)?;
            let child = tx.create("forge.null", Some(parent), None)?;
            tx.set_track(
                child,
                "transform.pos.x",
                Some(Track::new(vec![Key::new(0.0, 0.0), Key::new(1.0, 5.0)])),
            )?;
            Ok(child)
        })
        .unwrap();
    let err = export_bytes("fbx", moving.scene(), Some(&[child]), &Options::new()).unwrap_err();
    assert!(err.to_string().contains("animated"), "{err}");
    assert!(export_bytes("fbx", moving.scene(), None, &Options::new()).is_ok());
}

#[test]
fn nothing_to_export_and_bad_options() {
    let doc = Document::new(registry());
    assert_eq!(
        export_bytes("fbx", doc.scene(), None, &Options::new()).unwrap_err(),
        IoError::NothingToExport
    );
    let doc = hierarchy();
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            export_bytes("fbx", doc.scene(), None, &options(scale)).is_err(),
            "{scale}"
        );
    }
}

#[test]
fn the_format_is_registered_for_export_too() {
    let info = format_by_id("fbx").unwrap();
    assert!(info.import && info.export);
    let descs = export_options("fbx").unwrap();
    assert_eq!(descs.len(), 1);
    assert_eq!(descs[0].id, "scale");
    let doc = hierarchy();
    let (text, count) = export_text("fbx", doc.scene(), None, &Options::new()).unwrap();
    assert!(text.starts_with("; FBX 7.4.0 project file"));
    assert_eq!(count, 4);
}
