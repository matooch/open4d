//! docs/specs/CORE.md section 5b: dependency graph, scene diff, cycle breaking.

mod common;

use forge_core::depgraph::{DepGraph, EdgeKind, Vertex, diff};
use forge_core::{NodeId, Value};

struct F {
    doc: forge_core::Document,
    a: NodeId,
    b: NodeId,
    c: NodeId,
    tag: NodeId,
    other: NodeId,
}

fn fixture() -> F {
    let mut doc = common::doc();
    let (a, b, c, tag, other) = doc
        .transact("build", |tx| {
            let a = tx.create("test.box", None, None)?;
            let b = tx.create("test.box", Some(a), None)?;
            let c = tx.create("test.bend", Some(b), None)?;
            let tag = tx.attach_tag("test.sel", a, None)?;
            let other = tx.create("test.box", None, None)?;
            Ok((a, b, c, tag, other))
        })
        .unwrap();
    F {
        doc,
        a,
        b,
        c,
        tag,
        other,
    }
}

fn graph(f: &F) -> DepGraph {
    DepGraph::build(f.doc.scene(), &|_, _| Vec::new())
}

fn nodes(set: std::collections::BTreeSet<Vertex>) -> Vec<NodeId> {
    let mut v: Vec<NodeId> = set
        .into_iter()
        .filter(|v| v.facet == forge_core::depgraph::Facet::Result)
        .map(|v| v.node)
        .collect();
    v.dedup();
    v
}

#[test]
fn child_edit_dirties_ancestors_but_not_siblings_or_descendants() {
    let f = fixture();
    let g = graph(&f);
    let dirty = nodes(g.dependants([Vertex::result(f.b)]));
    assert_eq!(dirty, vec![f.a, f.b]);
    let dirty = nodes(g.dependants([Vertex::result(f.tag)]));
    assert_eq!(dirty, vec![f.a, f.tag]);
}

#[test]
fn frame_edit_dirties_descendants_and_their_ancestors() {
    let f = fixture();
    let g = graph(&f);
    let dirty = nodes(g.dependants([Vertex::frame(f.b)]));
    assert_eq!(dirty, vec![f.a, f.b, f.c]);
    assert!(!dirty.contains(&f.other));
}

#[test]
fn links_are_edges() {
    let mut f = fixture();
    let (a, o) = (f.a, f.other);
    f.doc
        .transact("link", |tx| tx.set_param(o, "target", Value::Link(Some(a))))
        .unwrap();
    let g = graph(&f);
    assert!(
        g.dependencies(Vertex::result(f.other))
            .contains(&(Vertex::result(f.a), EdgeKind::Link))
    );
    let dirty = nodes(g.dependants([Vertex::result(f.a)]));
    assert_eq!(dirty, vec![f.a, f.other]);
}

#[test]
fn diff_classifies_param_edits() {
    let mut f = fixture();
    let old = f.doc.scene().clone();
    let b = f.b;
    f.doc
        .transact("e", |tx| {
            tx.set_param(b, "size", Value::Float(5.0))?;
            tx.set_param(b, "transform.pos", Value::Vec3([1.0, 0.0, 0.0]))
        })
        .unwrap();
    let d = diff(&old, f.doc.scene());
    assert!(!d.structure_changed);
    assert_eq!(
        d.seeds.into_iter().collect::<Vec<_>>(),
        vec![Vertex::frame(b), Vertex::result(b)]
    );
    assert!(diff(f.doc.scene(), f.doc.scene()).is_empty());
}

#[test]
fn diff_is_bit_exact_for_signed_zero() {
    let mut f = fixture();
    let b = f.b;
    f.doc
        .transact("e", |tx| tx.set_param(b, "size", Value::Float(0.0)))
        .unwrap();
    let old = f.doc.scene().clone();
    f.doc
        .transact("e", |tx| tx.set_param(b, "size", Value::Float(-0.0)))
        .unwrap();
    assert!(!diff(&old, f.doc.scene()).is_empty());
}

#[test]
fn diff_sees_structure_changes_and_removals() {
    let mut f = fixture();
    let old = f.doc.scene().clone();
    let o = f.other;
    f.doc.transact("rm", |tx| tx.delete(o)).unwrap();
    let d = diff(&old, f.doc.scene());
    assert!(d.structure_changed);
    assert_eq!(d.removed, vec![f.other]);
}

#[test]
fn cycles_are_broken_deterministically_and_reported() {
    let mut f = fixture();
    let (a, o) = (f.a, f.other);
    f.doc
        .transact("cycle", |tx| {
            tx.set_param(a, "target", Value::Link(Some(o)))?;
            tx.set_param(o, "target", Value::Link(Some(a)))
        })
        .unwrap();
    let g1 = graph(&f);
    let g2 = graph(&f);
    assert_eq!(g1.cycle_breaks(), g2.cycle_breaks());
    assert_eq!(g1.cycle_breaks().len(), 1);
    let br = &g1.cycle_breaks()[0];
    // the walk starts at the lowest id (a): a -> other is followed, other -> a closes it
    assert_eq!((br.from.node, br.to.node), (f.other, f.a));
    assert_eq!(br.kind, EdgeKind::Link);
    // dirtiness still follows the broken edge
    let dirty = nodes(g1.dependants([Vertex::result(f.a)]));
    assert!(dirty.contains(&f.other) && dirty.contains(&f.a));
    // acyclic levels exist for every vertex
    assert!(g1.level(Vertex::result(f.a)).is_some());
}

#[test]
fn levels_order_dependencies_first() {
    let f = fixture();
    let g = graph(&f);
    assert!(g.level(Vertex::result(f.a)) > g.level(Vertex::result(f.b)));
    assert!(g.level(Vertex::result(f.b)) > g.level(Vertex::result(f.c)));
}
