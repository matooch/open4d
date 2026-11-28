//! Materials and shaders hosted in the document, through the command registry (MATERIAL.md M6,
//! UV.md U5, brief 194), and `uv.generate` against the brief 127 `proj-*` cases.

use std::path::Path;

use forge_app::App;
use forge_core::{Category, NodeId, Value};
use forge_generators::from_poly_mesh;
use forge_material::{Material, TEXTURE_TAG, UVW_TAG};
use forge_parity_harness::discover_cases;
use serde_json::json;

fn id(v: &serde_json::Value, key: &str) -> NodeId {
    NodeId(v[key].as_u64().unwrap())
}

fn cube(app: &mut App) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    app.selection.active.unwrap()
}

#[test]
fn node_material_graph_is_validated_undoable_hosted_and_round_trips() {
    use forge_material::node_material::{
        GRAPH, NODE_MATERIAL, compile_hosted, default_graph, registry,
    };
    use forge_nodes::PortValue;

    let mut app = App::new_default();
    let object = cube(&mut app);
    let material = id(
        &app.run("material.create", json!({"type": NODE_MATERIAL}))
            .unwrap(),
        "material",
    );
    let mut graph = default_graph();
    let surface = graph.nodes().next().unwrap().id;
    graph
        .set_value(registry(), surface, "roughness", PortValue::Float(0.125))
        .unwrap();
    app.run(
        "material.set_graph",
        json!({"material": material.0, "graph": graph.to_json()}),
    )
    .unwrap();
    assert!(app.doc.undo());
    assert!(app.doc.redo());
    let before = app.doc.undo_depth();
    let mut invalid = graph.clone();
    invalid.remove_node(surface).unwrap();
    assert!(
        app.run(
            "material.set_graph",
            json!({"material": material.0, "graph": invalid.to_json()})
        )
        .is_err()
    );
    assert_eq!(app.doc.undo_depth(), before);
    app.run(
        "material.assign",
        json!({"material": material.0, "nodes": [object.0]}),
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("node-material.forge");
    app.save(path.to_str().unwrap()).unwrap();
    let mut reopened = App::new_default();
    reopened.open(path.to_str().unwrap()).unwrap();
    let scene = reopened.doc.scene();
    assert_eq!(
        scene.param(material, GRAPH),
        Some(&Value::String(graph.to_json()))
    );
    let ir = compile_hosted(scene.node(material).unwrap()).unwrap();
    assert_eq!(ir.outputs.diffuse.unwrap().roughness, 0.125);
}

#[test]
fn a_script_builds_a_material_with_a_gradient_and_it_survives_save_and_reopen() {
    let mut app = App::new_default();
    let obj = cube(&mut app);
    let m = id(
        &app.run("material.create", json!({"name": "Wood"})).unwrap(),
        "material",
    );
    let s = id(
        &app.run(
            "shader.set",
            json!({"type": "forge.shader.gradient", "material": m.0, "channel": "color",
                   "params": {"type": "v"}}),
        )
        .unwrap(),
        "shader",
    );
    let tags = app
        .run(
            "material.assign",
            json!({"material": m.0, "nodes": [obj.0]}),
        )
        .unwrap();
    let tag = NodeId(tags["tags"][0].as_u64().unwrap());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scene.forge");
    app.save(path.to_str().unwrap()).unwrap();

    let mut back = App::new_default();
    back.open(path.to_str().unwrap()).unwrap();
    let scene = back.doc.scene();
    assert_eq!(scene.materials(), [m]);
    assert_eq!(scene.node(m).unwrap().name, "Wood");
    assert_eq!(scene.param(m, "color.texture"), Some(&Value::Link(Some(s))));
    assert_eq!(scene.node(m).unwrap().children, [s]);
    assert_eq!(scene.node(s).unwrap().type_id, "forge.shader.gradient");
    assert_eq!(scene.param(s, "type"), Some(&Value::Enum("v".into())));
    assert_eq!(scene.node(tag).unwrap().type_id, TEXTURE_TAG);
    assert_eq!(scene.param(tag, "material"), Some(&Value::Link(Some(m))));
    // the hosted material reads as a texture name the shader set resolves
    let mat = Material::from_node(scene.node(m).unwrap());
    assert_eq!(mat.e("color.texture"), forge_material::shader_name(s));
    let (set, failed) =
        forge_shaders::ShaderSet::from_material(scene, m, &forge_shaders::no_images);
    assert!(failed.is_empty() && set.shaders.len() == 1);
}

#[test]
fn deleting_a_material_clears_the_tag_links_in_one_undo_step() {
    let mut app = App::new_default();
    let obj = cube(&mut app);
    let m = id(&app.run("material.create", json!({})).unwrap(), "material");
    let tags = app
        .run(
            "material.assign",
            json!({"material": m.0, "nodes": [obj.0]}),
        )
        .unwrap();
    let tag = NodeId(tags["tags"][0].as_u64().unwrap());
    let depth = app.doc.undo_depth();
    app.run("material.delete", json!({"materials": [m.0]}))
        .unwrap();
    assert_eq!(app.doc.undo_depth(), depth + 1);
    assert!(app.doc.scene().materials().is_empty());
    assert_eq!(
        app.doc.scene().param(tag, "material"),
        Some(&Value::Link(None))
    );
    app.doc.undo();
    assert_eq!(app.doc.scene().materials(), [m]);
    assert_eq!(
        app.doc.scene().param(tag, "material"),
        Some(&Value::Link(Some(m)))
    );
}

#[test]
fn shader_set_replaces_the_slot_adds_inputs_and_checks_its_arguments() {
    let mut app = App::new_default();
    let m = id(&app.run("material.create", json!({})).unwrap(), "material");
    let first = id(
        &app.run(
            "shader.set",
            json!({"type": "forge.shader.checkerboard", "material": m.0, "channel": "color"}),
        )
        .unwrap(),
        "shader",
    );
    // a second shader replaces the first, which is deleted with its links cleared
    let second = id(
        &app.run(
            "shader.set",
            json!({"type": "forge.shader.layer", "material": m.0, "channel": "color.texture"}),
        )
        .unwrap(),
        "shader",
    );
    let scene = app.doc.scene();
    assert!(!scene.contains(first));
    assert_eq!(scene.node(m).unwrap().children, [second]);
    assert_eq!(
        scene.param(m, "color.texture"),
        Some(&Value::Link(Some(second)))
    );
    // inputs of a shader, in order
    let a = id(
        &app.run(
            "shader.set",
            json!({"type": "forge.shader.color", "parent": second.0}),
        )
        .unwrap(),
        "shader",
    );
    let b = id(
        &app.run(
            "shader.set",
            json!({"type": "forge.shader.color", "parent": second.0,
                   "params": {"brightness": 50.0}}),
        )
        .unwrap(),
        "shader",
    );
    assert_eq!(app.doc.scene().node(second).unwrap().children, [a, b]);
    assert_eq!(
        app.doc.scene().param(b, "brightness"),
        Some(&Value::Float(0.5))
    );
    // null removes the slot's shader
    app.run(
        "shader.set",
        json!({"type": null, "material": m.0, "channel": "color"}),
    )
    .unwrap();
    assert!(!app.doc.scene().contains(second));
    assert_eq!(
        app.doc.scene().param(m, "color.texture"),
        Some(&Value::Link(None))
    );
    // errors change nothing
    let depth = app.doc.undo_depth();
    for args in [
        json!({"type": "forge.shader.color", "material": m.0, "channel": "nope"}),
        json!({"type": "forge.shader.color", "material": m.0}),
        json!({"type": "forge.shader.color", "material": 999, "channel": "color"}),
        json!({"type": "forge.cube", "material": m.0, "channel": "color"}),
        json!({"type": "forge.shader.color", "material": m.0, "channel": "color",
               "params": {"nope": 1}}),
        json!({"type": null, "parent": a.0}),
    ] {
        assert!(app.run("shader.set", args.clone()).is_err(), "{args}");
    }
    assert_eq!(app.doc.undo_depth(), depth);
    assert_eq!(app.doc.scene().node(m).unwrap().children.len(), 0);
}

#[test]
fn assign_needs_objects_and_a_material_and_create_rejects_other_types() {
    let mut app = App::new_default();
    let m = id(&app.run("material.create", json!({})).unwrap(), "material");
    assert!(
        app.run("material.assign", json!({"material": m.0}))
            .is_err()
    );
    let obj = cube(&mut app);
    assert!(
        app.run(
            "material.assign",
            json!({"material": obj.0, "nodes": [obj.0]})
        )
        .is_err()
    );
    assert!(
        app.run("material.create", json!({"type": "forge.cube"}))
            .is_err()
    );
    assert!(
        app.run("material.delete", json!({"materials": [obj.0]}))
            .is_err()
    );
    let r = app
        .run(
            "material.assign",
            json!({"material": m.0, "nodes": [obj.0], "projection": "cubic"}),
        )
        .unwrap();
    let tag = NodeId(r["tags"][0].as_u64().unwrap());
    assert_eq!(
        app.doc.scene().param(tag, "projection"),
        Some(&Value::Enum("cubic".into()))
    );
    assert_eq!(
        app.doc
            .scene()
            .registry()
            .get(forge_material::MATERIAL)
            .unwrap()
            .category,
        Category::Material
    );
}

// --- uv.generate against the brief 127 cases ---

type Parsed = (Vec<Vec<usize>>, Vec<[f64; 2]>, Vec<Vec<usize>>);

fn parse(src: &str) -> Parsed {
    let (mut vt, mut faces, mut fvt) = (vec![], vec![], vec![]);
    for l in src.lines() {
        let t: Vec<&str> = l.split_whitespace().collect();
        match t.first().copied() {
            Some("vt") => vt.push([t[1].parse().unwrap(), t[2].parse().unwrap()]),
            Some("f") => {
                let c: Vec<Vec<&str>> = t[1..].iter().map(|s| s.split('/').collect()).collect();
                faces.push(
                    c.iter()
                        .map(|x| x[0].parse::<usize>().unwrap() - 1)
                        .collect(),
                );
                fvt.push(
                    c.iter()
                        .map(|x| x[1].parse::<usize>().unwrap() - 1)
                        .collect(),
                );
            }
            _ => {}
        }
    }
    (faces, vt, fvt)
}

fn f3(v: &toml::Value) -> [f64; 3] {
    let a = v.as_array().unwrap();
    std::array::from_fn(|i| {
        a[i].as_float()
            .or_else(|| a[i].as_integer().map(|x| x as f64))
            .unwrap()
    })
}

const EXACT: &[&str] = &[
    "proj-flat",
    "proj-cubic",
    "proj-cubic-scaled",
    "proj-spherical",
    "proj-cylindrical",
    "proj-cylindrical-offset",
];
/// Built but different: the recorded camera does not reproduce the reference (UV.md U4).
const KNOWN_MISMATCH: &[&str] = &["proj-frontal"];

fn max_error(name: &str) -> f64 {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.sphere"}))
        .unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    let obj = app.selection.active.unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let case = discover_cases(&root)
        .unwrap()
        .into_iter()
        .find(|c| c.feature == "uv" && c.name == name)
        .unwrap();
    let m = case.manifest().unwrap();
    let tag = m.params["texture_tag"].as_table().unwrap();
    let proj = match tag["projection_set"].as_str().unwrap() {
        "Flat" => "flat",
        "Cubic" => "cubic",
        "Spherical" => "spherical",
        "Cylindrical" => "cylindrical",
        "Frontal" => "frontal",
        other => panic!("{other}"),
    };
    let mat = id(&app.run("material.create", json!({})).unwrap(), "material");
    let tags = app
        .run(
            "material.assign",
            json!({"material": mat.0, "nodes": [obj.0], "projection": proj}),
        )
        .unwrap();
    let ttag = NodeId(tags["tags"][0].as_u64().unwrap());
    app.doc
        .transact("tag", |tx| {
            tx.set_param(ttag, "position", Value::Vec3(f3(&tag["position"])))?;
            tx.set_param(
                ttag,
                "rotation",
                Value::Vec3(f3(&tag["rotation_deg"]).map(f64::to_radians)),
            )?;
            tx.set_param(ttag, "size", Value::Vec3(f3(&tag["size"])))
        })
        .unwrap();
    let args = if proj == "frontal" {
        json!({"camera": {"position": f3(&tag["camera_position"]),
                          "rotation_deg": f3(&tag["camera_rotation_deg"])}})
    } else {
        json!({})
    };
    let r = app.run("uv.generate", args).unwrap();
    let uvw = NodeId(r["tag"].as_u64().unwrap());
    assert_eq!(app.doc.scene().node(uvw).unwrap().type_id, UVW_TAG);
    let Some(Value::FloatList(flat)) = app.doc.scene().param(uvw, "uvw").cloned() else {
        panic!("no uvw list")
    };
    let (faces, vt, fvt) = parse(&std::fs::read_to_string(case.dir.join("mesh.obj")).unwrap());
    let Some(Value::Mesh(mesh)) = app.doc.scene().param(obj, "mesh").cloned() else {
        panic!()
    };
    assert_eq!(faces.len(), mesh.polys.len());
    assert_eq!(r["corners"].as_u64().unwrap() as usize, flat.len() / 2);
    let mut worst: f64 = 0.0;
    for (k, t) in fvt.iter().flatten().enumerate() {
        worst = worst
            .max((flat[2 * k] - vt[*t][0]).abs())
            .max((flat[2 * k + 1] - vt[*t][1]).abs());
    }
    let _ = from_poly_mesh;
    worst
}

#[test]
fn uv_generate_matches_the_exact_projection_cases() {
    for name in EXACT {
        let e = max_error(name);
        assert!(e <= 1e-5, "{name}: {e}");
    }
}

#[test]
fn uv_generate_known_mismatch_still_differs() {
    for name in KNOWN_MISMATCH {
        let e = max_error(name);
        assert!(e > 1e-5 && e < 1e-3, "{name}: {e}");
    }
}

#[test]
fn uv_generate_errors_and_reuse() {
    let mut app = App::new_default();
    let obj = cube(&mut app);
    // not a polygon object
    assert!(app.run("uv.generate", json!({})).is_err());
    app.run("mesh.make_editable", json!({})).unwrap();
    let obj2 = app.selection.active.unwrap();
    assert_ne!(obj, obj2);
    // no texture tag
    assert!(app.run("uv.generate", json!({})).is_err());
    let m = id(&app.run("material.create", json!({})).unwrap(), "material");
    let r = app
        .run(
            "material.assign",
            json!({"material": m.0, "nodes": [obj2.0], "projection": "frontal"}),
        )
        .unwrap();
    let tag = NodeId(r["tags"][0].as_u64().unwrap());
    // frontal without a camera, and the projections without a rule
    assert!(app.run("uv.generate", json!({})).is_err());
    for p in ["spatial", "uvw", "shrink_wrap", "camera"] {
        app.doc
            .transact("p", |tx| {
                tx.set_param(tag, "projection", Value::Enum(p.into()))
            })
            .unwrap();
        assert!(app.run("uv.generate", json!({})).is_err(), "{p}");
    }
    // a supported projection writes the tag once and reuses it
    app.doc
        .transact("p", |tx| {
            tx.set_param(tag, "projection", Value::Enum("flat".into()))
        })
        .unwrap();
    let a = app.run("uv.generate", json!({})).unwrap();
    let depth = app.doc.undo_depth();
    let b = app
        .run("uv.generate", json!({"object": obj2.0, "tag": tag.0}))
        .unwrap();
    assert_eq!(a["tag"], b["tag"]);
    assert_eq!(app.doc.undo_depth(), depth, "same result: no new step");
}

#[test]
fn conversion_preserves_shading_and_assignments_and_allows_graph_edits() {
    use forge_material::node_material::{NODE_MATERIAL, compile_hosted, load_graph, registry};

    let mut app = App::new_default();
    let object = cube(&mut app);
    let material = id(
        &app.run(
            "material.create",
            json!({"name": "Copper", "params": {
                "color.color": [0.7, 0.3, 0.15],
                "color.roughness": 0.24,
                "reflectance.spec.attenuation": "metal",
                "enable.transparency": true
            }}),
        )
        .unwrap(),
        "material",
    );
    let shader = id(
        &app.run(
            "shader.set",
            json!({
                "material": material.0, "channel": "color", "type": "forge.shader.gradient"
            }),
        )
        .unwrap(),
        "shader",
    );
    let assignments = app
        .run(
            "material.assign",
            json!({"material": material.0, "nodes": [object.0]}),
        )
        .unwrap();
    let tag = NodeId(assignments["tags"][0].as_u64().unwrap());
    let before = compile_hosted(app.doc.scene().node(material).unwrap()).unwrap();
    let depth = app.doc.undo_depth();
    app.run("material.convert_to_node", json!({"material": material.0}))
        .unwrap();
    assert_eq!(app.doc.undo_depth(), depth + 1);
    let scene = app.doc.scene();
    let converted = scene.node(material).unwrap();
    assert_eq!(converted.type_id, NODE_MATERIAL);
    assert_eq!(converted.name, "Copper");
    assert_eq!(converted.children, [shader]);
    assert_eq!(compile_hosted(converted).unwrap(), before);
    assert_eq!(
        scene.param(tag, "material"),
        Some(&Value::Link(Some(material)))
    );
    assert!(app.doc.undo());
    assert_eq!(
        app.doc.scene().node(material).unwrap().type_id,
        forge_material::MATERIAL
    );
    assert!(app.doc.redo());

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("converted.forge");
    app.save(path.to_str().unwrap()).unwrap();
    let mut app = App::new_default();
    app.open(path.to_str().unwrap()).unwrap();
    let converted = app.doc.scene().node(material).unwrap();
    assert_eq!(compile_hosted(converted).unwrap(), before);
    let mut graph = load_graph(converted).unwrap();
    let surface = graph.nodes().next().unwrap().id;
    graph
        .set_value(
            registry(),
            surface,
            "roughness",
            forge_nodes::PortValue::Float(0.8),
        )
        .unwrap();
    app.run(
        "material.set_graph",
        json!({"material": material.0, "graph": graph.to_json()}),
    )
    .unwrap();
    let edited = compile_hosted(app.doc.scene().node(material).unwrap()).unwrap();
    assert_eq!(edited.outputs.diffuse.unwrap().roughness, 0.8);
    assert_eq!(edited.outputs.specular.unwrap().width, 0.8);
    assert_eq!(edited.outputs.transmission, before.outputs.transmission);
}
