# 03 · Polygon object, Make Editable, Current State to Object

Status: ready · Branch: `batch-08` · Planner: main session · Implementer: Claude Sonnet 5.5 (local, macOS)

## Goal
An editable polygon object exists: it stores its own mesh, saves and loads, takes
deformers, renders in the viewport. Two commands turn any generator into one:
**Make Editable** (replace in place) and **Current State to Object** (new object, deformers
applied, original kept). This is the base every modeling tool (briefs 05, 06, batch 11)
builds on.

## Inputs
- `docs/specs/CORE.md` §1 (values, descriptors), §4 (transactions), §6 (file).
- `docs/specs/UI.md` §5 (commands).
- `crates/generators/src/lib.rs` (`evaluate`, `evaluate_geometry`, `deformer_children`,
  `is_consumed`).
- Golden cases: every case under `tests/golden/{plane,cube,sphere,cylinder,cone,disc,torus,tube,capsule,pyramid}/`
  (used only to check that Make Editable keeps the exact mesh).
- Observation `docs/parity/make-editable.md` (O3): **may still be empty.** Where it is,
  use the rules below and mark them parity-pending. Don't wait for it.

## Design decisions (planner)
1. **Mesh data is a parameter value.** Add to `forge-core::param`:
   `pub struct MeshData { pub points: Vec<[f64; 3]>, pub polys: Vec<Vec<u32>> }`
   (serde, `PartialEq`) and `Value::Mesh(Arc<MeshData>)` (enable serde's `rc` feature in
   `crates/core/Cargo.toml`). `ParamKind::Mesh`; `ParamDesc::mesh(id, label)` is
   `hidden()` and `animatable(false)`. `validate`: every index `< points.len()`, every
   poly has ≥ 3 corners and no repeated index, all coordinates finite; otherwise
   `ParamError`. core stays free of `forge-geometry` (geometry doesn't depend on core);
   conversions `MeshData ↔ PolyMesh` live in `forge-generators` (`fn to_poly_mesh`,
   `fn from_poly_mesh`).
2. **Node type** `forge.polygon`, label "Polygon Object", `Category::Object`, in
   `forge-generators` (new file `src/polygon.rs`), registered by `Primitives::register`.
   Params: the usual transform (whatever every object gets) and `mesh` (default: empty
   mesh). `evaluate_geometry` returns the stored mesh; deformer children apply through
   the existing `evaluate` path. Not in the Create menu (it has nothing to edit until
   brief 05); hide it there the way other non-placeable types are hidden, or add a
   `NodeType` flag if none exists (say which in the report).
3. **File format**: bump `file::VERSION` to 2. Version-1 files still load (they have no
   mesh values); version > 2 is `UnsupportedVersion`. RON of a `Value::Mesh` must stay
   deterministic. Update CORE.md §6.
4. **Commands** in `crates/ui/src/commands.rs`: `Command::MakeEditable` and
   `Command::CurrentStateToObject`, both acting on the selected nodes, each **one undo
   step** for the whole selection, both in an Edit (or new "Mesh") menu. Put the scene
   logic in a pure function in `forge-generators` (`pub fn make_editable(tx, id)` /
   `pub fn current_state_to_object(tx, id) -> NodeId`) so it's testable without UI.
5. **Rules (own design, parity-pending until O3 is observed):**
   - Make Editable on a node that `evaluate_geometry` builds a mesh for: a
     `forge.polygon` with the same name, transform, tags (smoothing included) and tree
     position replaces it. The mesh is `evaluate_geometry`'s (**deformers not
     applied**); deformer children stay as children. Children the generator consumed
     (`is_consumed`) are deleted; other children are kept.
   - Current State to Object: a new `forge.polygon` named like the source, inserted
     **directly after** it under the same parent, same transform and tags (copied), mesh =
     `evaluate` (deformers applied), no children. The source is untouched. On a
     generator whose output is several meshes (if any exist), merge into one.
   - Nodes that make no mesh (Null, splines, deformers, tags): skip them, and put a
     status message in `EditorState.status` ("Make Editable: <name> has no mesh").
     Editable splines are out of scope.
   - A `forge.polygon` is already editable: Make Editable leaves it alone.

## Hints (planner)
- Look at how `Tx` creates/deletes/moves nodes and attaches tags in
  `crates/core/src/document.rs`; copying a tag = `attach_tag` + `set_param` per param.
- The free parity check: for every primitive golden case, build the primitive from its
  `case.toml` (copy the `apply` helper approach of `crates/generators/tests/parity.rs`),
  run `make_editable`, evaluate the polygon object, compare with `compare_meshes`
  against `mesh.obj`. Every case that passes today in `parity.rs` must pass here too (the
  ones that are geometric-only there stay geometric here).
- Arc equality: `PartialEq` on `Arc<MeshData>` compares contents, which is what undo and
  the file round-trip tests need.

## Done when
- [ ] CORE.md §1 and §6 updated (Mesh value, version 2); new spec section
      `docs/specs/EDITABLE.md` §1 (polygon object, both commands, the rules above with
      **parity-pending** markers, citing make-editable.md when it has data).
- [ ] Tests: `crates/core/tests/params.rs` (Mesh validation), `crates/core/tests/file.rs`
      (v2 round-trip with a mesh, v1 still loads), new
      `crates/generators/tests/editable.rs` (every primitive golden case survives Make
      Editable; deformer child kept and still applied; Current State to Object bakes the
      deformer; one undo step restores the original; consumed children removed),
      a UI test in `crates/ui/tests/editing.rs` running both commands.
- [ ] `sh scripts/gate.sh` prints `GATE: all passed`.
- [ ] `docs/parity/STATUS.md` row "Polygon object + Make Editable".
- [ ] Committed on `batch-08` with the trailer below; report written.

Commit trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`

## Out of scope
Component selection and editing (05, 06), editable splines, OBJ import (04), any change
to golden data or observation files.

## Report (implementer fills in)
