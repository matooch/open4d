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
