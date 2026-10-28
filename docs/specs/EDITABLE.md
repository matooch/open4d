# Editable polygon objects (`crates/generators/src/polygon.rs`)

Sources: own design (brief docs/agents/tasks/03-polygon-object-make-editable.md). The
observation docs/parity/make-editable.md has no data yet, so **every rule below that says
what the commands do is parity-pending**; the geometry itself is checked against the
primitive golden cases (§1.5). Mesh values: CORE.md §1; file format: CORE.md §6.

## 1. Polygon object, Make Editable, Current State to Object

### 1.1 The polygon object (`forge.polygon`)
- Label "Polygon Object", `Category::Object`, so it gets the transform params and can
  carry tags, children and deformers. Registered by `Primitives::register`.
- Params: the transform, and `mesh` (`ParamKind::Mesh`, hidden, not animatable; default:
  the empty mesh). `NodeType.in_create_menu` is false: the Create menu does not offer it
  (there is nothing to edit until the modeling tools of briefs 05 and 06 exist).
- `evaluate_geometry` returns the stored mesh (`MeshData` to `PolyMesh`, same point and
  polygon order). `evaluate` then applies the deformer children as for any other object
  (MODIFIERS.md §0), and a Null parent passes its deformers on to a polygon object like
  to a generator.
- `forge_generators::{to_poly_mesh, from_poly_mesh}` convert between `MeshData` and
  `PolyMesh`; both keep order and values exactly.

### 1.2 Make Editable (`make_editable(tx, id) -> Result<NodeId, EditableError>`) — parity-pending
For a node for which `evaluate_geometry` builds a mesh:
1. A new `forge.polygon` is created at the source's place (same parent, same index). It
   gets the source's name, the `transform.*` params, and a copy of every tag in order
   (type, name and params; the smoothing tag included).
2. Its `mesh` is `evaluate_geometry`'s result: **deformers are not applied**.
3. Children of the source: those the generator consumed (`is_consumed`) are deleted;
   every other child (deformers, other objects) is moved under the new polygon object,
   keeping their order and local params.
4. The source is deleted. The function returns the new id.
- A `forge.polygon` is already editable: nothing changes, its own id is returned.
- A node without a mesh (Null, spline, deformer, a generator with no usable input) gives
  `EditableError::NoMesh(name)` before anything is edited. A tag gives `NoMesh` too.
  Editable splines are out of scope.
- A generator whose build returns `BuildError::Unsupported` gives `EditableError::Build`.
- Links from other nodes to the replaced node are not rewritten (**parity-pending**).

### 1.3 Current State to Object (`current_state_to_object(tx, id) -> Result<NodeId, EditableError>`) — parity-pending
- A new `forge.polygon` is inserted directly after the source (same parent, index + 1),
  named like the source, with the source's `transform.*` params and copies of its tags.
- Its `mesh` is `evaluate`'s result: the source's deformers **are applied**. The source
  and its children are untouched; the new object has no children.
- The mesh generators produce exactly one mesh each, so there is nothing to merge; if a
  generator with several outputs appears, the outputs must be merged into one mesh.
- Same errors as §1.2. Known limit: a deformer that deforms the source as a *sibling*
  (a deformer under a Null parent) also deforms the new object, because it sits under
  the same parent: the baked deformation is applied twice (**parity-pending**).

### 1.4 Commands (UI.md §5)
`Command::MakeEditable` and `Command::CurrentStateToObject` act on the selected nodes in
ascending id order, inside one transaction (one undo step, labels "Make Editable" and
"Current State to Object"). A node that no longer exists (it was a consumed child of an
earlier selected node) is skipped silently. A `NoMesh` node is skipped and its message
`"<command>: <name> has no mesh"` goes to `EditorState.status`; any other error aborts and
reverts the whole step. Afterwards the selection is the new objects (for Make Editable on
a `forge.polygon`, the polygon itself).

### 1.5 Checks
- Every primitive golden case that matches `tests/golden/<feature>/` in
  `crates/generators/tests/parity.rs` also matches after Make Editable, with the same
  comparison mode (`crates/generators/tests/editable.rs`).
- Deformer child kept and still applied; Current State to Object bakes the deformer and
  leaves the source; one undo restores the original; consumed children removed.
- Save and load keep the mesh (CORE.md §6).
