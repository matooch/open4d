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
- Every `Link` param of any other node (tags included, e.g. an Instance's `reference`)
  that points to the replaced node is set to the new polygon object in the same step
  (**parity-pending**). A link whose `accepts` list excludes the `Object` category makes
  the whole step fail and revert.

### 1.3 Current State to Object (`current_state_to_object(tx, id) -> Result<NodeId, EditableError>`) — parity-pending
- A new `forge.polygon` is inserted directly after the source (same parent, index + 1),
  named like the source, with the source's `transform.*` params and copies of its tags.
- Its `mesh` is `evaluate`'s result: the source's deformers **are applied**. The source
  and its children are untouched; the new object has no children.
- The mesh generators produce exactly one mesh each, so there is nothing to merge; if a
  generator with several outputs appears, the outputs must be merged into one mesh.
- Same errors as §1.2.
- **Sibling deformers (parity-pending).** If the source gets deformers from its parent
  (the parent has no geometry of its own and has deformer children, MODIFIERS.md §0, e.g.
  Null > [Cube, Bend]), the new object would be deformed a second time under that parent.
  So it is placed directly after the **parent**, under the grandparent (or at root level),
  gets the **parent's** local `transform.*` params (name and tags still come from the
  source), and its mesh is the deformed points taken into the parent's space with the
  source's local matrix: `M_src · p`. World points are then identical to the source's
  deformed world points. (A deformer-carrying, geometry-less grandparent would again
  deform it; not handled.)

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

## 2. Component modes and selection (`crates/ui/src/components.rs`)

Sources: own design (brief docs/agents/tasks/05-component-modes-selection.md). No
observation of the reference app exists yet for modes or selection, so **every rule in
this section is parity-pending** (brief 10 observes them). Geometry used: picking and
rectangle selection in VIEWPORT.md §2, selection sets and conversions in GEOMETRY.md §4,
edge ids in GEOMETRY.md §1.

### 2.1 Storage
Three params on `forge.polygon`, all `ParamKind::IndexList`, hidden and not animatable,
default empty:

| id | holds |
|---|---|
| `selection.points` | point indices of the stored mesh |
| `selection.edges` | edge ids, as numbered by `PolyMesh::edges()` of the stored mesh (GEOMETRY.md §1) |
| `selection.polys` | polygon indices of the stored mesh |

`Value::IndexList(Vec<u32>)`: validation sorts ascending and removes duplicates (CORE.md
§1). The three sets are independent: changing one never changes another, and switching the
mode converts nothing. Indices refer to the **stored** mesh. Deformers do not change
topology (MODIFIERS.md §0), so the same indices address the evaluated mesh. An index at or
past the element count (the mesh was edited later) is ignored everywhere it is read.

Every change is one `doc.transact("Select", ...)` writing the one set of the current mode,
so it undoes, redoes and saves like any other edit. A change that would leave the set equal
to the stored one writes nothing (no empty undo step; an empty click on an empty selection
is a no-op). One rectangle drag is one transaction, written when the button is released.
The file keeps the sets (CORE.md §6: version 3; older files load with empty sets).

### 2.2 Mode
`EditMode { Model, Points, Edges, Polys }`, `EditorState.mode`, default `Model`. New and Open
reset it to `Model`. The mode is editor state, not document state: it is not saved and not
undoable.

The **component target** is the active node when `mode != Model` and the active node is a
`forge.polygon`. Otherwise (Model mode, nothing active, or an active node of another type)
there is no target and viewport clicks pick objects exactly as in Model mode (MANIPULATE.md
§5). Component modes never touch other objects' selections.

### 2.3 Picking space
The pick mesh is `forge_generators::evaluate` of the target (deformers applied) with every
point taken to world space by the node's world matrix. If the evaluated mesh has another
point or polygon count than the stored mesh (not expected), the stored mesh is used.
Cursor and rectangles are in render pixels, as for object picking.

### 2.4 Interaction
`SelectOp` from the modifiers: Ctrl/Cmd held gives `Remove`; else Shift held gives `Add`;
else `Replace`.

| Gesture | Result |
|---|---|
| click on an element | `Replace`: selection = {element}; `Add`: union; `Remove`: difference |
| click on nothing | `Replace`: selection cleared; `Add` and `Remove`: unchanged |
| drag on empty space, release | the rectangle's elements combined like a click's single element |

Picking per mode (`pick_component`): Points `pick_point`, radius 8 px; Edges `pick_edge`,
radius 6 px; Polys `pick_poly` along the cursor ray. Points and edges use `visible_only =
true`. A drag starts a rectangle when it starts on no gizmo handle of the active object.
`rect_components`: `rect_select` with `visible_only = true` and `Mode::All`, so an edge or
polygon is taken only when all its points are in the rectangle and visible. While dragging
the viewport draws the rectangle outline; nothing is stored until release.

### 2.5 Menus and shortcuts
Edit menu: "Select All" (Ctrl/Cmd+A), "Deselect All" (Ctrl/Cmd+Shift+A), "Invert
Selection". Select menu: "Grow Selection", "Shrink Selection". They act on the current
mode's set of the target (one "Select" transaction each):

- Points, Edges, Polys: all = every element `0..count`; none = empty; invert = complement
  within `0..count`.
- Grow, shrink: Points `grow_points`/`shrink_points`; Polys `grow_polys`/`shrink_polys`
  (GEOMETRY.md §4). Edges: the neighbours of an edge are the edges that share a point with
  it; grow adds all neighbours of selected edges; shrink keeps the selected edges whose
  neighbours are all selected.
- With no target in a component mode, they do nothing and the status bar says why
  ("select a polygon object").
- Model mode: the set is the object selection. All = every node of the hierarchy (not
  tags), none = clear, invert = the other nodes; grow and shrink do nothing. The active node
  becomes the last selected one.

Ctrl/Cmd+A and Ctrl/Cmd+Shift+A were unbound before this section (shortcuts: UI.md §5), so
nothing clashes. Like the other shortcuts they are ignored while a text field has focus.
Keys for switching modes are out of scope (brief 10).

### 2.6 Display (UI.md §6, VIEWPORT.md §6)
In a component mode with a target, the target is drawn in the plain object colour (no
object tint) with its wire visible (a shading without wire is drawn as its wire twin:
Smooth as Smooth + wire, Flat as Flat + wire), and:

| Mode | Drawn |
|---|---|
| Points | the points as 7 px squares, depth tested (hidden points stay hidden); selected ones in the component selection colour |
| Edges | all edges; selected edges in the component selection colour |
| Polys | all polygon outlines; selected polygons filled and outlined in the component selection colour |

Other objects draw as usual (their points are not drawn). The component selection colour is
new: `Theme.component_selection`, `#38C6E8`, passed to the renderer as `Palette.selection`
(object selection stays the orange tint). It differs from the object selection colour so a
selected object and selected components can be told apart.
The mode buttons are four toggle buttons in the viewport header labelled "Model", "Points",
"Edges", "Polygons" (text only; icons and the tool palette come with brief 10); the header
label also names the mode.

### 2.7 Checks (`crates/ui/tests/components.rs`)
- Picking: on a made-editable cube seen from a known camera, each element type is picked at
  the pixel of a corner, an edge midpoint and a face centre; a hidden element is not picked.
- Click, Shift and Ctrl rules; empty click; rectangle select; one transaction per
  operation; no-op writes nothing.
- Undo restores the previous selection; the selection survives save and load (and a
  version 2 file loads with empty sets).
- With a non-polygon active object, clicks pick objects.
- Picking uses the deformed mesh and stores base indices; select all / none / invert / grow /
  shrink; header buttons, Ctrl/Cmd+A and the Edit and Select menus (egui_kittest, no GPU).
- Display: `display_selection` per mode, `component_shading`; the renderer's sized points
  (`crates/viewport/tests/render.rs`, GPU) draw a square in the wire or selection colour and
  are hidden behind filled faces.
