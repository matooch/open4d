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
use forge_generators::BuildError;
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
#[derive(Debug, Clone, PartialEq)]
pub struct NodeOutput {
    pub mesh: Option<PolyMesh>,
    pub error: Option<BuildError>,
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
    /// The expression stage ran.
    pub expressions_ran: bool,
    /// Edges removed to break cycles.
    pub cycles: Vec<CycleBreak>,
    pub diagnostics: Vec<forge_nodes::Diagnostic>,
}

/// The result of one pass.
#[derive(Debug)]
pub struct Frame {
    /// The scene with tracks and expressions applied.
    pub scene: Arc<Scene>,
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
    outputs: BTreeMap<NodeId, Arc<NodeOutput>>,
}

pub struct Scheduler {
    threads: Threads,
    pool: Option<rayon::ThreadPool>,
    rules: Rules,
    seed: u64,
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
                }
            } else {
                NodeOutput {
                    mesh: None,
                    error: None,
                }
            }
        }
        Ok(None) => NodeOutput {
            mesh: None,
            error: None,
        },
        Err(e) => NodeOutput {
            mesh: None,
            error: Some(e),
        },
    }
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
            if !time_changed {
                let mut stats = Stats {
                    unchanged: true,
                    ..Default::default()
                };
                stats.reused = st.outputs.len();
                return Arc::new(Frame {
                    scene: st.evaluated.clone(),
                    outputs: st.outputs.clone(),
                    stats,
                });
            }
            if !st.has_tracks && !st.has_tags && st.readers.is_empty() {
                // nothing in the scene can see the time
                st.time = time;
                let stats = Stats {
                    unchanged: true,
                    reused: st.outputs.len(),
                    ..Default::default()
                };
                return Arc::new(Frame {
                    scene: st.evaluated.clone(),
                    outputs: st.outputs.clone(),
                    stats,
                });
            }
        }

        // Stage 1 and 2: tracks, then expressions (one ordered stage).
        let ids = depgraph::all_nodes(scene);
        let has_tracks = ids
            .iter()
            .any(|id| scene.node(*id).is_some_and(|n| !n.tracks.is_empty()));
        let has_tags = expression_tags_enabled(scene, &ids);
        let animated = if has_tracks {
            forge_anim::scene_at(scene, time)
        } else {
            scene.clone()
        };
        let (expressed, diagnostics) = if has_tags {
            forge_nodes::apply_expressions(&animated, time)
        } else {
            (animated, Vec::new())
        };
        let evaluated = Arc::new(expressed);

        // Dirty set.
        let extra = {
            let rules = self.rules.extra_deps.clone();
            move |s: &Scene, id: NodeId| rules.iter().flat_map(|f| f(s, id)).collect::<Vec<_>>()
        };
        let old = if global { None } else { self.state.as_ref() };
        let d = old.map(|s| depgraph::diff(&s.evaluated, &evaluated));
        let rebuild = d.as_ref().is_none_or(|d| d.structure_changed);
        let graph = match (&d, old) {
            (Some(d), Some(s)) if !d.structure_changed => s.graph.clone(),
            _ => Arc::new(DepGraph::build(&evaluated, &extra)),
        };
        let readers: BTreeSet<NodeId> = match (&d, old) {
            (Some(d), Some(s)) if d.is_empty() => s.readers.clone(),
            _ => ids
                .iter()
                .copied()
                .filter(|id| self.rules.time_readers.iter().any(|f| f(&evaluated, *id)))
                .collect(),
        };
        let dirty: Option<BTreeSet<Vertex>> = d.as_ref().map(|d| {
            let mut seeds = d.seeds.clone();
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
        let drawn = drawn_nodes(&evaluated);
        let old_outputs = old.map(|s| &s.outputs);
        let todo: Vec<NodeId> = drawn
            .iter()
            .copied()
            .filter(|id| match (&dirty, old_outputs) {
                (Some(dirty), Some(prev)) => {
                    dirty.contains(&Vertex::result(*id)) || !prev.contains_key(id)
                }
                _ => true,
            })
            .collect();
        let computed = self.map_nodes(&todo, |id| Arc::new(node_output(&evaluated, id, &ctx)));
        let mut outputs: BTreeMap<NodeId, Arc<NodeOutput>> = BTreeMap::new();
        let mut fresh: BTreeMap<NodeId, Arc<NodeOutput>> =
            todo.iter().copied().zip(computed).collect();
        let mut reused = 0;
        for id in &drawn {
            if let Some(o) = fresh.remove(id) {
                outputs.insert(*id, o);
            } else if let Some(o) = old_outputs.and_then(|p| p.get(id)) {
                outputs.insert(*id, o.clone());
                reused += 1;
            }
        }
        let mut dirty_nodes: Vec<NodeId> = match &dirty {
            Some(set) => set
                .iter()
                .filter(|v| v.facet == depgraph::Facet::Result)
                .map(|v| v.node)
                .collect(),
            None => ids.clone(),
        };
        dirty_nodes.sort();
        let mut evaluated_ids = todo;
        evaluated_ids.sort();
        let stats = Stats {
            evaluated: evaluated_ids,
            reused,
            dirty: dirty_nodes,
            unchanged: false,
            expressions_ran: has_tags,
            cycles: graph.cycle_breaks().to_vec(),
            diagnostics,
        };
        let out = Arc::new(Frame {
            scene: evaluated.clone(),
            outputs: outputs.clone(),
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
            outputs,
        });
        out
    }
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
