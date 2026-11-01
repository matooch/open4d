# Phase 4 — editor shell (`crates/ui`, package `forge-ui`; `apps/studio`)

Sources: project architecture only. The shell is built on public crates (eframe/egui
0.37 on winit, egui-wgpu sharing the viewport's wgpu 30 device; egui_kittest for UI
tests). **Clean-room:** layout, colours, labels and key bindings are our own. Screens
of the reference app may inform *which* managers exist (docs/PARITY.md allows
workflow observation), never their look. Anything a later observation may change
(default bindings, frame rate, unit display) is **parity-pending**.

Why no docking crate: egui_dock 0.21 and egui_tiles 0.17 still target egui 0.36, which
would put two eguis in the build. A small dock model of our own (§2) covers what the
shell needs and is plain, testable data.

Logic lives in UI-free modules with unit tests; egui code only draws them and turns
input into the same commands.

## 1. Editor state (`state`)

`EditorState { doc: Document, selection: NodeSelection, expanded: BTreeSet<NodeId>,
dock: Dock, camera: Camera, shading: Shading, timeline: Timeline, status: String,
mode: EditMode }`,
built with `EditorState::new(Arc<Registry>)`. `mode` is `EditMode::Model` at start
(`Model`, `Points`, `Edges`, `Polys`; EDITABLE.md §2.2).

`NodeSelection { nodes: BTreeSet<NodeId>, active: Option<NodeId>, anchor: Option<NodeId> }`:
- `click(id)`: select only `id` (active and anchor = id).
- `toggle(id)` (Ctrl-click): add/remove; active = id if added, else the largest
  remaining id or `None`; anchor = id.
- `range(id, rows)` (Shift-click): select every visible row between the anchor and
  `id` inclusive (row order), active = id; without an anchor behaves like `click`.
- `retain_existing(scene)`: drops deleted nodes (active/anchor too).

## 2. Dock model (`dock`)

Areas: `Top, Left, RightTop, RightBottom, Bottom, Center`. Panels: `CommandBar, ToolPalette,
Viewport, ObjectTree, Attributes, Timeline, Coordinates, Script` (the Script Manager, §10,
default area Bottom). Each area holds an ordered tab list and an active index; each
panel appears at most once in the whole dock.
- `Dock::default()` (the layout of §11): Top [CommandBar], Left [ToolPalette], Center
  [Viewport], RightTop [ObjectTree], RightBottom [Attributes], Bottom [Timeline,
  Coordinates]. The Script Manager
  is closed by default (`Panel::DEFAULT` lists the panels a default layout opens); the
  Window menu opens it.
- `area_of(panel)`, `tabs(area)`, `active(area) -> Option<Panel>`.
- `move_panel(panel, to)`: removes it from its area (fixing that area's active index:
  stays on the same panel if possible, else clamps), appends to `to`, makes it
  active there. Moving to its own area is a no-op.
- `close(panel)`, `open(panel)`: reopen in its default area (no-op if present) and
  make it active. `set_active(area, i)` (ignored if out of range).
- An area with no tabs is not drawn; a Top, Left or Centre area with one panel has no tab header. Left/right/bottom areas are resizable.

## 3. Object tree (`tree`)

`tree_rows(scene, expanded) -> Vec<TreeRow { id, depth, name, type_label,
has_children, expanded, tags: Vec<(NodeId, String)>, missing_type }>`: depth-first
pre-order over hierarchy nodes; children of collapsed nodes are omitted; tags are
listed on their host row (label = type label), never as rows. `type_label` is the
registered label, or the raw type id with `missing_type = true`.

Interactions (egui): click/Ctrl/Shift selection (§1); arrow toggles expansion;
double-click renames (Enter commits, Esc cancels); drag a row onto another row
reparents it as that row's last child; drop on empty space moves it to the root end;
context menu: Rename, Delete, Move to root. Failed edits (e.g. cycles) set
`status` and change nothing.

## 4. Attribute panel (`attributes`)

`attribute_rows(scene, id) -> AttributeView { name, type_label, editable, groups:
Vec<(String, Vec<AttrRow>)> }`. One `AttrRow { param, label, widget, value }` per
non-hidden `ParamDesc`, grouped by `ui.group` in first-appearance order (empty group
→ "Parameters"). Unknown types: `editable = false`, rows from raw params with
`Widget::ReadOnly`.

`Widget` from `ParamKind`: `Bool → Checkbox`, `Int → IntDrag {min, max}`,
`Float → FloatDrag {min, max, unit}`, `Vec3 → Vec3Drag {min, max, unit}`,
`Color → Color`, `Enum → Combo {options}`, `String → Text`, `Link → Link {accepts}`.

Display units (**parity-pending**): `Angle` shows degrees, `Percent` shows ×100 with
"%", others raw. `to_display(unit, v)` / `from_display(unit, v)` are exact inverses
for these factors; ranges are converted too.

Groups may name a tab, `"Tab/Group"` (§12); `attribute_rows_multi(scene, ids)` is the
multi-object form and `attribute_rows` is its one-object case. `Widget::LinkList {accepts}`
is added; `AttrRow` also holds `mixed`, `mixed_axes`, `step`, `animatable`.

Edits: `apply_edit(doc, id, param, value, gesture)`; `Gesture::Commit` → one
`transact("Set <label>")`; `Gesture::Drag(n)` → `transact_merged` keyed
`"drag:<id>:<param>:<n>"`, so one continuous drag is one undo step. Validation
errors are returned (and shown in `status`) and change nothing. Name edits go through
`rename`.

## 5. Commands (`commands`)

Every action is a registry command (COMMANDS.md); the former `Command` enum is gone.
`execute(&mut EditorState, id, args: serde_json::Value) -> Result<Value, String>` runs the
command `id` through `EditorState.commands`. Menus, shortcuts and panels call it with the
ids `object.create`, `object.delete`, `edit.undo`, `select.all`, `mesh.make_editable` ... (the
bullets below keep the old names for the same behaviour: Create = `object.create`,
DeleteSelected = `object.delete`, Undo/Redo = `edit.undo`/`edit.redo`, Reparent =
`object.reparent`, MakeEditable = `mesh.make_editable`, CurrentStateToObject =
`mesh.current_state_to_object`, SetMode = `select.mode`, Select = `select.*`,
FrameAll = `view.frame_all`).
- `Create`: new node at the end of the root list, selected alone; a new Generator node
  also gets one `forge.smoothing` tag in the same undo step (SMOOTHING.md §2); a `Tag` type is
  attached to the active node instead (error without one).
- `DeleteSelected`: one transaction deleting every selected node not already inside
  another selected node's subtree; selection cleared.
- `Undo`/`Redo`: then `retain_existing`.
- `Reparent`: appends as last child (None = root end).
- `MakeEditable`, `CurrentStateToObject`: act on every selected node, one undo step for
  the whole selection; rules in EDITABLE.md §1. Nodes that make no mesh are skipped with a
  message in `EditorState.status`. The selection becomes the new objects. Edit menu items
  "Make Editable" and "Current State to Object".
- `SetMode(mode)`: sets `EditorState.mode`; nothing else changes.
- `Select(cmd)`: Select All / Deselect All / Invert / Grow / Shrink on the current mode's
  set (EDITABLE.md §2.5); one "Select" transaction for a component set.
- `FrameAll`: no geometry yet → resets the camera to default (placeholder until
  Phase 5).

`create_menu(registry) -> Vec<(Category, Vec<(type_id, label)>)>`: placeable
categories only (hierarchy + Tag), in `Category` order, types sorted by label; types
with `in_create_menu == false` (the polygon object) are left out.

Key bindings (**parity-pending**, ours): Ctrl+Z undo, Ctrl+Shift+Z / Ctrl+Y redo,
Ctrl+A select all, Ctrl+Shift+A deselect all (Cmd on macOS),
Delete delete selected, F2 rename active, 1–5 shading (Smooth, Flat, Wire,
SmoothWire, FlatWire).

## 6. Viewport panel (`viewport_panel`)

Renders with forge-viewport's `Renderer` on the eframe wgpu device into an offscreen
`Rgba8Unorm` texture registered with egui as-is, so palette values display as
authored (matching the snapshot PNGs). Recreated on resize. Draws the grid and every
generator node (`forge_generators::build`, placed by `Scene::world_matrix`); selected
nodes use the selection colour. GPU meshes are rebuilt only when `Scene::revision()` or
the selection changes; nodes whose build is unsupported are skipped. Without a wgpu render state (tests) it shows a
"Viewport unavailable" label.

Navigation `nav_action(input) -> NavAction` (**parity-pending**, ours):

| Input | Action |
|---|---|
| wheel `s` (points) | `Dolly(0.999^s)`, `s > 0` zooms in |
| Alt + primary drag `(dx, dy)` | `Orbit(−0.01·dx, 0.01·dy)` radians |
| middle drag, or Alt + middle drag | `Pan(dx, dy)` pixels |
| Alt + secondary drag | `Dolly(exp(−0.01·dx))` |
| anything else | `None` |

Wheel takes precedence over drags in the same frame.

Component modes (EDITABLE.md §2): the header of the viewport holds four toggle buttons
"Model", "Points", "Edges", "Polygons" (`EditorState.mode`). With a component target, left
click and left drag (not on a gizmo handle) select components instead of objects, and the
target is drawn with its points / edges / polygon outlines and selected components in the
component selection colour (§8); the panel rebuilds its GPU meshes when the revision, the
node selection, the mode or the target changes.

## 7. Timeline (`timeline`, `animate`)

Behaviour, data and rules: docs/specs/ANIM.md §2. Panel summary (own design,
**parity-pending**):
- Row 1: Play / Stop, Record, Auto-Key toggle, Previous / Next key, Delete Keys.
- Row 2: Start, End and FPS fields (edit the scene settings, undoable), the current frame
  field, and the label of the current time.
- Row 3: a frame strip. Click or drag scrubs the current frame; the playhead and one tick
  per key frame of the selected objects are drawn by us.
- `Timeline { current: 0, playing: false }` is editor state; start, end and fps live in
  the scene (`SceneSettings`, default 0, 90, 30). `set_current` clamps to the range.
- The Attributes panel draws a marker left of every animatable row: filled diamond =
  key on this frame, hollow = animated without a key here (ANIM.md §2.5).

## 8. Theme (`theme`)

Our own dark palette (sRGB): window `#1E2024`, panel `#25282D`, faint
`#2E3238`, text `#D6D9DE`, weak text `#9AA0A8`, accent `#E08A2E` (selection), accent
text `#1A1A1A`, error `#E5534B`, component selection `#38C6E8` (selected points, edges
and polygons of the component target, EDITABLE.md §2.6). `apply(&egui::Context)`. Requirement: text on panel
and on window has WCAG contrast ≥ 7, weak text ≥ 4.5, accent text on accent ≥ 4.5.

## 9. App (`app`, `apps/studio`)

`forge_ui::run()` opens a window titled "Forge3D" (eframe, wgpu backend) with a menu
bar (File: New, Open, Save, Save As, Import OBJ, Export OBJ, Export Selected as OBJ (IO.md §2); Edit: Undo/Redo/Delete/Make Editable/Current State to Object, Select All/Deselect All/Invert
Selection (EDITABLE.md §2.5); Select: Grow Selection, Shrink Selection; Create: §5 menu; View: shading; Window:
open/move panels, reset layout), the dock (§2) and a status bar. `apps/studio`
calls it with `Registry::with_builtins()`. Window menu: one toggle per panel labelled
"<Panel> panel", a "Move <Panel>" submenu per panel with "<Panel> to <area>" items,
and "Reset layout".

UI tests (egui_kittest, no GPU): create via the Create menu, select in the tree,
edit an attribute, undo, and move a panel via the Window menu.

## 10. Script Manager (`script_panel`)

Window > "Script Manager panel" (default area Bottom, closed by default). Contents:
- a multi-line monospace `TextEdit` for the source, with a **Run** button (also Ctrl+Enter
  while the editor has focus) and a **Clear** button for the console;
- an **Open...** and **Save...** pair (native dialogs, like File, loading/saving the
  source text; the panel's source is not part of the scene);
- the output console below: `print` lines, then for a failure the message in the error
  colour (`line N: ...`), for a success a dim line `ran N command(s)`;
- a recent-scripts list (paths, newest first, at most 10), stored in the settings file under
  `scripts.recent` (a JSON array of strings; same file as `rpc.*`, RPC.md §2). Clicking
  an entry loads the file into the editor. Opening, saving or running a file adds it.
A run is `forge_script::run_script` on the editor's context with the default limits
(SCRIPTING.md): one undo step, rolled back on failure. The panel state (source, console,
recent list) lives in `EditorState::script` (`ScriptPanelState`) so tests can drive it:
`run_source(&mut self, &mut App-like)`. Tests (egui_kittest, no GPU): open the panel via
the Window menu, set a source, click Run, see the output and the created node, undo it
in one step; a failing script shows its error and changes nothing.

## 11. Default layout, command bar, tool palette (`layout`; brief 147)

Sources: own design from docs/roadmap/areas/L-ui.md; GUI session G1 (docs/parity/GUI-SESSIONS.md)
is unanswered, so **the arrangement and the order of the button groups are parity-pending**.
Labels, glyphs and sizes are ours. The dock is the model of §2 (egui_dock does not build
against our egui, see the top of this file); the brief's "on egui_dock" is met by `Layout`
being plain data over that model.

`Layout { version, dock: Dock, sizes: Sizes }` is serde data (`to_json` / `from_json`;
`from_json` rejects a version other than `LAYOUT_VERSION`, a panel listed twice and an
active tab out of range). `Layout::standard()` is the default; `Editor::layout()` /
`set_layout()` read and apply one, and Window > Reset layout applies the standard layout.
`Sizes { left_width 150, right_width 340, right_top_fraction 0.45, bottom_height 120 }` are
initial sizes of the resizable areas (egui remembers later drags).

Standard arrangement, top to bottom: menu bar; Top area: command bar; Left: tool palette
column; Centre: viewport; right column: Object manager above Attribute manager; Bottom
strip: Timeline, with Coordinates as a second tab; status bar. Menus are unchanged
(File, Edit, Select, Mesh, Create, Animate, View, Window).

Every button is a `CommandButton { command, args, glyph, label, tip }` that runs the
registry command `command` with `args` through `Editor::run`; nothing else happens on a
click. Button text is `"<glyph> <label>"` (a short ASCII placeholder symbol, since the bundled fonts lack most Unicode symbols; icons are
brief 148), which also keeps labels distinct from the menu items. `command_bar()` groups:
History (`edit.undo`, `edit.redo`, `object.delete`); Objects (`object.create` for
`forge.cube`, `forge.sphere`, `forge.cylinder`, `forge.cone`, `forge.plane`, `forge.torus`;
`mesh.make_editable`); View (`view.shading` smooth and wire, `view.frame_all`); Animation
(`timeline.play`, `timeline.stop`, `key.record`). `tool_palette()` groups: Mode
(`select.mode` model, points, edges, polys); Selection (`select.all`, `select.none`,
`select.invert`, `select.grow`, `select.shrink`); Modeling (`mesh.extrude`,
`mesh.extrude_inner`, `mesh.bevel`, `mesh.subdivide`, `mesh.triangulate`, `mesh.optimize`,
`mesh.reverse_normals`, run with the stored selection and default settings). The
`select.mode` and `view.shading` buttons show as selected for the current value. A failed
command goes to the status bar like any other.

Not in the palette yet (no registry command exists): snapping, workplane, axis mode.
Pointer-driven tools (cuts, bridge, pen) stay in the Mesh menu.

## 12. Attribute manager (`attributes`, `widgets`; brief 150)

Sources: own design from docs/roadmap/areas/L-ui.md; GUI session G4 (docs/parity/GUI-SESSIONS.md)
is unanswered, so **tabs look, scrub modifiers, arrow behaviour and widget look are
parity-pending**. Every row is derived from a `ParamDesc`; nothing is hand-laid per type.

Tabs and groups: `UiHint.group` is `"Tab/Group"` (split at the first `/`) or just `"Group"`
(tab `Basic`; empty group is `Parameters`). `AttributeView.tabs()` lists tabs in
first-appearance order, `groups_of(tab)` the groups of one; a tab bar is drawn only with two
or more tabs, groups are collapsible. The shown tab is editor state (`attr_tab`); a tab
that no longer exists falls back to the first.

Widgets, one module per kind in `widgets/`: `float` (scrub, display units), `int` (scrub,
range), `boolean` (checkbox with the row label), `enumeration` (drop-down), `vector` (X, Y, Z
scrub fields), `colour` (picker), `string` (single line), `link` (drop-down, clear, accepts an
object dragged from the object manager), `link_list` (one line per entry with move up/down
and remove, an "Add..." drop-down, drop target), `readonly` (mesh and index-list summaries,
unknown types). Spline-curve, gradient and field-list parameter kinds do not exist yet; their
widgets are added with the kind (briefs 064 and later). A widget returns an `Edit`
(display-unit JSON, optional vec3 component, `Phase::Commit | Drag`); the editor runs
`param.edit` (COMMANDS.md §6) on every selected object. A commit is one undo step; a scrub,
colour drag or typing run shares a gesture number and merges into one.

Scrubbing (`widgets::scrub`, pure): the base step is `UiHint.step` (display units per pixel)
else 1 for ints, 0.1 for angle and percent, 0.01 otherwise. Shift = 10 times finer, Ctrl = 10
times coarser, any other combination (including Alt, which navigates the viewport) = base.
Clicking a field types a value; Up/Down arrows in a focused field add one step. Values are
shown and edited in display units (degrees, percent, UI.md §4).

Multi-edit: the rows are those of the active object whose id and kind exist in every other
selected object's type. A row is `mixed` when values differ (vec3: per component, `mixed_axes`).
A mixed number shows "mixed", a checkbox is indeterminate, a drop-down shows "(mixed)", a
string is empty with a hint, a colour has a "mixed" note, a list shows the active object's.
Other widgets show the active object's value. A set applies to all selected objects in one
undo step; a vec3 component edit keeps each object's other components.

Keyframe hook (097 does the feature): every animatable float/vec3 row has a 12 px marker slot
left of its label (`key_marker`, filled = key here, hollow = animated, ANIM.md §2.5); 097
attaches click and context menu to that slot. `AttrRow.animatable` mirrors the descriptor.

Not in this brief: lock/unlock, history back/forward, the field-list widget.
