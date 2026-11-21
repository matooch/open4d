//! Dynamics in the app's scene evaluation (docs/specs/DYNAMICS.md §5.1).

use forge_app::App;
use forge_core::{NodeId, Value};

fn drop_scene(app: &mut App) -> (NodeId, NodeId) {
    app.doc
        .transact("dynamics scene", |tx| {
            let plane = tx.create("forge.plane", None, None)?;
            let t = tx.attach_tag("forge.rigid_body", plane, None)?;
            tx.set_param(t, "dynamic", Value::Enum("off".into()))?;
            tx.set_param(t, "bounce", Value::Float(1.0))?;
            tx.set_param(t, "friction", Value::Float(1.0))?;
            let cube = tx.create("forge.cube", None, None)?;
            for a in ["size_x", "size_y", "size_z"] {
                tx.set_param(cube, a, Value::Float(50.0))?;
            }
            tx.set_param(cube, "transform.pos", Value::Vec3([0.0, 300.0, 0.0]))?;
            tx.attach_tag("forge.rigid_body", cube, None)?;
            Ok((plane, cube))
        })
        .unwrap()
}

fn y(app: &App, node: NodeId, frame: i64) -> f64 {
    let scene = app.eval.get(app.doc.scene(), frame);
    scene.world_matrix(node).unwrap().w_axis.y
}

#[test]
fn a_body_falls_when_frames_are_evaluated_and_scrubbing_is_stable() {
    let mut app = App::new_default();
    let (plane, cube) = drop_scene(&mut app);
    assert_eq!(y(&app, cube, 0), 300.0);
    let y30 = y(&app, cube, 30);
    assert!((y30 - 25.0).abs() < 0.1, "at rest on the plane: {y30}");
    let y10 = y(&app, cube, 10);
    assert!(y10 < 290.0 && y10 > 200.0, "{y10}");
    // Scrub back and forward: same values.
    assert_eq!(y(&app, cube, 30), y30);
    assert_eq!(y(&app, cube, 10), y10);
    // The collider does not move.
    assert_eq!(y(&app, plane, 30), 0.0);
}

#[test]
fn editing_the_scene_restarts_the_simulation() {
    let mut app = App::new_default();
    let (_, cube) = drop_scene(&mut app);
    let before = y(&app, cube, 20);
    app.doc
        .transact("move", |tx| {
            tx.set_param(cube, "transform.pos", Value::Vec3([0.0, 600.0, 0.0]))
        })
        .unwrap();
    assert!(y(&app, cube, 20) > before + 200.0);
}

#[test]
fn clones_fall_through_the_cloner_evaluation() {
    let mut app = App::new_default();
    let cloner = app
        .doc
        .transact("cloner", |tx| {
            let plane = tx.create("forge.plane", None, None)?;
            let t = tx.attach_tag("forge.rigid_body", plane, None)?;
            tx.set_param(t, "dynamic", Value::Enum("off".into()))?;
            let c = tx.create("forge.cloner", None, None)?;
            tx.set_param(c, "grid_count", Value::Vec3([2.0, 1.0, 2.0]))?;
            tx.set_param(c, "grid_size", Value::Vec3([100.0; 3]))?;
            tx.set_param(c, "transform.pos", Value::Vec3([0.0, 300.0, 0.0]))?;
            let cube = tx.create("forge.cube", Some(c), None)?;
            for a in ["size_x", "size_y", "size_z"] {
                tx.set_param(cube, a, Value::Float(30.0))?;
            }
            tx.attach_tag("forge.rigid_body", c, None)?;
            Ok(c)
        })
        .unwrap();
    let lowest = |frame: i64| {
        let scene = app.eval.get(app.doc.scene(), frame);
        let mesh = forge_generators::evaluate(&scene, cloner, &Default::default())
            .unwrap()
            .unwrap();
        mesh.points.iter().map(|p| p.y).fold(f64::MAX, f64::min)
    };
    assert_eq!(mesh_points(&app, cloner), 32);
    let (y0, y30) = (lowest(0), lowest(30));
    // The mesh is in cloner space (the cloner sits at y 300): the cubes' lowest points start at
    // -15 and end on the plane, at -300.
    assert!((y0 + 15.0).abs() < 1e-9, "{y0}");
    assert!(
        (y30 + 300.0).abs() < 1.0,
        "the clones fell to the plane: {y30}"
    );
}

fn mesh_points(app: &App, cloner: NodeId) -> usize {
    let scene = app.eval.get(app.doc.scene(), 0);
    forge_generators::evaluate(&scene, cloner, &Default::default())
        .unwrap()
        .unwrap()
        .points
        .len()
}

#[test]
fn connectors_and_springs_act_in_the_evaluation() {
    let mut app = App::new_default();
    let (left, right) = app
        .doc
        .transact("hinge", |tx| {
            let mk = |tx: &mut forge_core::Tx, x: f64, dynamic: &str| {
                let c = tx.create("forge.cube", None, None)?;
                for a in ["size_x", "size_y", "size_z"] {
                    tx.set_param(c, a, Value::Float(50.0))?;
                }
                tx.set_param(c, "transform.pos", Value::Vec3([x, 300.0, 0.0]))?;
                let t = tx.attach_tag("forge.rigid_body", c, None)?;
                tx.set_param(t, "dynamic", Value::Enum(dynamic.into()))?;
                Ok::<_, forge_core::EditError>(c)
            };
            let left = mk(tx, -26.0, "off")?;
            let right = mk(tx, 26.0, "on")?;
            let c = tx.create("forge.connector", None, None)?;
            tx.set_param(c, "object_a", Value::Link(Some(left)))?;
            tx.set_param(c, "object_b", Value::Link(Some(right)))?;
            Ok((left, right))
        })
        .unwrap();
    let at = |n: NodeId, f: i64| {
        app.eval
            .get(app.doc.scene(), f)
            .world_matrix(n)
            .unwrap()
            .w_axis
    };
    assert_eq!(at(left, 30).y, 300.0);
    let p = at(right, 30);
    let r = (p.x * p.x + p.y * p.y).sqrt();
    assert!((r - 301.0).abs() < 1.5, "{r}");
    assert!(p.y < 299.0, "swung down: {}", p.y);
}

#[test]
fn a_soft_body_mesh_follows_the_simulation() {
    let mut app = App::new_default();
    let sphere = app
        .doc
        .transact("soft", |tx| {
            let plane = tx.create("forge.plane", None, None)?;
            let t = tx.attach_tag("forge.rigid_body", plane, None)?;
            tx.set_param(t, "dynamic", Value::Enum("off".into()))?;
            let s = tx.create("forge.sphere", None, None)?;
            tx.set_param(s, "radius", Value::Float(50.0))?;
            tx.set_param(s, "transform.pos", Value::Vec3([0.0, 200.0, 0.0]))?;
            let t = tx.attach_tag("forge.rigid_body", s, None)?;
            tx.set_param(t, "soft_body", Value::Enum("polygons".into()))?;
            Ok(s)
        })
        .unwrap();
    let lowest = |frame: i64| {
        let scene = app.eval.get(app.doc.scene(), frame);
        let mesh = forge_generators::evaluate(&scene, sphere, &Default::default())
            .unwrap()
            .unwrap();
        // The mesh is in the object's own space; the object stays at y 200.
        mesh.points
            .iter()
            .map(|p| p.y + 200.0)
            .fold(f64::MAX, f64::min)
    };
    assert!((lowest(0) - 150.0).abs() < 1e-9);
    assert!((lowest(60) - 0.5).abs() < 0.3, "{}", lowest(60));
}
