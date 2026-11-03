//! docs/specs/CORE.md §7 and COMMANDS.md §6 (brief 158): the `layer.*` commands and what
//! each layer toggle does to evaluation, including the scheduler's dirtiness.

use std::collections::BTreeMap;

use forge_app::App;
use forge_app::animate::Evaluated;
use forge_app::schedule::{Frame, NodeOutput, Path as Found, Scheduler, Threads, drawn_nodes};
use forge_core::layers::LAYER;
use forge_core::{Key, NodeId, Value, visibility};
use serde_json::{Value as Json, json};

fn make(app: &mut App, ty: &str) -> NodeId {
    NodeId(
        app.run("object.create", json!({"type": ty})).unwrap()["node"]
            .as_u64()
            .unwrap(),
    )
}

fn layer(app: &mut App, name: &str) -> NodeId {
    NodeId(
        app.run("layer.create", json!({"name": name})).unwrap()["layer"]
            .as_u64()
            .unwrap(),
    )
}

fn set(app: &mut App, layer: NodeId, args: Json) -> Json {
    let mut a = args;
    a["layer"] = json!(layer.0);
    app.run("layer.set", a).unwrap()
}

fn assign(app: &mut App, layer: Option<NodeId>, nodes: &[NodeId]) {
    app.run(
        "layer.assign",
        json!({"layer": layer.map(|l| l.0), "nodes": nodes.iter().map(|n| n.0).collect::<Vec<_>>()}),
    )
    .unwrap();
}

fn drawn(app: &App) -> Vec<NodeId> {
    drawn_nodes(app.doc.scene())
}

fn points(f: &Frame, id: NodeId) -> Option<Vec<[u64; 3]>> {
    f.outputs[&id].mesh.as_ref().map(|m| {
        m.points
            .iter()
            .map(|p| p.to_array().map(f64::to_bits))
            .collect()
    })
}

fn out_bits(o: &NodeOutput) -> Option<Vec<[u64; 3]>> {
    o.mesh.as_ref().map(|m| {
        m.points
            .iter()
            .map(|p| p.to_array().map(f64::to_bits))
            .collect()
    })
}

/// The full evaluation of a scene by a fresh scheduler: what an incremental pass must equal.
fn fresh(app: &App, frame: i64) -> BTreeMap<NodeId, Option<Vec<[u64; 3]>>> {
    let mut s = Scheduler::new(Threads::Serial);
    let f = s.evaluate(app.doc.scene(), frame);
    f.outputs.iter().map(|(k, v)| (*k, out_bits(v))).collect()
}

fn frame_bits(f: &Frame) -> BTreeMap<NodeId, Option<Vec<[u64; 3]>>> {
    f.outputs.iter().map(|(k, v)| (*k, out_bits(v))).collect()
}

// --- commands -------------------------------------------------------------------------------

#[test]
fn every_layer_command_is_one_undo_step() {
    let mut app = App::new_default();
    let a = make(&mut app, "forge.cube");
    let b = make(&mut app, "forge.sphere");
    let depth = |app: &App| app.doc.undo_depth();
    let mut n = depth(&app);
    let l1 = layer(&mut app, "Props");
    n += 1;
    assert_eq!(depth(&app), n);
    let l2 = NodeId(
        app.run(
            "layer.create",
            json!({"assign": true, "nodes": [a.0, b.0], "color": [1.0, 0.0, 0.0]}),
        )
        .unwrap()["layer"]
            .as_u64()
            .unwrap(),
    );
    n += 1;
    assert_eq!(depth(&app), n, "create and assign is one step");
    assert_eq!(app.doc.scene().layers().get(l2).unwrap().name, "Layer 1");
    assert_eq!(
        app.doc.scene().param(a, LAYER),
        Some(&Value::Link(Some(l2)))
    );
    app.run("layer.rename", json!({"layer": l1.0, "name": "Walls"}))
        .unwrap();
    n += 1;
    assert_eq!(depth(&app), n);
    let r = set(&mut app, l1, json!({"view": false, "locked": true}));
    assert_eq!(r["changed"], 1);
    n += 1;
    assert_eq!(depth(&app), n);
    let r = app
        .run(
            "layer.set",
            json!({"layers": [l1.0, l2.0], "solo": true, "color": [0.1, 0.2, 0.3]}),
        )
        .unwrap();
    assert_eq!(r["changed"], 2);
    n += 1;
    assert_eq!(depth(&app), n, "several layers, one step");
    let r = app
        .run("layer.assign", json!({"layer": l1.0, "nodes": [a.0]}))
        .unwrap();
    assert_eq!(r["assigned"], 1);
    n += 1;
    assert_eq!(depth(&app), n);
    let r = app
        .run("layer.merge", json!({"layers": [l1.0, l2.0], "into": l1.0}))
        .unwrap();
    assert_eq!(
        (r["merged"].as_u64(), r["members"].as_u64()),
        (Some(1), Some(1))
    );
    n += 1;
    assert_eq!(depth(&app), n);
    assert_eq!(app.doc.scene().layers().len(), 1);
    assert_eq!(
        app.doc.scene().param(b, LAYER),
        Some(&Value::Link(Some(l1)))
    );
    let r = app.run("layer.delete", json!({"layers": [l1.0]})).unwrap();
    assert_eq!(
        (r["deleted"].as_u64(), r["released"].as_u64()),
        (Some(1), Some(2))
    );
    n += 1;
    assert_eq!(depth(&app), n);
    assert!(app.doc.scene().layers().is_empty());
    // each undo reverses exactly one command
    for _ in 0..8 {
        assert!(app.doc.undo());
        n -= 1;
        assert_eq!(depth(&app), n);
    }
    assert!(app.doc.scene().layers().is_empty());
}

#[test]
fn layer_commands_reject_bad_arguments_without_changing_anything() {
    let mut app = App::new_default();
    let a = make(&mut app, "forge.cube");
    let l = layer(&mut app, "L");
    let rev = app.doc.scene().revision();
    let bad = |app: &mut App, id: &str, args: Json| {
        let e = app.run(id, args).unwrap_err();
        assert!(
            matches!(e, forge_command::CommandError::InvalidArgs(_)),
            "{id}: {e:?}"
        );
    };
    bad(&mut app, "layer.set", json!({"layer": 999, "view": false}));
    bad(&mut app, "layer.set", json!({"layer": l.0}));
    bad(&mut app, "layer.set", json!({"view": true}));
    bad(
        &mut app,
        "layer.set",
        json!({"layer": l.0, "layers": [l.0], "view": true}),
    );
    bad(&mut app, "layer.set", json!({"layer": l.0, "name": "  "}));
    bad(&mut app, "layer.rename", json!({"layer": l.0, "name": ""}));
    bad(&mut app, "layer.rename", json!({"layer": 999, "name": "x"}));
    bad(&mut app, "layer.delete", json!({"layers": []}));
    bad(&mut app, "layer.delete", json!({"layers": [l.0, 999]}));
    bad(
        &mut app,
        "layer.assign",
        json!({"layer": 999, "nodes": [a.0]}),
    );
    bad(
        &mut app,
        "layer.merge",
        json!({"layers": [l.0], "into": l.0}),
    );
    bad(
        &mut app,
        "layer.merge",
        json!({"layers": [l.0, 999], "into": l.0}),
    );
    assert_eq!(app.doc.scene().revision(), rev);
}

#[test]
fn assign_uses_the_selection_and_skips_tags() {
    let mut app = App::new_default();
    let a = make(&mut app, "forge.cube");
    let tag = make(&mut app, forge_nodes::TAG);
    let l = layer(&mut app, "L");
    app.selection.click(a);
    app.run("layer.assign", json!({"layer": l.0})).unwrap();
    assert_eq!(app.doc.scene().param(a, LAYER), Some(&Value::Link(Some(l))));
    let r = app
        .run("layer.assign", json!({"layer": l.0, "nodes": [tag.0]}))
        .unwrap();
    assert_eq!(r["assigned"], 0, "a tag follows its host");
    app.run("layer.assign", json!({"layer": null})).unwrap();
    assert_eq!(app.doc.scene().param(a, LAYER), Some(&Value::Link(None)));
}

// --- view, solo, render ---------------------------------------------------------------------

#[test]
fn view_and_solo_change_what_the_scheduler_draws() {
    let mut app = App::new_default();
    let a = make(&mut app, "forge.cube");
    let b = make(&mut app, "forge.sphere");
    let c = make(&mut app, "forge.cone");
    let (l1, l2) = (layer(&mut app, "One"), layer(&mut app, "Two"));
    assign(&mut app, Some(l1), &[a]);
    assign(&mut app, Some(l2), &[b]);
    assert_eq!(app.scheduled_draw(0).drawn, vec![a, b, c]);
    set(&mut app, l1, json!({"view": false}));
    assert_eq!(drawn(&app), vec![b, c]);
    let f = app.scheduled_draw(0);
    assert_eq!(f.drawn, vec![b, c]);
    assert!(!f.outputs.contains_key(&a));
    set(&mut app, l2, json!({"solo": true}));
    assert_eq!(app.scheduled_draw(0).drawn, vec![b]);
    // the object's own settings are unchanged: layers do not write object.* params
    assert_eq!(
        visibility::own(app.doc.scene(), a),
        (visibility::Vis::Default, visibility::Vis::Default, true)
    );
    // undo restores the previous picture
    assert!(app.doc.undo());
    assert_eq!(app.scheduled_draw(0).drawn, vec![b, c]);
    assert!(app.doc.undo());
    assert_eq!(app.scheduled_draw(0).drawn, vec![a, b, c]);
}

#[test]
fn render_off_leaves_the_viewport_alone() {
    let mut app = App::new_default();
    let a = make(&mut app, "forge.cube");
    let l = layer(&mut app, "L");
    assign(&mut app, Some(l), &[a]);
    set(&mut app, l, json!({"render": false}));
    assert_eq!(drawn(&app), vec![a]);
    assert!(!visibility::render_visible(app.doc.scene(), a));
}

// --- animation ------------------------------------------------------------------------------

fn animated_cube(app: &mut App) -> NodeId {
    let c = make(app, "forge.cube");
    app.doc
        .transact("keys", |tx| {
            tx.set_key(c, "transform.pos.x", Key::new(0.0, 0.0))?;
            tx.set_key(c, "transform.pos.x", Key::new(1.0, 100.0))
        })
        .unwrap();
    c
}

#[test]
fn animation_off_keeps_the_stored_values() {
    let mut app = App::new_default();
    let c = animated_cube(&mut app);
    let l = layer(&mut app, "L");
    assign(&mut app, Some(l), &[c]);
    let x = |app: &App, frame| match app
        .eval
        .get(app.doc.scene(), frame)
        .param(c, "transform.pos")
    {
        Some(Value::Vec3(v)) => v[0],
        other => panic!("{other:?}"),
    };
    assert!(x(&app, 15) > 1.0, "the track moves the cube");
    set(&mut app, l, json!({"animation": false}));
    assert_eq!(x(&app, 15), 0.0);
    assert_eq!(x(&app, 30), 0.0);
    set(&mut app, l, json!({"animation": true}));
    assert!(x(&app, 15) > 1.0);
    // forge_anim::scene_at (render side) agrees
    set(&mut app, l, json!({"animation": false}));
    let scene = forge_anim::scene_at(app.doc.scene(), 0.5);
    assert_eq!(
        scene.param(c, "transform.pos"),
        app.doc.scene().param(c, "transform.pos")
    );
}

// --- generators -----------------------------------------------------------------------------

#[test]
fn generators_off_is_pass_through() {
    let mut app = App::new_default();
    let (sub, cube) = app
        .doc
        .transact("c", |tx| {
            let sub = tx.create("forge.subdivision", None, None)?;
            let cube = tx.create("forge.cube", Some(sub), None)?;
            Ok((sub, cube))
        })
        .unwrap();
    let l = layer(&mut app, "Gens");
    assign(&mut app, Some(l), &[sub]);
    let f = app.scheduled(0);
    assert_eq!(f.drawn, vec![sub], "the input is consumed");
    let smooth = points(&f, sub).expect("subdivision has geometry");
    set(&mut app, l, json!({"generators": false}));
    let f = app.scheduled(0);
    assert_eq!(f.drawn, vec![sub, cube], "the input is drawn as itself");
    assert!(f.outputs[&sub].mesh.is_none(), "no geometry of its own");
    assert!(f.outputs[&cube].mesh.is_some());
    assert_eq!(frame_bits(&f), fresh(&app, 0));
    set(&mut app, l, json!({"generators": true}));
    let f = app.scheduled(0);
    assert_eq!(f.drawn, vec![sub]);
    assert_eq!(points(&f, sub), Some(smooth));
}

#[test]
fn generators_off_leaves_other_categories_alone() {
    let mut app = App::new_default();
    let n = make(&mut app, "forge.null");
    let l = layer(&mut app, "L");
    assign(&mut app, Some(l), &[n]);
    set(&mut app, l, json!({"generators": false}));
    assert!(forge_core::layers::generators_on(app.doc.scene(), n));
}

// --- deformers ------------------------------------------------------------------------------

#[test]
fn deformers_off_passes_the_points_through() {
    let mut app = App::new_default();
    let (cube, bend) = app
        .doc
        .transact("c", |tx| {
            let cube = tx.create("forge.cube", None, None)?;
            let bend = tx.create("forge.bend", Some(cube), None)?;
            tx.set_param(bend, "size", Value::Vec3([400.0, 400.0, 400.0]))?;
            tx.set_param(bend, "strength", Value::Float(1.0))?;
            Ok((cube, bend))
        })
        .unwrap();
    let plain = {
        let mut other = App::new_default();
        let c = make(&mut other, "forge.cube");
        points(&other.scheduled(0), c).unwrap()
    };
    let l = layer(&mut app, "Defs");
    assign(&mut app, Some(l), &[bend]);
    let bent = points(&app.scheduled(0), cube).unwrap();
    assert_ne!(bent, plain, "the bend deforms by default");
    set(&mut app, l, json!({"deformers": false}));
    let f = app.scheduled(0);
    assert_eq!(points(&f, cube).unwrap(), plain);
    assert_eq!(frame_bits(&f), fresh(&app, 0));
    set(&mut app, l, json!({"deformers": true}));
    assert_eq!(points(&app.scheduled(0), cube).unwrap(), bent);
}

// --- expressions ----------------------------------------------------------------------------

#[test]
fn expressions_off_stops_the_tags_of_the_members() {
    let mut app = App::new_default();
    let c = make(&mut app, "forge.cube");
    app.selection.click(c);
    let tag = make(&mut app, forge_nodes::TAG);
    let add = |app: &mut App, ty: &str, values: Json| {
        app.run(
            "graph.add_node",
            json!({"tag": tag.0, "type": ty, "values": values}),
        )
        .unwrap()["node"]
            .as_u64()
            .unwrap()
    };
    let t = add(&mut app, "time.now", json!({}));
    let v = add(&mut app, "vector.compose", json!({}));
    let w = add(
        &mut app,
        "object.set_param",
        json!({"param": "transform.pos"}),
    );
    for (f, fp, to, tp) in [(t, "seconds", v, "x"), (v, "out", w, "value")] {
        app.run(
            "graph.connect",
            json!({"tag": tag.0, "from_node": f, "from_port": fp, "to_node": to, "to_port": tp}),
        )
        .unwrap();
    }
    let l = layer(&mut app, "Expr");
    assign(&mut app, Some(l), &[c]);
    let x = |app: &App, frame| match app
        .eval
        .get(app.doc.scene(), frame)
        .param(c, "transform.pos")
    {
        Some(Value::Vec3(v)) => v[0],
        other => panic!("{other:?}"),
    };
    assert!(x(&app, 15) > 0.0, "the tag moves the cube");
    set(&mut app, l, json!({"expressions": false}));
    assert_eq!(x(&app, 15), 0.0);
    let mut s = Scheduler::new(Threads::Serial).with_validation(true);
    s.evaluate(app.doc.scene(), 0);
    let f = s.evaluate(app.doc.scene(), 15);
    assert_eq!(
        f.scene.param(c, "transform.pos"),
        app.doc.scene().param(c, "transform.pos")
    );
}

// --- locked and manager have no evaluation effect -------------------------------------------

#[test]
fn lock_and_manager_do_not_touch_evaluation() {
    let mut app = App::new_default();
    let a = make(&mut app, "forge.cube");
    let b = make(&mut app, "forge.sphere");
    let l = layer(&mut app, "L");
    assign(&mut app, Some(l), &[a]);
    let mut s = Scheduler::new(Threads::Serial).with_validation(true);
    s.evaluate(app.doc.scene(), 0);
    for args in [
        json!({"locked": true}),
        json!({"manager": false}),
        json!({"name": "Renamed", "color": [0.0, 0.0, 0.0]}),
    ] {
        set(&mut app, l, args);
        let f = s.evaluate(app.doc.scene(), 0);
        assert_eq!(f.stats.path, Found::Diff);
        assert!(f.stats.evaluated.is_empty(), "{:?}", f.stats.evaluated);
        assert_eq!(f.stats.reused, 2);
    }
    let _ = b;
}

// --- dirtiness ------------------------------------------------------------------------------

#[test]
fn toggles_dirty_exactly_the_members_and_match_a_full_evaluation() {
    let mut app = App::new_default();
    let moving = animated_cube(&mut app);
    let still: Vec<NodeId> = (0..3).map(|_| make(&mut app, "forge.sphere")).collect();
    let (sub, input) = app
        .doc
        .transact("c", |tx| {
            let sub = tx.create("forge.subdivision", None, None)?;
            let input = tx.create("forge.cube", Some(sub), None)?;
            Ok((sub, input))
        })
        .unwrap();
    let (l_anim, l_gen) = (layer(&mut app, "Anim"), layer(&mut app, "Gen"));
    assign(&mut app, Some(l_anim), &[moving]);
    assign(&mut app, Some(l_gen), &[sub]);
    let mut s = Scheduler::new(Threads::Fixed(2)).with_validation(true);
    let check = |s: &mut Scheduler, app: &App, frame| -> std::sync::Arc<Frame> {
        let f = s.evaluate(app.doc.scene(), frame);
        let full = Evaluated::default().get(app.doc.scene(), frame);
        assert_eq!(f.scene.revision() > 0, full.revision() > 0);
        assert_eq!(frame_bits(&f), fresh(app, frame), "frame {frame}");
        f
    };
    check(&mut s, &app, 0);
    check(&mut s, &app, 15);

    // animation off: the moving cube only, and only its geometry is recomputed
    set(&mut app, l_anim, json!({"animation": false}));
    let f = check(&mut s, &app, 15);
    assert_eq!(f.stats.evaluated, vec![moving]);
    assert_eq!(f.stats.reused, f.drawn.len() - 1);
    // while the layer is off, time moving evaluates nothing
    let f = check(&mut s, &app, 20);
    assert!(f.stats.evaluated.is_empty(), "{:?}", f.stats.evaluated);
    set(&mut app, l_anim, json!({"animation": true}));
    let f = check(&mut s, &app, 20);
    assert_eq!(f.stats.evaluated, vec![moving]);

    // generators off: the subdivision and its (now drawn) input, no sphere
    set(&mut app, l_gen, json!({"generators": false}));
    let f = check(&mut s, &app, 20);
    assert_eq!(f.stats.evaluated, vec![sub, input]);
    assert!(still.iter().all(|id| !f.stats.evaluated.contains(id)));
    set(&mut app, l_gen, json!({"generators": true}));
    let f = check(&mut s, &app, 20);
    assert_eq!(f.stats.evaluated, vec![sub]);

    // moving an object to another layer changes its effect
    assign(&mut app, Some(l_anim), &[still[0]]);
    set(&mut app, l_anim, json!({"view": false}));
    let f = check(&mut s, &app, 20);
    assert!(!f.drawn.contains(&moving) && !f.drawn.contains(&still[0]));
    // undo walks back through the same states
    for _ in 0..3 {
        assert!(app.doc.undo());
        check(&mut s, &app, 20);
    }
}

#[test]
fn random_layer_edits_match_a_fresh_evaluation() {
    use forge_clone_engine::SplitMix64;
    let mut app = App::new_default();
    let mut nodes = vec![animated_cube(&mut app)];
    for ty in ["forge.sphere", "forge.cone", "forge.cylinder"] {
        nodes.push(make(&mut app, ty));
    }
    let (sub, input, cube2, bend) = app
        .doc
        .transact("c", |tx| {
            let sub = tx.create("forge.subdivision", None, None)?;
            let input = tx.create("forge.cube", Some(sub), None)?;
            let cube2 = tx.create("forge.cube", None, None)?;
            let bend = tx.create("forge.bend", Some(cube2), None)?;
            Ok((sub, input, cube2, bend))
        })
        .unwrap();
    nodes.extend([sub, input, cube2, bend]);
    let layers: Vec<NodeId> = (0..3).map(|i| layer(&mut app, &format!("L{i}"))).collect();
    let mut rng = SplitMix64::new(0x1a7e5);
    let toggles = [
        "solo",
        "view",
        "render",
        "manager",
        "locked",
        "animation",
        "generators",
        "deformers",
        "expressions",
    ];
    let mut s = Scheduler::new(Threads::Fixed(2)).with_validation(true);
    for step in 0..60 {
        let frame = (rng.next_u64() % 31) as i64;
        match rng.next_u64() % 3 {
            0 => {
                let n = nodes[(rng.next_u64() % nodes.len() as u64) as usize];
                let l = layers[(rng.next_u64() % layers.len() as u64) as usize];
                let target = (!rng.next_u64().is_multiple_of(4)).then_some(l);
                assign(&mut app, target, &[n]);
            }
            _ => {
                let l = layers[(rng.next_u64() % layers.len() as u64) as usize];
                let t = toggles[(rng.next_u64() % toggles.len() as u64) as usize];
                let on = rng.next_u64().is_multiple_of(2);
                set(&mut app, l, json!({ t: on }));
            }
        }
        let f = s.evaluate(app.doc.scene(), frame);
        assert_eq!(frame_bits(&f), fresh(&app, frame), "step {step}");
    }
}
