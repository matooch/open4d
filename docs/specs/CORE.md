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
