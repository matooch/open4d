# Phase 1 — core (`crates/core`, package `forge-core`)

Sources: project architecture, not reference-app behaviour. Implements the CLAUDE.md
conventions ("everything is a plugin-registered node"; "parameters are data-driven
descriptors so UI, undo, animation and serialization derive from one definition";
deterministic behaviour) and the `core` line of docs/ARCHITECTURE.md. No
reference-app observation is involved. Behaviour that must match the reference app
(rotation order, units, default values of real node types) is out of scope here and
needs its own docs/parity/ observation first.

Out of scope for Phase 1: transforms/matrices (need docs/parity/transform.md;
Phase 2), materials and shader graphs (Phase 9), animation tracks (Phase 11),
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

`ParamDesc { id, label, kind, default, ui, animatable }`:
- `id`: non-empty, chars `[a-z0-9_.]`, unique within its node type. Stable forever:
  it is the serialization key.
- `kind: ParamKind`: `Bool`, `Int { min, max }`, `Float { min, max }`,
  `Vec3 { min, max }` (per component), `Color`, `Enum { options }` (each option
  `{ id, label }`, ids unique, at least one), `String`, `Link { accepts }` (list of
  categories; empty = any), `Mesh`.
- `ui: UiHint { group, unit, soft_min, soft_max, step, hidden }`. `unit` ∈
  `None | Distance | Angle | Percent | Time`. UI-only; never affects validation.
- `animatable`: defaults to true for `Int/Float/Vec3/Color/Bool`, false otherwise.
- `MeshData { points: Vec<[f64; 3]>, polys: Vec<Vec<u32>> }` (serde, `PartialEq`): points
  and polygons as corner-index lists. core does not depend on forge-geometry; the
  conversion to `PolyMesh` lives in forge-generators (EDITABLE.md §1). `Value::Mesh`
  holds it in an `Arc` (cheap clones for undo; `PartialEq` compares contents).
  `ParamDesc::mesh(id, label)` has an empty mesh as default, is `hidden()` and
  `animatable(false)`.
- Builders: `ParamDesc::float(id, label, default)`, `.int`, `.bool`, `.vec3`,
  `.color`, `.enumeration(id, label, default_id, &[(id, label)])`, `.string`,
  `.link(id, label, accepts)`, `.mesh(id, label)`, then `.range(min, max)`, `.unit(u)`, `.group(g)`,
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
  reserved (`RegistryError::ReservedParam`). See docs/specs/TRANSFORM.md §2.

`trait Plugin { fn id(&self) -> &str; fn register(&self, r: &mut Registry) -> Result<(), RegistryError>; }`

## 3. Scene graph (`scene`)

- `NodeId(u64)`: allocated from a per-scene counter, never reused within a scene's
  life (undo restores the original ids).
- `Node { id, type_id, name, params: BTreeMap<String, Value>, parent, children, tags,
  host, rev }`. A new node gets every declared param at its default and the type's
  label as name.
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
| `move_node(id, parent, index)` | hierarchy nodes only; new parent must not be the node or a descendant; index is into the list *after* removal |

Errors: `EditError` (`UnknownType`, `UnknownNode`, `UnknownParam`, `NotPlaceable`,
`WrongCategory`, `IndexOutOfRange`, `Cycle`, `InvalidLink`, `Param(ParamError)`).

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

## 6. Native scene file (`file`)

RON text, deterministic (same scene → byte-identical output):

```ron
(
    format: "forge3d-scene",
    version: 2,
    roots: [
        (id: 1, type_id: "forge.null", name: "Null", params: {}, tags: [], children: []),
    ],
)
```

- `save(&Scene) -> String`; params serialized from a `BTreeMap` (sorted keys); every
  param is written, defaults included.
- `load(Arc<Registry>, &str) -> Result<(Scene, LoadReport), LoadError>`.
  - `LoadError`: `Parse` (with message), `WrongFormat`, `UnsupportedVersion`
    (> 2), `DuplicateId`, `Structure` (known Tag type in `children`, or known
    non-Tag type in `tags`).
  - Recoverable issues go to `LoadReport.warnings` (strings naming the node id):
    unknown type (node kept as-is), unknown param on a known type (dropped), invalid
    value (type mismatch → default; out of range → clamped).
  - Missing params on known types are filled with defaults, silently.
  - The id counter resumes at max id + 1.
- `save(load(save(s)))` equals `save(s)`.
- **Version 2** added `Value::Mesh` (written as `Mesh((points: [...], polys: [...]))`, points
  in order, so output stays deterministic). Version 1 files have no mesh values and still
  load unchanged; `save` always writes version 2.
