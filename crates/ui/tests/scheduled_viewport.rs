//! The viewport draws from the evaluation scheduler (docs/specs/VIEWPORT.md §10.8,
//! CORE.md section 5b): the draw items equal those of a from-scratch evaluation, and a frame
//! of playback rebuilds only the animated nodes.

use std::collections::BTreeSet;
use std::sync::Arc;

use forge_core::{EvalCtx, Key, NodeId, Value};
use forge_ui::components::EditMode;
use forge_ui::state::EditorState;
use forge_ui::viewport_panel::{DrawItem, SlotCache, scene_draw_items_at, scheduled_draw_items};

fn state() -> EditorState {
    EditorState::new(Arc::new(forge_app::default_registry().unwrap()))
}

struct Setup {
    cloner: NodeId,
    moving: NodeId,
    statics: Vec<NodeId>,
}

/// An animated cube, a cloner whose prototype is animated too, and static cubes.
fn setup(st: &mut EditorState) -> Setup {
    st.doc
        .transact("setup", |tx| {
            let moving = tx.create("forge.cube", None, None)?;
            tx.set_key(moving, "transform.pos.x", Key::new(0.0, 0.0))?;
            tx.set_key(moving, "transform.pos.x", Key::new(2.0, 400.0))?;
            let cloner = tx.create("forge.cloner", None, None)?;
            tx.set_param(cloner, "mode", Value::Enum("linear".into()))?;
            tx.set_param(cloner, "linear_count", Value::Int(12))?;
            tx.set_param(cloner, "transform_position", Value::Vec3([300.0, 0.0, 0.0]))?;
            let proto = tx.create("forge.cube", Some(cloner), None)?;
            tx.set_key(proto, "size_x", Key::new(0.0, 50.0))?;
            tx.set_key(proto, "size_x", Key::new(2.0, 150.0))?;
            let mut statics = Vec::new();
            for i in 0..40 {
                let c = tx.create("forge.cube", None, None)?;
                tx.set_param(
                    c,
                    "transform.pos",
                    Value::Vec3([i as f64 * 250.0, 0.0, -1000.0]),
                )?;
                statics.push(c);
            }
            Ok(Setup {
                cloner,
                moving,
                statics,
            })
        })
        .unwrap()
}

fn describe(items: &[DrawItem]) -> Vec<String> {
    items
        .iter()
        .map(|i| match i {
            DrawItem::Mesh {
                id,
                mesh,
                selected,
                smoothing,
            } => format!(
                "mesh {id} {selected} {smoothing:?} {:?} {:?}",
                mesh.points, mesh.polys
            ),
            DrawItem::Instanced {
                id,
                instances,
                world,
                selected,
                smoothing,
            } => format!(
                "inst {id} {selected} {smoothing:?} {world:?} {:?} {:?} {:?} {:?} {:?}",
                instances.prototypes,
                instances.clones.matrices,
                instances.clones.colors,
                instances.clones.visible,
                instances.clones.child
            ),
        })
        .collect()
}

#[test]
fn scheduled_draw_items_equal_a_from_scratch_evaluation_at_every_frame() {
    let mut st = state();
    let s = setup(&mut st);
    let selected: BTreeSet<NodeId> = [s.moving, s.statics[3]].into_iter().collect();
    // forwards, backwards, jumps and a repeat: each pass reuses the last one
    for frame in [0, 1, 2, 30, 31, 15, 60, 60, 0, 90, 45] {
        let want = {
            let scene = st.doc.scene();
            let evaluated = st.app.eval.get(scene, frame);
            let ctx = EvalCtx::at_frame(frame, f64::from(scene.settings().fps));
            describe(&scene_draw_items_at(&evaluated, &selected, &ctx))
        };
        let pass = st.app.scheduled_draw(frame);
        let got = describe(&scheduled_draw_items(&pass, &selected));
        assert_eq!(got, want, "frame {frame}");
    }
    // the cloner is drawn as instances
    let pass = st.app.scheduled_draw(10);
    assert!(pass.outputs[&s.cloner].instances.is_some());
}

#[test]
fn a_frame_of_playback_rebuilds_only_the_animated_nodes() {
    let mut st = state();
    let s = setup(&mut st);
    let none = BTreeSet::new();
    let mut cache: SlotCache<NodeId> = SlotCache::default();
    let id_of = |i: DrawItem| i.id();
    let pass = st.app.scheduled_draw(0);
    let total = pass.drawn.len();
    assert_eq!(total, 2 + 40);
    cache.refresh(&pass, &none, None, false, id_of);
    assert_eq!(cache.rebuilt, total, "the first pass builds everything");

    let pass = st.app.scheduled_draw(1);
    cache.refresh(&pass, &none, None, false, id_of);
    assert_eq!(cache.rebuilt, 2, "the moving cube and the cloner");
    assert_eq!(cache.items().len(), total);

    // asking again for the same pass does nothing
    let again = st.app.scheduled_draw(1);
    cache.refresh(&again, &none, None, false, id_of);
    assert_eq!(cache.rebuilt, 0);

    // selecting a static cube rebuilds just it; so does deselecting it
    let sel: BTreeSet<NodeId> = [s.statics[7]].into_iter().collect();
    cache.refresh(&again, &sel, None, false, id_of);
    assert_eq!(cache.rebuilt, 1);
    cache.refresh(&again, &none, None, false, id_of);
    assert_eq!(cache.rebuilt, 1);

    // a component target is rebuilt with every pass; the others are kept
    let target = Some((s.statics[0], EditMode::Points));
    cache.refresh(&again, &none, target, false, id_of);
    assert_eq!(cache.rebuilt, 1);
    let pass = st.app.scheduled_draw(2);
    cache.refresh(&pass, &none, target, false, id_of);
    assert_eq!(cache.rebuilt, 3, "the target, the moving cube, the cloner");

    // an edit of a static cube rebuilds only it
    st.doc
        .transact("e", |tx| {
            tx.set_param(s.statics[9], "size_x", Value::Float(33.0))
        })
        .unwrap();
    let pass = st.app.scheduled_draw(2);
    cache.refresh(&pass, &none, None, false, id_of);
    assert_eq!(cache.rebuilt, 2, "the edited cube and the old target");
}
