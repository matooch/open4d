//! docs/specs/CORE.md §7: layers (brief 158): table edits and undo, resolution, link
//! validation, delete and merge, file round trip, dependency diff.

mod common;

use forge_core::depgraph::{Vertex, diff};
use forge_core::file;
use forge_core::layers::{self, LAYER};
use forge_core::visibility;
use forge_core::{Document, EditError, NodeId, Value};

fn doc() -> Document {
    Document::new(common::registry())
}

/// Three boxes a, b, c at the root.
fn three() -> (Document, [NodeId; 3]) {
    let mut d = doc();
    let ids = d
        .transact("build", |tx| {
            let mut out = [NodeId(0); 3];
            for (i, name) in ["a", "b", "c"].iter().enumerate() {
                out[i] = tx.create("test.box", None, None)?;
                tx.rename(out[i], name)?;
            }
            Ok(out)
        })
        .unwrap();
    (d, ids)
}

fn assign(d: &mut Document, node: NodeId, layer: Option<NodeId>) {
    d.transact("assign", |tx| tx.set_param(node, LAYER, Value::Link(layer)))
        .unwrap();
}

fn edit(d: &mut Document, id: NodeId, f: impl FnOnce(&mut forge_core::Layer)) {
    let mut l = d.scene().layers().get(id).unwrap().clone();
    f(&mut l);
    d.transact("edit layer", |tx| tx.update_layer(l)).unwrap();
}

#[test]
fn a_new_layer_has_the_defaults_and_ids_never_equal_node_ids() {
    let (mut d, ids) = three();
    let l = d.transact("new", |tx| tx.add_layer(None, None)).unwrap();
    let layer = d.scene().layers().get(l).unwrap();
    assert_eq!(layer.name, "Layer 1");
    assert!(!layer.solo && !layer.locked);
    assert!(
        layer.view
            && layer.render
            && layer.manager
            && layer.animation
            && layer.generators
            && layer.deformers
            && layer.expressions
    );
    assert!(!ids.contains(&l) && !d.scene().contains(l));
    let l2 = d.transact("new", |tx| tx.add_layer(None, None)).unwrap();
    assert_eq!(d.scene().layers().get(l2).unwrap().name, "Layer 2");
    assert_ne!(
        d.scene().layers().get(l).unwrap().color,
        d.scene().layers().get(l2).unwrap().color
    );
    assert!(
        d.transact("bad", |tx| tx.add_layer(Some("  "), None))
            .is_err()
    );
    assert_eq!(d.scene().layers().len(), 2);
}

#[test]
fn layer_edits_are_one_undo_step_and_undo_exactly() {
    let (mut d, [a, ..]) = three();
    let before = d.scene().layers().clone();
    let steps = d.undo_depth();
    let l = d
        .transact("new", |tx| {
            tx.add_layer(Some("Walls"), Some([2.0, -1.0, 0.5]))
        })
        .unwrap();
    assert_eq!(d.undo_depth(), steps + 1);
    assert_eq!(
        d.scene().layers().get(l).unwrap().color,
        [1.0, 0.0, 0.5],
        "channels are clamped"
    );
    edit(&mut d, l, |l| l.view = false);
    assign(&mut d, a, Some(l));
    assert!(d.undo() && d.undo());
    assert!(d.scene().layers().get(l).unwrap().view);
    assert!(d.undo());
    assert_eq!(d.scene().layers(), &before);
    assert!(d.redo() && d.redo() && d.redo());
    assert!(!d.scene().layers().get(l).unwrap().view);
    assert_eq!(d.scene().param(a, LAYER), Some(&Value::Link(Some(l))));
}

#[test]
fn the_layer_link_accepts_only_layers() {
    let (mut d, [a, b, _]) = three();
    let l = d.transact("new", |tx| tx.add_layer(None, None)).unwrap();
    let e = d
        .transact("bad", |tx| tx.set_param(a, LAYER, Value::Link(Some(b))))
        .unwrap_err();
    assert_eq!(e, EditError::UnknownLayer(b));
    let e = d
        .transact("bad", |tx| {
            tx.set_param(a, LAYER, Value::Link(Some(NodeId(999))))
        })
        .unwrap_err();
    assert_eq!(e, EditError::UnknownLayer(NodeId(999)));
    assign(&mut d, a, Some(l));
    assert_eq!(layers::layer_id(d.scene(), a), Some(l));
    assign(&mut d, a, None);
    assert_eq!(layers::layer_id(d.scene(), a), None);
}

#[test]
fn the_param_is_hidden_and_in_the_object_group() {
    let (d, [a, ..]) = three();
    let ty = d.scene().node_type(a).unwrap();
    let p = ty.find_param(LAYER).expect("every hierarchy type has it");
    assert!(p.ui.hidden);
    assert_eq!(p.default, Value::Link(None));
    assert_eq!(visibility::LAYER, LAYER);
}

#[test]
fn a_tag_belongs_to_its_hosts_layer() {
    let (mut d, [a, ..]) = three();
    let l = d.transact("new", |tx| tx.add_layer(None, None)).unwrap();
    let tag = d
        .transact("tag", |tx| tx.attach_tag("test.sel", a, None))
        .unwrap();
    assign(&mut d, a, Some(l));
    assert_eq!(layers::layer_id(d.scene(), tag), Some(l));
    edit(&mut d, l, |l| l.expressions = false);
    assert!(!layers::expressions_on(d.scene(), tag));
    assert!(layers::members(d.scene(), l) == vec![a]);
}

#[test]
fn view_and_solo_resolve_together_with_editor_visibility() {
    let (mut d, [a, b, c]) = three();
    let (l1, l2) = d
        .transact("new", |tx| {
            Ok((tx.add_layer(None, None)?, tx.add_layer(None, None)?))
        })
        .unwrap();
    assign(&mut d, a, Some(l1));
    assign(&mut d, b, Some(l2));
    let shown = |d: &Document| [a, b, c].map(|n| visibility::editor_visible(d.scene(), n));
    assert_eq!(shown(&d), [true; 3]);
    // view off hides the members only
    edit(&mut d, l1, |l| l.view = false);
    assert_eq!(shown(&d), [false, true, true]);
    // solo: only soloed layers (with view on) show; the unassigned c hides
    edit(&mut d, l2, |l| l.solo = true);
    assert_eq!(shown(&d), [false, true, false]);
    // solo does not override view
    edit(&mut d, l2, |l| l.view = false);
    assert_eq!(shown(&d), [false, false, false]);
    edit(&mut d, l2, |l| l.view = true);
    edit(&mut d, l1, |l| (l.view, l.solo) = (true, true));
    assert_eq!(shown(&d), [true, true, false]);
    // the object's own state still applies
    d.transact("off", |tx| {
        tx.set_param(a, visibility::ENABLED, Value::Bool(false))
    })
    .unwrap();
    assert_eq!(shown(&d), [false, true, false]);
}

#[test]
fn render_flag_affects_render_visibility_only() {
    let (mut d, [a, ..]) = three();
    let l = d.transact("new", |tx| tx.add_layer(None, None)).unwrap();
    assign(&mut d, a, Some(l));
    edit(&mut d, l, |l| l.render = false);
    assert!(visibility::editor_visible(d.scene(), a));
    assert!(!visibility::render_visible(d.scene(), a));
    // solo is a viewport matter
    edit(&mut d, l, |l| (l.render, l.solo) = (true, true));
    assert!(visibility::render_visible(d.scene(), a));
}

#[test]
fn flags_resolve_per_node_without_inheritance() {
    let (mut d, [a, b, _]) = three();
    let l = d.transact("new", |tx| tx.add_layer(None, None)).unwrap();
    d.transact("nest", |tx| tx.move_node(b, Some(a), None))
        .unwrap();
    assign(&mut d, a, Some(l));
    edit(&mut d, l, |l| {
        l.locked = true;
        l.manager = false;
        l.animation = false;
        l.deformers = false;
    });
    let s = d.scene();
    assert!(layers::locked(s, a) && !layers::locked(s, b));
    assert!(!layers::in_manager(s, a) && layers::in_manager(s, b));
    assert!(!layers::animation_on(s, a) && layers::animation_on(s, b));
    assert!(!layers::deformers_on(s, a) && layers::deformers_on(s, b));
    // test.box is a generator
    assert!(layers::generators_on(s, a));
    edit(&mut d, l, |l| l.generators = false);
    assert!(!layers::generators_on(d.scene(), a));
}

#[test]
fn deleting_a_layer_releases_its_members_in_the_same_step() {
    let (mut d, [a, b, c]) = three();
    let (l1, l2) = d
        .transact("new", |tx| {
            Ok((tx.add_layer(None, None)?, tx.add_layer(None, None)?))
        })
        .unwrap();
    assign(&mut d, a, Some(l1));
    assign(&mut d, b, Some(l1));
    assign(&mut d, c, Some(l2));
    let steps = d.undo_depth();
    d.transact("delete", |tx| tx.remove_layer(l1)).unwrap();
    assert_eq!(d.undo_depth(), steps + 1);
    assert!(!d.scene().layers().contains(l1));
    assert_eq!(d.scene().param(a, LAYER), Some(&Value::Link(None)));
    assert_eq!(d.scene().param(c, LAYER), Some(&Value::Link(Some(l2))));
    assert!(d.undo());
    assert_eq!(d.scene().param(b, LAYER), Some(&Value::Link(Some(l1))));
    assert!(d.scene().layers().contains(l1));
    assert_eq!(
        d.transact("delete", |tx| tx.remove_layer(NodeId(999)))
            .unwrap_err(),
        EditError::UnknownLayer(NodeId(999))
    );
}

#[test]
fn merging_moves_the_members_and_removes_the_other_layers() {
    let (mut d, [a, b, c]) = three();
    let ls = d
        .transact("new", |tx| {
            Ok([
                tx.add_layer(None, None)?,
                tx.add_layer(None, None)?,
                tx.add_layer(None, None)?,
            ])
        })
        .unwrap();
    assign(&mut d, a, Some(ls[0]));
    assign(&mut d, b, Some(ls[1]));
    assign(&mut d, c, Some(ls[2]));
    let moved = d
        .transact("merge", |tx| tx.merge_layers(&[ls[0], ls[1]], ls[0]))
        .unwrap();
    assert_eq!(moved, 1);
    assert_eq!(d.scene().layers().len(), 2);
    assert_eq!(d.scene().param(b, LAYER), Some(&Value::Link(Some(ls[0]))));
    assert_eq!(d.scene().param(c, LAYER), Some(&Value::Link(Some(ls[2]))));
    assert!(d.undo());
    assert_eq!(d.scene().layers().len(), 3);
    assert_eq!(d.scene().param(b, LAYER), Some(&Value::Link(Some(ls[1]))));
    assert!(
        d.transact("bad", |tx| tx.merge_layers(&[ls[0]], NodeId(999)))
            .is_err()
    );
}

#[test]
fn layers_and_membership_survive_save_and_load() {
    let (mut d, [a, ..]) = three();
    let l = d
        .transact("new", |tx| {
            tx.add_layer(Some("Props"), Some([0.1, 0.2, 0.3]))
        })
        .unwrap();
    edit(&mut d, l, |l| {
        l.solo = true;
        l.locked = true;
        l.generators = false;
    });
    assign(&mut d, a, Some(l));
    let text = file::save(d.scene());
    let (loaded, report) = file::load(common::registry(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(loaded.layers(), d.scene().layers());
    assert_eq!(loaded.param(a, LAYER), Some(&Value::Link(Some(l))));
    assert_eq!(file::save(&loaded), text);
    // the id counter is above the layer ids
    let mut d2 = Document::from_scene(loaded);
    let n = d2
        .transact("n", |tx| tx.create("test.box", None, None))
        .unwrap();
    assert!(n.0 > l.0);
}

#[test]
fn a_scene_without_layers_writes_no_layer_block() {
    let (d, _) = three();
    assert!(!file::save(d.scene()).contains("layers:"));
}

#[test]
fn bad_layer_data_loads_with_warnings() {
    let text = r#"(format: "forge3d-scene", version: 9,
        layers: [
            (id: 5, name: "ok", color: (0.5, 0.5, 0.5)),
            (id: 5, name: "twice", color: (0.5, 0.5, 0.5)),
            (id: 1, name: "clash", color: (0.5, 0.5, 0.5)),
            (id: 6, name: "  ", color: (3.0, -1.0, 0.5)),
        ],
        roots: [
            (id: 1, type_id: "forge.null", name: "n", params: {"object.layer": Link(Some(9))}, tags: [], children: []),
            (id: 2, type_id: "forge.null", name: "m", params: {"object.layer": Link(Some(5))}, tags: [], children: []),
        ])"#;
    let (scene, report) = file::load(common::registry(), text).unwrap();
    assert_eq!(scene.layers().len(), 2, "{:?}", report.warnings);
    assert_eq!(report.warnings.len(), 3, "{:?}", report.warnings);
    let named = scene.layers().get(NodeId(6)).unwrap();
    assert_eq!(named.name, "Layer 6");
    assert_eq!(named.color, [1.0, 0.0, 0.5]);
    assert_eq!(scene.param(NodeId(1), LAYER), Some(&Value::Link(None)));
    assert_eq!(
        scene.param(NodeId(2), LAYER),
        Some(&Value::Link(Some(NodeId(5))))
    );
    assert!(
        scene.layers().get(NodeId(5)).unwrap().view,
        "serde defaults"
    );
}

#[test]
fn the_diff_seeds_only_nodes_whose_layer_effect_changed() {
    let (mut d, [a, b, c]) = three();
    let (l1, l2) = d
        .transact("new", |tx| {
            Ok((tx.add_layer(None, None)?, tx.add_layer(None, None)?))
        })
        .unwrap();
    assign(&mut d, a, Some(l1));
    assign(&mut d, b, Some(l2));
    let before = d.scene().clone();
    // a colour or a rename changes no effect
    edit(&mut d, l1, |l| {
        l.name = "Renamed".into();
        l.color = [0.0, 0.0, 0.0];
    });
    assert!(diff(&before, d.scene()).is_empty());
    // a lock changes none either
    let before = d.scene().clone();
    edit(&mut d, l1, |l| l.locked = true);
    assert!(diff(&before, d.scene()).is_empty());
    // animation off on l1 seeds a, not b or c
    let before = d.scene().clone();
    edit(&mut d, l1, |l| l.animation = false);
    let seeds = diff(&before, d.scene()).seeds;
    assert!(seeds.contains(&Vertex::result(a)) && seeds.contains(&Vertex::frame(a)));
    assert!(!seeds.iter().any(|v| v.node == b || v.node == c));
    // solo on l2 changes what is shown for every node (a and c hide)
    let before = d.scene().clone();
    edit(&mut d, l2, |l| l.solo = true);
    let nodes: std::collections::BTreeSet<NodeId> = diff(&before, d.scene())
        .seeds
        .iter()
        .map(|v| v.node)
        .collect();
    assert_eq!(nodes, [a, c].into_iter().collect());
    // moving a node to another layer seeds it
    let before = d.scene().clone();
    assign(&mut d, c, Some(l2));
    assert!(diff(&before, d.scene()).seeds.contains(&Vertex::result(c)));
}
