//! Dependency graph of evaluation and scene diffing (docs/specs/CORE.md section 5b).
//!
//! Every node has two vertices: [`Facet::Frame`] (what flows down the hierarchy: the
//! transform and object-state params) and [`Facet::Result`] (the node's evaluated output).
//! An edge `a -> b` means "`a` depends on `b`". Domain knowledge (which nodes also depend
//! on which) comes in through the `extra` callback of [`DepGraph::build`].

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::{NodeId, Scene, Value, transform, visibility};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Facet {
    /// Transform and object-state params composed down the hierarchy.
    Frame,
    /// The evaluated output of the node.
    Result,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vertex {
    pub node: NodeId,
    pub facet: Facet,
}

impl Vertex {
    pub fn frame(node: NodeId) -> Vertex {
        Vertex {
            node,
            facet: Facet::Frame,
        }
    }

    pub fn result(node: NodeId) -> Vertex {
        Vertex {
            node,
            facet: Facet::Result,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeKind {
    OwnFrame,
    Ancestor,
    Child,
    Tag,
    Link,
    Extra,
}

/// An edge removed to make the graph acyclic, with the cycle it closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleBreak {
    pub from: Vertex,
    pub to: Vertex,
    pub kind: EdgeKind,
    /// The vertices of the cycle, from `to` along the walk down to `from`.
    pub path: Vec<Vertex>,
}

/// Every node, tags included, in ascending id order.
pub fn all_nodes(scene: &Scene) -> Vec<NodeId> {
    let mut ids = Vec::with_capacity(scene.len());
    let mut stack: Vec<NodeId> = scene.roots().to_vec();
    while let Some(id) = stack.pop() {
        ids.push(id);
        if let Some(n) = scene.node(id) {
            stack.extend(n.children.iter().chain(&n.tags));
        }
    }
    ids.sort();
    ids
}

/// The nodes a param value links to.
pub fn link_targets(value: &Value) -> Vec<NodeId> {
    match value {
        Value::Link(Some(t)) => vec![*t],
        Value::LinkList(ts) => ts.clone(),
        Value::LinkRows(rows) => rows.iter().map(|r| r.target).collect(),
        _ => Vec::new(),
    }
}

#[derive(Debug, Clone, Default)]
pub struct DepGraph {
    deps: BTreeMap<Vertex, Vec<(Vertex, EdgeKind)>>,
    rdeps: BTreeMap<Vertex, Vec<Vertex>>,
    breaks: Vec<CycleBreak>,
    levels: BTreeMap<Vertex, usize>,
}

impl DepGraph {
    /// Builds the graph of `scene`. `extra(scene, id)` lists nodes whose result `id`
    /// additionally depends on.
    pub fn build(scene: &Scene, extra: &dyn Fn(&Scene, NodeId) -> Vec<NodeId>) -> DepGraph {
        let mut deps: BTreeMap<Vertex, Vec<(Vertex, EdgeKind)>> = BTreeMap::new();
        for id in all_nodes(scene) {
            let Some(n) = scene.node(id) else { continue };
            let mut r: Vec<(Vertex, EdgeKind)> = Vec::new();
            let mut f: Vec<(Vertex, EdgeKind)> = Vec::new();
            if !n.is_tag() {
                r.push((Vertex::frame(id), EdgeKind::OwnFrame));
                if let Some(p) = n.parent {
                    f.push((Vertex::frame(p), EdgeKind::Ancestor));
                }
            }
            let add = |list: &mut Vec<(Vertex, EdgeKind)>, t: NodeId, k: EdgeKind| {
                if scene.contains(t) {
                    list.push((Vertex::result(t), k));
                }
            };
            for c in &n.children {
                add(&mut r, *c, EdgeKind::Child);
            }
            for t in &n.tags {
                add(&mut r, *t, EdgeKind::Tag);
            }
            for v in n.params.values() {
                for t in link_targets(v) {
                    add(&mut r, t, EdgeKind::Link);
                }
            }
            for t in extra(scene, id) {
                add(&mut r, t, EdgeKind::Extra);
            }
            r.sort();
            r.dedup();
            deps.insert(Vertex::result(id), r);
            if !n.is_tag() {
                deps.insert(Vertex::frame(id), f);
            }
        }
        let mut rdeps: BTreeMap<Vertex, Vec<Vertex>> = BTreeMap::new();
        for (v, ds) in &deps {
            for (d, _) in ds {
                rdeps.entry(*d).or_default().push(*v);
            }
        }
        let mut g = DepGraph {
            deps,
            rdeps,
            breaks: Vec::new(),
            levels: BTreeMap::new(),
        };
        g.break_cycles();
        g
    }

    /// Depth-first walk in ascending order; an edge to a vertex on the current path is a
    /// cycle edge: it is recorded and left out of the levels.
    fn break_cycles(&mut self) {
        #[derive(PartialEq, Clone, Copy)]
        enum Mark {
            Open,
            Done,
        }
        let mut mark: BTreeMap<Vertex, Mark> = BTreeMap::new();
        let roots: Vec<Vertex> = self.deps.keys().copied().collect();
        for root in roots {
            if mark.contains_key(&root) {
                continue;
            }
            // (vertex, next edge index, level so far)
            let mut stack: Vec<(Vertex, usize, usize)> = vec![(root, 0, 0)];
            mark.insert(root, Mark::Open);
            while let Some(&(v, i, lvl)) = stack.last() {
                let edges = self.deps.get(&v).map_or(&[][..], Vec::as_slice);
                if i == edges.len() {
                    self.levels.insert(v, lvl);
                    mark.insert(v, Mark::Done);
                    stack.pop();
                    if let Some(top) = stack.last_mut() {
                        top.2 = top.2.max(lvl + 1);
                    }
                    continue;
                }
                stack.last_mut().expect("non-empty").1 += 1;
                let (to, kind) = edges[i];
                match mark.get(&to) {
                    None => {
                        mark.insert(to, Mark::Open);
                        stack.push((to, 0, 0));
                    }
                    Some(Mark::Open) => {
                        let start = stack.iter().position(|s| s.0 == to).unwrap_or(0);
                        let path = stack[start..].iter().map(|s| s.0).collect();
                        self.breaks.push(CycleBreak {
                            from: v,
                            to,
                            kind,
                            path,
                        });
                    }
                    Some(Mark::Done) => {
                        let l = self.levels.get(&to).copied().unwrap_or(0);
                        let top = stack.last_mut().expect("non-empty");
                        top.2 = top.2.max(l + 1);
                    }
                }
            }
        }
    }

    pub fn contains(&self, v: Vertex) -> bool {
        self.deps.contains_key(&v)
    }

    pub fn vertex_count(&self) -> usize {
        self.deps.len()
    }

    /// What `v` directly depends on.
    pub fn dependencies(&self, v: Vertex) -> &[(Vertex, EdgeKind)] {
        self.deps.get(&v).map_or(&[], Vec::as_slice)
    }

    /// The edges removed to break cycles, in discovery order.
    pub fn cycle_breaks(&self) -> &[CycleBreak] {
        &self.breaks
    }

    /// Dependency depth of `v` in the acyclic graph (0 = depends on nothing).
    pub fn level(&self, v: Vertex) -> Option<usize> {
        self.levels.get(&v).copied()
    }

    /// The seeds and everything that depends on them, directly or not (cycle safe).
    pub fn dependants(&self, seeds: impl IntoIterator<Item = Vertex>) -> BTreeSet<Vertex> {
        let mut out = BTreeSet::new();
        let mut stack: Vec<Vertex> = seeds.into_iter().collect();
        while let Some(v) = stack.pop() {
            if !out.insert(v) {
                continue;
            }
            if let Some(rs) = self.rdeps.get(&v) {
                stack.extend(rs.iter().filter(|r| !out.contains(r)));
            }
        }
        out
    }
}

/// Bit-exact equality of two param values (`-0.0` differs from `0.0`; meshes by identity).
pub fn identical(a: &Value, b: &Value) -> bool {
    let bits = |x: &[f64; 3]| x.map(f64::to_bits);
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::Vec3(x), Value::Vec3(y)) | (Value::Color(x), Value::Color(y)) => bits(x) == bits(y),
        (Value::Mesh(x), Value::Mesh(y)) => Arc::ptr_eq(x, y),
        (Value::Mesh(_), _) | (_, Value::Mesh(_)) => false,
        (Value::Float(_), _) | (_, Value::Float(_)) => false,
        _ => a == b,
    }
}

/// True for params that flow down the hierarchy (transform and object state).
pub fn is_frame_param(key: &str) -> bool {
    key.starts_with(transform::PREFIX) || key.starts_with(visibility::PREFIX)
}

/// What changed between two evaluated scenes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneDiff {
    /// Vertices whose input changed (including those of removed nodes).
    pub seeds: BTreeSet<Vertex>,
    /// Hierarchy, tags or links changed, or nodes came or went: edges may differ.
    pub structure_changed: bool,
    pub added: Vec<NodeId>,
    pub removed: Vec<NodeId>,
}

impl SceneDiff {
    pub fn is_empty(&self) -> bool {
        self.seeds.is_empty()
    }
}

/// Compares two scenes node by node, bit-exactly.
pub fn diff(old: &Scene, new: &Scene) -> SceneDiff {
    let mut d = SceneDiff::default();
    let seed_all = |d: &mut SceneDiff, scene: &Scene, id: NodeId| {
        d.seeds.insert(Vertex::result(id));
        if scene.node(id).is_some_and(|n| !n.is_tag()) {
            d.seeds.insert(Vertex::frame(id));
        }
    };
    let layers_differ = old.layers() != new.layers();
    for id in all_nodes(new) {
        let n = new.node(id).expect("listed node");
        let Some(o) = old.node(id) else {
            d.added.push(id);
            d.structure_changed = true;
            seed_all(&mut d, new, id);
            continue;
        };
        if o.type_id != n.type_id {
            d.structure_changed = true;
            seed_all(&mut d, new, id);
            continue;
        }
        if o.parent != n.parent || o.host != n.host {
            d.structure_changed = true;
            d.seeds.insert(Vertex::frame(id));
            d.seeds.insert(Vertex::result(id));
        }
        if o.children != n.children || o.tags != n.tags {
            d.structure_changed = true;
            d.seeds.insert(Vertex::result(id));
        }
        // what the layers decide about the node (CORE.md §7.5)
        if (layers_differ || !std::ptr::eq(o, n))
            && crate::layers::effect(old, id) != crate::layers::effect(new, id)
        {
            seed_all(&mut d, new, id);
        }
        if std::ptr::eq(o, n) {
            continue;
        }
        let keys: BTreeSet<&String> = o.params.keys().chain(n.params.keys()).collect();
        for k in keys {
            let same = match (o.params.get(k), n.params.get(k)) {
                (Some(a), Some(b)) => identical(a, b),
                (None, None) => true,
                _ => false,
            };
            if same {
                continue;
            }
            let linkish = |v: Option<&Value>| {
                matches!(
                    v,
                    Some(Value::Link(_) | Value::LinkList(_) | Value::LinkRows(_))
                )
            };
            if linkish(o.params.get(k)) || linkish(n.params.get(k)) {
                d.structure_changed = true;
            }
            if is_frame_param(k) && !n.is_tag() {
                d.seeds.insert(Vertex::frame(id));
            } else {
                d.seeds.insert(Vertex::result(id));
            }
        }
    }
    for id in all_nodes(old) {
        if !new.contains(id) {
            d.removed.push(id);
            d.structure_changed = true;
            seed_all(&mut d, old, id);
        }
    }
    d
}
