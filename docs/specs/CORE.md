# Phase 1 — core (`crates/core`, package `forge-core`)

Sources: project architecture, not reference-app behaviour. Implements the CLAUDE.md
conventions ("everything is a plugin-registered node"; "parameters are data-driven
descriptors so UI, undo, animation and serialization derive from one definition";
deterministic behaviour) and the `core` line of docs/ARCHITECTURE.md. No
reference-app observation is involved. Behaviour that must match the reference app
(rotation order, units, default values of real node types) is out of scope here and
needs its own docs/parity/ observation first.

Out of scope for Phase 1: transforms/matrices (need docs/parity/transform.md;
Phase 2), materials and shader graphs (Phase 9), animation tracks (data model and defaults: docs/specs/ANIM.md; UI Phase 11),
evaluation of real node types (Phases 5+).

## 1. Values and parameter descriptors (`param`)

`Value` — the only parameter value representation:

| Variant | Payload | Notes |
|---|---|---|
| `Bool` | `bool` | |
| `Int` | `i64` | |
| `Float` | `f64` | must be finite |
| `Vec3` | `[f64; 3]` | components finite |
| `Color` | `[f64; 3]` | linear RGB, finite, each ≥ 0 (HDR allowed) |
| `Enum` | `String` | an option id, not an index (stable across versions) |
| `String` | `String` | |
| `Link` | `Option<NodeId>` | reference to another node |
| `Mesh` | `Arc<MeshData>` | an editable polygon mesh (see below) |
| `IndexList` | `Vec<u32>` | a sorted set of element indices (component selections) |
| `LinkList` | `Vec<NodeId>` | an ordered list of node references (a cloner's effector list, an effector's field list); order is meaningful |

`ParamDesc { id, label, kind, default, ui, animatable }`:
- `id`: non-empty, chars `[a-z0-9_.]`, unique within its node type. Stable forever:
  it is the serialization key.
- `kind: ParamKind`: `Bool`, `Int { min, max }`, `Float { min, max }`,
  `Vec3 { min, max }` (per component), `Color`, `Enum { options }` (each option
  `{ id, label }`, ids unique, at least one), `String`, `Link { accepts }` (list of
  categories; empty = any), `LinkList { accepts }` (the same for each entry), `Mesh`, `IndexList`.
- `ui: UiHint { group, unit, soft_min, soft_max, step, hidden }`. `unit` ∈
  `None | Distance | Angle | Percent | Time`. UI-only; never affects validation.
- `animatable`: defaults to true for `Int/Float/Vec3/Color/Bool`, false otherwise.
  A Float or Vec3 parameter with `animatable` can carry keyframe tracks (§3, ANIM.md §1.2).
- `MeshData { points: Vec<[f64; 3]>, polys: Vec<Vec<u32>> }` (serde, `PartialEq`): points
  and polygons as corner-index lists. core does not depend on forge-geometry; the
  conversion to `PolyMesh` lives in forge-generators (EDITABLE.md §1). `Value::Mesh`
  holds it in an `Arc` (cheap clones for undo; `PartialEq` compares contents).
  `ParamDesc::mesh(id, label)` has an empty mesh as default, is `hidden()` and
  `animatable(false)`.
- `Value::IndexList(Vec<u32>)`: element indices (point, edge or polygon numbers) for the component
  selections of EDITABLE.md §2. `ParamDesc::index_list(id, label)` has the empty list as
  default, is `hidden()` and `animatable(false)`. Core does not know what the indices refer to.
- Builders: `ParamDesc::float(id, label, default)`, `.int`, `.bool`, `.vec3`,
  `.color`, `.enumeration(id, label, default_id, &[(id, label)])`, `.string`,
  `.link(id, label, accepts)`, `.link_list(id, label, accepts)`, `.mesh(id, label)`, `.index_list(id, label)`, then `.range(min, max)`, `.unit(u)`, `.group(g)`,
  `.soft_range(a, b)`, `.step(s)`, `.hidden()`, `.animatable(bool)`.

`ParamDesc::validate(Value) -> Result<Value, ParamError>`:
1. Variant must match the kind, except an `Int` value is accepted for a `Float`
   kind (converted). Otherwise `ParamError::TypeMismatch`.
2. Non-finite floats → `ParamError::NotFinite`; negative colour component →
   `ParamError::OutOfRange`.
3. `Int`/`Float`/`Vec3` are **clamped** into `[min, max]` (hard range).
4. `Enum` id must be one of the options → else `ParamError::UnknownOption`.
5. `Link` targets are checked by the scene (needs the node), not here.
6. `Mesh`: every coordinate finite (`ParamError::NotFinite`); every polygon has at least 3
   corners with at least 3 distinct indices, and every index `< points.len()`
   (`ParamError::OutOfRange`). No clamping. A repeated index is allowed on purpose: the
   cone primitive's side quads at the apex repeat it (`[10, 11, 11, 21]`,
   tests/golden/cone), and Make Editable must keep the face lists exactly.
7. `IndexList`: the list is **sorted ascending and deduplicated** (not an error). No range
   check (the kind does not know the element count).
8. `LinkList`: the order is kept; a node twice in the list is `ParamError::DuplicateLink`.
   `Tx::set_param` checks every entry like a `Link` (exists, not the node itself, accepted
   category) and `param.set` takes an array of node ids. The default is the empty list and the
   parameter is not animatable. Targets count for `deep_revision`. Brief 068 added it for
   the clone toolset (MOGRAPH.md E1). The file version did not change for it at first; version 6
   (§6.4) declares it, and §6.3 says how files written in between load.

## 2. Node types and the plugin registry (`registry`)

`Category`: `Object, Generator, Deformer, Effector, Field, Tag, Material, Shader,
RenderEngine`. Hierarchy categories (live in the object tree): `Object, Generator,
Deformer, Effector, Field`. `Tag` nodes attach to a hierarchy node. The other
categories can be registered but not yet placed in a scene (`NotPlaceable`).

`NodeType { id, label, category, params: Vec<ParamDesc> }`, built with
`NodeType::new(id, label, category).param(desc)…`. `NodeType.in_create_menu` (default
true; `.not_in_create_menu()` clears it) says whether the UI's Create menu offers the type;
it has no other effect (such nodes can still be created by code and loaded from files).
- Type id: `namespace.name`, at least one `.`, chars `[a-z0-9_.-]`, no empty
  segments. Built-ins use namespace `forge`.

`Registry`:
- `register(NodeType) -> Result<(), RegistryError>`; errors: `InvalidTypeId`,
  `DuplicateType`, `InvalidParamId`, `DuplicateParam`, `InvalidDefault` (default
  fails `validate` or changes under it, i.e. it was out of range), `InvalidEnum`.
- `install(&dyn Plugin)`: atomic — if any registration fails, none of that plugin's
  types are added.
- `get(id)`, `types()` (sorted by id), `types_in(category)`.
- `Registry::with_builtins()` registers `forge.null` (Object; only the transform params).
  It also registers the `forge.smoothing` tag (docs/specs/SMOOTHING.md §2).
- Hierarchy types get `transform.pos/rot/scale` prepended; `transform.*` ids are
  reserved (`RegistryError::ReservedParam`), and so is `object.*` (visibility and enable
  flags, brief 149). See docs/specs/TRANSFORM.md §2.

`trait Plugin { fn id(&self) -> &str; fn register(&self, r: &mut Registry) -> Result<(), RegistryError>; }`

## 3. Scene graph (`scene`)

- `NodeId(u64)`: allocated from a per-scene counter, never reused within a scene's
  life (undo restores the original ids).
- `Node { id, type_id, name, params: BTreeMap<String, Value>, parent, children, tags,
  host, tracks: BTreeMap<String, Track>, rev }`. A new node gets every declared param at its default and the type's
  label as name.
- `Track { keys: Vec<Key> }`, `Key { time (seconds), value, interp: Spline|Linear|Step }`:
  the keyframe data of one Float parameter or one Vec3 component, addressed by a track
  key (`"<param>"` or `"<param>.<x|y|z>"`). Keys are sorted with unique times; values are
  in parameter units. Evaluation lives in forge-anim (ANIM.md §1).
- `Scene::with_params(overrides) -> (Scene, skipped)`: a copy with parameter values replaced,
  validated like `set_param` (an override that fails is skipped and returned); changed nodes
  get globally fresh revisions. Used by `forge_anim::scene_at`.
- Hierarchy nodes: `parent: Option<NodeId>` (None = scene root), ordered `children`.
  Tag nodes: `host: Some(id)`, listed in the host's ordered `tags`, never in the
  hierarchy.
- Read API: `node(id)`, `roots()`, `param(id, pid)`, `contains(id)`,
  `descendants(id)` (depth-first pre-order, excluding tags), `len()`.
- Nodes whose type is not in the registry ("unknown", from files written with
  plugins that are now missing) keep their raw params and round-trip unchanged;
  editing their params is an error.
- Links to nodes that no longer exist read as dangling; deleting a node never
  rewrites other nodes' links (keeps undo trivial).

## 4. Edits, transactions, undo (`document`)

All mutation goes through `Document { scene, history }`:

```rust
doc.transact("Add null", |tx| { let id = tx.create("forge.null", None, None)?; Ok(id) })
```

`Tx` operations (each validates first and changes nothing on error):

| Op | Rules |
|---|---|
| `create(type_id, parent, index) -> NodeId` | type registered, hierarchy category, parent exists & is a hierarchy node, `index ≤ len` (None = append) |
| `attach_tag(type_id, host, index) -> NodeId` | type is `Tag`; host is a hierarchy node |
| `delete(id)` | removes the node with its whole subtree and all their tags |
| `set_param(id, pid, value)` | known type, declared param, `validate` passes; `Link` target must exist, not be the node itself, and match `accepts` |
| `rename(id, name)` | |
| `set_track(id, key, Option<Track>)` | node exists; if `Some`, `key` names an animatable Float param or a component (`.x|.y|.z`) of an animatable Vec3 param of the node's type, and the track validates (non-empty, finite, strictly increasing times); `None` removes. Stamps the node |
| `set_key(id, key, Key)` / `remove_key(id, key, time)` | add or replace the key at exactly that time / remove it (the track vanishes with its last key; a missing track or key is a no-op) |
| `move_node(id, parent, index)` | hierarchy nodes only; new parent must not be the node or a descendant; index is into the list *after* removal |

Errors: `EditError` (`UnknownType`, `UnknownNode`, `UnknownParam`, `NotPlaceable`,
`WrongCategory`, `IndexOutOfRange`, `Cycle`, `InvalidLink`, `Param(ParamError)`, `Track(TrackError)`).

Transactions:
- If the closure returns `Err`, every edit it applied is reverted and history is
  unchanged.
- A transaction with no edits is not recorded.
- `undo()` / `redo()` return `false` when there is nothing to do; a new transaction
  clears the redo stack. Undo/redo restore node ids, order, names, params and tags
  exactly (scene content equality; revisions are not restored, see §5).
- `transact_merged(label, key, f)`: if the previous history action was a recorded
  transaction with the same `key`, the edits merge into it (one undo step, label of
  the first). Undo, redo, or any other transaction ends the merge window. For slider
  drags.
- `history().undo_labels()` lists labels oldest first.
- Grouping (SCRIPTING.md §2): `undo_depth()` is the number of undo steps.
  `group_undo_since(depth, label)` merges every step recorded after `depth` into one step
  with that label (edits in their original order); no steps newer than `depth` (or a stack
  shorter than `depth`, the history having been replaced) is a no-op; exactly one newer
  step is only renamed. `rollback_to(depth) -> bool` reverts every step
  above `depth` and drops them, clearing the redo stack if anything was reverted
  (false, nothing done, if the stack is shorter than `depth`). Both end the merge window.

## 5. Dirty tracking and evaluation cache (`cache`)

- The scene has a monotonically increasing revision counter. Every edit stamps the
  nodes it touches with a new revision: `set_param`/`rename` → that node; `create`/
  `attach_tag`/`delete`/`move_node` → the affected parent(s) or host (and new
  nodes).
- `scene.revision()`: the current value of that counter (any edit raises it).
- `scene.deep_revision(id) -> Option<u64>` = max of the node's own revision and the
  deep revisions of its children, its tags, and the nodes its `Link` params point to
  (cycle-safe; dangling links ignored). `None` if the node does not exist.
  Rationale: generators consume their children, tags and links.
- `EvalCache<T>`: `get_or_compute(&scene, id, |scene, id| -> T) -> Option<Arc<T>>`
  recomputes only when `deep_revision(id)` differs from the stored one. `purge(&scene)`
  drops entries for deleted nodes. `clear()`.
- Undo produces new revisions, so caches recompute after undo (conservative, correct).

### 5a. Evaluation order of one frame (area H, brief 108; refined by brief 111)

`forge-app` builds the evaluated scene of a frame in this order: (1) animation tracks,
`scene_at`; (2) expression tags, `forge_nodes::apply_expressions`, in document order (an
object, its tags in order, then its children), each tag seeing the writes of the earlier
ones; (3) generators and deformers, which read the evaluated scene when geometry is asked
for; (4) dynamics (later). An expression therefore overrides an animated value of the same
parameter and every generator sees the expression's output. Priorities between expression tags
and constraints/IK are brief 111. Details: NODES.md section 5.

### 5b. Dependency graph and evaluation scheduler (brief 180)

Source: own design (no reference-app observation). Code: `forge_core::depgraph` (graph,
diff), `forge_app::schedule` (scheduler), `forge_generators::eval_dependencies`,
`forge_fields::reads_time`, `forge_clone_engine::effector_reads_time`,
`forge_nodes::tag_reads_time`.

**Vertices.** Every node `n` has two vertices. `Frame(n)` is what flows *down* the
hierarchy: the `transform.*` params and the `object.*` visibility/enable params, composed
with the ancestors. `Result(n)` is the evaluated output of `n` (its world-space mesh).
Two vertices keep the parent/child relation acyclic: a child's result needs its parent's
frame, a parent's result needs its children's results.

**Edges** (`a -> b` means "`a` depends on `b`"):

| Edge | Rule |
|---|---|
| own frame | `Result(n) -> Frame(n)` (not for tags) |
| ancestor | `Frame(n) -> Frame(parent(n))` |
| child | `Result(n) -> Result(c)` for each child (generator inputs, deformer children, cloner prototypes, boolean A/B) |
| tag | `Result(n) -> Result(t)` for each tag |
| link | `Result(n) -> Result(t)` for each `Link` / `LinkList` param target that exists (effectors, field lists, instance reference, deformer `fields`) |
| extra | `Result(g) -> Result(d)` for the deformers of a geometry-less parent that also deform `g` (`forge_generators::eval_dependencies`, MODIFIERS.md section 0) |

**Dirty rule.** The scheduler compares the evaluated scene of this pass (tracks, then
expressions) with the previous one, bit-exactly (`f64` by bit pattern, so `-0.0 != 0.0`).
A changed node seeds `Frame(n)` if a `transform.*` or `object.*` param (or its parent)
changed, otherwise `Result(n)`; a changed type, child list or tag list seeds `Result(n)`.
Added and removed nodes seed their vertices. The dirty set is the set of vertices that
reach a seed over the edges, taken in the new graph and in the previous graph (so a
removed link or node still dirties its former dependants). A changed fps or seed dirties
everything. A pass whose scene revision, node count and time are unchanged does nothing.

**Time.** A frame change dirties (a) whatever the diff finds changed (animated tracks,
expression writes) and (b) every implicit time reader: a Random field in a noise mode
with `animation_speed != 0`, a Random effector in a noise/turbulence mode with
`animation_speed != 0`, and the nodes depending on them. The time-dependent set reported
by the scheduler is these readers plus nodes with tracks plus all their dependants. A scene
with no tracks, no readers and no enabled expression tag does no work at all on a frame
change.

**Expressions** stay one ordered stage (section 5a): the stage reruns as a whole whenever
the document changes or the frame changes and an enabled tag exists, in document order,
each tag seeing the earlier writes. Independence of tags is never assumed.

**Change set (brief 098).** When only the time moved (same document revision, node count,
fps and seed as the last pass) the tracks and expression stage does not copy the scene and
does not diff it. It reports what it wrote, `touched: Vec<(NodeId, param)>`: every
parameter a track override or an enabled tag's `ParamWrite` addressed (a write that leaves
the value as it was still counts). Starting from the stored document, only these pairs can
differ between two evaluated scenes of the same document, so the dirty seeds are found by
comparing, bit-exactly (`depgraph::identical`), the old and new value of the pairs touched
in the previous pass or in this one: a changed `transform.*` / `object.*` param of a
non-tag seeds `Frame(n)`, any other seeds `Result(n)`, the same rule as the diff. A
changed `Link` / `LinkList` value (edges may differ) falls back to the full diff. The
time-reader set is updated only for the nodes that changed (the readers are node-local),
the dependency graph is kept, the node lists (all nodes, nodes with tracks, expression tags)
are kept per document revision, and the drawn list is recomputed only when a dirty node or
one of its children can have changed its drawn state. The full diff stays the path for a
changed document (first pass, edit, undo, fps) and is the validator: `Scheduler::
with_validation(true)` runs it on every change-set pass and panics if the seeds differ
(all scheduler tests run with it on). `Stats::path` says which path a pass took
(`None`, `ChangeSet`, `Diff`).

**Sharing.** `Scene` stores nodes as `Arc<Node>`; cloning a scene clones one pointer per
node, and the mutating primitives (`Scene::apply_params`, the document edits) copy a node
on write (`Arc::make_mut`). `forge_app::schedule::animate` builds the evaluated scene as
`scene.clone()` plus the track overrides plus the tags' writes, so a frame copies only the
animated nodes and the nodes tags write to; every other node is the same allocation as in
the document (tested with pointer equality). `animate` equals
`apply_expressions(scene_at(scene, t), t)` (the tests compare the schedulers against the old
`Evaluated` path bit for bit, and `Evaluated` itself now goes through `animate`).

**Parallelism.** The units of work are the drawn nodes (enabled, editor-visible,
not consumed by a parent generator) that are dirty or have no result yet. Each is a pure
function of the immutable evaluated scene and the `EvalCtx`, so they run in parallel
(rayon `par_iter`, indexed collect) and the results are stored in node id order. No
evaluator keeps mutable shared state, and every random draw is a hash of
(seed, clone/point index), so output does not depend on thread scheduling: serial
(`Threads::Serial`) and parallel results are bit-identical (tests compare point bit
patterns). Cloner expansion also runs in parallel over clones (offsets by prefix sum,
per-clone arithmetic unchanged) once there are at least 4096 output points.

**Cycles.** Links can form cycles. A depth-first walk from the vertices in ascending
`(node id, Frame < Result)` order, following dependencies in ascending order, removes every
edge that points at a vertex on the current path; each removal is reported as a
`CycleBreak { from, to, kind, path }`. The remaining graph is acyclic and gives
`levels()` (0 = no dependencies). Dirty propagation uses the unbroken graph (cycles are
harmless for reachability). The scheduler returns the breaks in its stats; evaluation
itself does not follow links recursively beyond what the evaluators already guard.

**Outputs for drawing.** `Scheduler::with_instancing(true)` (the app's `draw_scheduler`,
`App::scheduled_draw`) stores a cloner that `cloner_instances` can show as
`NodeOutput::instances` instead of an expanded mesh (VIEWPORT.md section 10.8); every other
output is unchanged. `Frame` carries `drawn` (the drawn nodes in scene order) and
`generation`, which changes exactly when some output was recomputed or the drawn list
changed; a pass that reused everything keeps the previous generation. OBJ export keeps
`export_scene` on the evaluated scene of `Evaluated` (it exports hidden objects and
evaluates with the default context, so it is not the drawn set; only its animation stage is
shared).

**Benchmark** (`crates/app/benches/schedule.rs`, criterion): see the table below.

Measured on 4 cores, release bench profile (criterion, median of 10 samples, frame advanced
by one per iteration). "180" is the pass of brief 180 (scene copy plus bit-exact diff);
"098" the change-set pass:

| Scene | Path | Time | vs old full serial |
|---|---|---|---|
| 1 animated cube + 1000 static cubes | old path: animate, evaluate every drawn node | 29.0 ms | 1.0x |
| | scheduler, full evaluation, serial | 36.4 ms (180: 40.0) | 0.8x |
| | scheduler, full evaluation, 4 threads | 11.9 ms (180: 14.2) | 2.4x |
| | scheduler, dirty only, serial | **192 us** (180: 6.2 ms) | **151x** |
| | scheduler, dirty only, 4 threads | **101 us** (180: 6.6 ms) | **287x** |
| One cloner, 10 000 clones of a cube (full evaluation) | serial (1-thread pool) | 8.5 ms | 1.0x |
| | 4 threads (parallel expansion) | 5.6 ms | 1.5x |

The target of brief 098 (playback at least 10x the full-evaluation rate) is met by more
than 15x. What is left of the 190 us is cloning the node map (1001 pointers), the output
map, one dirty-set walk and the bookkeeping; it still grows with the node count but at
microseconds per thousand nodes, not milliseconds. A single node cannot be spread over
nodes, so the 10k cloner gains only from the parallel expansion (effectors stay serial: the
Random effector's draws are index-keyed hashes but the effector trait takes a sequential
closure). The old path's variance between runs is about 10 %.

## 6. Native scene file (`file`)

RON text, deterministic (same scene → byte-identical output):

```ron
(
    format: "forge3d-scene",
    version: 7,
    settings: (fps: 30, start: 0, end: 90),
    time: (preview: None, mode: Loop, drop_frames: true, display: Frames, markers: []),
    roots: [
        (id: 1, type_id: "forge.null", name: "Null", params: {}, tracks: {}, tags: [], children: []),
    ],
)
```

- `save(&Scene) -> String`; params serialized from a `BTreeMap` (sorted keys); every
  param is written, defaults included.
- `load(Arc<Registry>, &str) -> Result<(Scene, LoadReport), LoadError>`.
  - `LoadError`: `Parse` (with message), `WrongFormat`, `UnsupportedVersion`
    (> 5), `DuplicateId`, `Structure` (known Tag type in `children`, or known
    non-Tag type in `tags`).
  - Recoverable issues go to `LoadReport.warnings` (strings naming the node id):
    unknown type (node kept as-is), unknown param on a known type (dropped), invalid
    value (type mismatch → default; out of range → clamped).
  - Missing params on known types are filled with defaults, silently.
  - A saved scene keeps its values when a type's descriptor defaults change (the R25 switch,
    docs/parity/r25-defaults.md): every param is written, so only a param missing from the
    file takes the new default. A saved value outside a changed range is clamped on load.
  - The id counter resumes at max id + 1.
- `save(load(save(s)))` equals `save(s)`.
- **Version 2** added `Value::Mesh` (written as `Mesh((points: [...], polys: [...]))`, points
  in order, so output stays deterministic). Version 1 files have no mesh values and still
  load unchanged; version 2 ended here.
- **Version 3** added `Value::IndexList` (written as `IndexList([0, 3, 5])` in RON, ascending; the pretty
  printer puts one index per line). A
  version 1 or 2 file has no such values and loads unchanged (missing params get their
  defaults); `save` wrote version 3 until version 4. The bump is for older builds: they reject a
  version 3 file with `UnsupportedVersion` instead of a parse error on the unknown variant.
- **Version 4** added keyframe tracks: each node record has `tracks: { "<track key>": (keys:
  [(time: 0.0, value: 1.0, interp: Spline), ...]) }` (sorted by track key, keys in time
  order; `Interp` is `Spline | Linear | Step`). `save` always writes version 4 with
  `tracks` on every node (empty `{}` included); a version 1–3 file has no `tracks` field
  and loads unchanged. On load a track whose key does not address an animatable Float
  param / Vec3 component of the node's (known) type, or that does not validate, is dropped
  with a warning naming the node and the key; tracks of unknown types are kept as-is.
  Older builds reject a version 4 file with `UnsupportedVersion`.
- **Version 5** added scene settings: `settings: (fps: 30, start: 0, end: 90)` at the top
  level (ANIM.md §2.1). `save` always writes it; a version 1-4 file has none and loads with
  the defaults; saved settings that do not validate (`fps` outside 1..=240, `start > end`)
  load as the defaults with a warning. Older builds reject a version 5 file with
  `UnsupportedVersion`.

### 6.1 Layout, header and the two entry points (brief 163)

A native file is one of:
- **Text file**: the RON document above and nothing else. This is the form of every file of
  versions 1-5 and of any version 6 file whose large values are small enough (§6.2). It is
  deterministic and diff-friendly; `save(&Scene) -> String` always writes this form.
- **Chunked file**: the same RON document, then the marker line `\n%%FORGE-CHUNKS 1\n`, then
  the chunk section. Text writers escape line breaks inside strings, so the first occurrence
  of the marker is always the real one. The header is the first two RON fields,
  `format: "forge3d-scene"` and `version: N`; a reader needs nothing else to pick the
  migration path.

API (`forge_core::file`):
- `save(&Scene) -> String` and `load(Arc<Registry>, &str)`: text only. `load` fails with
  `LoadError::Chunk` if the text refers to a chunk (it has no chunk section).
- `save_with(&Scene, &SaveOptions) -> Vec<u8>` and `load_with(Arc<Registry>, &[u8],
  &LoadOptions) -> Result<(Scene, LoadReport), LoadError>`: text or chunked files, external
  references (§6.5). `SaveOptions { base_dir: Option<PathBuf>, chunk_min_bytes: usize }`
  (default: no base dir, `CHUNK_MIN_BYTES`); `LoadOptions { base_dir: Option<PathBuf>,
  check_external: bool }` (default: none, false).
- `load_bytes(Arc<Registry>, &[u8])` = `load_with` with the default options. **It is the
  fuzz entry point (brief 182)**: a pure function of its bytes (no file or clock access),
  total (every input gives `Ok` or `Err`, never a panic, never an allocation larger than the
  input allows: every count is checked against the bytes that remain before anything is
  allocated), and `save_with(load_bytes(b))` is valid input again.
- The file commands (`file.open`, `file.save`, autosave) use `save_with`/`load_with` with
  `base_dir` set to the file's directory.

### 6.2 Binary chunks

Only values whose binary payload is at least `CHUNK_MIN_BYTES` = 1024 bytes go into a
chunk, and only the kinds below. Everything smaller stays inline text, so small scenes and
all hand-edited files stay readable and a diff of a big scene still shows its structure
(names, params, tracks); only the large arrays become opaque. 1024 bytes is 42 mesh points,
or 256 indices: about where the text form (roughly 3-4 times the binary size) starts to
dominate a file.

In the RON, the value is replaced by `MeshChunk(i)` or `IndexListChunk(i)` (`i` = chunk
number, 0-based, numbered in the order the nodes and their params are written, so the file
stays deterministic). Chunk section, all integers little-endian:

```
count: u32
count x { kind: u8, len: u64, hash: u64 (FNV-1a 64 of the payload), payload: len bytes }
```
Kinds: `1` mesh = `points: u32`, `polys: u32`, `points x 3 x f64` (raw IEEE bits), then per
polygon `corners: u32` and that many `u32` indices; `2` index list = `n: u32`, `n x u32`.
Values are stored exactly as in memory, so a round trip is bit-identical for every f64
(`-0.0` and subnormals included; the text form also keeps the bits, via the shortest
round-trip printing of the `ron` crate, which the tests check). Other kinds are reserved for
per-point data and caches: a new large value kind gets the next kind number and a version
bump. Errors (all `LoadError::Chunk`): marker without a valid section, a chunk running past
the end, a hash mismatch, a trailing byte after the last chunk, an unknown kind, a
reference to a missing chunk, a chunk of the wrong kind for the reference. Unreferenced chunks
are ignored with a warning. Validation of the loaded values (mesh indices, finite floats) is
the normal load validation of §1.

### 6.3 Migrations

`VERSION` is the version written; `MIGRATIONS[v - 1]` is the function that turns a version
`v` document into a version `v + 1` document (a pure function of the parsed document that
returns warnings for anything it had to drop; they go to the load report). A file of version `v < VERSION` runs steps `v` ...
`VERSION - 1` in order, then loads like a current file. Version 0 and versions above `VERSION`
are `UnsupportedVersion`. Rules for a new bump: append one function, raise `VERSION`, add a
fixture `crates/core/tests/fixtures/scene-v<old>.forge` that the previous build would have
written (never regenerated), and extend the fixture test. The tests check that the table has
one step per version and that every version from 1 to `VERSION` has a fixture that loads to
the same logical scene.

| Step | What changed | What the step does |
|---|---|---|
| 1 -> 2 | `Value::Mesh` added | nothing (additive) |
| 2 -> 3 | `Value::IndexList` added | nothing (additive) |
| 3 -> 4 | keyframe `tracks` per node | nothing (a missing `tracks` is empty) |
| 4 -> 5 | scene `settings` | nothing (absent settings read as the defaults) |
| 5 -> 6 | see §6.4 | nothing (additive) |
| 6 -> 7 | scene `time` (§6.6) | nothing (an absent `time` reads as the defaults) |
| 7 -> 8 | `forge.expression` tag (§6.7) | nothing (the graph is a string parameter) |

Missing params of known types take their defaults on load (§6), so params added by later
builds need no step.

### 6.4 Version 6

Declares what builds between brief 068 and brief 163 already wrote under version 5:
`Value::LinkList` (brief 068, this paragraph was a known gap) and the hidden `object.*`
params (brief 149; a file without them loads with the defaults). A version 5 file that
contains `LinkList(...)` values is a valid file and migrates unchanged. Version 6 also
introduces the chunk section (§6.2) and file reference params (§6.5). Older builds reject a
version 6 file with `UnsupportedVersion`.

### 6.5 External file references

A parameter is a file reference when its descriptor says so (`ParamDesc::file_path(id, label,
default)`, `UiHint.file_ref`); its value is a `Value::String`, empty for "none". In memory the
path is whatever the user gave, normally absolute.
- **Save** with `base_dir`: an absolute path is written relative to `base_dir` with `/`
  separators and `..` segments when needed (lexical, no file system access), so a project folder
  can be moved or shared. A path that cannot be made relative (another drive or root) or that
  is already relative is written as is.
- **Load** with `base_dir`: a relative path is joined to `base_dir` and normalised lexically
  (`.` and `x/..` removed); the param then holds that absolute path. With `check_external` a
  path that does not exist adds the warning `node N: external file "..." not found` (the value
  is kept, so a later save keeps the link). Without `base_dir`, values are not touched.
- `file::external_refs(&Scene) -> Vec<ExternalRef { node, param, path }>` lists the references
  (for "collect files" and missing-file reports).
- Nodes of unknown types keep their values untouched.

### 6.6 Version 7: time configuration (brief 094)

Adds `time: (preview: .., mode: .., drop_frames: .., display: .., markers: [..])` at the top
level, after `settings` (ANIM.md §2.9). `preview` is `None` or `Some((start, end))`; `mode` is
`Once | Loop | PingPong`; `display` is `Frames | Smpte | Seconds`; a marker is `(id: 1, frame:
10, name: "Hit", color: (224, 160, 48))`. Every field has a serde default, so a version 1-6 file
loads with `TimeConfig::default()` and step 6 -> 7 has nothing to rewrite. `save` always writes
the field. A `time` block that does not validate (preview order or outside the project range,
duplicate marker ids) loads as the defaults with a warning, like `settings`. Older builds reject a
version 7 file with `UnsupportedVersion`. The fixture `scene-v7.forge` pins the bytes.

### 6.7 Version 8: expression tag (brief 108)

Declares the `forge.expression` tag type (NODES.md section 5). Its graph is the hidden string
parameter `graph` (JSON), so the file schema does not change and step 7 -> 8 has nothing to
rewrite; a version 7 file loads unchanged and saves as version 8. Older builds reject a version 8
file with `UnsupportedVersion` instead of loading a scene whose expressions they could not run.
The fixtures `scene-v8*.forge` pin the current bytes; `scene-v7*.forge` stay as the old build wrote them.

### 6.8 Version 9: layers (brief 158)

Adds `layers: [(id: 12, name: "Layer 1", color: (r, g, b), solo: false, view: true, render: true,
manager: true, locked: false, animation: true, generators: true, deformers: true, expressions:
true)]` at the top level, after `time` (section 7). The field has a serde default and is not
written while the list is empty, so a scene without layers saves exactly as a version 8 scene
does apart from the version number, and step 8 -> 9 has nothing to rewrite (a version 1-8 file has
no layers). Every hierarchy type also gets the hidden `object.layer` link param (section 7.1);
a file written before it loads with the default (no layer). On load: a layer id that repeats or
equals a node id drops that layer, a `object.layer` link that names no layer is cleared; both with
a warning. The id counter starts above every node and layer id. Older builds reject a version 9
file with `UnsupportedVersion`. The fixtures `scene-v9*.forge` pin the current bytes (they hold a
layer and a member); `scene-v8*.forge` stay as the old build wrote them.

## 7. Layers (`layers`; brief 158)

Sources: own design from docs/roadmap/areas/L-ui.md; GUI session G1 (docs/parity/GUI-SESSIONS.md)
is unanswered, so **the toggle set's meaning (what "solo" hides, whether a layer reaches the
children of its members, what "generators off" leaves) is parity-pending**.

### 7.1 Data

A layer is document level data, not a tree node: `Layer { id, name, color, solo, view, render,
manager, locked, animation, generators, deformers, expressions }`, kept in order of creation in
`Scene::layers() -> &Layers`. `id` is a `NodeId` taken from the scene's id counter, so a layer id
never equals a node id. Defaults of a new layer: `solo` off, `locked` off, every other toggle on;
`color` is taken from an 8-colour palette by the layer count. A name is non-empty after trimming
and need not be unique.

Objects link to at most one layer through the hidden param `object.layer` (`Link`, default none,
in the reserved `object.` group, hidden from the attribute manager like the other `object.*`
params). A tag has no param of its own: it belongs to its host's layer. Membership is the node's
**own** link; children do not inherit it (parity-pending). `Tx::set_param` accepts for this
param only an existing layer id (or none); the link is not a node reference, so it is skipped by
the dependency graph and by copy remapping and a copy of an object stays on the layer.

### 7.2 Edits

The table is replaced as a whole by one primitive edit `SetLayers`, so undo restores it exactly.
`Tx` offers `add_layer(name) -> LayerId`, `update_layer(Layer)`, `remove_layer(id)` (clears the
`object.layer` of every member in the same transaction) and `merge_layers(from, into)` (members
of each `from` layer move to `into`, the `from` layers go). Assigning is `set_param`.

### 7.3 Resolution (`forge_core::layers`, all pure)

| function | value for a node whose own layer is `L` (none = no layer) |
|---|---|
| `shown(scene, id)` | `L.view` and (no layer is solo, or `L.solo`); a node without a layer is shown iff no layer is solo |
| `render_shown` | `L.render` (none: true); solo does not affect rendering |
| `locked` | `L.locked` |
| `in_manager` | `L.manager` |
| `animation_on`, `generators_on`, `deformers_on`, `expressions_on` | the toggle (none: true) |

`visibility::editor_visible` is its old value and `layers::shown`; `render_visible` is its old
value and `layers::render_shown` (TRANSFORM.md §2). Hiding by layer does not change the
`object.*` params.

### 7.4 How the toggles take effect

- **view, solo**: through `editor_visible`, so the viewport, snapping and the scheduler's drawn
  list skip the node (the OBJ export exports hidden objects as before, brief 098).
- **render**: `render_visible` (the render bridge does not read it yet, brief 149).
- **locked**: the viewport cannot pick the object (`pick_object`, `viewport_click`) and does not
  start a gizmo, axis or component drag on it, and a locked polygon object is not a component
  target. Object manager selection, attribute editing and commands still work (parity-pending:
  the reference may also block those).
- **animation off**: `forge_anim::overrides` skips the node (and its tags), so its tracks are not
  evaluated and it keeps its stored values at every time.
- **expressions off**: an expression tag whose host is on the layer does not run.
- **deformers off**: `apply_deformer` leaves the points alone for a deformer on the layer (the
  deformer's own `enabled` is untouched).
- **generators off**: a `Category::Generator` node on the layer is pass-through: it produces no
  geometry of its own and the objects it consumes as inputs (subdivision, boole, cloner
  prototypes, modeling and spline generators) are no longer consumed, so they are drawn as
  themselves; its instances are not drawn.
- **manager off**: the object manager omits the row (its children are still listed, at the same
  depth).

### 7.5 Scheduling and dirtiness (briefs 180, 098)

The layer table is part of the scene, so editing it raises the revision and the next pass is a
diff pass. `depgraph::diff` additionally compares, when the two tables differ or a node's
`object.layer` differs, the node's effect `layers::effect(scene, id)` (shown, render, animation,
generators, deformers, expressions bits; a tag uses its host's) and seeds the frame and result
vertices of every node whose effect changed, so dependants (parents, cloners reading the node,
hosts of changed tags) are re-evaluated and nothing else is. A toggle that does not change any
node's effect (a lock, a colour, a rename) seeds nothing. The animation stage reads the toggles
directly: with animation off the evaluated scene holds the stored values, which the diff sees as
param changes.

### 7.6 Tests

`crates/core/tests/layers.rs` (table edits and undo, resolution, link validation, delete and merge,
file round trip, bad layer data, the dependency diff), `crates/core/tests/native_format.rs` (v8
fixtures migrate to 9 without layers, v9 fixtures byte stable), `crates/app/tests/layers.rs` (the
commands, one undo step each, each toggle's effect on evaluation, scheduler dirtiness against a
fresh full evaluation, a seeded random edit sequence), `crates/ui/tests/layer_manager.rs` (UI.md §22).
