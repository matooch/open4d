//! docs/specs/CORE.md §5: dirty tracking and evaluation cache.

mod common;

use std::cell::Cell;

use forge_core::{Document, EvalCache, NodeId, Value};

struct Fixture {
    doc: Document,
    gen_: NodeId,
    child: NodeId,
    tag: NodeId,
    linked: NodeId,
    other: NodeId,
}

fn fixture() -> Fixture {
    let mut doc = common::doc();
    let (gen_, child, tag, linked, other) = doc
        .transact("build", |tx| {
            let gen_ = tx.create("test.box", None, None)?;
            let child = tx.create("test.bend", Some(gen_), None)?;
            let tag = tx.attach_tag("test.sel", gen_, None)?;
            let linked = tx.create("forge.null", None, None)?;
            let other = tx.create("test.box", None, None)?;
            tx.set_param(gen_, "target", Value::Link(Some(linked)))?;
            Ok((gen_, child, tag, linked, other))
        })
        .unwrap();
    Fixture {
        doc,
        gen_,
        child,
        tag,
        linked,
        other,
    }
}

fn rev(f: &Fixture) -> u64 {
    f.doc.scene().deep_revision(f.gen_).unwrap()
}

#[test]
fn deep_revision_tracks_self_children_tags_and_links() {
    let mut f = fixture();
    let steps: Vec<fn(&mut Fixture)> = vec![
        |f| {
            let g = f.gen_;
            f.doc
                .transact("p", |tx| tx.set_param(g, "size", Value::Float(1.0)))
                .unwrap()
        },
        |f| {
            let c = f.child;
            f.doc
                .transact("p", |tx| tx.set_param(c, "strength", Value::Float(1.0)))
                .unwrap()
        },
        |f| {
            let t = f.tag;
            f.doc
                .transact("p", |tx| tx.set_param(t, "set", Value::String("s".into())))
                .unwrap()
        },
        |f| {
            let l = f.linked;
            f.doc.transact("p", |tx| tx.rename(l, "moved")).unwrap()
        },
        |f| {
            let g = f.gen_;
            f.doc
                .transact("p", |tx| tx.create("forge.null", Some(g), None).map(|_| ()))
                .unwrap()
        },
        |f| {
            let c = f.child;
            f.doc
                .transact("p", |tx| tx.move_node(c, None, None))
                .unwrap()
        },
    ];
    for (i, step) in steps.iter().enumerate() {
        let before = rev(&f);
        step(&mut f);
        assert!(rev(&f) > before, "step {i} did not dirty the generator");
    }
}

#[test]
fn unrelated_edits_do_not_dirty() {
    let mut f = fixture();
    let before = rev(&f);
    let o = f.other;
    f.doc
        .transact("p", |tx| tx.set_param(o, "size", Value::Float(9.0)))
        .unwrap();
    f.doc
        .transact("n", |tx| tx.create("forge.null", None, None))
        .unwrap();
    assert_eq!(rev(&f), before);
}

#[test]
fn link_cycles_terminate_and_missing_nodes_are_none() {
    let mut f = fixture();
    let (g, o) = (f.gen_, f.other);
    f.doc
        .transact("cycle", |tx| {
            tx.set_param(g, "target", Value::Link(Some(o)))?;
            tx.set_param(o, "target", Value::Link(Some(g)))
        })
        .unwrap();
    assert!(f.doc.scene().deep_revision(g).is_some());
    assert_eq!(f.doc.scene().deep_revision(NodeId(9999)), None);
}

#[test]
fn eval_cache_recomputes_only_when_dirty() {
    let mut f = fixture();
    let mut cache: EvalCache<usize> = EvalCache::new();
    let calls = Cell::new(0);
    let eval = |cache: &mut EvalCache<usize>, f: &Fixture| {
        cache
            .get_or_compute(f.doc.scene(), f.gen_, |scene, id| {
                calls.set(calls.get() + 1);
                scene.descendants(id).len()
            })
            .map(|v| *v)
    };
    assert_eq!(eval(&mut cache, &f), Some(1));
    assert_eq!(eval(&mut cache, &f), Some(1));
    assert_eq!(calls.get(), 1);

    let o = f.other;
    f.doc
        .transact("p", |tx| tx.set_param(o, "size", Value::Float(9.0)))
        .unwrap();
    eval(&mut cache, &f);
    assert_eq!(calls.get(), 1, "unrelated edit must not recompute");

    let g = f.gen_;
    f.doc
        .transact("c", |tx| tx.create("forge.null", Some(g), None))
        .unwrap();
    assert_eq!(eval(&mut cache, &f), Some(2));
    assert_eq!(calls.get(), 2);

    f.doc.undo();
    assert_eq!(eval(&mut cache, &f), Some(1));
    assert_eq!(calls.get(), 3, "undo must recompute");
}

#[test]
fn cache_misses_for_missing_nodes_and_purges_deleted() {
    let mut f = fixture();
    let mut cache: EvalCache<u8> = EvalCache::new();
    assert!(
        cache
            .get_or_compute(f.doc.scene(), NodeId(9999), |_, _| 1)
            .is_none()
    );
    cache.get_or_compute(f.doc.scene(), f.gen_, |_, _| 1);
    cache.get_or_compute(f.doc.scene(), f.other, |_, _| 2);
    assert_eq!(cache.len(), 2);
    let o = f.other;
    f.doc.transact("d", |tx| tx.delete(o)).unwrap();
    cache.purge(f.doc.scene());
    assert_eq!(cache.len(), 1);
    cache.clear();
    assert_eq!(cache.len(), 0);
}
