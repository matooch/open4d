//! Public OpenUSD subset round trips (brief 168).

use forge_app::App;
use forge_core::{EvalCtx, Interp, Key, Value, transform::ROT};
use forge_io::{export_scene, formats, usda};
use glam::DVec3;

fn import(text: &str) -> App {
    let ir = formats::read_scene("usda", text.as_bytes(), &formats::Options::new()).unwrap();
    let mut app = App::new_default();
    app.doc.transact("Import USDA", |tx| ir.apply(tx)).unwrap();
    app
}

#[test]
fn ten_primitives_keep_exact_points_polygons_and_corner_uvs() {
    let mut app = App::new_default();
    for ty in [
        "cube", "cone", "cylinder", "sphere", "plane", "disc", "torus", "tube", "pyramid",
        "capsule",
    ] {
        app.doc
            .transact("Make primitive", |tx| {
                let node = tx.create(&format!("forge.{ty}"), None, None)?;
                let mesh = forge_generators::evaluate(tx.scene(), node, &EvalCtx::default())
                    .unwrap()
                    .unwrap();
                let tag = tx.attach_tag("forge.uvw", node, None)?;
                let uv = mesh
                    .polys
                    .iter()
                    .enumerate()
                    .flat_map(|(f, face)| {
                        (0..face.len()).flat_map(move |i| [i as f64 / 7.0, f as f64 / 11.0])
                    })
                    .collect();
                tx.set_param(tag, "uvw", Value::FloatList(uv))
            })
            .unwrap();
    }
    let scene = app.doc.scene();
    let (text, count) = usda::write(scene, None).unwrap();
    let restored = import(&text);
    assert_eq!(count, 10);
    let (old, new) = (
        export_scene(scene, None),
        export_scene(restored.doc.scene(), None),
    );
    assert_eq!(old.len(), new.len());
    for (a, b) in old.iter().zip(&new) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.mesh, b.mesh, "{}", a.name);
        assert!(a.uvs.is_some());
        assert_eq!(a.uvs, b.uvs);
    }
}

#[test]
fn animated_hierarchy_and_material_round_trip() {
    let mut app = App::new_default();
    let (root, mesh) = app
        .doc
        .transact("Build hierarchy", |tx| {
            let root = tx.create("forge.null", None, None)?;
            tx.rename(root, "Animated Rig")?;
            let mesh = tx.create("forge.cube", Some(root), None)?;
            tx.set_param(mesh, ROT, Value::Vec3([0.3, -0.6, 0.9]))?;
            for (track, end) in [
                ("transform.pos.z", 210.0),
                ("transform.rot.x", 0.8),
                ("transform.scale.y", 1.8),
            ] {
                tx.set_key(
                    root,
                    track,
                    Key::new(0.0, if track.contains("scale") { 1.0 } else { 0.0 }),
                )?;
                tx.set_key(root, track, Key::new(1.0, end).with_interp(Interp::Linear))?;
            }
            let mat = tx.create_material("forge.material.standard", None)?;
            tx.rename(mat, "Rust Red")?;
            tx.set_param(mat, "color.color", Value::Color([0.6, 0.14, 0.23]))?;
            tx.set_param(mat, "color.roughness", Value::Float(0.3))?;
            let tag = tx.attach_tag("forge.tag.texture", mesh, None)?;
            tx.set_param(tag, "material", Value::Link(Some(mat)))?;
            Ok((root, mesh))
        })
        .unwrap();
    let (text, _) = usda::write(app.doc.scene(), None).unwrap();
    let restored = import(&text);
    let s = restored.doc.scene();
    assert_eq!(s.roots().len(), 1);
    let parent = s.roots()[0];
    let child = s.node(parent).unwrap().children[0];
    assert_eq!(s.node(parent).unwrap().name, "Animated Rig");
    let tag = s
        .node(child)
        .unwrap()
        .tags
        .iter()
        .copied()
        .find(|&n| s.node(n).unwrap().type_id == "forge.tag.texture")
        .unwrap();
    let Some(Value::Link(Some(mat))) = s.param(tag, "material") else {
        panic!("no binding");
    };
    assert_eq!(s.node(*mat).unwrap().name, "Rust Red");
    assert_eq!(
        s.param(*mat, "color.color"),
        Some(&Value::Color([0.6, 0.14, 0.23]))
    );
    assert_eq!(s.param(*mat, "color.roughness"), Some(&Value::Float(0.3)));
    for f in [0, 9, 15, 30] {
        let t = f as f64 / 30.0;
        let a = forge_anim::scene_at(app.doc.scene(), t);
        let b = forge_anim::scene_at(s, t);
        for (x, y) in [(root, parent), (mesh, child)] {
            assert!(
                a.world_matrix(x)
                    .unwrap()
                    .abs_diff_eq(b.world_matrix(y).unwrap(), 1e-9)
            );
        }
    }
}

#[test]
fn public_schema_sample_preserves_winding_and_vertex_st() {
    let app = import(
        r#"#usda 1.0
(
    upAxis = "Y"
    metersPerUnit = 1
    timeCodesPerSecond = 24
)
def Xform "World" {
    double3 xformOp:translate = (1, 2, 3)
    uniform token[] xformOpOrder = ["xformOp:translate"]
    def Mesh "Triangle" {
        uniform token subdivisionScheme = "none"
        point3f[] points = [(0,0,0), (1,0,0), (0,1,0)]
        int[] faceVertexCounts = [3]
        int[] faceVertexIndices = [0,1,2]
        normal3f[] normals = [(0,0,1), (0,0,1), (0,0,1)] (
            interpolation = "vertex"
        )
        texCoord2f[] primvars:st = [(0,0), (1,0), (0,1)] (
            interpolation = "vertex"
        )
    }
}
"#,
    );
    let objects = export_scene(app.doc.scene(), None);
    let o = &objects[0];
    assert_eq!(o.world.w_axis.truncate(), DVec3::new(100.0, 200.0, -300.0));
    assert_eq!(o.mesh.points[1], DVec3::new(100.0, 0.0, 0.0));
    assert_eq!(o.mesh.polys, [vec![2, 1, 0]]);
    assert_eq!(
        o.uvs.as_ref().unwrap().corners,
        [vec![[0., 1.], [1., 0.], [0., 0.]]]
    );
}

#[test]
fn invalid_topology_and_external_layers_are_rejected() {
    let bad = r#"#usda 1.0
def Mesh "Broken" {
    point3f[] points = [(0,0,0), (1,0,0), (0,1,0)]
    int[] faceVertexCounts = [3]
    int[] faceVertexIndices = [0,1,7]
}
"#;
    assert!(matches!(usda::read(bad), Err(usda::Error::Invalid(_))));
    assert!(matches!(
        usda::read("#usda 1.0\n(subLayers = [@external.usda@])\n"),
        Err(usda::Error::Unsupported(_))
    ));
}
