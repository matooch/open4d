//! O44 FBX import (brief 165, IO.md §8): the six exchange scenes against their OBJ and the
//! values recorded in `case.toml`, plus the unit and axis rules on hand-written ASCII files.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, Registry, Scene, Value};
use forge_generators::{Primitives, evaluate};
use forge_io::formats::{Options, read_scene};
use forge_io::{export_scene, fbx};
use forge_material::node_material::{SurfaceValues, load_graph};
use forge_modifiers::Deformers;
use forge_parity_harness::{GoldenCase, Mesh, compare_geometric, discover_cases};
use forge_rig::Rig;
use glam::DVec3;

/// Cases whose FBX options are not imported (none: asserted exactly).
const UNSUPPORTED: &[&str] = &[];
/// Cases that import but differ from the reference (none: asserted exactly).
const KNOWN_MISMATCH: &[&str] = &[];

fn registry() -> Arc<Registry> {
    let mut registry = Registry::with_builtins();
    registry.install(&Primitives).unwrap();
    registry.install(&Deformers).unwrap();
    registry.install(&Rig).unwrap();
    registry.install(&forge_material::MaterialPlugin).unwrap();
    Arc::new(registry)
}

fn import_with(bytes: &[u8], options: &Options) -> Document {
    let mut doc = Document::new(registry());
    let scene = read_scene("fbx", bytes, options).unwrap();
    doc.transact("Import FBX", |tx| scene.apply(tx)).unwrap();
    doc
}

fn import(bytes: &[u8]) -> Document {
    import_with(bytes, &Options::new())
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

fn find(scene: &Scene, name: &str) -> NodeId {
    scene
        .roots()
        .iter()
        .flat_map(|&r| std::iter::once(r).chain(scene.descendants(r)))
        .find(|&n| scene.node(n).unwrap().name == name)
        .unwrap_or_else(|| panic!("no object named {name}"))
}

fn cases() -> Vec<GoldenCase> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let cases: Vec<_> = discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "exchange")
        .collect();
    assert_eq!(cases.len(), 6);
    cases
}

fn fixture(case: &GoldenCase) -> Vec<u8> {
    std::fs::read(case.dir.join("scene.fbx")).unwrap()
}

fn float(v: &toml::Value) -> f64 {
    v.as_float()
        .or(v.as_integer().map(|i| i as f64))
        .expect("number")
}

fn triple(v: &toml::Value) -> [f64; 3] {
    let a = v.as_array().unwrap();
    [float(&a[0]), float(&a[1]), float(&a[2])]
}

/// Local position, `(h, p, b)` in degrees and scale of an object at `time`.
fn pose(scene: &Scene, id: NodeId, time: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let at = forge_anim::scene_at(scene, time);
    let t = at.local_transform(id).unwrap();
    (
        t.pos.to_array(),
        t.rot.to_array().map(f64::to_degrees),
        t.scale.to_array(),
    )
}

fn close(a: [f64; 3], b: [f64; 3], tolerance: f64) -> bool {
    a.iter().zip(&b).all(|(a, b)| (a - b).abs() <= tolerance)
}

#[test]
fn o44_fbx_geometry_matches_the_obj() {
    let (mut unsupported, mut mismatched) = (BTreeSet::new(), BTreeSet::new());
    for case in cases() {
        let m = case.manifest().unwrap();
        let doc = import(&fixture(&case));
        let start = forge_anim::scene_at(doc.scene(), 0.0);
        let expected = case.expected_mesh().unwrap().unwrap();
        let ours = world_mesh(&start);
        let r = compare_geometric(&expected, &ours, &m.tolerance);
        let key = case.name.clone();
        if UNSUPPORTED.contains(&key.as_str()) {
            unsupported.insert(key);
        } else if KNOWN_MISMATCH.contains(&key.as_str()) {
            assert!(!r.passed(), "{key} matches now: remove it from the list");
            mismatched.insert(key);
        } else {
            assert!(r.passed(), "{key}: {:?}", r.failures);
        }
    }
    let set = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
    assert_eq!(unsupported, set(UNSUPPORTED));
    assert_eq!(mismatched, set(KNOWN_MISMATCH));
}

#[test]
fn o44_fbx_transforms_match_the_case_files() {
    let tolerance = 1e-4;
    // case file object key -> object name in the FBX
    let objects: &[(&str, &[(&str, &str)])] = &[
        ("static-mesh", &[("cube", "Cube"), ("sphere", "Sphere")]),
        ("hierarchy", &[("root", "Root"), ("cube", "Cube")]),
        ("anim-transform", &[("cube", "Animated_Cube")]),
        (
            "camera-light",
            &[("camera", "Camera"), ("omni", "Omni"), ("spot", "Spot")],
        ),
    ];
    for case in cases() {
        let Some((_, names)) = objects.iter().find(|(c, _)| *c == case.name) else {
            continue;
        };
        let m = case.manifest().unwrap();
        let doc = import(&fixture(&case));
        for (key, name) in *names {
            let t = &m.params[*key];
            let (pos, rot, scale) = pose(doc.scene(), find(doc.scene(), name), 0.0);
            assert!(
                close(pos, triple(&t["position"]), tolerance),
                "{}/{key}: position {pos:?}",
                case.name
            );
            assert!(
                close(rot, triple(&t["rotation_degrees"]), 1e-6),
                "{}/{key}: rotation {rot:?}",
                case.name
            );
            assert!(
                close(scale, triple(&t["scale"]), tolerance),
                "{}/{key}: scale {scale:?}",
                case.name
            );
        }
        if case.name == "hierarchy" {
            let scene = doc.scene();
            let (root, cube) = (find(scene, "Root"), find(scene, "Cube"));
            assert_eq!(scene.node(cube).unwrap().parent, Some(root));
            assert_eq!(scene.node(root).unwrap().type_id, "forge.null");
            // the child's world position follows the parent's H 45 turn about Y
            let w = scene.world_matrix(cube).unwrap().w_axis.truncate();
            let (s, c) = 45f64.to_radians().sin_cos();
            assert!((w - DVec3::new(100.0 + 0.0 * c, 100.0, 0.0 * s)).length() < tolerance);
        }
    }
}

#[test]
fn o44_fbx_animation_matches_the_recorded_keys() {
    let case = cases()
        .into_iter()
        .find(|c| c.name == "anim-transform")
        .unwrap();
    let m = case.manifest().unwrap();
    let fps = float(&m.params["fps"]);
    let doc = import(&fixture(&case));
    let cube = find(doc.scene(), "Animated_Cube");
    let keys = m.params["keyframes"].as_array().unwrap();
    assert_eq!(keys.len(), 2);
    for k in keys {
        let frame = float(&k["frame"]);
        let (pos, rot, scale) = pose(doc.scene(), cube, frame / fps);
        assert!(close(pos, triple(&k["position"]), 1e-4), "{frame}: {pos:?}");
        assert!(
            close(rot, triple(&k["rotation_degrees"]), 1e-4),
            "{frame}: {rot:?}"
        );
        assert!(
            close(scale, triple(&k["scale"]), 1e-4),
            "{frame}: {scale:?}"
        );
    }
    // the file bakes the move on every frame; it is linear in between
    let track = &doc.scene().node(cube).unwrap().tracks["transform.pos.x"];
    assert_eq!(track.keys.len(), 31);
    let (mid, _, _) = pose(doc.scene(), cube, 0.5 / fps);
    assert!(mid[0] > 0.0 && mid[0] < 1.0, "{mid:?}");
}

#[test]
fn o44_fbx_cameras_and_lights() {
    let case = cases()
        .into_iter()
        .find(|c| c.name == "camera-light")
        .unwrap();
    let m = case.manifest().unwrap();
    let doc = import(&fixture(&case));
    let scene = doc.scene();
    let get = |id: NodeId, p: &str| match scene.param(id, p) {
        Some(Value::Float(f)) => *f,
        other => panic!("{p}: {other:?}"),
    };
    let camera = find(scene, "Camera");
    assert_eq!(scene.node(camera).unwrap().type_id, "forge.camera");
    let want = &m.params["camera"]["parameters"];
    assert!((get(camera, "sensor_size") - float(&want["sensor_size_film_gate"])).abs() < 1e-9);
    let fov = float(&want["field_of_view_horizontal_degrees"]);
    assert!((get(camera, "fov_horizontal").to_degrees() - fov).abs() < 1e-6);
    // the exporter also writes its viewport camera: every camera is imported
    let cameras = scene
        .roots()
        .iter()
        .filter(|&&r| scene.node(r).unwrap().type_id == "forge.camera")
        .count();
    assert_eq!(cameras, 2);
    for (key, name, kind) in [("omni", "Omni", "omni"), ("spot", "Spot", "spot")] {
        let id = find(scene, name);
        assert_eq!(scene.node(id).unwrap().type_id, "forge.light");
        assert_eq!(scene.param(id, "type"), Some(&Value::Enum(kind.into())));
        let want = &m.params[key]["parameters"];
        assert!((get(id, "intensity") - float(&want["intensity_percent"]) / 100.0).abs() < 1e-9);
        if kind == "spot" {
            let outer = float(&want["outer_angle_degrees"]);
            assert!((get(id, "outer_angle").to_degrees() - outer).abs() < 1e-6);
            assert!(get(id, "inner_angle").abs() < 1e-9);
        }
    }
}

#[test]
fn o44_fbx_materials_are_assigned_to_their_cubes() {
    let case = cases().into_iter().find(|c| c.name == "materials").unwrap();
    let m = case.manifest().unwrap();
    let doc = import(&fixture(&case));
    let scene = doc.scene();
    for entry in m.params["materials"].as_array().unwrap() {
        let want = &entry["material"];
        let x = float(&entry["object"]["position"].as_array().unwrap()[0]);
        let cube = scene
            .roots()
            .iter()
            .copied()
            .find(|&r| scene.local_transform(r).unwrap().pos.x == x)
            .unwrap();
        let tag = scene
            .node(cube)
            .unwrap()
            .tags
            .iter()
            .copied()
            .find_map(|t| match scene.param(t, "material") {
                Some(Value::Link(Some(id))) => Some(*id),
                _ => None,
            });
        let node = scene.node(tag.expect("texture tag")).unwrap();
        assert_eq!(node.name, want["name"].as_str().unwrap());
        let surface = SurfaceValues::from_graph(&load_graph(node).unwrap()).unwrap();
        assert_eq!(surface.base_color, triple(&want["color"]), "{}", node.name);
    }
}

#[test]
fn o44_fbx_skin_follows_the_recorded_pose() {
    let case = cases()
        .into_iter()
        .find(|c| c.name == "skinned-arm")
        .unwrap();
    let m = case.manifest().unwrap();
    let doc = import(&fixture(&case));
    let scene = doc.scene();
    let mesh = find(scene, "Arm_Mesh");
    let joints = ["J1", "J2", "J3"].map(|n| find(scene, n));
    for j in joints {
        assert_eq!(scene.node(j).unwrap().type_id, "forge.joint");
    }
    // weights of the recorded points, joints in the case's order
    let tag = forge_rig::weights::find_weight_tag(scene, mesh).expect("weight tag");
    assert_eq!(forge_rig::weights::tag_joints(scene, tag), joints);
    let weights = forge_rig::weights::read_weights(scene, tag);
    for (point, want) in m.params["weights"]["samples"].as_table().unwrap() {
        let row = weights.get(point.parse().unwrap());
        let got: Vec<f64> = (0..3)
            .map(|s| row.iter().find(|(j, _)| *j == s).map_or(0.0, |(_, w)| *w))
            .collect();
        assert!(
            close([got[0], got[1], got[2]], triple(want), 2e-4),
            "point {point}: {got:?}"
        );
    }
    // J2 keyed B 0 -> 90 degrees over frames 0..30
    for (frame, key) in [
        (15.0, "f15_j2_rotation_degrees"),
        (30.0, "f30_j2_rotation_degrees"),
    ] {
        let (_, rot, _) = pose(scene, joints[1], frame / 30.0);
        assert!(
            close(rot, triple(&m.params["posed_samples"][key]), 1e-3),
            "{key}: {rot:?}"
        );
    }
    // posed bounds of frames 15 and 30
    for (frame, key) in [(15.0, "f15_bbox"), (30.0, "f30_bbox")] {
        let posed = forge_anim::scene_at(scene, frame / 30.0);
        let mesh_points = evaluate(&posed, mesh, &EvalCtx::default())
            .unwrap()
            .unwrap()
            .points;
        let lo = mesh_points.iter().copied().reduce(DVec3::min).unwrap();
        let hi = mesh_points.iter().copied().reduce(DVec3::max).unwrap();
        let want = m.params["posed_samples"][key].as_array().unwrap();
        for (got, want) in [lo, hi].into_iter().zip(want) {
            assert!(
                close(got.to_array(), triple(want), 1e-4),
                "{key}: {got:?} vs {want:?}"
            );
        }
    }
}

/// A one-quad scene in an ASCII FBX file with the given axes, unit and numbers.
fn quad_file(
    axes: [i32; 6],
    unit_scale_factor: f64,
    points: [[f64; 3]; 4],
    at: [f64; 3],
) -> String {
    let [up, up_sign, front, front_sign, coord, coord_sign] = axes;
    let flat: Vec<String> = points.iter().flatten().map(|v| v.to_string()).collect();
    format!(
        r#"; FBX 7.4.0 project file
FBXHeaderExtension:  {{
	FBXHeaderVersion: 1003
	FBXVersion: 7400
}}
GlobalSettings:  {{
	Version: 1000
	Properties70:  {{
		P: "UpAxis", "int", "Integer", "",{up}
		P: "UpAxisSign", "int", "Integer", "",{up_sign}
		P: "FrontAxis", "int", "Integer", "",{front}
		P: "FrontAxisSign", "int", "Integer", "",{front_sign}
		P: "CoordAxis", "int", "Integer", "",{coord}
		P: "CoordAxisSign", "int", "Integer", "",{coord_sign}
		P: "UnitScaleFactor", "double", "Number", "",{unit_scale_factor}
	}}
}}
Objects:  {{
	Geometry: 1000, "Geometry::Quad", "Mesh" {{
		Vertices: *12 {{
			a: {}
		}}
		PolygonVertexIndex: *4 {{
			a: 0,1,2,-4
		}}
	}}
	Model: 2000, "Model::Quad", "Mesh" {{
		Version: 232
		Properties70:  {{
			P: "Lcl Translation", "Lcl Translation", "", "A",{},{},{}
		}}
	}}
}}
Connections:  {{
	C: "OO",2000,0
	C: "OO",1000,2000
}}
"#,
        flat.join(","),
        at[0],
        at[1],
        at[2]
    )
}

fn world_points(doc: &Document) -> (Vec<[f64; 3]>, Vec<Vec<u32>>) {
    let m = world_mesh(doc.scene());
    (m.positions, m.faces)
}

/// IO.md §8.2: the same quad written Z-up in metres and Y-up in centimetres imports to the
/// same objects, at `(x, z, y)` of the Z-up numbers times 100, with the corner order reversed.
#[test]
fn axes_and_units_are_converted() {
    // Z-up right-handed (front -Y), 1 unit = 1 m
    let z_up = quad_file(
        [2, 1, 1, -1, 0, 1],
        100.0,
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        [1.0, 2.0, 3.0],
    );
    // the same scene Y-up right-handed (front +Z), 1 unit = 1 cm: (x, y, z) -> (x, z, -y)
    let y_up = quad_file(
        [1, 1, 2, 1, 0, 1],
        1.0,
        [
            [0.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [100.0, 100.0, 0.0],
            [0.0, 100.0, 0.0],
        ],
        [100.0, 300.0, -200.0],
    );
    let (a, faces_a) = world_points(&import(z_up.as_bytes()));
    let (b, faces_b) = world_points(&import(y_up.as_bytes()));
    let want = [
        [100.0, 300.0, 200.0],
        [200.0, 300.0, 200.0],
        [200.0, 400.0, 200.0],
        [100.0, 400.0, 200.0],
    ];
    for (name, got) in [("z-up metres", &a), ("y-up centimetres", &b)] {
        assert_eq!(got.len(), 4, "{name}");
        for (g, w) in got.iter().zip(&want) {
            assert!(close(*g, *w, 1e-9), "{name}: {g:?} vs {w:?}");
        }
    }
    assert_eq!(faces_a, vec![vec![0, 3, 2, 1]]);
    assert_eq!(faces_a, faces_b);
    // the scale option multiplies on top of the unit
    let doubled = import_with(
        z_up.as_bytes(),
        &Options::from([("scale".into(), Value::Float(2.0))]),
    );
    let (c, _) = world_points(&doubled);
    for (g, w) in c.iter().zip(&want) {
        assert!(close(*g, w.map(|v| v * 2.0), 1e-9), "{g:?}");
    }
}

fn mesh_file(normal: [f64; 3]) -> String {
    let n = (0..8).map(|_| format!("{},{},{}", normal[0], normal[1], normal[2]));
    format!(
        r#"; FBX 7.4.0 project file
FBXHeaderExtension:  {{
	FBXHeaderVersion: 1003
	FBXVersion: 7400
}}
Objects:  {{
	Geometry: 1000, "Geometry::Two", "Mesh" {{
		Vertices: *24 {{
			a: 0,0,0,1,0,0,1,1,0,0,1,0,2,0,0,3,0,0,3,1,0,2,1,0
		}}
		PolygonVertexIndex: *8 {{
			a: 0,1,2,-4,4,5,6,-8
		}}
		LayerElementNormal: 0 {{
			Version: 101
			Name: ""
			MappingInformationType: "ByPolygonVertex"
			ReferenceInformationType: "Direct"
			Normals: *24 {{
				a: {}
			}}
		}}
		LayerElementUV: 0 {{
			Version: 101
			Name: "UVMap"
			MappingInformationType: "ByPolygonVertex"
			ReferenceInformationType: "Direct"
			UV: *16 {{
				a: 0,0,0.25,0,0.25,0.5,0,0.5,0.5,0,1,0,1,1,0.5,1
			}}
		}}
		LayerElementMaterial: 0 {{
			Version: 101
			Name: ""
			MappingInformationType: "ByPolygon"
			ReferenceInformationType: "IndexToDirect"
			Materials: *2 {{
				a: 1,0
			}}
		}}
		Layer: 0 {{
			Version: 100
			LayerElement:  {{
				Type: "LayerElementNormal"
				TypedIndex: 0
			}}
			LayerElement:  {{
				Type: "LayerElementMaterial"
				TypedIndex: 0
			}}
			LayerElement:  {{
				Type: "LayerElementUV"
				TypedIndex: 0
			}}
		}}
	}}
	Model: 2000, "Model::Two", "Mesh" {{
		Version: 232
	}}
	Material: 3000, "Material::Red", "" {{
		Version: 102
		ShadingModel: "lambert"
		Properties70:  {{
			P: "DiffuseColor", "Color", "", "A",1,0,0
		}}
	}}
	Material: 3001, "Material::Green", "" {{
		Version: 102
		ShadingModel: "phong"
		Properties70:  {{
			P: "DiffuseColor", "Color", "", "A",0,1,0
			P: "Shininess", "double", "Number", "",30
		}}
	}}
}}
Connections:  {{
	C: "OO",2000,0
	C: "OO",1000,2000
	C: "OO",3000,2000
	C: "OO",3001,2000
}}
"#,
        n.collect::<Vec<_>>().join(",")
    )
}

fn tags_of(scene: &Scene, id: NodeId, type_id: &str) -> Vec<NodeId> {
    scene
        .node(id)
        .unwrap()
        .tags
        .iter()
        .copied()
        .filter(|&t| scene.node(t).unwrap().type_id == type_id)
        .collect()
}

/// IO.md §8.3: corner UVs, polygons grouped by material, faceted meshes without smoothing.
#[test]
fn uvs_materials_and_shading_of_a_polygon_mesh() {
    let doc = import(mesh_file([0.0, 0.0, 1.0]).as_bytes());
    let scene = doc.scene();
    let mesh = find(scene, "Two");
    // flat normals: the mesh is faceted, so no smoothing tag
    assert!(tags_of(scene, mesh, "forge.smoothing").is_empty());
    let uvw = tags_of(scene, mesh, "forge.uvw");
    assert_eq!(uvw.len(), 1);
    let Some(Value::FloatList(uv)) = scene.param(uvw[0], "uvw") else {
        panic!("uvw");
    };
    // corners keep the first and reverse the rest: [0, 3, 2, 1]
    assert_eq!(
        &uv[..8],
        &[0.0, 0.0, 0.0, 0.5, 0.25, 0.5, 0.25, 0.0],
        "{uv:?}"
    );
    assert_eq!(&uv[8..], &[0.5, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.0]);
    // two materials: one selection per material, polygon 1 uses the first material (Red)
    let selections = tags_of(scene, mesh, "forge.polygon_selection");
    assert_eq!(selections.len(), 2);
    let named = |name: &str| {
        let sel = selections
            .iter()
            .copied()
            .find(|&s| scene.node(s).unwrap().name == name)
            .unwrap();
        let Some(Value::IndexList(l)) = scene.param(sel, "indices") else {
            panic!()
        };
        l.clone()
    };
    assert_eq!(named("Red"), [1]);
    assert_eq!(named("Green"), [0]);
    let textures = tags_of(scene, mesh, "forge.tag.texture");
    assert_eq!(textures.len(), 2);
    for t in textures {
        let Some(Value::Link(Some(m))) = scene.param(t, "material") else {
            panic!()
        };
        let node = scene.node(*m).unwrap();
        assert_eq!(
            scene.param(t, "selection"),
            Some(&Value::String(node.name.clone()))
        );
        let surface = SurfaceValues::from_graph(&load_graph(node).unwrap()).unwrap();
        match node.name.as_str() {
            "Red" => {
                assert_eq!(surface.base_color, [1.0, 0.0, 0.0]);
                assert_eq!(surface.roughness, 1.0);
            }
            "Green" => {
                assert_eq!(surface.base_color, [0.0, 1.0, 0.0]);
                assert!((surface.roughness - (2.0f64 / 32.0).sqrt()).abs() < 1e-9);
            }
            other => panic!("{other}"),
        }
    }
    // normals that differ from the polygon normals: shaded smoothly
    let smooth = import(mesh_file([0.0, 0.6, 0.8]).as_bytes());
    let mesh = find(smooth.scene(), "Two");
    assert_eq!(tags_of(smooth.scene(), mesh, "forge.smoothing").len(), 1);
}

#[test]
fn bad_input_is_an_error() {
    assert!(fbx::read(b"", 1.0).is_err());
    assert!(fbx::read(b"this is not a scene", 1.0).is_err());
    let case = &cases()[0];
    let bytes = fixture(case);
    assert!(fbx::read(&bytes[..bytes.len() / 2], 1.0).is_err());
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(fbx::read(&bytes, scale).is_err(), "{scale}");
    }
    assert!(read_scene("fbx", b"", &Options::new()).is_err());
}

#[test]
fn skipped_content_is_reported() {
    let two_takes = r#"; FBX 7.4.0 project file
FBXHeaderExtension:  {
	FBXHeaderVersion: 1003
	FBXVersion: 7400
}
Objects:  {
	AnimationStack: 4000, "AnimStack::Take 001", "" {
	}
	AnimationStack: 4001, "AnimStack::Take 002", "" {
	}
}
Connections:  {
}
"#;
    let scene = fbx::read(two_takes.as_bytes(), 1.0).unwrap();
    assert_eq!(scene.warnings(), ["1 extra animation stack(s) skipped"]);
    let plain = fbx::read(&fixture(&cases()[0]), 1.0).unwrap();
    assert!(plain.warnings().is_empty(), "{:?}", plain.warnings());
}

#[test]
fn importing_is_one_undo_step_and_deterministic() {
    let case = &cases()[0];
    let bytes = fixture(case);
    let mut doc = Document::new(registry());
    let scene = read_scene("fbx", &bytes, &Options::new()).unwrap();
    doc.transact("Import FBX", |tx| scene.apply(tx)).unwrap();
    let again = import(&bytes);
    assert_eq!(world_mesh(doc.scene()), world_mesh(again.scene()));
    assert!(doc.undo());
    assert!(doc.scene().roots().is_empty());
}

/// A Z-up scene (metres) whose null moves 1 m along file Y and turns 90 degrees about the
/// file's up axis in one second, with linear keys.
const ANIMATED_Z_UP: &str = r#"; FBX 7.4.0 project file
FBXHeaderExtension:  {
	FBXHeaderVersion: 1003
	FBXVersion: 7400
}
GlobalSettings:  {
	Version: 1000
	Properties70:  {
		P: "UpAxis", "int", "Integer", "",2
		P: "UpAxisSign", "int", "Integer", "",1
		P: "FrontAxis", "int", "Integer", "",1
		P: "FrontAxisSign", "int", "Integer", "",-1
		P: "CoordAxis", "int", "Integer", "",0
		P: "CoordAxisSign", "int", "Integer", "",1
		P: "UnitScaleFactor", "double", "Number", "",100
	}
}
Objects:  {
	Model: 2000, "Model::Mover", "Null" {
		Version: 232
	}
	AnimationStack: 4000, "AnimStack::Take 001", "" {
	}
	AnimationLayer: 4001, "AnimLayer::BaseLayer", "" {
	}
	AnimationCurveNode: 4002, "AnimCurveNode::T", "" {
		Properties70:  {
			P: "d|X", "Number", "", "A",0
			P: "d|Y", "Number", "", "A",0
			P: "d|Z", "Number", "", "A",0
		}
	}
	AnimationCurveNode: 4003, "AnimCurveNode::R", "" {
		Properties70:  {
			P: "d|X", "Number", "", "A",0
			P: "d|Y", "Number", "", "A",0
			P: "d|Z", "Number", "", "A",0
		}
	}
	AnimationCurve: 4004, "AnimCurve::", "" {
		Default: 0
		KeyVer: 4009
		KeyTime: *2 {
			a: 0,46186158000
		}
		KeyValueFloat: *2 {
			a: 0,1
		}
		KeyAttrFlags: *1 {
			a: 4
		}
		KeyAttrDataFloat: *4 {
			a: 0,0,0,0
		}
		KeyAttrRefCount: *1 {
			a: 2
		}
	}
	AnimationCurve: 4005, "AnimCurve::", "" {
		Default: 0
		KeyVer: 4009
		KeyTime: *2 {
			a: 0,46186158000
		}
		KeyValueFloat: *2 {
			a: 0,90
		}
		KeyAttrFlags: *1 {
			a: 4
		}
		KeyAttrDataFloat: *4 {
			a: 0,0,0,0
		}
		KeyAttrRefCount: *1 {
			a: 2
		}
	}
}
Connections:  {
	C: "OO",2000,0
	C: "OO",4001,4000
	C: "OO",4002,4001
	C: "OO",4003,4001
	C: "OP",4002,2000,"Lcl Translation"
	C: "OP",4003,2000,"Lcl Rotation"
	C: "OP",4004,4002,"d|Y"
	C: "OP",4005,4003,"d|Z"
}
"#;

/// IO.md §8.3 and §8.2 together: keys of a Z-up file in metres are converted like the static
/// transforms (file Y -> our Z, a turn about the file's up axis -> H).
#[test]
fn animation_of_a_z_up_file_is_converted() {
    let doc = import(ANIMATED_Z_UP.as_bytes());
    let id = find(doc.scene(), "Mover");
    for (t, z, h) in [(0.0, 0.0, 0.0), (0.5, 50.0, 45.0), (1.0, 100.0, 90.0)] {
        let (pos, rot, scale) = pose(doc.scene(), id, t);
        assert!(close(pos, [0.0, 0.0, z], 1e-6), "{t}: {pos:?}");
        assert!(close(rot, [h, 0.0, 0.0], 1e-6), "{t}: {rot:?}");
        assert!(close(scale, [1.0; 3], 1e-9), "{t}: {scale:?}");
    }
}

/// A left-handed Y-up file (front axis -Z) already is our world: nothing is reflected and the
/// polygons keep their corner order.
#[test]
fn a_left_handed_file_is_kept_as_it_is() {
    let file = quad_file(
        [1, 1, 2, -1, 0, 1],
        1.0,
        [
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
        ],
        [1.0, 2.0, 3.0],
    );
    let (points, faces) = world_points(&import(file.as_bytes()));
    assert_eq!(faces, vec![vec![0, 1, 2, 3]]);
    assert!(close(points[2], [11.0, 12.0, 3.0], 1e-9), "{points:?}");
}
