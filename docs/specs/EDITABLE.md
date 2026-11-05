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
  A spline primitive becomes an editable spline (`forge.spline`, SPLINES.md §3) instead.
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
§5). Component modes never touch the selections of objects that are not targets. With
several polygon objects selected there are several targets (§6).

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

### 2.8 Selection commands (brief 023)

Sources: observation O26 ([docs/parity/selection-ops.md](../parity/selection-ops.md), cases in
`tests/golden/selection-ops/`, 6 observed); everything else is own design and
**parity-pending** until GUI session G3 answers (GUI-SESSIONS.md §G3 item 4). Pure functions
live in `forge_geometry::selection`; `forge_app::select` applies them to the target of
§2.2 (a component mode and an active polygon object, else the command fails with "select a
polygon object"). Every command works headless (CLI, RPC, MCP, Rhai) through the registry,
reads the stored mesh, and writes with `write_selection`: **one "Select" step** (or none if
nothing changes). Edge ids are those of `PolyMesh::edges()`.

| Command | Args | Acts on | Result |
|---|---|---|---|
| `select.invert` | - | current mode's set | complement within `0..count` (`invert-poly`: 4 top polys give the other 20, points and edges untouched) |
| `select.grow` | - | current mode's set | adds neighbours (below) |
| `select.shrink` | - | current mode's set | keeps elements whose neighbours are all selected (`shrink-poly`: the 16 front polys of a 4x4x4 cube give 5, 6, 9, 10) |
| `select.connected` | - | current mode's set | every element of the connected pieces that contain a selected element |
| `select.convert` | `to` points, edges, polys; `mode` strict (default) or tolerant | current mode's set | writes the `to` set; **the source set stays selected**; the edit mode becomes `to` |
| `select.loop` | `across?` 0 or 1 (polys only, default 0) | edges or polys | edge loops / polygon loops of the selected elements |
| `select.ring` | `across?` | edges or polys | edge rings / the orthogonal polygon strips |

Neighbours: points are neighbours along an edge; edges share a point; polygons share an
**edge** (`grow-poly` and `grow-point` were not observed, the dialog "Grow Selection…" crashed
the observer: **parity-pending**). Grow and shrink are exact inverses on the observed shrink
case only.

**Convert** (`convert(mesh, sel, from, to, mode)`). Same kind is a no-op. Strict needs all
parts of the target element selected, tolerant any part:

| from → to | strict | tolerant |
|---|---|---|
| polys → points, edges | every point / edge of the selected polys (no tolerance) | same |
| points → edges | both end points selected | either |
| points → polys | all corners selected (`convert-point-to-poly`: 9 top points give polys 16-19; side polys with 2 selected points are not chosen) | any corner (not observed) |
| edges → points | end points of the selected edges | same |
| edges → polys | all edges of the polygon selected (`convert-edge-to-poly`: the 8 border edges of the top give no polygon) | any edge (not observed) |

`convert-poly-to-point`: the 4 top polys give the 9 top points, polys stay selected.

**Connected** uses shared **points**: two polygons are in one piece if they share a point
(union-find over polygon corners). Points mode: pieces of points; edges mode: edges of the
pieces. `select-connected` (after Disconnect every top polygon owns its points): poly 16 stays
alone. Spreading through shared points, versus through shared edges only, was not observed:
**parity-pending** (the case cannot tell, see report).

**Loop and ring** (own design, parity-pending; `loop-edge`, `loop-poly`, `ring-edge` need
the GUI, O26).
- Edge loop: from each end of a selected edge continue straight. At an interior point of
  valence 4 whose faces around it are all quads, the next edge is the one sharing no face with
  the incoming one. At a border point with exactly two border edges, a border edge continues
  to the other one. Anything else (poles, N-gons, non-manifold) stops. A loop that returns to
  its start closes. Edges mode only.
- Edge ring: from an edge, for each quad that contains it, the opposite edge; continue from
  that edge into its other quad. A non-quad or a border stops. Closed rings close.
- Polygon loop (polys mode): a strip of quads. For a selected quad, the strip crosses the
  edge pair `(p0p1, p2p3)` when `across` is 0, `(p1p2, p3p0)` when 1; it continues through
  each crossed edge into the neighbour quad, which is entered through the edge shared and left
  through the opposite one. Stops at borders, non-quads and when the strip closes. Polygon
  ring is the strip with `across` flipped. Selected non-quads contribute only themselves.
- Points mode: `select.loop` and `select.ring` fail ("needs edges or polygons").

Model mode: `select.connected`, `select.convert`, `select.loop`, `select.ring` do nothing
(like grow and shrink).

Checks: `crates/geometry/tests/selection_ops_parity.rs` rebuilds each O26 case from its
`mesh.obj` and `case.toml` start sets and compares with `selection.json` exactly; unit tests
for loop, ring, connected, tolerant convert and edge grow/shrink (`selection.rs`); command
tests in `crates/app/tests/select_commands.rs` (headless run, one undo step, no step when
nothing changes).

## 3. Moving, rotating and scaling components (`crates/ui/src/component_edit.rs`)

Sources: own design (brief docs/agents/tasks/06-component-transform.md); gizmo, axes, drag
deltas and snapping are MANIPULATE.md §2–3 and VIEWPORT.md §3. No observation of the
reference app exists, so **every rule below is parity-pending** (brief 10 observes them).

### 3.1 Where the core lives
The pure maths is in `forge-ui` (`component_edit.rs`), not in `forge-generators`: it needs
`DragDelta` (viewport crate), the stored selection sets and the mode, none of which the
generators crate knows. It uses `MeshData` and the geometry crate's `IndexSet`,
`edges_to_points` and `polys_to_points`, so it needs no GPU and no egui.

### 3.2 Affected points and pivot
- `affected_points(mesh, mode, sets)`: Points mode uses the point set, Edges mode
  `edges_to_points` of the edge set, Polys mode `polys_to_points` of the polygon set.
  Indices past the element count are ignored. Model mode gives the empty set.
- `pivot(mesh, points)`: the mean of the affected points' positions, in object space
  (the stored mesh). `None` for an empty set.

### 3.3 `transform_points(mesh, points, m) -> MeshData`
Applies the object-space matrix `m` (`transform_point3`) to the listed points only; the
other points and all polygons are copied unchanged. Indices past the point count are
ignored. Pure and deterministic.

### 3.4 From a gizmo delta to `m`
The gizmo sits at the world position of the pivot, `P = W · c` (`W` the node's world matrix,
`c` the pivot). Its axes follow the existing axis setting (object frame, or world axes for
Move and Rotate; Scale always uses the object frame, MANIPULATE.md §2). A `DragDelta` is in
world space (MANIPULATE.md §3), so the world-space transform `M` is:

| Delta | `M` |
|---|---|
| `Translate(w)` | translation by `w` |
| `Rotate{axis, angle}` | rotation about `axis` through `P` |
| `Scale(f)` | scale by `f` along the object axes `A = rotation_part(W)` through `P`: `T(P) · A · diag(f) · Aᵀ · T(−P)` |

and the object-space matrix is `m = W⁻¹ · M · W`. This handles non-uniform scale and any
rotation of the object or its parents: a world move becomes `W⁻¹ · w` (as `apply_drag`
does for parents); a rotation about a world axis of a non-uniformly scaled object becomes
whatever object-space map gives that world result; a Scale in the object frame reduces to
`diag(f)` about `c`.

### 3.5 Drag and undo
A drag starts when a primary press hits a gizmo handle and the mode is a component mode with
a target and a non-empty affected set. The drag keeps the **start mesh**, the affected set
and the pivot `c` taken at the start. On every frame
`mesh = transform_points(start_mesh, set, m(delta))` is written with
`doc.transact_merged(label, "gizmo:<id>:<n>", ...)`, the pattern of `apply_drag`
(MANIPULATE.md §3): the whole drag is one undo step ("Move", "Rotate", "Scale"), and undo
restores the mesh at drag start. Only the `mesh` param is written; the stored selection
sets are unchanged. Snapping is the gizmo's own (`Drag::update` with the quantize step, `snap.rs`; defaults 10 units,
15 degrees, 0.1) plus target snapping (MANIPULATE.md §6.7, brief 032).

A component mode with a target but an empty affected set draws no gizmo, and a press on
the spot where it would be does nothing special (it is a click or a rectangle select). In
a component mode without a target (no polygon object active) the object gizmo works as in
Model mode. In Model mode nothing changes.

### 3.6 Display and Coordinates panel
With a target and a non-empty set the gizmo is drawn at `P` (the pivot of the current
mesh, so it follows the drag). The Coordinates panel gets a read-only "Pivot" row in
component modes: the pivot in object space (Local) or world space (World), whichever
coordinate space is chosen; it shows "no selection" if the set is empty. Numeric entry is
§6.7 (brief 034). With several targets the pivot is the one of §6.4.

### 3.7 Checks (`crates/ui/tests/component_edit.rs`)
- `transform_points` on a made-editable default cube: top polygon +50 in y gives y = 150 for
  the four points and the others unchanged; scale 2 about the centroid; 90 degrees about y;
  empty set and out-of-range indices change nothing.
- Points, edges and polygons modes move exactly the points of their set.
- World-axis move and rotate on a rotated, non-uniformly scaled object: the world positions
  of the affected points change by the delta (move) or by the world rotation (rotate); the
  others keep their world position.
- Scale about the pivot in the object frame under non-uniform scale.
- A drag over several frames is one undo step; undo restores the mesh; redo reapplies it.
  Frames are computed from the start mesh (no accumulation).
- A Bend child above an edited polygon object: `evaluate` reflects both the edit and the bend.

## 4. Animation tracks (brief 10)
Make Editable and Current State to Object also copy the source's `transform.*` keyframe
tracks to the new polygon object (ANIM.md §2.8); other tracks are dropped. The mesh is
baked from the stored parameter values (parity-pending).

## 5. Soft selection and symmetry (brief 026; `crates/modeling/src/soft.rs`)

Sources: own design (brief 026, roadmap area B). GUI session G3 item 6 (where soft selection
lives and its modes) is unanswered, so **every rule below is parity-pending**. The maths is
pure and lives in `forge-modeling` (`ComponentPlan`, `SoftSelection`, `Symmetry`); the
editor keeps the current values in `ComponentSettings` (`App::component_settings`, not saved,
not undoable); `forge-ui` and the `mesh.transform` command both go through the same plan.
No falloff code is shared with `forge-fields` (that crate is an empty stub).

### 5.1 Settings
Editor settings (`edit.soft_selection`, `edit.symmetry`; the Components bar in the studio):

| Id | Kind | Default | Range / values |
|---|---|---|---|
| soft `enabled` | bool | false | |
| soft `radius` | float | 100 | 0.001 .. 1e9 (object-space units) |
| soft `falloff` | enum | `linear` | `linear`, `dome`, `bell`, `spline` |
| soft `distance` | enum | `volume` | `volume`, `surface` |
| soft `curve` | float list | `[1, 0.896, 0.5, 0.104, 0]` | 2..64 values in 0..1 (used by `spline`) |
| symmetry `enabled` | bool | false | |
| symmetry `axis` | enum | `x` | `x`, `y`, `z`: the plane is perpendicular to that object-space axis |
| symmetry `offset` | float | 0 | plane position along the axis |
| symmetry `tolerance` | float | 0.1 | 0 .. 1e6; how far a mirror image may be from a point |

### 5.2 Weights (soft selection)
Per point, `w` in 0..1 is the blend factor of the transform. Selected points (the affected
points of §3.2) have `w = 1`. For any other point, `d` is its distance to the selection and
`t = d / radius`; `w = 0` for `t >= 1`, else:

| Falloff | `w(t)` |
|---|---|
| linear | `1 - t` |
| dome | `sqrt(1 - t^2)` (stays high, drops steeply at the rim) |
| bell | `0.5 (1 + cos(pi t))` (smooth S shape, zero slope at both ends) |
| spline | the user curve: values `c_0..c_{n-1}` at `t_i = i/(n-1)`, Catmull-Rom between samples, clamped to 0..1 |

Distance: **volume** is the straight-line distance to the nearest selected point (object
space); **surface** is the shortest path along mesh edges (Dijkstra, edge lengths), so
points on another sheet or across a gap are `w = 0`. Unreachable points have `w = 0`.
With soft selection off, `w` is 1 for selected points and 0 elsewhere (the behaviour of §3.3).

### 5.3 Applying a matrix
`p' = p + w (m p - p)`: the point moves by the fraction `w` of its displacement under `m`
(rotations and scales blend positions linearly, not the matrix). Selected points get the
full transform, so the result equals `transform_points` when soft selection is off.

### 5.4 Symmetry
The plane is `{x : x[axis] = offset}` in object space. The **partner** of point `p` is the
point nearest to its mirror image `R p` within `tolerance` (ties: lowest index); a point
on the plane is its own partner; `None` if there is none. Let `d_a(p) = w(p)(m p - p)` be the
displacement without symmetry (weights from the selected points only) and `R_lin` the linear
reflection. The displacement used is:
- no partner: `d_a(p)`;
- own partner (on the plane): `(d_a + R_lin d_a) / 2`, which drops the part of the move
  normal to the plane, so the point stays on the plane;
- partner `q != p`: `d_a(p)` if `w(p) > w(q)` or the weights are equal (within 1e-12) and
  `p < q`; else `R_lin d_a(q)`. The point with the larger weight drives its mirror partner.

Selecting one side therefore moves the other side by the mirror image; selecting both sides
moves the lower-index point and mirrors it. A symmetric mesh stays symmetric (to the
tolerance) under any move, rotation or scale.
Scope: symmetry applies to component transforms (drag and `mesh.transform`). The modeling
tools (extrude, bevel, ...) are **not** mirrored; the brief names only transforms
(deviation noted in the brief report; Mirror and Center Axis of MODELING.md §4 stay the
tools for mirroring geometry).

### 5.5 Commands (COMMANDS.md §6)
- `edit.soft_selection {enabled?, radius?, falloff?, distance?, curve?}` and
  `edit.symmetry {enabled?, axis?, offset?, tolerance?}` change the editor settings (omitted
  keys keep their value; numbers outside the range are an error); both return the
  resulting settings and are not undoable.
- `mesh.transform {node?, points?, move?, rotate?, scale?, pivot?, soft?, symmetry?}`:
  transforms components of a polygon object in object space as one undo step
  ("Move", "Rotate", "Scale" or "Transform"). `points` lists point ids; without it the
  stored selection of the current edit mode is used (points, points of the edges, points of
  the polygons; Model mode is an error). `move` `[x,y,z]`; `rotate` `{axis:[x,y,z], angle}`
  in degrees; `scale` `[x,y,z]` or one number; `pivot` defaults to the mean of the selected
  points. The matrix is `T(move) T(pivot) R S T(-pivot)`. `soft` and `symmetry` override the
  editor settings for this call: `false` turns the feature off, `true` turns it on with
  the editor values, an object turns it on with those keys over the editor values. The
  stored selection sets are unchanged. An empty or out-of-range point list is an error.
- The result is `{node, moved, soft, symmetry}` (`moved`: number of points that changed).

### 5.6 Gizmo drags
`ComponentDrag::begin` snapshots the editor settings and builds the plan once (weights and
partners depend only on the start mesh); each frame writes `plan.apply(start, m)`. With both
features off the old `transform_points` path runs. The Components bar in the header has
Soft (checkbox, radius, falloff, distance) and Sym (checkbox, axis) controls.

### 5.7 Checks
- `crates/modeling/tests/soft.rs`: falloff values at t = 0, 0.25, 0.5, 1 for each shape;
  a move of one point of a 11-point row (spacing 10) with radius 100 moves the neighbours
  by exactly the falloff fraction; surface distance does not cross a gap that volume does;
  spline curve interpolation and validation; partners within tolerance; symmetric
  moves/rotations/scales keep a symmetric mesh symmetric; one-sided selection moves the
  mirror side; on-plane points stay on the plane; soft and symmetry combined.
- `crates/app/tests/soft_commands.rs`: the two settings commands, `mesh.transform` on a
  subdivided plane with radius 100, symmetry on a cube, overrides, one undo step, errors
  change nothing, schemas.
- `crates/ui/tests/component_edit.rs`: a drag with soft selection and symmetry active.


## 6. Multi-object component editing (brief 034; `forge_app::select`, `forge_ui::component_edit`)

Sources: own design (brief docs/agents/tasks/034-multi-object-component-editing.md, roadmap
area B). GUI session G3 has no answer for multi-object component editing, so **every rule in
this section is parity-pending**. Single-object behaviour (§2, §3, §5) is unchanged: with one
polygon object selected every rule below reduces to the old one.

### 6.1 The targets
The **component targets** are the selected polygon objects (`forge.polygon`, members of
`NodeSelection.nodes`) in ascending node-id order, when `mode != Model` and the **active**
node is a polygon object (the old rule of §2.2; an active node of another type means no
targets). `select::targets(app)` returns the list, `select::target(app)` stays the active
node. Headless commands take an optional `nodes` list instead: every id must be a polygon
object (else `Failed`, nothing changes); without it the targets above are used.

### 6.2 Model
There is no combined selection object. Each target keeps its own three stored sets
(`selection.points|edges|polys`, §2.1), indices refer to that object's stored mesh, and each
object keeps its own topology. A combined view exists only in the editor: all targets are
drawn with their components (VIEWPORT.md §6, one `component` mode for all), and a command
walks the targets one by one. Nothing crosses objects: Grow, Shrink, Connected, Loop, Ring and
Convert use each object's own topology, so a selected point of object A never selects anything
in object B.

### 6.3 Selection
- Commands `select.all|none|invert|grow|shrink|connected|convert|loop|ring` take `nodes?`
  (`select.mode` does not). Each target gets the result of the single-object rule; all
  changes are written in **one "Select" step**, none if no object changes. If the rule fails
  for any object (for example Loop on points) nothing is written. In Model mode `nodes` is an
  error ("nodes needs a component mode"); the Model-mode rules are unchanged.
- Click: the element is picked on every target and the nearest one wins (depth along the pick
  ray: the point, the middle of the edge, the polygon hit). `Replace`: the picked element is
  the only selected element of the mode in all targets (the other targets' sets of this mode
  are emptied); `Add`/`Remove`: only the hit object's set changes. A click on nothing
  `Replace`s every target's set by the empty set; `Add` and `Remove` change nothing.
- Rectangle drag: each target's set becomes the combination of its old set with the
  elements inside the rectangle (`Replace` also empties targets with no element inside).
  One "Select" step for the whole drag.

### 6.4 Pivot and transform
The pivot `P` is the mean world position of **all affected points of all targets** (each
point counts once; §3.2 decides the affected points per object). Gizmo drags and
`mesh.transform` build one world matrix `M` per delta (translation; rotation about `P`; scale
about `P` along the axes of the **active** object's frame, §3.4) and apply to every target the
object-space matrix `W_i⁻¹ · M · W_i` (`W_i` the object's world matrix), so moving points in
two objects moves them by the same world vector. A target with no affected points is left
alone. All meshes are written in **one undo step** ("Move", "Rotate", "Scale"; a drag merges
like §3.5, key `gizmo:<first target>:<n>`).

`mesh.transform` (§5.5) takes `nodes?` (or `node?`) and `space?` (`object` | `world`).
Default `object` for one object (unchanged), `world` for several. In `world`, `move`,
`rotate`, `scale` and `pivot` are world-space and the default pivot is `P`; each object gets
`W_i⁻¹ · M · W_i`. In `object` with several objects the same arguments are applied literally
in every object's object space and the default pivot is that object's own mean. `points`
names one object's indices, so it needs exactly one object. Each object's points come from
its stored selection of the current edit mode; an object with no affected points is skipped
(`moved: 0`), no affected points anywhere is `Failed`. Result: `{node, moved, soft,
symmetry}` for the first object plus `nodes: [{node, moved}]`; `moved` at top level is the
total.

**Soft selection and symmetry do not cross objects (decision).** `ComponentPlan` (§5) is
built per object from that object's mesh and selected points: weights come from the distance
to the *same object's* selected points (volume: object-space straight line; surface: along
its own edges), the radius is in that object's object-space units, and the symmetry plane is
that object's own object-space plane. A selected point therefore never drags a neighbouring
object's points, even when they touch in the world. Reasons: objects have separate topology
and units (a scaled object would need a world-to-object radius conversion), partners and
surface distance are defined on one mesh, and the user expects an unselected object to stay
put. Cross-object falloff by world distance is a possible later option.

### 6.5 Modeling commands
The commands of `ToolCommand` (MODELING.md §0.1: `mesh.extrude`, `extrude_inner`, `bevel`,
`slide`, `subdivide`, `triangulate`, `untriangulate`, `reverse_normals`, `optimize`, `weld`,
`melt`, `dissolve`, `split`, `disconnect`, `connect_points`, `mirror`, `center_axis`, `array`)
take `nodes?` next to `node?` and work on every target in **one undo step** labelled with the
tool; each object uses its own stored selection and its own topology. Details in MODELING.md
§0.1. The single-object tools (`mesh.loop_cut`, `mesh.knife`, `mesh.bridge`, `mesh.stitch`,
`mesh.close_hole`, `mesh.pen`) act on one object (`node`, else the active
one): they read an edge or points of one mesh, and joining components of two objects would
mean merging objects, which is `Connect Objects`, not a component tool. They reject a `nodes`
argument and the viewport tools say "<Tool> works on the active object only" in the status
bar and the tool panel when more than one polygon object is selected.

### 6.6 Snapping and display
The points moved by a drag are excluded as snap targets in every target
(`collect_sources_multi`, MANIPULATE.md §6.7). All targets are drawn in the component display
of §2.6 (the active one is not special); the other selected objects keep the object colour.

### 6.7 Numeric coordinates
With targets and a non-empty affected set the Coordinates panel shows three editable rows
instead of the read-only "Pivot":

| Row | Shown | Entering `v` |
|---|---|---|
| Pivot X Y Z | `P` (Local: the active object's object space when there is one target, else World) | moves all affected points by `v - P` |
| Rotation H P B | always `0 0 0` (a selection has no orientation) | rotates about `P` by the HPB rotation `v` (degrees) |
| Size X Y Z | extent of the affected points' bounding box (Local with one target: object space; else the world axis-aligned box) | scales about `P` so that the extent becomes `v`; an axis with extent 0 keeps its value |

Each entry is one `Gesture` like the object rows (`Commit` on typing, merged while dragging
the field) and writes through the same per-object matrices as §6.4 ("Set coordinates"), with
soft selection and symmetry as for a gizmo drag; with soft selection on, a dragged field is
relative to the mesh of the previous frame, so use typed values for exactness. Local with one
target uses the object-space matrix directly.

### 6.8 Checks
- `crates/app/tests/multi_object.rs`: selection commands on two objects (select.all,
  invert, grow, convert, loop; explicit `nodes`; one step; nothing crosses objects; errors
  write nothing), `mesh.transform` on two objects as one step (world-space `move`, pivot is
  the common centroid, rotation about it), extrude / bevel / subdivide / weld on two objects
  in one step with per-object selections and skipping of an object with an empty selection,
  soft selection not crossing objects, single-object tools rejecting `nodes`.
- `crates/ui/tests/multi_object.rs`: `targets`, click picking the nearer object, rectangle
  select across two objects, a gizmo `ComponentDrag` on two objects (one step), numeric
  coordinates, snap exclusion; an egui_kittest box select and drag across two objects.
