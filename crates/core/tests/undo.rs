//! docs/specs/CORE.md §4: history.

mod common;

use forge_core::{Document, EditError, NodeId, Value, file};

fn snap(doc: &Document) -> String {
    file::save(doc.scene())
}

/// A scene touching every edit kind; returns (snapshots after each step, ids).
fn scripted(doc: &mut Document) -> Vec<String> {
    let mut snaps = vec![snap(doc)];
    let p = doc
        .transact("create", |tx| tx.create("forge.null", None, None))
        .unwrap();
    snaps.push(snap(doc));
    let b = doc
        .transact("child", |tx| tx.create("test.box", Some(p), None))
        .unwrap();
    snaps.push(snap(doc));
    doc.transact("tag", |tx| tx.attach_tag("test.sel", b, None))
        .unwrap();
    snaps.push(snap(doc));
    doc.transact("param", |tx| tx.set_param(b, "size", Value::Float(5.0)))
        .unwrap();
    snaps.push(snap(doc));
    doc.transact("link", |tx| tx.set_param(b, "target", Value::Link(Some(p))))
        .unwrap();
    snaps.push(snap(doc));
    doc.transact("rename", |tx| tx.rename(b, "Crate")).unwrap();
    snaps.push(snap(doc));
    doc.transact("move", |tx| tx.move_node(b, None, Some(0)))
        .unwrap();
    snaps.push(snap(doc));
    doc.transact("delete", |tx| tx.delete(b)).unwrap();
    snaps.push(snap(doc));
    snaps
}

#[test]
fn undo_and_redo_walk_every_state_exactly() {
    let mut doc = common::doc();
    let snaps = scripted(&mut doc);
    assert_eq!(
        doc.history().undo_labels(),
        [
            "create", "child", "tag", "param", "link", "rename", "move", "delete"
        ]
    );
    for want in snaps.iter().rev().skip(1) {
        assert!(doc.undo());
        assert_eq!(&snap(&doc), want);
    }
    assert!(!doc.undo());
    for want in snaps.iter().skip(1) {
        assert!(doc.redo());
        assert_eq!(&snap(&doc), want);
    }
    assert!(!doc.redo());
}

#[test]
fn undo_restores_original_ids() {
    let mut doc = common::doc();
    let p = doc
        .transact("p", |tx| tx.create("forge.null", None, None))
        .unwrap();
    let c = doc
        .transact("c", |tx| tx.create("forge.null", Some(p), None))
        .unwrap();
    doc.transact("del", |tx| tx.delete(p)).unwrap();
    doc.undo();
    assert!(doc.scene().contains(p) && doc.scene().contains(c));
    assert_eq!(doc.scene().node(p).unwrap().children, vec![c]);
    // a new node after undo still gets a fresh id
    let n = doc
        .transact("n", |tx| tx.create("forge.null", None, None))
        .unwrap();
    assert!(n != p && n != c);
}

#[test]
fn new_transaction_clears_redo() {
    let mut doc = common::doc();
    doc.transact("a", |tx| tx.create("forge.null", None, None))
        .unwrap();
    doc.undo();
    assert_eq!(doc.history().redo_labels(), ["a"]);
    doc.transact("b", |tx| tx.create("forge.null", None, None))
        .unwrap();
    assert!(doc.history().redo_labels().is_empty());
    assert!(!doc.redo());
}

#[test]
fn empty_and_failed_transactions_are_not_recorded() {
    let mut doc = common::doc();
    doc.transact("nothing", |_tx| Ok(())).unwrap();
    let r: Result<NodeId, EditError> = doc.transact("fail", |tx| tx.create("nope.x", None, None));
    assert!(r.is_err());
    assert!(doc.history().undo_labels().is_empty());
    assert!(!doc.undo());
}

#[test]
fn merged_transactions_form_one_undo_step() {
    let mut doc = common::doc();
    let b = doc
        .transact("add", |tx| tx.create("test.box", None, None))
        .unwrap();
    let before = snap(&doc);
    for v in [1.0, 2.0, 3.0] {
        doc.transact_merged("Drag size", "drag:size", |tx| {
            tx.set_param(b, "size", Value::Float(v))
        })
        .unwrap();
    }
    assert_eq!(doc.history().undo_labels(), ["add", "Drag size"]);
    assert_eq!(doc.scene().param(b, "size"), Some(&Value::Float(3.0)));
    doc.undo();
    assert_eq!(snap(&doc), before);
    doc.redo();
    assert_eq!(doc.scene().param(b, "size"), Some(&Value::Float(3.0)));
}

#[test]
fn merge_window_ends_on_other_actions() {
    let mut doc = common::doc();
    let b = doc
        .transact("add", |tx| tx.create("test.box", None, None))
        .unwrap();
    let drag = |doc: &mut Document, v: f64| {
        doc.transact_merged("drag", "k", move |tx| {
            tx.set_param(b, "size", Value::Float(v))
        })
        .unwrap()
    };
    drag(&mut doc, 1.0);
    doc.transact("rename", |tx| tx.rename(b, "x")).unwrap();
    drag(&mut doc, 2.0);
    assert_eq!(
        doc.history().undo_labels(),
        ["add", "drag", "rename", "drag"]
    );

    // undo also ends the window: the next drag is a new step
    doc.undo();
    drag(&mut doc, 4.0);
    assert_eq!(
        doc.history().undo_labels(),
        ["add", "drag", "rename", "drag"]
    );
    drag(&mut doc, 5.0);
    assert_eq!(doc.history().undo_labels().len(), 4);

    // different key does not merge
    doc.transact_merged("other", "k2", |tx| {
        tx.set_param(b, "size", Value::Float(6.0))
    })
    .unwrap();
    assert_eq!(doc.history().undo_labels().len(), 5);
}
