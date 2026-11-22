//! The rig and volume overlays of the editor viewport, headless (docs/specs/VIEWPORT.md
//! sections 13.4 and 14, brief 192).

use std::collections::BTreeSet;

use forge_app::App;
use forge_core::{EvalCtx, NodeId, Value};
use forge_rig::WeightMap;
use forge_ui::scene_overlay::{SceneOverlay, VolumeViews};
use forge_viewport::camera::Camera;
use forge_viewport::render::Palette;
use forge_viewport::volume::BOX_COLOR;
use serde_json::json;

const SIZE: [f64; 2] = [640.0, 480.0];

fn node(app: &mut App, ty: &str) -> NodeId {
    NodeId(
        app.run("object.create", json!({"type": ty})).unwrap()["node"]
            .as_u64()
            .unwrap(),
    )
}

fn rigged() -> (App, Vec<NodeId>, NodeId) {
    let mut app = App::new_default();
    let out = app
        .run(
            "joint.create_chain",
            json!({"points": [[0, 0, 0], [0, 100, 0], [0, 200, 0]], "align": "y"}),
        )
        .unwrap();
    let chain: Vec<NodeId> = out["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| NodeId(v.as_u64().unwrap()))
        .collect();
    let cube = node(&mut app, "forge.cube");
    let points = {
        let mesh = forge_generators::evaluate(app.doc.scene(), cube, &EvalCtx::default())
            .unwrap()
            .unwrap();
        mesh.points.len()
    };
    // joint slot 0 gets weight 1 on the points of even index, slot 1 the rest
    let rows: Vec<Vec<f64>> = (0..points)
        .map(|i| {
            if i % 2 == 0 {
                vec![1.0, 0.0]
            } else {
                vec![0.0, 1.0]
            }
        })
        .collect();
    app.doc
        .transact("tag", |tx| {
            let tag = tx.attach_tag("forge.weight_tag", cube, None)?;
            tx.set_param(tag, "joints", Value::LinkList(vec![chain[0], chain[1]]))?;
            tx.set_param(tag, "weights", WeightMap::from_dense(&rows).to_value())
        })
        .unwrap();
    (app, chain, cube)
}

#[test]
fn a_joint_chain_draws_bones_and_markers() {
    let (app, chain, _) = rigged();
    let none = BTreeSet::new();
    let o = SceneOverlay::collect(app.doc.scene(), &none, &EvalCtx::default());
    assert_eq!(o.joints.len(), 3);
    assert!(
        o.weights.is_empty(),
        "weights show for a selected joint only"
    );
    let pal = Palette::default();
    let lines = o.lines(&pal, &Camera::default(), SIZE);
    // two bones of 12 edges and three joints of three axis lines
    assert!(lines.len() >= 2 * 12 + 3 * 3, "{}", lines.len());
    // selecting a joint switches its colour
    let sel: BTreeSet<NodeId> = [chain[1]].into();
    let o2 = SceneOverlay::collect(app.doc.scene(), &sel, &EvalCtx::default());
    let with_sel = o2.lines(&pal, &Camera::default(), SIZE);
    assert!(with_sel.iter().any(|l| l.color == pal.selection));
    assert!(!lines.iter().any(|l| l.color == pal.selection));
}

#[test]
fn the_selected_joint_shows_its_weights() {
    let (app, chain, _) = rigged();
    let sel: BTreeSet<NodeId> = [chain[0]].into();
    let o = SceneOverlay::collect(app.doc.scene(), &sel, &EvalCtx::default());
    assert_eq!(o.weights.len(), 1);
    let w = &o.weights[0];
    assert_eq!(w.points.len(), w.weights.len());
    assert!(w.weights.iter().step_by(2).all(|x| *x == 1.0));
    assert!(w.weights.iter().skip(1).step_by(2).all(|x| *x == 0.0));
    let n = w.points.len();
    let base = SceneOverlay::collect(app.doc.scene(), &BTreeSet::new(), &EvalCtx::default());
    let pal = Palette::default();
    let cam = Camera::default();
    let more = o.lines(&pal, &cam, SIZE).len() - base.lines(&pal, &cam, SIZE).len();
    // squares only where the weight is not zero; the selection recolours, adds no bone lines
    assert!(more > 0 && more <= 4 * n, "{more} lines for {n} points");
    // a joint that is not in the tag has no markers
    let other: BTreeSet<NodeId> = [chain[2]].into();
    assert!(
        SceneOverlay::collect(app.doc.scene(), &other, &EvalCtx::default())
            .weights
            .is_empty()
    );
}

#[test]
fn a_volume_mesher_draws_the_box_of_its_grid() {
    let mut app = App::new_default();
    let mesher = node(&mut app, "forge.volume_mesher");
    let builder = node(&mut app, "forge.volume_builder");
    let cube = node(&mut app, "forge.cube");
    app.run(
        "object.reparent",
        json!({"node": builder.0, "parent": mesher.0}),
    )
    .unwrap();
    app.run(
        "object.reparent",
        json!({"node": cube.0, "parent": builder.0}),
    )
    .unwrap();
    let mut views = VolumeViews::synchronous();
    let (lines, pending) = views.lines(app.doc.scene(), &EvalCtx::default());
    assert!(!pending);
    assert_eq!(lines.len(), 12);
    assert!(lines.iter().all(|l| l.color == BOX_COLOR));
    // the box encloses the cube (200 wide) and its padding
    let max = lines
        .iter()
        .map(|l| l.a.x.max(l.b.x))
        .fold(f64::MIN, f64::max);
    assert!(max >= 100.0, "{max}");
    // moving the mesher moves the box
    app.doc
        .transact("move", |tx| {
            tx.set_param(mesher, "transform.pos", Value::Vec3([1000.0, 0.0, 0.0]))
        })
        .unwrap();
    let (moved, _) = views.lines(app.doc.scene(), &EvalCtx::default());
    let min = moved
        .iter()
        .map(|l| l.a.x.min(l.b.x))
        .fold(f64::MAX, f64::min);
    assert!(min > 800.0, "{min}");
}

#[test]
fn the_worker_path_delivers_the_box_without_blocking() {
    let mut app = App::new_default();
    let mesher = node(&mut app, "forge.volume_mesher");
    let builder = node(&mut app, "forge.volume_builder");
    let cube = node(&mut app, "forge.cube");
    app.run(
        "object.reparent",
        json!({"node": builder.0, "parent": mesher.0}),
    )
    .unwrap();
    app.run(
        "object.reparent",
        json!({"node": cube.0, "parent": builder.0}),
    )
    .unwrap();
    let mut views = VolumeViews::default();
    let ctx = EvalCtx::default();
    let mut lines = Vec::new();
    for _ in 0..400 {
        let (l, pending) = views.lines(app.doc.scene(), &ctx);
        lines = l;
        if !pending && !lines.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(lines.len(), 12);
}

#[test]
fn a_plain_scene_has_no_overlay() {
    let mut app = App::new_default();
    let cube = node(&mut app, "forge.cube");
    let sel: BTreeSet<NodeId> = [cube].into();
    let (o, pending) =
        VolumeViews::synchronous().overlay(app.doc.scene(), &sel, &EvalCtx::default());
    assert!(!pending);
    assert_eq!(o, SceneOverlay::default());
    assert!(
        o.lines(&Palette::default(), &Camera::default(), SIZE)
            .is_empty()
    );
}

#[test]
fn the_create_menu_lists_volume_and_character_entries() {
    use forge_ui::commands::{CHARACTER_TYPES, character_menu, create_menu};
    let reg = forge_app::default_registry().unwrap();
    let menu = create_menu(&reg);
    let volumes = menu.iter().find(|(c, _)| c == "Volume").unwrap();
    for t in [
        "forge.volume_builder",
        "forge.volume_mesher",
        "forge.volume_filter",
    ] {
        assert!(volumes.1.iter().any(|(id, _)| id == t), "{t}");
    }
    let character = character_menu(&reg);
    let labels: Vec<&str> = character.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(
        labels,
        ["Joint", "Skin", "Weight", "Align Joints", "Mirror Joints"]
    );
    assert_eq!(CHARACTER_TYPES.len(), 3);
    for e in &character {
        let mut app = App::new_default();
        if e.command == "object.create" {
            // a tag needs a host object
            app.run("object.create", json!({"type": "forge.cube"}))
                .unwrap();
            app.run(e.command, e.args.clone()).unwrap();
        } else {
            assert!(app.commands.get(e.command).is_some());
        }
    }
}
