//! The evaluation scheduler: re-evaluates only the dirty part of the scene, in parallel,
//! with results bit-identical to serial evaluation (docs/specs/CORE.md section 5b).
//!
//! One pass: tracks (`scene_at`), then the expression stage (one ordered stage, section 5a),
//! then the drawn nodes that are dirty or new, each evaluated from the immutable evaluated
//! scene. What is dirty comes from a bit-exact diff against the previous pass pushed through
//! the dependency graph ([`forge_core::depgraph`]).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use forge_core::depgraph::{self, CycleBreak, DepGraph, Vertex};
use forge_core::{EvalCtx, NodeId, Scene};
use forge_generators::{BuildError, ClonerInstances};
use forge_geometry::PolyMesh;
use rayon::prelude::*;

use crate::animate::frame_time;

/// How many threads evaluate nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Threads {
    /// One thread, no rayon dispatch: the reference the parallel results are compared to.
    Serial,
    /// rayon's global pool.
    #[default]
    Auto,
    /// A private pool of this many threads.
    Fixed(usize),
}

/// The rules that connect the dependency graph to the node types (plugins add theirs).
#[derive(Clone)]
pub struct Rules {
    /// Nodes whose result a node depends on beyond children, tags and links.
    pub extra_deps: Vec<fn(&Scene, NodeId) -> Vec<NodeId>>,
    /// Nodes that change with time although none of their params do.
    pub time_readers: Vec<fn(&Scene, NodeId) -> bool>,
    /// Expression tags that read the time (reporting only: their writes are found by diff).
    pub tag_time_readers: Vec<fn(&Scene, NodeId) -> bool>,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            extra_deps: vec![forge_generators::eval_dependencies],
            time_readers: vec![
                forge_fields::reads_time,
                forge_clone_engine::effector_reads_time,
            ],
            tag_time_readers: vec![forge_nodes::tag_reads_time],
        }
    }
}

/// The evaluated output of one drawn node: its mesh in world space (as the viewport and
/// the OBJ export see it) or why there is none.
#[derive(Debug, Clone)]
pub struct NodeOutput {
    pub mesh: Option<PolyMesh>,
    pub error: Option<BuildError>,
    /// A cloner the viewport draws by GPU instancing ([`Scheduler::with_instancing`]):
    /// its prototypes and clones in cloner space, with `mesh` left empty.
    pub instances: Option<Arc<ClonerInstances>>,
}

/// A scene with tracks and expressions applied, and the writes that did it.
pub struct Animated {
    pub scene: Scene,
    pub diagnostics: Vec<forge_nodes::Diagnostic>,
    /// The (node, param) pairs the tracks and the expression tags wrote (or tried to).
    pub touched: Vec<(NodeId, String)>,
}

/// Stage 1 and 2 of a pass: the tracks of `track_ids` at `time` (seconds), then the
/// expression tags `tag_ids` in order (docs/specs/NODES.md section 5). Equal to
/// `apply_expressions(scene_at(scene, time), time)` when the ids are the scene's tracks and
/// tags, but only the nodes written to are copied: the rest stays shared with `scene`.
pub fn animate(
    scene: &Scene,
    track_ids: &[NodeId],
    tag_ids: &[NodeId],
    has_tags: bool,
    time: f64,
) -> Animated {
    let mut working = scene.clone();
    let mut touched: Vec<(NodeId, String)> = Vec::new();
    if !track_ids.is_empty() {
        let overrides = forge_anim::overrides(scene, track_ids, time);
        touched.extend(overrides.iter().map(|(i, p, _)| (*i, p.clone())));
        working.apply_params(overrides);
    }
    let mut diagnostics = Vec::new();
    if has_tags {
        let run = forge_nodes::apply_expressions_in(
            working,
            forge_nodes::NodeRegistry::builtin(),
            time,
            tag_ids,
        );
        working = run.scene;
        diagnostics = run.diagnostics;
        touched.extend(run.touched);
    }
    Animated {
        scene: working,
        diagnostics,
        touched,
    }
}

/// The nodes with tracks and the expression tags of a scene (the `animate` inputs).
pub fn animation_inputs(scene: &Scene) -> (Vec<NodeId>, Vec<NodeId>, bool) {
    let tracks = forge_anim::track_nodes(scene);
    let tags = forge_nodes::expression_tags(scene);
    let enabled = expression_tags_enabled(scene, &tags);
    (tracks, tags, enabled)
}

/// How a pass found what changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Path {
    /// Nothing could have changed.
    #[default]
    None,
    /// Only the time moved: the change set came from the writes of the tracks and the
    /// expression tags, and untouched nodes were shared, not copied.
    ChangeSet,
    /// The document changed (or the first pass): a bit-exact diff of the evaluated scenes.
    Diff,
}

/// What one pass did.
#[derive(Debug, Clone, Default)]
pub struct Stats {
    /// Drawn nodes evaluated this pass, ascending.
    pub evaluated: Vec<NodeId>,
    /// Drawn nodes whose previous output was reused.
    pub reused: usize,
    /// Every node whose result vertex is dirty (drawn or not), ascending.
    pub dirty: Vec<NodeId>,
    /// Nothing was recomputed: same document revision and time as the last pass.
    pub unchanged: bool,
    /// How the dirty set was found.
    pub path: Path,
    /// The expression stage ran.
    pub expressions_ran: bool,
    /// Edges removed to break cycles.
    pub cycles: Vec<CycleBreak>,
    pub diagnostics: Vec<forge_nodes::Diagnostic>,
}

/// The result of one pass.
#[derive(Debug)]
pub struct Frame {
    /// Changes whenever any output changed; a pass that reused everything keeps the
    /// previous value.
    pub generation: u64,
    /// The scene with tracks and expressions applied.
    pub scene: Arc<Scene>,
    /// The drawn nodes in scene order (the keys of `outputs`).
    pub drawn: Vec<NodeId>,
    pub outputs: BTreeMap<NodeId, Arc<NodeOutput>>,
    pub stats: Stats,
}

struct State {
    rev: u64,
    len: usize,
    time: f64,
    fps: u32,
    seed: u64,
    evaluated: Arc<Scene>,
    graph: Arc<DepGraph>,
    readers: BTreeSet<NodeId>,
    has_tracks: bool,
    has_tags: bool,
    /// Every node (tags included) in ascending order; valid for `rev`.
    ids: Vec<NodeId>,
    /// Nodes that own tracks; valid for `rev`.
    track_ids: Vec<NodeId>,
    /// Expression tags in document order; valid for `rev`.
    tag_ids: Vec<NodeId>,
    /// The (node, param) pairs tracks and expression tags wrote in this pass.
    touched: Vec<(NodeId, String)>,
    drawn: Vec<NodeId>,
    drawn_set: BTreeSet<NodeId>,
    frame: Arc<Frame>,
}

pub struct Scheduler {
    threads: Threads,
    pool: Option<rayon::ThreadPool>,
    rules: Rules,
    seed: u64,
    validate: bool,
    instancing: bool,
    generation: u64,
    state: Option<State>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Scheduler::new(Threads::Auto)
    }
}

/// The nodes the viewport draws, in scene order: enabled and editor-visible, not inputs
/// that a parent generator consumes.
pub fn drawn_nodes(scene: &Scene) -> Vec<NodeId> {
    let mut out = Vec::new();
    for root in scene.roots() {
        for id in std::iter::once(*root).chain(scene.descendants(*root)) {
            if forge_generators::is_consumed(scene, id) {
                continue;
            }
            if !forge_core::visibility::editor_visible(scene, id) {
                continue;
            }
            out.push(id);
        }
    }
    out
}

/// One node's output at `ctx`, the same computation as the viewport's `scene_meshes_at`.
pub fn node_output(scene: &Scene, id: NodeId, ctx: &EvalCtx) -> NodeOutput {
    match forge_generators::evaluate(scene, id, ctx) {
        Ok(Some(mut mesh)) => {
            if let Some(m) = scene.world_matrix(id) {
                for p in &mut mesh.points {
                    *p = m.transform_point3(*p);
                }
                NodeOutput {
                    mesh: Some(mesh),
                    error: None,
                    instances: None,
                }
            } else {
                NodeOutput {
                    mesh: None,
                    error: None,
                    instances: None,
                }
            }
        }
        Ok(None) => NodeOutput {
            mesh: None,
            error: None,
            instances: None,
        },
        Err(e) => NodeOutput {
            mesh: None,
            error: Some(e),
            instances: None,
        },
    }
}

/// [`node_output`] for drawing: a cloner that `cloner_instances` can show is returned as
/// instances instead of expanded geometry (docs/specs/VIEWPORT.md section 10.8).
pub fn draw_output(scene: &Scene, id: NodeId, ctx: &EvalCtx) -> NodeOutput {
    if scene.world_matrix(id).is_some()
        && let Ok(Some(instances)) = forge_generators::cloner_instances(scene, id, ctx)
    {
        return NodeOutput {
            mesh: None,
            error: None,
            instances: Some(Arc::new(instances)),
        };
    }
    node_output(scene, id, ctx)
}

fn expression_tags_enabled(scene: &Scene, ids: &[NodeId]) -> bool {
    ids.iter().any(|id| {
        scene.node(*id).is_some_and(|n| {
            n.type_id == forge_nodes::TAG
                && !matches!(
                    n.params.get("enabled"),
                    Some(forge_core::Value::Bool(false))
                )
        })
    })
}

impl Scheduler {
    pub fn new(threads: Threads) -> Scheduler {
        let pool = match threads {
            Threads::Auto => None,
            Threads::Serial => rayon::ThreadPoolBuilder::new().num_threads(1).build().ok(),
            Threads::Fixed(n) => rayon::ThreadPoolBuilder::new()
                .num_threads(n.max(1))
                .build()
                .ok(),
        };
        Scheduler {
            threads,
            pool,
            rules: Rules::default(),
            seed: 0,
            validate: false,
            instancing: false,
            generation: 0,
            state: None,
        }
    }

    pub fn with_rules(mut self, rules: Rules) -> Scheduler {
        self.rules = rules;
        self
    }

    /// The seed of the evaluation context (changing it dirties everything).
    pub fn with_seed(mut self, seed: u64) -> Scheduler {
        self.seed = seed;
        self
    }

    /// Outputs for drawing: showable cloners come as instances ([`draw_output`]) instead of
    /// expanded meshes.
    pub fn with_instancing(mut self, on: bool) -> Scheduler {
        self.instancing = on;
        self
    }

    /// Checks every incremental pass against the bit-exact scene diff and panics if the
    /// change set and the diff disagree (tests; it costs what the old full diff cost).
    pub fn with_validation(mut self, on: bool) -> Scheduler {
        self.validate = on;
        self
    }

    /// Forgets all results (after loading another document).
    pub fn clear(&mut self) {
        self.state = None;
    }

    /// Nodes whose result may change with time: nodes with tracks, implicit time readers,
    /// expression tags that read the time, and everything depending on them. Valid for the
    /// last pass.
    pub fn time_dependent(&self) -> BTreeSet<NodeId> {
        let Some(st) = &self.state else {
            return BTreeSet::new();
        };
        time_sources(&st.evaluated, &st.graph, &self.rules, &st.readers)
    }

    fn map_nodes<T: Send>(&self, ids: &[NodeId], f: impl Fn(NodeId) -> T + Sync + Send) -> Vec<T> {
        match (self.threads, &self.pool) {
            // the one-thread pool also keeps the parallel loops inside a node on this thread
            (Threads::Serial, Some(pool)) => pool.install(|| ids.iter().map(|id| f(*id)).collect()),
            (Threads::Serial, None) => ids.iter().map(|id| f(*id)).collect(),
            (_, Some(pool)) => pool.install(|| ids.par_iter().map(|id| f(*id)).collect()),
            _ => ids.par_iter().map(|id| f(*id)).collect(),
        }
    }

    /// Evaluates `scene` at `frame`, recomputing only what changed since the last call.
    pub fn evaluate(&mut self, scene: &Scene, frame: i64) -> Arc<Frame> {
        let fps = scene.settings().fps;
        let time = frame_time(frame, fps);
        let ctx = EvalCtx {
            time,
            fps: f64::from(fps),
            seed: self.seed,
        };
        let global = self
            .state
            .as_ref()
            .is_none_or(|s| s.fps != fps || s.seed != self.seed);
        let doc_changed = self
            .state
            .as_ref()
            .is_none_or(|s| s.rev != scene.revision() || s.len != scene.len());
        let time_changed = self
            .state
            .as_ref()
            .is_none_or(|s| s.time.to_bits() != time.to_bits());
        if !global
            && !doc_changed
            && let Some(st) = &mut self.state
        {
            let quiet = !time_changed || (!st.has_tracks && !st.has_tags && st.readers.is_empty());
            if quiet {
                // nothing in the scene can see the time
                st.time = time;
                let stats = Stats {
                    unchanged: true,
                    reused: st.frame.outputs.len(),
                    ..Default::default()
                };
                return Arc::new(Frame {
                    generation: st.frame.generation,
                    scene: st.evaluated.clone(),
                    drawn: st.drawn.clone(),
                    outputs: st.frame.outputs.clone(),
                    stats,
                });
            }
        }

        // The document is the same as in the last pass and only the time moved: the
        // structure lists are still valid and the change set is the writes of this pass and
        // of the last one.
        let incremental = !global && !doc_changed;
        let old = if global { None } else { self.state.as_ref() };
        let (ids, track_ids, tag_ids) = match old {
            Some(s) if incremental => (s.ids.clone(), s.track_ids.clone(), s.tag_ids.clone()),
            _ => (
                depgraph::all_nodes(scene),
                forge_anim::track_nodes(scene),
                forge_nodes::expression_tags(scene),
            ),
        };
        let has_tracks = !track_ids.is_empty();
        let has_tags = expression_tags_enabled(scene, &tag_ids);

        // Stage 1 and 2: tracks, then expressions (one ordered stage). Only the nodes with
        // tracks and the nodes the tags write to are copied; the rest stays shared with
        // `scene`.
        let animated = animate(scene, &track_ids, &tag_ids, has_tags, time);
        let (working, diagnostics, touched) =
            (animated.scene, animated.diagnostics, animated.touched);
        let evaluated = Arc::new(working);

        // The seeds of the dirty set.
        let extra = {
            let rules = self.rules.extra_deps.clone();
            move |s: &Scene, id: NodeId| rules.iter().flat_map(|f| f(s, id)).collect::<Vec<_>>()
        };
        let mut path = Path::Diff;
        let mut readers_known: Option<BTreeSet<NodeId>> = None;
        let mut change: Option<(BTreeSet<Vertex>, bool)> = None;
        if let (true, Some(s)) = (incremental, old)
            && let Some(seeds) = change_set(&s.evaluated, &evaluated, &s.touched, &touched)
        {
            if self.validate {
                let full = depgraph::diff(&s.evaluated, &evaluated);
                assert!(
                    !full.structure_changed && full.seeds == seeds,
                    "change set {seeds:?} differs from the scene diff {:?}",
                    full.seeds
                );
            }
            path = Path::ChangeSet;
            // time readers are node-local: only the changed nodes can have switched
            let changed: BTreeSet<NodeId> = seeds.iter().map(|v| v.node).collect();
            let mut r = s.readers.clone();
            for id in &changed {
                r.remove(id);
                if self.rules.time_readers.iter().any(|f| f(&evaluated, *id)) {
                    r.insert(*id);
                }
            }
            readers_known = Some(r);
            change = Some((seeds, false));
        }
        if change.is_none() {
            change = old.map(|s| {
                let d = depgraph::diff(&s.evaluated, &evaluated);
                (d.seeds, d.structure_changed)
            });
        }
        let rebuild = change.as_ref().is_none_or(|c| c.1);
        let graph = match (&change, old) {
            (Some((_, false)), Some(s)) => s.graph.clone(),
            _ => Arc::new(DepGraph::build(&evaluated, &extra)),
        };
        let readers: BTreeSet<NodeId> = match (readers_known, &change, old) {
            (Some(r), _, _) => r,
            (None, Some((seeds, _)), Some(s)) if seeds.is_empty() => s.readers.clone(),
            _ => ids
                .iter()
                .copied()
                .filter(|id| self.rules.time_readers.iter().any(|f| f(&evaluated, *id)))
                .collect(),
        };
        let dirty: Option<BTreeSet<Vertex>> = change.map(|(mut seeds, _)| {
            if time_changed {
                seeds.extend(readers.iter().map(|r| Vertex::result(*r)));
                if let Some(s) = old {
                    seeds.extend(s.readers.iter().map(|r| Vertex::result(*r)));
                }
            }
            let mut set = graph.dependants(seeds.iter().copied());
            if rebuild && let Some(s) = old {
                set.extend(s.graph.dependants(seeds.iter().copied()));
            }
            set
        });

        // Stage 3: drawn nodes, dirty or new, in parallel.
        let drawn = match (old, &dirty) {
            (Some(s), Some(d))
                if path == Path::ChangeSet && !drawn_may_change(&evaluated, s, d) =>
            {
                s.drawn.clone()
            }
            _ => drawn_nodes(&evaluated),
        };
        let drawn_set: BTreeSet<NodeId> = match old {
            Some(s) if s.drawn == drawn => s.drawn_set.clone(),
            _ => drawn.iter().copied().collect(),
        };
        let old_outputs = old.map(|s| &s.frame.outputs);
        let same_drawn = old.is_some_and(|s| s.drawn == drawn);
        let todo: Vec<NodeId> = match (&dirty, old_outputs) {
            (Some(dirty), Some(prev)) if same_drawn => dirty
                .iter()
                .filter(|v| v.facet == depgraph::Facet::Result && prev.contains_key(&v.node))
                .map(|v| v.node)
                .collect(),
            (Some(dirty), Some(prev)) => drawn
                .iter()
                .copied()
                .filter(|id| dirty.contains(&Vertex::result(*id)) || !prev.contains_key(id))
                .collect(),
            _ => drawn.clone(),
        };
        let instancing = self.instancing;
        let computed = self.map_nodes(&todo, |id| {
            Arc::new(if instancing {
                draw_output(&evaluated, id, &ctx)
            } else {
                node_output(&evaluated, id, &ctx)
            })
        });
        let mut outputs: BTreeMap<NodeId, Arc<NodeOutput>> = match old_outputs {
            Some(prev) if same_drawn => prev.clone(),
            _ => BTreeMap::new(),
        };
        let mut reused = drawn.len() - todo.len();
        if same_drawn {
            outputs.extend(todo.iter().copied().zip(computed));
        } else {
            let mut fresh: BTreeMap<NodeId, Arc<NodeOutput>> =
                todo.iter().copied().zip(computed).collect();
            reused = 0;
            for id in &drawn {
                if let Some(o) = fresh.remove(id) {
                    outputs.insert(*id, o);
                } else if let Some(o) = old_outputs.and_then(|p| p.get(id)) {
                    outputs.insert(*id, o.clone());
                    reused += 1;
                }
            }
        }
        let dirty_nodes: Vec<NodeId> = match &dirty {
            Some(set) => set
                .iter()
                .filter(|v| v.facet == depgraph::Facet::Result)
                .map(|v| v.node)
                .collect(),
            None => ids.clone(),
        };
        let mut evaluated_ids = todo;
        evaluated_ids.sort();
        let stats = Stats {
            evaluated: evaluated_ids,
            reused,
            dirty: dirty_nodes,
            unchanged: false,
            path,
            expressions_ran: has_tags,
            cycles: graph.cycle_breaks().to_vec(),
            diagnostics,
        };
        if !stats.evaluated.is_empty() || old.is_none() || !same_drawn {
            self.generation += 1;
        }
        let out = Arc::new(Frame {
            generation: self.generation,
            scene: evaluated.clone(),
            drawn: drawn.clone(),
            outputs,
            stats,
        });
        self.state = Some(State {
            rev: scene.revision(),
            len: scene.len(),
            time,
            fps,
            seed: self.seed,
            evaluated,
            graph,
            readers,
            has_tracks,
            has_tags,
            ids,
            track_ids,
            tag_ids,
            touched,
            drawn,
            drawn_set,
            frame: out.clone(),
        });
        out
    }
}

/// Whether the set of drawn nodes can differ from the last pass: a node's state depends on
/// its own and its ancestors' object state (their frame vertices reach it) and on its
/// parent's generator (a parent whose result is dirty).
fn drawn_may_change(scene: &Scene, old: &State, dirty: &BTreeSet<Vertex>) -> bool {
    dirty
        .iter()
        .filter(|v| v.facet == depgraph::Facet::Result)
        .flat_map(|v| {
            let n = scene.node(v.node);
            std::iter::once(v.node).chain(n.into_iter().flat_map(|n| n.children.iter().copied()))
        })
        .filter(|id| scene.node(*id).is_some_and(|n| !n.is_tag()))
        .any(|id| {
            let drawn = !forge_generators::is_consumed(scene, id)
                && forge_core::visibility::editor_visible(scene, id);
            drawn != old.drawn_set.contains(&id)
        })
}

/// The vertices whose input differs between two evaluations of the same document, found
/// from the (node, param) pairs the tracks and tags wrote in either pass; no other value
/// can differ. `None` when a link changed (the edges may differ: use the scene diff).
fn change_set(
    old: &Scene,
    new: &Scene,
    touched_old: &[(NodeId, String)],
    touched_new: &[(NodeId, String)],
) -> Option<BTreeSet<Vertex>> {
    let mut seeds = BTreeSet::new();
    let pairs: BTreeSet<&(NodeId, String)> = touched_old.iter().chain(touched_new).collect();
    for (id, key) in pairs {
        let (Some(o), Some(n)) = (old.node(*id), new.node(*id)) else {
            return None;
        };
        if std::ptr::eq(o, n) {
            continue;
        }
        let (a, b) = (o.params.get(key), n.params.get(key));
        let same = match (a, b) {
            (Some(a), Some(b)) => depgraph::identical(a, b),
            (None, None) => true,
            _ => false,
        };
        if same {
            continue;
        }
        let linkish = |v: Option<&forge_core::Value>| {
            matches!(
                v,
                Some(forge_core::Value::Link(_) | forge_core::Value::LinkList(_))
            )
        };
        if linkish(a) || linkish(b) {
            return None;
        }
        seeds.insert(if depgraph::is_frame_param(key) && !n.is_tag() {
            Vertex::frame(*id)
        } else {
            Vertex::result(*id)
        });
    }
    Some(seeds)
}

fn time_sources(
    scene: &Scene,
    graph: &DepGraph,
    rules: &Rules,
    readers: &BTreeSet<NodeId>,
) -> BTreeSet<NodeId> {
    let mut seeds: Vec<Vertex> = readers.iter().map(|r| Vertex::result(*r)).collect();
    for id in depgraph::all_nodes(scene) {
        let Some(n) = scene.node(id) else { continue };
        for key in n.tracks.keys() {
            seeds.push(if depgraph::is_frame_param(key) && !n.is_tag() {
                Vertex::frame(id)
            } else {
                Vertex::result(id)
            });
        }
        if n.type_id == forge_nodes::TAG
            && !matches!(
                n.params.get("enabled"),
                Some(forge_core::Value::Bool(false))
            )
            && rules.tag_time_readers.iter().any(|f| f(scene, id))
        {
            seeds.push(Vertex::result(id));
            if let Some(h) = n.host {
                seeds.push(Vertex::result(h));
            }
        }
    }
    graph
        .dependants(seeds)
        .into_iter()
        .filter(|v| v.facet == depgraph::Facet::Result)
        .map(|v| v.node)
        .collect()
}
