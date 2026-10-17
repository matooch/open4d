//! docs/specs/CORE.md §3–4: scene graph and edits.

mod common;

use forge_core::{Document, EditError, NodeId, ParamError, Value, file};

fn names(doc: &Document, ids: &[NodeId]) -> Vec<String> {
    ids.iter()
        .map(|id| doc.scene().node(*id).unwrap().name.clone())
        .collect()
}

fn root_names(doc: &Document) -> Vec<String> {
    names(doc, doc.scene().roots())
}

/// Creates a root node of `ty` named `name`.
fn add(doc: &mut Document, ty: &str, name: &str) -> NodeId {
    let name = name.to_string();
    let ty = ty.to_string();
    doc.transact("add", move |tx| {
        let id = tx.create(&ty, None, None)?;
        tx.rename(id, &name)?;
        Ok(id)
    })
    .unwrap()
}

#[test]
fn create_fills_defaults_and_label() {
    let mut doc = common::doc();
    let id = doc
        .transact("Add", |tx| tx.create("test.box", None, None))
        .unwrap();
    let n = doc.scene().node(id).unwrap();
    assert_eq!(n.name, "Box");
    assert_eq!(n.type_id, "test.box");
    assert_eq!(n.params.len(), 8);
    assert_eq!(doc.scene().param(id, "size"), Some(&Value::Float(100.0)));
    assert_eq!(
        doc.scene().param(id, "mode"),
        Some(&Value::Enum("solid".into()))
    );
    assert_eq!(doc.scene().roots(), &[id]);
    assert_eq!(n.parent, None);
}

#[test]
fn ids_are_unique_and_never_reused() {
    let mut doc = common::doc();
    let a = add(&mut doc, "forge.null", "a");
    doc.transact("del", |tx| tx.delete(a)).unwrap();
    let b = add(&mut doc, "forge.null", "b");
    assert_ne!(a, b);
    assert!(!doc.scene().contains(a));
}

#[test]
fn hierarchy_insert_order_and_index() {
    let mut doc = common::doc();
    let p = add(&mut doc, "forge.null", "p");
    let (c1, c2, c0) = doc
        .transact("kids", |tx| {
            let c1 = tx.create("forge.null", Some(p), None)?;
            let c2 = tx.create("forge.null", Some(p), None)?;
            let c0 = tx.create("test.bend", Some(p), Some(0))?;
            Ok((c1, c2, c0))
        })
        .unwrap();
    let n = doc.scene().node(p).unwrap();
    assert_eq!(n.children, vec![c0, c1, c2]);
    assert_eq!(doc.scene().node(c1).unwrap().parent, Some(p));
    assert_eq!(doc.scene().descendants(p), vec![c0, c1, c2]);
    assert_eq!(doc.scene().len(), 4);
}

#[test]
fn create_rejects_bad_requests() {
    let mut doc = common::doc();
    let p = add(&mut doc, "forge.null", "p");
    let err = |doc: &mut Document, ty: &'static str, parent, index| {
        doc.transact("x", move |tx| tx.create(ty, parent, index))
            .unwrap_err()
    };
    assert!(matches!(
        err(&mut doc, "nope.type", None, None),
        EditError::UnknownType(_)
    ));
    assert!(matches!(
        err(&mut doc, "test.mat", None, None),
        EditError::NotPlaceable(_)
    ));
    assert!(matches!(
        err(&mut doc, "test.sel", None, None),
        EditError::WrongCategory(_)
    ));
    assert!(matches!(
        err(&mut doc, "forge.null", Some(NodeId(999)), None),
        EditError::UnknownNode(_)
    ));
    assert!(matches!(
        err(&mut doc, "forge.null", Some(p), Some(1)),
        EditError::IndexOutOfRange
    ));
    assert!(matches!(
        err(&mut doc, "forge.null", None, Some(2)),
        EditError::IndexOutOfRange
    ));
    assert_eq!(doc.scene().len(), 1);
}

#[test]
fn tags_attach_to_hierarchy_nodes_only() {
    let mut doc = common::doc();
    let p = add(&mut doc, "forge.null", "p");
    let (t1, t0) = doc
        .transact("tags", |tx| {
            let t1 = tx.attach_tag("test.sel", p, None)?;
            let t0 = tx.attach_tag("test.sel", p, Some(0))?;
            Ok((t1, t0))
        })
        .unwrap();
    let n = doc.scene().node(p).unwrap();
    assert_eq!(n.tags, vec![t0, t1]);
    assert!(n.children.is_empty());
    assert_eq!(doc.scene().node(t1).unwrap().host, Some(p));
    assert_eq!(doc.scene().node(t1).unwrap().parent, None);
    assert!(!doc.scene().roots().contains(&t1));

    let e = doc
        .transact("x", |tx| tx.attach_tag("forge.null", p, None))
        .unwrap_err();
    assert!(matches!(e, EditError::WrongCategory(_)));
    let e = doc
        .transact("x", |tx| tx.attach_tag("test.sel", t0, None))
        .unwrap_err();
    assert!(matches!(e, EditError::WrongCategory(_)));
    let e = doc
        .transact("x", |tx| tx.create("forge.null", Some(t0), None))
        .unwrap_err();
    assert!(matches!(e, EditError::WrongCategory(_)));
    let e = doc
        .transact("x", |tx| tx.move_node(t0, None, None))
        .unwrap_err();
    assert!(matches!(e, EditError::WrongCategory(_)));
}

#[test]
fn delete_removes_subtree_and_tags() {
    let mut doc = common::doc();
    let keep = add(&mut doc, "forge.null", "keep");
    let p = add(&mut doc, "forge.null", "p");
    let (c, t) = doc
        .transact("build", |tx| {
            let c = tx.create("forge.null", Some(p), None)?;
            let t = tx.attach_tag("test.sel", c, None)?;
            Ok((c, t))
        })
        .unwrap();
    doc.transact("del", |tx| tx.delete(p)).unwrap();
    for id in [p, c, t] {
        assert!(!doc.scene().contains(id));
    }
    assert_eq!(doc.scene().roots(), &[keep]);
    assert_eq!(doc.scene().len(), 1);
    // deleting a tag alone
    let t2 = doc
        .transact("tag", |tx| tx.attach_tag("test.sel", keep, None))
        .unwrap();
    doc.transact("del tag", |tx| tx.delete(t2)).unwrap();
    assert!(doc.scene().node(keep).unwrap().tags.is_empty());
}

#[test]
fn set_param_validates_and_clamps() {
    let mut doc = common::doc();
    let b = add(&mut doc, "test.box", "b");
    doc.transact("set", |tx| tx.set_param(b, "segments", Value::Int(5000)))
        .unwrap();
    assert_eq!(doc.scene().param(b, "segments"), Some(&Value::Int(1000)));
    doc.transact("set", |tx| tx.set_param(b, "size", Value::Int(3)))
        .unwrap();
    assert_eq!(doc.scene().param(b, "size"), Some(&Value::Float(3.0)));

    let e = doc
        .transact("x", |tx| tx.set_param(b, "nope", Value::Int(1)))
        .unwrap_err();
    assert!(matches!(e, EditError::UnknownParam(_)));
    let e = doc
        .transact("x", |tx| tx.set_param(b, "mode", Value::Enum("zz".into())))
        .unwrap_err();
    assert_eq!(e, EditError::Param(ParamError::UnknownOption));
    let e = doc
        .transact("x", |tx| {
            tx.set_param(NodeId(77), "size", Value::Float(1.0))
        })
        .unwrap_err();
    assert!(matches!(e, EditError::UnknownNode(_)));
}

#[test]
fn links_must_target_existing_accepted_other_nodes() {
    let mut doc = common::doc();
    let b = add(&mut doc, "test.box", "b");
    let n = add(&mut doc, "forge.null", "n");
    let d = add(&mut doc, "test.bend", "d");
    let link = |doc: &mut Document, to: Option<NodeId>| {
        doc.transact("link", move |tx| tx.set_param(b, "target", Value::Link(to)))
    };
    assert_eq!(link(&mut doc, Some(n)), Ok(()));
    assert_eq!(doc.scene().param(b, "target"), Some(&Value::Link(Some(n))));
    assert!(matches!(
        link(&mut doc, Some(d)),
        Err(EditError::InvalidLink(_))
    ));
    assert!(matches!(
        link(&mut doc, Some(b)),
        Err(EditError::InvalidLink(_))
    ));
    assert!(matches!(
        link(&mut doc, Some(NodeId(500))),
        Err(EditError::InvalidLink(_))
    ));
    assert_eq!(link(&mut doc, None), Ok(()));

    // Deleting a link target leaves the link dangling, untouched.
    link(&mut doc, Some(n)).unwrap();
    doc.transact("del", |tx| tx.delete(n)).unwrap();
    assert_eq!(doc.scene().param(b, "target"), Some(&Value::Link(Some(n))));
}

#[test]
fn move_reparents_reorders_and_prevents_cycles() {
    let mut doc = common::doc();
    let a = add(&mut doc, "forge.null", "a");
    let b = add(&mut doc, "forge.null", "b");
    let c = add(&mut doc, "forge.null", "c");
    // reorder roots: move c to front
    doc.transact("mv", |tx| tx.move_node(c, None, Some(0)))
        .unwrap();
    assert_eq!(root_names(&doc), ["c", "a", "b"]);
    // index counts after removal: move c to the end
    doc.transact("mv", |tx| tx.move_node(c, None, Some(2)))
        .unwrap();
    assert_eq!(root_names(&doc), ["a", "b", "c"]);
    // reparent b under a
    doc.transact("mv", |tx| tx.move_node(b, Some(a), None))
        .unwrap();
    assert_eq!(root_names(&doc), ["a", "c"]);
    assert_eq!(doc.scene().node(b).unwrap().parent, Some(a));
    // cycles
    let e = doc
        .transact("mv", |tx| tx.move_node(a, Some(b), None))
        .unwrap_err();
    assert_eq!(e, EditError::Cycle);
    let e = doc
        .transact("mv", |tx| tx.move_node(a, Some(a), None))
        .unwrap_err();
    assert_eq!(e, EditError::Cycle);
    let e = doc
        .transact("mv", |tx| tx.move_node(c, None, Some(3)))
        .unwrap_err();
    assert_eq!(e, EditError::IndexOutOfRange);
}

#[test]
fn failed_transaction_rolls_back_everything() {
    let mut doc = common::doc();
    let b = add(&mut doc, "test.box", "b");
    let before = file::save(doc.scene());
    let undo_len = doc.history().undo_labels().len();
    let r: Result<(), EditError> = doc.transact("mixed", |tx| {
        tx.set_param(b, "size", Value::Float(7.0))?;
        let n = tx.create("forge.null", Some(b), None)?;
        tx.rename(n, "temp")?;
        tx.attach_tag("test.sel", n, None)?;
        tx.delete(b)?;
        tx.create("nope.type", None, None)?;
        Ok(())
    });
    assert!(r.is_err());
    assert_eq!(file::save(doc.scene()), before);
    assert_eq!(doc.history().undo_labels().len(), undo_len);
}

#[test]
fn tx_sees_its_own_edits() {
    let mut doc = common::doc();
    doc.transact("x", |tx| {
        let id = tx.create("test.box", None, None)?;
        tx.set_param(id, "size", Value::Float(2.0))?;
        assert_eq!(tx.scene().param(id, "size"), Some(&Value::Float(2.0)));
        Ok(())
    })
    .unwrap();
}
