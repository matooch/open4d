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

The interactions (selection, drag and drop, rename, search, context menu, columns) are
specified in §13 (Object manager, brief 149), which replaces the first version of this
section. Failed edits (e.g. cycles) set `status` and change nothing.

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
- `FrameAll`: fits the visible geometry (§6.1); with no geometry resets the camera to default.

`create_menu(registry) -> Vec<(Category, Vec<(type_id, label)>)>`: placeable
categories only (hierarchy + Tag), in `Category` order, types sorted by label; types
with `in_create_menu == false` (the polygon object) are left out.

Key bindings (**parity-pending**, ours; now data in the key map, §14, these are its defaults): Ctrl+Z undo, Ctrl+Shift+Z / Ctrl+Y redo,
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

### 6.1 Navigation (brief 153)

Camera maths is pure and lives in `forge_viewport::nav` (VIEWPORT.md §7). The panel only
turns pointer input into calls. GUI session G2 §2 is unanswered: **all directions, speeds and
keys below are parity-pending, ours.** `nav_action(input) -> NavAction` with
`NavInput { primary, middle, secondary, alt, held: Option<NavMode>, drag, scroll, cursor }`:

| Input | Action |
|---|---|
| wheel `s` (points) at pointer `c` | `ZoomAt(0.999^s, c)`: zoom toward the cursor, `s > 0` zooms in |
| Alt + primary drag, or hold `V` + primary drag | `Orbit(dx, dy)` |
| middle drag (with or without Alt), or Alt + middle, or hold `X` + primary drag | `Pan(dx, dy)` |
| Alt + secondary drag, or hold `C` + primary drag | `Dolly(dx)`, about the view centre |
| anything else | `None` |

Wheel takes precedence over drags in the same frame. `NavMode` = `Pan | Dolly | Orbit` for the
held keys. The brief's "1/2/3 key + drag" is replaced by the hold keys X / C / V: 1 to 5 are the
shading keys of the key map (§14.2) and a press of `1` would change the shading, so the digit
keys cannot also be navigation modifiers. The hold keys are plain modifiers read from the
keyboard state (a key held with no Mod/Alt/Shift while the pointer is over the viewport), not
chords; they are not in the key map and are listed here only. While Alt or a hold key is down the viewport does no selection, rubber band or gizmo
interaction.

Navigation works even without a wgpu render state (it is evaluated before the "Viewport
unavailable" early return), so it is testable with egui_kittest.

The viewport stores its aspect ratio (`EditorState.view_aspect`, width / height, default 16/9)
each frame; the frame commands use it.

Key-map bindings (UI.md §14.2) for the commands of COMMANDS.md §6: `H` = `view.frame_all`,
`S` = `view.frame_selected`, `Mod+1` .. `Mod+7` = `view.set_view` perspective, front, right,
top, back, left, bottom. (`F1`..`F5` of the area design collide with `F2` rename; Mod+digit is
free.) View > menu entries: Frame all, Frame selected, and a "View" submenu with the seven
views, each showing its chord.

Tests (crates/ui/tests/navigation.rs): the input mapping of every gesture and hold key, `apply_nav`;
`view.frame_all` / `view.frame_selected` fit the bounds (aspect-aware), nothing selected frames
all, a selected null recentres, an empty scene resets; `view.set_view`, `view.set_camera`
(partial update, validation, pitch clamp) and `view.get_camera`; camera commands add no undo
step and Ctrl+Z leaves the camera alone; egui_kittest (no GPU): Alt + left/middle/right drags
orbit / pan / dolly, a held `V` turns a plain drag into an orbit (and not without it), the
wheel zooms toward the cursor, `H`, `S`, `Mod+4`, `Mod+1` run their commands.

Undo: camera moves, frames and view switches are **not** undo steps (the camera is view state,
not document state; COMMANDS.md §6). Gestures and `view.*` commands never touch the history.

Component modes (EDITABLE.md §2): the header of the viewport holds four toggle buttons
"Model", "Points", "Edges", "Polygons" (`EditorState.mode`). With a component target, left
click and left drag (not on a gizmo handle) select components instead of objects, and the
target is drawn with its points / edges / polygon outlines and selected components in the
component selection colour (§8); the panel rebuilds its GPU meshes when the revision, the
node selection, the mode or the target changes. With several polygon objects selected every
selected polygon object is a target and is drawn this way (EDITABLE.md §6); the Coordinates
panel shows editable Pivot, Rotation and Size rows (§6.7).

### 6.2 Panes, 4-view layout, lights and anti-aliasing (brief 143)

Maths: VIEWPORT.md §8-§9. G8 is unanswered: **the modes, keys, arrangement and the default
shading of the orthographic panes are ours, parity-pending G8.**

`EditorState.views: Views { layout, active, maximized, lighting, shadows, antialias }` plus the
saved per-pane `PaneState { camera, shading }`. `EditorState.camera` and `.shading` stay the
fields everything else uses: they are the **active pane's working copy**. `activate(pane)`
stores the working copy in the pane it leaves and loads the new one, so every tool, pick and
command that reads `state.camera` automatically acts on the active pane. `pane_camera(pane)` /
`pane_shading(pane)` read any pane (the working copy for the active one).

- Initial state: `Single` layout, active `Perspective`, `Top`/`Front`/`Right` cameras from
  `pane_camera`, shading `SmoothWire` for Perspective and `Wire` for the three orthographic panes
  (they start as line drawings, our choice). File > New resets all of it (the dock layout stays).
- `Single` shows the active pane only (the earlier behaviour). `Four` shows the 2×2 grid of
  VIEWPORT.md §9 in the viewport area, with a 2 px gap; the active pane has an accent outline.
  `maximized = Some(p)` in `Four` shows `p` alone, full size; toggling it off restores the quad.
- Hovering a pane, or pressing/dragging in it, makes it active before the input is read, so
  navigation, picking, gizmos, tools and `view.frame_*` act on the pane under the pointer. A drag
  that started in a pane keeps it active until it ends. Each pane draws its own label
  `"<Pane> · <shading>"` (plus the component mode), its own grid (from its camera), and its
  own texture; meshes are uploaded once and shared.
- Lighting: `LightingMode = Headlight | Rig` (default Headlight), `shadows` (default on; only a
  shadow-casting light, i.e. the Rig's key, produces shadows) and `antialias` (default on, 4× MSAA).
  `panes::display_settings(views, shading, scene_lights)` builds the viewport `Settings`; scene
  lights (VIEWPORT.md §8.1) override the mode.
- `viewport.screenshot` (RPC.md §10.2) renders what the viewport shows: the active pane in
  `Single`, the maximised pane, or in `Four` a composite of the four panes (gap strips in the
  theme's dark colour) at the requested size.
- Commands (COMMANDS.md §6): `view.layout`, `view.maximize`, `view.lighting`, `view.shadows`,
  `view.antialias`; `view.shading` and `view.set_view` take an optional `pane`.
- View menu: the shading list (all nine modes), "Layout: single / four", "Maximise pane"
  (checkbox, enabled in the 4-view layout), "Lighting: headlight / rig", "Shadows", "Anti-aliasing".

Tests: crates/ui/tests/panes.rs (state and commands, no GPU): initial panes, `view.layout`
toggle and explicit, `view.maximize` toggle and with a pane, per-pane `view.shading` and
`view.set_view` leave the other panes alone, activation keeps cameras, File > New resets, settings
builder (lighting, shadows, antialias, scene lights), screenshot composite; egui_kittest: F4
shows four panes and back, `Mod+M` maximises the active pane to one and back, hovering another
pane makes it active, the View menu radio entries.

## 7. Timeline (`timeline`, `animate`)

Behaviour, data and rules: docs/specs/ANIM.md §2. Panel summary (own design,
**parity-pending**):
- Row 1: Play / Stop, Record, Auto-Key toggle, Previous / Next key, Delete Keys.
- Row 2: Start, End and FPS fields (edit the scene settings, undoable), the current frame
  field, and the label of the current time.
- Row 3: a frame strip. Click or drag scrubs the current frame; the playhead and one tick
  per key frame of the selected objects are drawn by us.
- Time controls (brief 094, own design, **parity-pending G5**), added to the two button rows so
  the panel keeps its height (view tests depend on the pane geometry): at the end of row 1 a
  "Playback mode" combo (Once, Loop, Ping-Pong), an "Every frame" checkbox (off = real-time with
  frame dropping) and a "Time display" combo (Frames, SMPTE, Seconds); at the end of row 2 a
  "Preview range" checkbox with "Preview start" / "Preview end" fields (while on) and the buttons
  "Add Marker", "Previous Marker", "Next Marker". The label after the frame field shows the
  current time (seconds in Frames display, else the chosen format). The FPS field runs
  `timeline.set_fps` (keys keep their seconds, ranges and markers convert); Start and End run
  `timeline.settings`.
- The 30 pt strip keeps its ruler and key ticks (ticks now start 10 pt from the top). The top
  10 pt are the marker lane: each marker is a flag in its colour with its name (label "Marker
  <name>"): click goes to its frame, drag moves it (`marker.move`), the context menu has the
  name, "Color <name>" entries and "Delete Marker". The preview range shades the outside of the
  strip, draws an accent band along the bottom and two handles (labels "Preview start handle" and
  "Preview end handle"; dragging runs `timeline.set_preview`, the handles cannot cross). Keys M
  and Alt+Comma / Alt+Period (ANIM.md §2.7).
- `Timeline { current: 0, playing: false }` is editor state; start, end and fps live in
  the scene (`SceneSettings`, default 0, 90, 30). `set_current` clamps to the range.
- The Attributes panel draws a marker left of every animatable row: filled diamond =
  key on this frame, hollow = animated without a key here (ANIM.md §2.5).

## 8. Theme (`theme`)

Three themes (own palettes, §24.1): dark (default), light, high contrast. The text below is
the dark one.

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

Snapping and axis mode exist as commands (`snap.*`, `edit.axis_mode`, MANIPULATE.md §6–7) and as Edit menu entries; they are not palette buttons yet.
Pointer-driven tools (cuts, bridge, pen) stay in the Mesh menu, as do the arrangement tools
(Array, Duplicate, Arrange, Randomize, Transfer; MODELING.md §9.7), which have a settings panel and no
viewport interaction.

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

## 13. Object manager (`tree`, `object_manager`; brief 149)

Sources: own design from docs/roadmap/areas/L-ui.md; GUI session G1 (docs/parity/GUI-SESSIONS.md)
is unanswered, so **the column order, what the dots and tick show, the dot click cycle, the
drop zones and the context menu items are parity-pending**. Look and labels are ours.
All edits are registry commands (COMMANDS.md §6); the panel only computes arguments, so an
agent gets the same behaviour. Each user gesture is one undo step.

### 13.1 Columns (left to right, parity-pending)

1. Expand arrow (`+` / `-`, only for rows with children), indented 14 px per depth.
2. Name (type label as a weak suffix), renamed in place.
3. Editor dot, Render dot, Enable tick.
4. Tags: one weak `[label]` per tag on the host.

Dot state (`object.editor_vis`, `object.render_vis`, `Vis`): `Default` (grey: follows the
parent, root = on), `Off` (red), `On` (green, forced on even under an Off parent). A click
cycles Default -> Off -> On -> Default and runs `object.visibility` for the clicked row (or
for the whole selection when the row is selected). The tick is `object.enabled`; a click runs
`object.enable` with the inverted value. A node is shown in the viewport iff it and every
ancestor are enabled and its resolved editor visibility (`forge_core::visibility`) is on.
Render visibility is stored and resolved the same way (`render_visible`) but honouring it in
the render bridge is a follow-up (the renderer does not read it yet).
Accessibility labels (used by the tests): `Editor visibility: <name>`, `Render visibility:
<name>`, `Enabled: <name>`.

### 13.2 Selection (pure, `tree::click_selection`)

Plain click selects only the row; Shift-click selects the visible rows from the anchor to
the row (`NodeSelection::range`); Ctrl/Cmd-click toggles the row. The anchor is set by plain
and Ctrl clicks. A Shift range replaces the selection (it does not add), like the viewport.
Clicking selects through `NodeSelection`; it is not a command and not undoable.

### 13.3 Drag and drop (pure, `tree::drop_target`, `tree::plan_drop`)

A drag starts on a row's name; the payload is the dragged `NodeId` (the attribute manager's
link fields accept it, UI.md §12). The set moved is the whole selection when the dragged row
is selected, else only the dragged row; nodes below another moved node move with it.
The pointer height within the hovered row decides the zone (`DropZone`): top quarter
`Before`, bottom quarter `After`, middle `Into`. `drop_target(rows, hovered, zone, moved)`
returns `DropTarget { parent: Option<NodeId>, index: Option<usize> }` (index counts the
parent's children without the moved nodes; `None` = last) or `None` when the drop is invalid:
dropping on a moved node, or into/next to a descendant of a moved node (a cycle).
`Into` = last child of the row; `Before` = at the row's index in its parent's child list;
`After` = right after it, except an expanded row with children: `After` then means first
child (the position the line is drawn at). The empty area below the rows = root list, last.
The drop runs `object.move` (`nodes` in visible order, `parent`, `index`). A drag hover
shows an insertion line (Before/After) or a row highlight (Into). Dragging is disabled while a
search filter is active. Moving keeps local transforms, like `object.reparent`.

### 13.4 Rename

Double-click (or F2, or the menu item) opens a text field on the row; Enter or focus loss
commits via `object.rename` (an unchanged or empty name is not committed), Escape cancels.

### 13.5 Search / filter (pure, `tree::filter_rows`)

A text field above the rows. A row matches when the query (trimmed, case-insensitive)
is a substring of its name, type label or one of its tag labels. The filtered list
is the matching rows plus their ancestors (so paths stay visible), in tree order, with
every ancestor of a match shown expanded; ancestors that do not match themselves are `dim`.
An empty query shows the normal tree. The query is panel state, not document state.

### 13.6 Context menu (pure, `tree::context_menu`)

Right-click on a row that is not selected selects it first (plain click); the menu acts on
the selection. Items, all top level (no sub-menus), each `{label, command, args}`:
Rename (no command: starts the in-place edit; single selection only), Group Objects
(`object.group`), Make Editable (`mesh.make_editable`), Hide / Show (`object.visibility`
editor and render both Off, or Default when all are already Off), Disable / Enable
(`object.enable`), Move to Root (`object.move` parent null), Delete (`object.delete`).
"Make Editable" is listed only if some selected node is a generator.

### 13.7 Tests

Pure unit tests (crates/ui/tests/object_manager.rs): drop targets for every zone,
cycles, ranges, filter, context items, visibility resolution (core). egui_kittest: reparent
by drag, reorder by drag, dots and tick (each with undo), rename, search, context menu
actions, a drag onto an attribute link field.


## 14. Key map (`keymap`, `keymap_editor`; brief 152)

Sources: own design from docs/roadmap/areas/L-ui.md; GUI session G2 (docs/parity/GUI-SESSIONS.md)
is unanswered, so **every default binding is parity-pending**: they follow common DCC
conventions where those do not collide with our other keys, and they are ours to change once
G2 is answered. Nothing is taken from the reference app's key files or presets.

### 14.1 Model

A `Chord { command, alt, shift, key }` is one key press with modifiers. `command` means
egui's `Modifiers::COMMAND`: Cmd on macOS, Ctrl elsewhere. A `Binding { chord, command,
args, scope }` runs the registry command `command` with the JSON object `args`. `scope` is
`global` (default) or `viewport` (only while the pointer was over the viewport; the Space
rule of ANIM.md §2.7). `KeyMap { bindings }` is the resource, held by `EditorState.keymap`.
Every shortcut runs a registry command through `commands::execute`; there is no key handling
outside the map, so a key press is exactly as undoable as the same menu item. Window-only
actions are commands too (`file.open_dialog`, `file.save_dialog`, `file.save_as_dialog`,
`object.begin_rename`, `view.gizmo_mode`, `view.gizmo_space`, `timeline.toggle_play`;
COMMANDS.md §6); headless contexts refuse the dialog ones.

Text form of a chord: modifiers then key joined by `+`, canonical order `Mod`, `Alt`,
`Shift`, key name = egui `Key::name()` (`Z`, `Num1`, `F2`, `Comma`, `Delete`). `Mod` is the
platform command key; the parser also accepts `Ctrl`, `Cmd` and `Command` as `Mod`, any
case, and a lone `+`-free key. Printing is canonical, so `parse(print(c)) == c`. Unknown key
names, repeated modifiers and an empty text are errors. The menus and the editor show a
chord in the platform's own spelling (`format_shortcut`).

### 14.2 Defaults (`crates/ui/keymap/default.toml`, embedded with `include_str!`)

```toml
version = 1
[[bind]]
chord = "Mod+Z"          # required
command = "edit.undo"    # required, a registry command id
args = { }               # optional table, default empty
scope = "global"         # optional, global | viewport
```

Several `[[bind]]` entries may name the same command (redo has two chords). Loading rejects
a version other than 1, an unparsable chord, an unknown scope and `args` that is not a table.
Default bindings (all parity-pending, ours):

| Chord | Command (args) |
|---|---|
| Mod+O / Mod+S / Mod+Shift+S | `file.open_dialog` / `file.save_dialog` / `file.save_as_dialog` |
| Mod+Z | `edit.undo` |
| Mod+Shift+Z, Mod+Y | `edit.redo` |
| Delete | `object.delete` |
| F2 | `object.begin_rename` |
| Mod+A / Mod+Shift+A / Mod+I | `select.all` / `select.none` / `select.invert` |
| Alt+1 / Alt+2 / Alt+3 / Alt+4 | `select.mode` model / points / edges / polys |
| Mod+Shift+E | `mesh.make_editable` |
| Mod+Shift+P | `ui.command_palette` (brief 159, §18) |
| 1 .. 5 | `view.shading` smooth, flat, wire, smooth_wire, flat_wire |
| 6 .. 9 | `view.shading` quick, constant, hidden_line, bounding_box (brief 143) |
| F4 | `view.layout` (toggle single / four; brief 143) |
| Mod+M | `view.maximize` (toggle the active pane; brief 143) |
| W / E / R | `view.gizmo_mode` move / rotate / scale |
| Q | `view.gizmo_space` (toggle) |
| Shift+S | `snap.toggle` (brief 032; X is the Pan hold key) |
| L | `edit.axis_mode` (toggle; brief 032) |
| H / S | `view.frame_all` / `view.frame_selected` (brief 153) |
| Mod+1 .. Mod+7 | `view.set_view` perspective, front, right, top, back, left, bottom (brief 153) |
| K / Shift+K | `key.record` / `key.autokey` |
| Space (viewport scope) | `timeline.toggle_play` |
| Comma / Period | `timeline.step` delta -1 / +1 |
| M | `marker.add` (brief 094) |
| Alt+Comma / Alt+Period | `marker.jump` backward / forward (brief 094) |
| Shift+Comma / Shift+Period | `key.jump` forward false / true |

These are the old hard-coded keys of §5 and ANIM.md §2.7 unchanged, plus the rows for
`select.invert`, `select.mode` and `mesh.make_editable`.

### 14.3 User overrides (settings file)

The shared settings file of RPC.md §2 (`forge_platform::settings_path()`: `$FORGE_SETTINGS`, else the per-OS config
directory, PLATFORM.md §1-2) holds
an optional object `keymap`, keyed by canonical or parseable chord text:

```json
{"keymap": {"Mod+Shift+K": {"command": "key.autokey", "args": {}, "scope": "global"},
            "Mod+Z": null}}
```

An object value binds that chord (replacing every default binding on the same chord); `null`
unbinds the chord. Other keys of the file (rpc, scripts) are kept when the editor writes it.
Merge (`KeyMap::apply_overrides`): start from the defaults; for each override chord remove
the bindings on that chord, then add the new one unless `null`. An entry with an unparsable
chord, no `command` or a non-object `args` is skipped with a warning (returned, shown in
the editor), never an error: a hand-edited file cannot disable the editor. A command id
unknown to the registry is kept (plugins may be missing) and listed in the editor as
unknown; running it reports the registry's error in the status bar. `KeyMap::overrides()`
is the inverse: the minimal JSON that, applied to the defaults, gives the current map.
When it is empty the `keymap` key is removed. A missing or unreadable file means defaults.

### 14.4 Conflicts

Two bindings conflict when they have the same chord and their scopes overlap (`global`
overlaps everything; `viewport` overlaps `viewport` and `global`). `KeyMap::conflicts()`
lists them; the default map has none (tested). Dispatch with a conflict is deterministic:
the first matching binding in map order runs, and exactly one command runs per key press.
Dispatch is matched exactly on `Mod`, `Alt`, `Shift` (so Mod+A and Mod+Shift+A need no
ordering) and the matching key event is consumed. Nothing runs while a text field has the
keyboard or while the editor is capturing a chord.

`bind` removes the bindings that conflict with the new one and returns them. The editor
never does that silently: it shows the conflicting command and asks to Replace or Cancel.

### 14.5 Key map editor (Window > Key map...)

A floating window (state in `Editor`, not in the layout). Top: a search field (matches
command id, label, or the chord text, case-insensitive), a "Reset all" button, a count of
conflicts and unknown commands, and the warnings of the last load. Below, a scrolling list,
one row per binding plus one row for each registry command that has no binding: command
label, id (weak), args summary, the chord as a button. Clicking the chord button starts
capture ("Press keys..."): the next key press with its modifiers becomes the chord; Escape
cancels. If the chord is taken by another binding the window shows "<chord> is bound to
<label>" with Replace and Cancel; Replace rebinds and removes the other. Each row has
Clear (unbind) and, when it differs from the default, Reset. Every change is stored to the
settings file at once (when the editor has one) and takes effect on the next key press.

### 14.6 Tests

Unit (crates/ui/tests/keymap.rs): chord parse/print round trip over every egui key, aliases
and errors; the default map loads, has no conflicts, and every bound command (with args)
runs against the registry's argument schema; conflict rules for scopes; override merge
(replace, unbind, invalid entry skipped, round trip through `overrides()`); the settings
file keeps unrelated keys. egui_kittest (no GPU): pressing each kind of default shortcut
runs its command and Mod+Z undoes it; a rebind in the editor takes effect; a conflicting
rebind asks first; Reset and Clear; a changed map is written to the settings file.

## 15. Autosave and recovery (`autosave`, `app`; brief 179)

Policy, files and crash detection: PLATFORM.md §7. `EditorState.autosave` owns the schedule,
the background writer and the session marker; `Editor::show` calls `autosave.tick` once per
frame, and `Editor`'s `on_exit` ends the session cleanly. When a crashed session left
autosaves, `show` draws a window titled "Recover unsaved work" (an `egui::Window`, shown
until the list is empty) with one row per entry (`Untitled` or the original path, the time,
the size) and the buttons "Recover", "Discard" and "Discard all". `Editor::recover_autosave(i)`
and `Editor::discard_autosave(i)` do what the buttons do (tests call them directly).

### 15.1 Closing the window

When the window is asked to close (`ViewportInfo::close_requested`) and the document is dirty,
`Editor::show` asks the same "save changes?" question as File > New (`Dialogs::confirm_unsaved`)
before the close goes ahead: **Save** saves (Save As when the document has no file; if that
dialog is cancelled the window stays open) and then closes, **Don't Save** closes (the clean
exit removes the autosave), **Cancel** sends `ViewportCommand::CancelClose`. A clean document
closes at once without a prompt.

### 15.2 Crash report notice (`crash_notice`; brief 178)

On start `CrashNotice::start_default` lists the unseen crash reports (PLATFORM.md §8.5). If
there are any the window "Forge3D crashed last time" is shown: the report file names (newest
first), a line "Autosaved work may be offered in the recovery window." when
`recoverable_count() > 0`, and the buttons **Open folder** (opens the crash folder with the
system file browser: `xdg-open` / `open` / `explorer`) and **Dismiss**. Either button marks the
reports seen and closes the window. Tests inject a folder opener (`CrashNotice::set_opener`).

## 16. Clone toolset UI (`widgets::link_list`, `clone_overlay`, `commands`; brief 079)

Sources: own design from docs/roadmap/areas/E-clone-toolset.md and the link-list model of
brief 068; GUI session G4 (docs/parity/GUI-SESSIONS.md) is unanswered, so **the look of the
list rows, the Create menu grouping and the overlay style are parity-pending G4**. The wording
is "cloner", "clone toolset", "effector".

### 16.1 Link list rows

The cloner's `effectors` list, an effector's `fields` list and a deformer's `fields` list are
the cloner's list is a `LinkList`, the two `fields` lists are `LinkRows` parameters (FIELDS.md F7),
both drawn by `widgets::link_list` (UI.md §12). One row per entry:

`[x] name   [blend] [strength]   ^  v  x` (blend and strength only on `LinkRows`)

- **Enable box** (`Enable <name>`): in a `LinkList` it is shown when the entry node has an
  `enabled` parameter (effectors) and edits that parameter **on the entry node**. In a
  `LinkRows` list (field rows) it is the row's own `enabled` and edits the list on its owner. A
  disabled entry keeps its place and is skipped when the list is applied (effector: no effect,
  MOGRAPH.md E1; field: the layer is left out of the `FieldList`, FIELDS.md F7).
- **Blend** (`Blend <name>`, a combo of the nine blend modes) and **Strength** (`Strength <name>`,
  percent): only in `LinkRows` lists; they edit the row (brief 059, FIELDS.md F7), so a field used by
  two lists has one blend mode and strength per list. Every row edit writes the whole list with
  `param.edit` on the owner: one undo step.
- `^` / `v` (`Move <name> up/down`) reorder: the new list order is the order the effectors
  or layers apply in (effectors in list order, MOGRAPH.md E1; layers top to bottom).
- `x` (`Remove <name>`) removes the entry from the list only; the node stays in the scene.
- The "Add..." drop-down lists the objects the parameter accepts that are not in the list; it
  is also the **drop target** for an object dragged from the object manager (the dragged
  node is appended; a node already in the list is ignored).

Every change is one `param.edit` command: a list edit on the list's owner (all selected
objects of that type), or an enable edit on the entry node of a `LinkList`. `Edit` carries an
optional `target` (`param`, `node`) for the latter. One undo step per click.

### 16.2 Create menu and "Cloner from selection"

The Create menu starts with a **Clone toolset** submenu (`clone_toolset_menu`, pure): the
cloner, **Cloner from Selection**, every registered effector type, every registered field
type, in that order, taken from the registry (so a new effector or field type appears with
no UI change). The usual per-category submenus stay.

`Cloner from Selection` runs `object.create_cloner` (COMMANDS.md §6): the selected objects
become children of a new cloner placed where the first selected object was, and the cloner
is selected. The children are the cloner's prototypes (MOGRAPH.md C3). Selected effectors
and fields are not prototypes: the effectors are put in the cloner's `effectors` list (in
selection order) and fields are left alone. With nothing selected the command creates an
empty cloner.

### 16.3 Clone display

The cloner's existing `viewport_mode` parameter (Off, Points, Matrix, Bounding Box, Object)
keeps drawing the clone geometry for every value (the other values are not wired yet,
**parity-pending**); `matrix` additionally turns the axes overlay on for that cloner. Two
view options (not undoable, `view.clone_display`, `Views.clone_indices`,
`Views.clone_axes`; View menu **Clone Indices**, **Clone Axes**):

- **Indices**: the index of each visible clone, drawn at its projected origin.
- **Axes**: the clone's X, Y, Z axes as red, green, blue lines of 24 px (the Matrix-style
  display); also drawn for a cloner whose `viewport_mode` is `matrix`.

Both apply to the selected cloners only. `clone_overlay::clone_marks` is pure: it takes the
clone data, the cloner's world matrix, the camera and the viewport size and returns the
marks (clone index, screen position, axis end points), skipping hidden clones, clones behind
the camera and clones outside the viewport, and at most `MAX_MARKS` (2000) marks, lowest
indices first, so the overlay stays interactive with 10 000 clones. The editor paints the marks
with the egui painter after the viewport image.

### 16.4 Viewport instancing

Cloners are drawn by GPU instancing (VIEWPORT.md §10.8): `scene_draw_items_at` returns
`DrawItem::Instanced` for every cloner `cloner_instances` can show, and the instance buffer
is rebuilt only when the cloner's content changes. Effector and field edits change the clone
data, so they rebuild it; selection changes do not. Nothing in 079 expands geometry for
display. Cloners with enabled deformer children keep the expanded fallback (VIEWPORT.md §10).

### 16.5 Tests

`crates/ui/tests/clone_ui.rs`: egui_kittest on the full registry.

- Cloner from selection: children and selection, one undo step restores the scene.
- Effectors: add by the list, add by dragging an effector from the object manager, remove.
- Reorder: with a Plain and a second effector whose rotation and scale differ, the clone
  matrices after swapping the order differ from before (the order matters, O14
  `two-effectors`), and swap back restores them; undo of each step.
- Enable box: the effector stops acting (clone matrices equal the no-effector ones) and undo
  brings it back; the same for a field row (the row's `enabled`); the strength box edits the row's
  `strength` (the field node is untouched) and the clone result follows.
- Pure: Create menu entries, `clone_marks` (counts, hidden clones, cap, behind camera,
  projection), the 10 000 clone instance data builds and culls.
- Instancing: a cloner of 10 000 clones is one `DrawItem::Instanced`.

## 17. Node editor (`node_canvas`, `node_editor`; brief 109)

Sources: project design (docs/roadmap/areas/H-rigging.md "Node expressions"), NODES.md, the
public node-graph editor literature. **Everything here is own design, parity-pending G6**
(the owner has not described the classic editor's node look or how nodes are added); the look
is deliberately plain and not modelled on any screenshot. Canvas decision: DECISIONS.md D17
(our own canvas; `egui-snarl` needs egui 0.36).

### 17.1 Two layers

- `node_canvas` is a **generic** canvas. It knows nothing about expressions. It draws a
  `CanvasModel` (a trait: the nodes with title, category, position and typed ports; the links;
  `check_link(from, to) -> Result<(), String>`; the entries of the add menu) and returns
  `CanvasAction`s (`AddNode`, `Move`, `Connect`, `Disconnect`, `Delete`, `SetConstant`). It never
  changes the model. Node materials (brief 124) implement the same trait.
- `node_editor` is the panel for expressions: `ExpressionModel` implements `CanvasModel` over a
  `forge_nodes::Graph` read from the tag, and `run_action` turns each action into one `graph.*`
  command (all edits are registry commands, one undo step each).

### 17.2 Panel

`Panel::NodeEditor`, title "Node Editor", default area Centre as a tab next to the viewport,
closed by default; opened from Window > "Node Editor panel" and by **double-clicking an
expression tag** (`[Expression]`, the `forge.expression` tags) in the object manager, which also
selects that tag in the panel. The header has a combo "Expression tag" listing every
`forge.expression` tag as `<object name> / <tag name>`, a **Add expression tag** button (runs
`object.create` with type `forge.expression` on the active object; it is disabled without an
active object) and the evaluation message. With no tag chosen the panel shows the first
expression tag on the active object, else the first in the scene, else a hint.
The chosen tag is panel state (`EditorState::node_editor`), not saved. The canvas view (pan, zoom)
and the selection are kept per tag while the editor runs.

### 17.3 Canvas model (pure, unit-tested in `node_canvas`)

World units. `View { pan, zoom }`: `screen = rect.min + pan + world * zoom`. Zoom is clamped to
0.25..2.5; `zoom_about(p, factor)` keeps the world point under `p` fixed.

Node box (world): width 200, title bar 24, rows 22, padding 8. A node's height is
`24 + 22 * (outputs + inputs) + 8`. **Outputs are listed first**, right aligned, with the port
dot on the right edge; then the inputs, left aligned, with the dot on the left edge. Port dots
are centred on the row at the edge, radius 5 (hit radius 10 screen pixels). `pos` is the box's
top-left corner. Links are cubic Béziers from an output dot to an input dot with horizontal
tangents of `max(40, |dx| / 2)`; the hit test samples 24 points.

Hit testing, in priority: a port (within 10 px), then a node (the last drawn is on top),
then a link (within 6 px of the sampled curve), then empty canvas. A node box selected by
`nodes_in_rect` when it intersects the box-select rectangle.

Colours (own design): port colour by type name: float blue, int teal, bool orange, vec3 yellow,
colour magenta, matrix violet, string grey, node cyan, any white. A port is a filled dot when
linked and a ring when free. Title bars take a hue from a hash of the category.

### 17.4 Interactions

| Gesture | Effect |
|---|---|
| Mouse wheel | zoom about the pointer (`factor = 1.0015^scroll`) |
| Middle-drag, or Alt/Space + left-drag | pan |
| Click a node | select it (Shift or Ctrl toggles) |
| Click empty canvas | clear the selection |
| Left-drag a node | move it and the other selected nodes (an unselected node becomes the selection); `Move` every frame with absolute positions, one gesture id per drag so the drag is **one undo step** |
| Left-drag on empty canvas | box select (Shift adds) |
| Left-drag from a port | draw a wire. From an output to an input, or from a free input to an output |
| Left-drag from a **linked input** | picks the link up (it is drawn from its output to the pointer) |
| Release on a port of the other side | `check_link`; accepted: `Connect` (a picked-up link is moved with `release_*`, one step); refused: nothing changes and the reason is shown |
| Release on empty canvas | a picked-up link is removed (`Disconnect`); a new wire is dropped |
| While dragging a wire | the port under the pointer gets a green ring if the link is allowed, a red one with the reason next to the pointer if it is not |
| Right-click or double-click empty canvas | the add menu at that point |
| Right-click a node | menu: Delete node (all selected nodes if it is selected), Disconnect inputs |
| Right-click a link | menu: Disconnect |
| Delete or Backspace (pointer over the canvas) | delete the selected nodes (one step). Delete is bound to `object.delete`; while the pointer is over the node editor the key goes to the canvas instead |
| Esc | closes the menu / cancels the wire drag |

Add menu: a window with a search field ("Search nodes") and the node types from
`graph.node_types`, grouped by category, each entry a button `Label (type.id)`. The search is a
case-insensitive substring match on id, label and category; no match shows "No match". Choosing
an entry adds the node at the menu's position (`AddNode`) and closes the menu.

Constant values: every input that has no link shows an inline editor in its row: Float and Int a
drag value, Bool a check box, String a text box, Vec3 and Colour three drag values. Matrix, Node and
Any inputs show their value as text (no editor); Text edits commit when the box loses focus
(Enter or a click elsewhere), one step per committed text. Editing runs `graph.set_value`; a drag of a
number is one gesture and one undo step. The editors are drawn at normal size and only when the
zoom is 0.75 or more; below that the row shows the value as text.

Refusals: the reason is the graph's own error text (`cannot connect float to node`, `the link
would close a cycle through ...`), shown on the canvas bottom line and in the status bar until the
next interaction.

### 17.5 Live values

The panel keeps a dry-run result from `graph.evaluate` (NODES.md §6) for the chosen tag and
refreshes it when the scene revision, the current frame or the tag changes (so it follows
playback and edits). Each output port row shows its value to the right of its label (floats to 3
decimals, Vec3 as `(x, y, z)`, Bool `true`/`false`, strings quoted and cut at 16 characters,
matrices as `mat4`, nodes as `#id` or `none`); a linked input shows the value of its source. If
the evaluation fails, no values are shown and the header shows the error (`graph node 3: ...`).
Values come from every node (the dry run evaluates all nodes), including ones no sink needs.

### 17.6 Node positions

`pos` of each graph node is stored in the graph JSON (NODES.md §6). `Move` becomes `graph.move`
(gesture-merged), `AddNode` `graph.add_node` with `pos`. Positions have no effect on evaluation,
but they are part of the tag's `graph` parameter, so moving a node is an edit of the document
(dirty marker, one undo step per drag).

### 17.7 Tests

- `crates/ui/src/node_canvas.rs`, `node_editor.rs` (unit): view transforms and round trip, zoom
  about a point and its clamp, node and port geometry, hit testing of ports (radius at any zoom),
  nodes (z-order), links, box selection, Bézier sampling, menu filtering, value formatting,
  constants by port type, menu entries from `graph.node_types`.
- `crates/ui/tests/node_editor.rs` (egui_kittest on the full editor): open from the Window menu and
  by double-click on the tag; add a node through the search menu (filter, pick) and undo; connect
  two nodes by dragging (one step) and undo; a refused connection (type) changes nothing and shows
  its reason, also a cycle; moving a node by dragging is one undo step and the position is saved in
  the graph and survives undo/redo; the verdict (green/red) on the port under a dragged wire; picking up a link and dropping it on empty canvas disconnects;
  moving a link to another input is one step; box select and Delete remove nodes (one step, links
  go); a value edit (`graph.set_value`); live values appear on ports; and the expression
  built in the editor drives the scene (a cube's position follows the graph after the edits, and
  after undo returns).

## 18. Command palette and undo history (`palette`, `history_panel`; brief 159)

Sources: own design from docs/roadmap/areas/L-ui.md (command-palette search is a common editor
convention); no reference-app observation. The binding Mod+Shift+P is ours (parity-pending like
every §14.2 default; it collides with no other default). Nothing is hard-coded: the palette reads
the command registry (COMMANDS.md) and the key map (§14).

### 18.1 Commands

- `ui.command_palette` (no arguments, not undoable): asks the window to open the palette
  (`UiRequest::CommandPalette`, like the file dialogs; headless contexts fail with `needs the
  editor window`). Bound to Mod+Shift+P; also Window > "Command palette...".
- `edit.history_jump {position}` (not undoable itself; moves along the document history): see §18.5.

### 18.2 Fuzzy matcher (pure, `palette::fuzzy_score(query, text) -> Option<i32>`)

Case-insensitive subsequence match: every character of `query` occurs in `text`, in order.
`None` if not; an empty query gives `Some(0)`. The best alignment (dynamic programming over all
alignments) maximises the sum of

- +10 per matched character,
- +20 when the matched character starts a word (text index 0, or the previous character is
  one of ` ` `.` `_` `-` `/` `:` `=` `,`),
- +15 when it directly follows the previous matched character (a run),
- +15 when the first match is at index 0,
- -1 per skipped character between two matches, and -1 per skipped leading character (at most 5),

minus `len(text) / 4` so that a shorter text wins a tie. Consequences the tests pin: a prefix
beats a later word start, a word start beats a mid-word hit, a contiguous run beats scattered
letters, and `undo` finds `edit.undo` and "Undo".

### 18.3 Entries (`palette::entries(registry, keys, shown)`)

One entry per registry command, in registry order: id, label, whether it needs a form, and the
first key-map binding of that command with no arguments (`shown` spells a chord for the platform,
as in the key map editor). In addition one *preset* entry per key-map binding with non-empty
arguments (`view.shading` `{"mode":"wire"}` bound to 3, ...): its label is the command label
followed by the arguments summary (`Shading: mode=wire`), its chord that binding's, and it runs
with those arguments without a form. So the palette lists every chord the key map has.

A command *needs a form* when `{}` does not validate against its `args_schema()`
(`forge_command::schema::validate`), i.e. it has required arguments. Other commands run
directly with `{}` (their optional arguments stay at their defaults).

### 18.4 Search and order (`palette::search(entries, query, recent) -> Vec<usize>`)

The query is split on whitespace; every token must match at least one *field* of the entry (id,
label, arguments summary, chord canonical text `Mod+Shift+P` and the shown spelling). An entry's
score is the sum over tokens of the best field score. Order: score descending, then recently run
first, then label, then id. With an empty query nothing is filtered: recent entries first (newest
first), then the others by label (case-insensitive), then id.

*Recent*: `Palette::recent`, keys of the commands run from anywhere in the window (menus, shortcuts,
palette; not `ui.command_palette`), newest first, at most 20, no duplicates. The key of a run is the
command id, or `id` + space + compact JSON arguments when the arguments are not empty; a run with
arguments records both keys (so a command run through its form comes back to the top as the generic
entry, and a preset recurs as itself). In memory only; it is not saved.

### 18.5 The palette window

A floating window anchored at the top centre of the editor (not part of the layout), opened by
`ui.command_palette`; Escape closes it. A single-line search field has the focus on opening. Below
it a scrolling list (at most 12 rows visible) of the matches: label, id in weak text, the chord
right-aligned. Up/Down move the selected row (wrapping not), Enter or a click runs it, the first
row is selected after every query change. Running closes the palette, then `Editor::run_id` runs the
command, so a failure shows in the status bar, exactly as for a menu item.

An entry that needs a form replaces the list by a form generated from `args_schema()`
(`palette::form_fields(schema)`): a heading with the label and id, one row per property of the schema
(required ones first, marked `*`, the description as hover text), with **Run** and **Back** buttons.
Field kinds: `string` a text field; `integer` / `number` a text field parsed as such; `boolean` and
enumerations (`enum`, `oneOf` of constants, `$ref` to such a definition, nullable `anyOf`) a drop
down (optional ones have an empty entry that omits the key); everything else (arrays, objects,
`null`able composites) a text field holding JSON. `palette::build_args(fields, values)` turns the
texts into the argument object: empty optional fields are left out, an empty required one is
`"<name> is required"`, a text that does not parse is `"<name>: ..."`; the object is then checked
with `schema::validate` and a failure is shown in the form (error colour) without closing it.
Enter in a text field also runs.

### 18.6 History panel (`history_panel`)

Window > "History panel" (panel `Panel::History`, title "History", default area right bottom, closed
by default, `Dock::default()` unchanged). It lists the document's transactions (CORE.md §4)
from `Document::history()`: first a row "0. Original state", then one row per undo step oldest
first ("1. Create Cube"), then the redo steps in the order they were done (dimmed). A gesture whose
edits were merged (CORE.md §4 merge keys, e.g. a drag) is one transaction and shows once. The row
of the current state (as many steps applied as the undo stack is long) is marked: strong text and a
"current" tag. Pure helper `history_rows(undo_labels, redo_labels) -> Vec<HistoryRow {position,
label, kind: Past | Current | Future}>`.

Clicking the row at `position` runs `edit.history_jump {position}`: `position` is the number of
steps that should be applied, 0 ..= undo steps + redo steps. The command calls undo (or redo)
repeatedly until the stack depth equals `position`, then does what `edit.undo` does afterwards
(`retain_existing`, clamp the timeline), and returns `{position, undone, redone}`. A `position` out
of range is `InvalidArgs` and changes nothing. Jumping to the current position does nothing. The
jump is not a new history step; a later edit clears the redo steps as usual, which the panel then
shows.

### 18.7 Tests

- Unit (`palette.rs`): `fuzzy_score` (subsequence, case, empty query, no match, the orderings of
  §18.2), `search` (tokens, fields, chord text, recency order, empty query order), `entries`
  (presets from bindings, `needs_form` from the schema), `form_fields` and `build_args` on schemas
  of real commands (`object.create`, `view.shading`, optional and enum fields, errors), recent
  bookkeeping; `history_rows`.
- `crates/command/tests/registry.rs`: `edit.history_jump` jumps back and forth over a headless
  document, and refuses a position out of range.
- `crates/ui/tests/palette.rs` (egui_kittest, no GPU): Mod+Shift+P opens the palette; typing finds
  a command, Enter runs it (a document edit with its undo step); a click runs another; a command
  with required arguments opens a form, a wrong value shows an error, a right one runs it; Escape
  closes; the recent order; the panels are closed by default and open from the Window menu. History
  panel: rows and the current mark follow edits, undo and redo; clicking an older row undoes to
  that state and a future row redoes; a merged gesture is one row.

## 19. Console (`console`; brief 160)

Sources: own design from docs/roadmap/areas/L-ui.md; the log line format is PLATFORM.md §8.2 (brief
178), the REPL runs the sandbox of SCRIPTING.md. No reference-app observation.

Window > "Console panel" (panel `Panel::Console`, title "Console", default area Bottom, closed by
default; `Dock::default()` unchanged). Two halves side by side (`ui.columns(2)`): the **log** on the
left, the **REPL** on the right.

### 19.1 Log

The lines come from `forge_platform::logging::recent_lines()`, the in-memory tail of the last 200
log lines (PLATFORM.md §8.2: `<time> <LEVEL> <target>: <message>`), read again every frame (the
panel requests a repaint once a second while it is open, so new lines show up). The source is a
function held by `ConsoleState::log_source` so that tests can feed lines.

- **Level filter**: a drop-down "Level" with Trace (all), Debug, Info, Warn, Error; a line shows
  when its level is at least the chosen one. Default Trace.
- **Search**: a text field "Search log"; a line shows when it contains the text, case-insensitive
  (an empty text shows all).
- **Copy**: the button "Copy" puts the shown lines, joined by newlines, on the clipboard
  (`Context::copy_text`) and remembers the text in `ConsoleState::last_copy`.
- Shown with the level colour (error: theme error colour, warn: amber, info: text, debug and trace:
  weak text), monospace, sticking to the bottom while the scroll is at the end. A count "N of M lines"
  is shown next to the toolbar.

Pure functions (unit-tested):

- `parse_level(line) -> Option<LogLevel>`: the level is the second whitespace-separated token of the
  line when it is exactly `TRACE`, `DEBUG`, `INFO`, `WARN` or `ERROR`; other lines have none.
- `filter_log(lines, min_level, search) -> Vec<LogLine {index, level, text}>`: a line without a
  level (a continuation line of a multi-line message) takes the level of the line before it, or Info
  at the start; then the two conditions above. `index` is the position in `lines`.
- `log_text(&[LogLine])`: the lines joined by `\n`.

### 19.2 REPL

A multi-line monospace text field "REPL input" (hint "Rhai: Enter runs, Shift+Enter adds a line, Up /
Down recall") and a **Run** button, **Clear** empties the transcript. Above them the transcript.

- **Enter** (no Shift) runs the text; **Shift+Enter** inserts a newline at the cursor. An empty or
  blank entry does nothing.
- **History**: every run text is kept (at most 200, a repeat of the last entry is not added). **Up**
  recalls the previous entry, **Down** the next, and past the newest the text that was being typed
  comes back. They apply while the text has no newline (in a multi-line text the keys move the
  cursor as usual); recalling replaces the field's text.
- **Running** is `forge_script::run_script` on the editor's own context with the default limits
  (SCRIPTING.md §5), i.e. the same sandbox and the same undo rule as the Script Manager (§10): the
  commands go through the registry, **one entry is one undo step** (`Run Script`, so the History
  panel shows one row per entry) and a failing entry is rolled back and changes nothing. A run
  that changes nothing leaves no step. Every entry starts a fresh scope: variables do not survive
  to the next entry (the document does; `scene()` reads it).
- **Transcript** (`ConsoleState::repl`, at most 1000 lines): the entry echoed as `> first line`
  (further lines `. line`), then its `print` lines, then `=> <value>` for a last expression that is
  not `()`/null (JSON text, strings quoted), or for a failure the message in the error colour
  (`line N: ...`, with the `print` lines before it). Entries with a successful run that made
  commands add no extra summary line.

### 19.3 Tests

- Unit (`console.rs`): `parse_level` on real tracing lines and on junk; `filter_log` for each level,
  search (case, empty), continuation lines, indices; `log_text`; the REPL history (up, down, restore
  of the typed text, duplicates, cap).
- `crates/ui/tests/console.rs` (egui_kittest, no GPU): the panel is closed by default and opens from
  the Window menu; fed lines show, the Level drop-down hides lower levels, the search field filters,
  Copy fills `last_copy` with exactly the shown lines; the REPL runs `create("test.box")` through
  the Run button and through Enter (the node exists, the transcript has the echo and the result), a
  multi-line entry typed with Shift+Enter, `print` output, a value result, one entry with two
  commands is one undo step and undo removes both, a failing entry shows its error and changes
  nothing, Up / Down recall earlier entries, and Clear empties the transcript.

## 20. Coordinates manager (`coordinates`, `forge_app::coords`, `app`; brief 151)

Sources: own design from docs/roadmap/areas/L-ui.md and MANIPULATE.md §4; GUI session G1
(docs/parity/GUI-SESSIONS.md) is unanswered, so **the layout, the mode names and the Apply
behaviour are parity-pending**. Component rows are EDITABLE.md §6.7 (brief 034).

### 20.1 Modes
- **Space** `Local | World` (`Space`): the object's own parameters, or the world matrix.
- **Third row** `Scale | Size` (`Third`): scale factors, or mesh bounding box times scale
  (a node without a mesh shows "no mesh" for Size).
- **Apply**: `Immediate` (default; every field change is applied at once, a drag of a field is
  one undo step) or `Manual` (check box "Manual"): field changes only fill a draft, shown in
  the fields in place of the object's values; the **Apply** button writes the draft in one
  command and one undo step, **Reset** drops the draft. In Immediate mode Apply and Reset are
  disabled. The draft is dropped when the selection, the space or the third row changes.
- Rotation is shown as H / P / B in degrees (MANIPULATE.md §1).

### 20.2 Several objects (brief 150 multi-edit rules)
The panel shows the active object. An axis is `mixed` when its value in the chosen space
differs (more than 1e-9 relative) between the selected objects: the field then reads "mixed".
Typing into a field sets that axis on every selected object and keeps each object's other
axes. Rows are the same for all objects; nodes without a transform (tags) are ignored.
`coord_view_multi(scene, ids, space, third) -> Option<(CoordView, Mixed)>` is pure.

### 20.3 Commands
Object edits run `transform.edit` (COMMANDS.md §6): one command, one undo step, for every
selected object and every row of an Apply. It is the animation-aware sibling of
`transform.set` (as `param.edit` is of `param.set`): autokey and ANIM.md §2.4 apply, a value
equal to the shown one is not written. Plain edits of one parameter, if a row ever needs
them, use `param.edit`. In a component mode (a polygon object with an affected point set) the
panel shows **Pivot, Rotation, Size** of the components instead of the object rows (the
Scale|Size switch is hidden; Local|World and Manual/Apply stay) and runs `mesh.transform`
(`gesture`, EDITABLE.md §6.7): Pivot becomes `move`, Rotation `rotate` (HPB turned into an
axis and angle), Size `scale`; a draft with several rows is one `mesh.transform`, applied as
`T(move) T(pivot) R S T(-pivot)`, so the result has the typed size and the typed pivot.

### 20.4 Tests
`crates/app/tests/transform_edit.rs` (command: local/world, axes with null, multi-object,
size, one step, autokey, errors, no change on error); `crates/ui/tests/coordinates_manager.rs`
(pure mixed detection, each mode through egui_kittest: world move of two objects is one
undo step, scale/size switch, manual draft and Apply, component rows via `mesh.transform`).

## 21. Workspaces and saved layouts (`workspaces`, `layout`, `app`; brief 156)

Sources: own design from docs/roadmap/areas/L-ui.md; the per-OS directories are PLATFORM.md §8.
GUI session G1 (docs/parity/GUI-SESSIONS.md) is unanswered, so **the workspace names' contents,
the panel placement and the menu bar switcher are parity-pending**. View state only: nothing
here touches the document, the selection or the undo history.

### 21.1 Layout data and versions
`Layout { version, dock, sizes, workspace }` (§11). `workspace` is the name of the workspace the
arrangement started from (`#[serde(default)]`, empty for none). `LAYOUT_VERSION` stays `1`: a
field added with a serde default keeps the version; the version is raised only for an
incompatible change. `Layout::load_text(text) -> Loaded { layout, warning }` never fails: a
file that is not JSON, has a missing or wrong field type, lists a panel twice, has an active tab
out of range, or has a version other than `LAYOUT_VERSION` ("older" below it, "newer" above)
gives `Layout::standard()` and a one-line warning that names the reason. `from_json` is the
strict parser (an error for each of those cases) and stays as it was.

### 21.2 Workspaces (`workspaces::Workspace`)
Six built-in arrangements, each `Workspace::layout() -> Layout` with `workspace` set to its
name. Names (also the `ui.workspace` argument, matched case-insensitively): `Standard`,
`Modeling`, `Animate`, `UV`, `Rigging`, `Node`. Every panel (Panel::ALL) is open in no more
than one area; the Viewport is open in all of them (the UV workspace has no UV editor panel yet,
brief 129 adds it), and panels not listed are closed (reopen from the Window menu).

| Workspace | Top | Left | Centre | Right top | Right bottom | Bottom | Sizes (left, right, fraction, bottom) |
|---|---|---|---|---|---|---|---|
| Standard | Command bar | Tools | Viewport | Objects | Attributes | Timeline, Coordinates | 150, 340, 0.45, 120 |
| Modeling | Command bar | Tools | Viewport | Objects | Attributes, Coordinates | History, Console | 150, 360, 0.35, 150 |
| Animate | Command bar | Tools | Viewport | Objects | Attributes, Coordinates | Timeline, History | 150, 340, 0.40, 240 |
| UV | Command bar | Tools | Viewport | - | Attributes, Coordinates, Objects | History | 150, 400, 0.45, 120 |
| Rigging | Command bar | Tools | Viewport | Objects | Attributes | Timeline, Coordinates, Script | 150, 380, 0.60, 160 |
| Node | Command bar | - | Node Editor, Viewport | Objects | Attributes | Console, History, Script | 150, 340, 0.45, 200 |

`Standard` equals `Layout::standard()`. Each tab list starts with the shown tab. Switching
applies the dock and the sizes (egui forgets its remembered panel sizes: a new size generation
re-seeds them).

### 21.3 Layout files (`workspaces::LayoutStore`)
User layouts live in `<config>/layouts/<name>.json` (PLATFORM.md §8), the layout as
`Layout::to_json`. A name is 1 to 48 characters of letters, digits, space, `-` and `_`, starting
with a letter or digit, without leading or trailing space, and is not a workspace name
(case-insensitive); otherwise the request fails with the reason. Saving replaces a file of the
same name (written to a temporary file and renamed). `list()` is the names, sorted, of valid
`*.json` files (hidden `_`-files and other extensions are ignored). `load(name)` returns a
`Loaded`: a missing file is an error ("no layout named X"), a broken or old file the standard
layout and a warning. `delete(name)` removes the file (missing: error). Without a config
directory (no home folder) save fails with "no layout folder", `list` is empty.
`_last.json` in the same folder remembers the arrangement at exit; the studio restores it at
start (`Editor::restore_last_layout`), a broken file falls back to the standard layout and
puts the warning into the status bar. Tests use a store over a temporary folder
(`Editor::set_layout_store`); the default store of `Editor::new` is empty so tests never touch
the user's folder.

### 21.4 Commands (COMMANDS.md §6; window actions like `ui.command_palette`)
`ui.workspace {name}`, `ui.layout_save {name}`, `ui.layout_load {name}`,
`ui.layout_delete {name}`. Not undoable, no document change. They ask the window
(`UiRequest::Workspace`, `LayoutSave`, `LayoutLoad`, `LayoutDelete`); headless contexts and the
RPC server refuse with `needs the editor window`. Errors from the window (bad name, no such
layout, no folder, I/O) and warnings (a broken file replaced by the standard layout) go to the
status bar. `ui.layout_save` stores the current arrangement; `ui.layout_load` applies a saved
one and takes its `workspace` as the shown name.

### 21.5 Menu bar
A "Workspace" menu in the menu bar (right of Window) lists the six workspaces (the current one
checked), then the saved layouts, then "Save layout as..." (a name field and a Save button in a
small window), "Delete layout" (a submenu of saved layouts) and "Reset layout" (the standard
workspace). Each entry runs the command above, so it is also in the command palette. The
Window menu's "Reset layout" is unchanged.

### 21.6 Tests
Pure in `workspaces.rs` (names, every workspace valid and containing the Viewport, store round
trip in a temporary folder, bad names, listing, delete, broken/old/newer/duplicated files fall
back with a warning); `crates/ui/tests/workspaces.rs` (egui_kittest: each workspace switch via
command and menu, the document and undo history untouched, save/load/delete through commands,
a broken file on disk loads the standard layout with a warning in the status bar, the last
layout restore); `crates/command/tests/registry.rs` (headless refusal).


## 22. Layer manager (`layer_manager`, `tree`, `object_manager`; brief 158)

Sources: own design from docs/roadmap/areas/L-ui.md; GUI session G1 is unanswered, so **the
column set and order, the toggle letters, the colour display and the menu placement are
parity-pending**. What the toggles do is CORE.md section 7. All edits are registry commands
(COMMANDS.md §6, `layer.*`); the panel only computes arguments, one gesture is one undo step.

### 22.1 Panel

`Panel::Layers`, title "Layers", default area right bottom, closed by default; the Window menu
lists it as "Layers panel". Rows are the layers in table order. Columns: colour swatch (click
opens a colour picker, the result runs `layer.set` `color`), name (double-click renames via
`layer.rename`; Enter commits, Escape cancels), then nine toggle cells in this order: Solo `S`,
View `V`, Render `R`, Manager `M`, Locked `L`, Animation `A`, Generators `G`, Deformers `D`,
Expressions `E`. A cell is a small button that is filled when the flag is on; click runs
`layer.set` for the row (for every selected row when the row is selected). Accessibility labels:
`Solo: <layer>`, `View: <layer>`, `Render: <layer>`, `Manager: <layer>`, `Locked: <layer>`,
`Animation: <layer>`, `Generators: <layer>`, `Deformers: <layer>`, `Expressions: <layer>`; the
name cell is `Layer <name>`. Clicking a name selects the row (Ctrl toggles); panel state only.
Buttons under the grid: "New layer" (`layer.create`), "Delete" (`layer.delete` of the selected
rows), "Merge" (`layer.merge` of the selected rows into the first selected one, two or more),
"Assign selection" (`layer.assign` of the selected objects to the selected row, exactly one
row), "Remove from layer" (`layer.assign` with none). Disabled buttons when the precondition
fails. With no layers the panel says "No layers".

### 22.2 Object manager

A row whose object is on a layer shows a small swatch in the layer colour before the name
(accessibility label `Layer: <layer name>`); `TreeRow` carries `layer: Option<(LayerId, String,
[f64; 3])>`. Rows of layers with `manager` off are omitted (CORE.md 7.4), also in search results.
The row context menu gets a "Layer" sub menu (pure, `tree::layer_menu`): "No layer" and one entry
per layer, each running `layer.assign` for the menu's selection; the layer the whole selection
shares is checked.

### 22.3 Tests

`crates/ui/tests/layer_manager.rs`: pure (`tree::layer_menu`, row colours, manager filtering);
egui_kittest: the Window menu opens the panel (closed by default), New/rename/toggle cells/
delete/merge/assign with undo after each, toggling View hides the object in the viewport draw
items and the object manager colour swatch, the context menu assignment, save/load.

## 23. Viewport HUD and overlays (`hud`, `app`, `layout`; brief 157)

Sources: own design from docs/roadmap/areas/L-ui.md; GUI session G8 (docs/parity/GUI-SESSIONS.md)
is unanswered, so **which items exist, their wording, their places, the safe frame percentages and
the defaults (everything off) are parity-pending**. View state only: nothing here touches the
document, the selection or the undo history.

### 23.1 Items and settings

Six items per pane, each a flag in `HudFlags` (all `false` by default so a fresh viewport looks as
before): `fps` (frame time of the editor), `counts` (objects, points, polygons of the scene and of
the selection), `tool` (active tool and snap state), `frame` (current frame/time), `camera` (name
and projection), `safe_frames`. `HudSettings { perspective, top, front, right }` holds one
`HudFlags` per pane (brief 143 panes), `HudSettings::flags(pane)`, `set(pane, item, on)`.
They live in `EditorState.hud`. Command `view.hud {item, pane?, all_panes?, enabled?}`
(COMMANDS.md §6). The View menu has a "HUD" sub menu with one sub menu per pane, each with a
check box per item (labels "Frame rate", "Counts", "Tool and snap", "Frame", "Camera", "Safe
frames"), running `view.hud`.

### 23.2 Persistence

`Layout` (§11, §21) gets `hud: Option<HudSettings>` with `#[serde(default)]`; `LAYOUT_VERSION`
stays 1. `Editor::layout()` writes `Some(current settings)` when any item is on (otherwise
`None`, so a session without a HUD still equals `Layout::standard()`) and `set_layout` applies a
`Some` (saved layouts and the layout remembered at exit carry the HUD); built-in workspaces and
layouts without a block have `None`, so switching workspace keeps the user's HUD. A settings block with an unknown field is read
leniently by serde (unknown fields are ignored, missing ones are `false`).

### 23.3 Draw list (pure, `hud::draw_list`)

`draw_list(&HudContext, &HudFlags, size: [f32; 2]) -> Vec<HudPrim>` in pane-local points
(origin top left, `size` = the pane). `HudPrim` is `Text { pos, anchor, text, tone }` or
`Rect { min, max, tone }` (a 1 point outline); `tone` is `Normal`, `Weak` or `Guide`. Placement
(left column starts at y = 26, below the pane label, line height 14, x = 8):

- left column, top to bottom: camera, tool and snap, then the three count lines;
- top right (x = width - 8, y = 6, right aligned): frame rate;
- bottom right (x = width - 8, y = height - 6, right aligned bottom): frame;
- safe frames: outlines only, no text.

Texts (`format_count` groups thousands with a comma):

| item | text |
|---|---|
| fps | `"{fps:.1} fps  {ms:.1} ms"`, or `"-- fps"` before the first measurement |
| counts | `"Objects  {n}  (selected {k})"`, `"Points  {n}  (selected {k})"`, `"Polygons  {n}  (selected {k})"` |
| tool | `"Tool  {name}"`, `"Snap  off"` or `"Snap  on  {3D\|2D}"`; with axis mode `"  Axis mode"` is appended to the tool line |
| frame | `"Frame  {format_time(frame, fps, display)}  /  {format_time(end, ...)}"` using the scene's time display (ANIM.md §2.9) |
| camera | `"Camera  {pane} ({Perspective, fov 45°\|Orthographic})"` |

Counts: the drawn objects of the scheduled frame (CORE.md §5b); points and polygons are those of
their evaluated meshes, a cloner drawn by instancing counts prototype geometry times visible
clones; "selected" sums the drawn objects that are selected. `HudContext` carries the numbers; it
is built by `hud::context(state, pane, frame, timing)` from the editor state.

### 23.4 Safe frames (pure, `hud::safe_frames`)

`safe_frames(size: [f32; 2], render: Option<[u32; 2]>) -> SafeFrames { frame, action, title }`,
each `[min_x, min_y, max_x, max_y]` in pane-local points. The frame is the render aspect fitted
inside the pane (centred, as large as fits, a 0 pixel size counts as absent); with no render size
(the scene has no render settings yet, so today always) it is the whole pane. Action safe is the
frame scaled to 90 % about its centre, title safe to 80 % (own values, parity-pending). The draw
list holds the three outlines (`Weak` frame when the render size is present, `Guide` action and
title), the frame outline is omitted when it equals the pane.

### 23.5 Frame rate

`hud::FrameTimer` keeps an exponential moving average of the frame time (`push(dt)`, factor 0.1,
the first sample taken as is, non-finite or non-positive samples ignored); `fps() = 1 / avg`,
`ms() = avg * 1000`. The editor pushes egui's `unstable_dt` once per frame, not per pane.

### 23.6 Painting

The editor paints the draw list over the viewport image of each visible pane after the existing
pane label (`Editor::paint_hud`), and also over the "Viewport unavailable" placeholder so
headless tests see it. The primitives of the last frame are kept in `EditorState.hud_last`
(`Vec<(Pane, Vec<HudPrim>)>`) for tests.

### 23.7 Tests

`crates/ui/tests/hud.rs`: pure (counts format, safe frame geometry for each aspect, draw list text
for each item, placement, ordering, settings round trip and leniency, `FrameTimer`, layout
persistence with `None`/`Some`, workspace switch keeps the HUD), a rasterised **baseline image**
of the draw list (`tests/baselines/hud_*.png`, own images; `FORGE_UPDATE_BASELINES=1` rewrites
them; text is drawn as 6 x 10 point boxes per glyph so the image does not depend on fonts), the
command (toggle, panes, all panes, errors), and egui_kittest (View > HUD menu check boxes, items
appear in `hud_last` only for enabled panes, four-view layout, layout save/load round trip).

## 24. HiDPI, IME, themes, keyboard navigation (`theme`, `prefs`, `app`; brief 161)

Sources: own design from docs/roadmap/areas/L-ui.md ("HiDPI, IME, keyboard-only navigation,
high-contrast theme"); egui's public input model (`Event::Ime`, `Memory::focused`). No reference
observation; the palettes are our own.

### 24.1 Theme tokens (one module, `theme`)
`ThemeKind { Dark, Light, HighContrast }`, names `dark`, `light`, `high-contrast`
(`name()`, `from_name()`, `ThemeKind::ALL`). `Theme::of(kind)` returns the token set; the UI
takes its colours from it. Tokens: `window`, `panel`, `faint`, `text`, `weak_text`, `accent`,
`accent_text`, `error`, `component_selection` and `focus` (keyboard focus ring and IME
composition underline). `Theme::default()` is `of(Dark)`, unchanged (§8).

| token | dark | light | high contrast |
|---|---|---|---|
| window | `#1E2024` | `#F4F5F7` | `#000000` |
| panel | `#25282D` | `#ECEEF1` | `#000000` |
| faint | `#2E3238` | `#DDE0E5` | `#1A1A1A` |
| text | `#D6D9DE` | `#1B1E22` | `#FFFFFF` |
| weak text | `#9AA0A8` | `#4A515A` | `#D8D8D8` |
| accent | `#E08A2E` | `#A64B00` | `#FFD400` |
| accent text | `#1A1A1A` | `#FFFFFF` | `#000000` |
| error | `#E5534B` | `#B3261E` | `#FF7B72` |
| component selection | `#38C6E8` | `#006C99` | `#00E5FF` |
| focus | `#5AA9FF` | `#0B57D0` | `#5CE1FF` |

Contrast (WCAG ratio, tested for every theme): text on panel and window >= 7 (high contrast
>= 15), weak text on panel >= 4.5, accent text on accent >= 4.5, focus on panel >= 3, error on
panel >= 3 (dark) or >= 4.5 (light, high contrast).
`Theme::visuals()` builds from egui's dark visuals (light visuals for Light) and sets the focus
style: the active/focused widget outline is a 2 point stroke in `focus`, and the IME
composition underlines use `focus`. `Theme::apply(ctx)` installs visuals and fonts and stores
the theme in the context; `Theme::current(&Context)` reads it back (dark before the first
apply). `Theme::apply_visuals` does the same without reinstalling fonts (used when switching).

### 24.2 Preferences (`prefs::UiPrefs`)
`UiPrefs { theme: ThemeKind, scale: f32 }`, default dark and 1.0. `scale` is the UI scale
factor, clamped to `0.5..=3.0` (`MIN_SCALE`, `MAX_SCALE`), multiplied with the OS scale
(`pixels_per_point = native * scale`, egui `set_zoom_factor`). Persistence: the `ui` key of the
settings file (`{"ui": {"theme": "light", "scale": 1.5}}`), other keys kept, written on every
change; `use_settings_file(path)` loads; an unknown theme name or a non-finite or out-of-range
scale falls back per field to the default (lenient, like the key map). `EditorState.prefs`
holds it.

### 24.3 Commands (not undoable, window actions like `ui.workspace`)
`ui.theme {name}` (`dark`, `light`, `high-contrast`) and `ui.scale {factor}`; a factor outside
`0.5..=3.0` or non-finite is `InvalidArgs`. They ask the window (`UiRequest::Theme`,
`UiRequest::Scale`); headless contexts and RPC refuse with `needs the editor window`. An unknown
theme name goes to the status bar listing the valid ones. The window applies the preferences at
the start of each frame when they differ from what is applied (`Editor::apply_prefs`). View
menu: a "Theme" submenu (one checked item per theme) and a "UI scale" submenu (50 %, 75 %,
100 %, 125 %, 150 %, 200 %, 300 %); both run the commands, so they are in the command palette.

### 24.4 HiDPI
All layout is in points; nothing assumes `pixels_per_point == 1`. The viewport image is sized in
physical pixels (`rect * pixels_per_point`, as before). Tested at pixels-per-point 1, 1.5 and 2
and at UI scale 1.5: the editor lays out, the central viewport has a positive area inside the
screen, and the screen size in points is `physical / (native * scale)`.

### 24.5 IME
Text fields are egui `TextEdit`s, which take `Event::Ime` (preedit, commit). Requirement: the
command palette search accepts composed text. Tested by feeding `Ime::Preedit` then
`Ime::Commit`: egui shows the preedit in the field while composing, the commit replaces it with exactly the
committed text, and non-ASCII (CJK) text is kept intact.
The default fonts have no CJK glyphs, so `install_fonts` adds the first system CJK font found
(`theme::system_cjk_font`, a list of usual Linux, macOS and Windows paths; read at start-up,
never bundled) as the last fallback of both families; without one, egui draws tofu boxes.

### 24.6 Keyboard navigation
Tab and Shift+Tab move keyboard focus through the widgets (egui's focus order); the default key
map binds neither (tested), so the shortcuts layer never swallows them. The focused widget is
drawn with the focus ring (§24.1). Test: with the editor shown, Tab moves focus from none to a
widget and then to a different one, Shift+Tab goes back.

### 24.7 Tests
`crates/ui/tests/accessibility.rs`: token tables and contrast for the three themes, names round
trip, `Theme::of(Dark) == Theme::default()`, visuals (focus stroke, light base), `UiPrefs`
clamp, settings round trip and leniency, commands (refused headless, argument validation),
window (switching theme changes `Theme::current` and the saved file, scale sets the zoom
factor), layout at pixels-per-point 1, 1.5 and 2, IME in the palette search, Tab focus order.

## 25. Icon set and icon atlas (`icons`; brief 148)

Sources: own design from docs/roadmap/areas/L-ui.md. GUI session G1 (docs/parity/GUI-SESSIONS.md) is
unanswered, so **which glyph stands for which command is parity-pending** (only layout was ever to
come from G1; icons are never traced or imitated). Every glyph is a simple geometric drawing made
for this project; no file comes from any other application.

### 25.1 Files
`crates/ui/icons/<glyph>.svg`, one per glyph, compiled into the binary (`include_str!`). All
share one form: `viewBox 0 0 24 24`, white (`#fff`) strokes of width 1.5 on a transparent
background, no text, no raster images, no external references. White means the atlas is a
tint mask: the UI multiplies it by the wanted colour (so themes need no second atlas). The
glyph name is the file stem (lowercase letters, digits, `_`).

### 25.2 Mapping (`icon_name`)
`icon_name(id) -> Option<&'static str>` maps a node-type id (`forge.cube`) or a command id
(`mesh.bevel`) to a glyph name. Several ids may share a glyph (`marker.add` and
`marker.rename` use `marker`/`rename`; the two gizmo commands share `gizmo`). The table is
`ICON_MAP`, sorted by id, with no duplicate ids.
- Every node type of `forge_app::default_registry()` (62 at the time of writing) has its own
  glyph. Every command of `forge_app::default_command_registry()` (141) maps to a glyph.
- Fallback: an id not in the table (a plugin type, a later command) gets `IconRef::Letter(c)`,
  where `c` is the first alphanumeric character of its label, upper-cased (`'?'` if none).
  The UI draws the letter with the normal font inside a rounded square. `icon_ref(id, label)`
  returns `IconRef::Glyph(name)` or the letter.
- Every glyph file is used by at least one id, and every mapped name has a file (tested), so
  the set cannot rot.

### 25.3 Atlas (`IconAtlas`)
`IconAtlas::build(pixels_per_point)` renders every glyph with `resvg` (no text or system-font
features, so it needs no fonts or GPU and runs headless). Scale `s` is clamped to `0.5..=4.0`.
- Cell size `cell = ceil(24 * s) + 2 * PAD` pixels (`PAD = 1`, transparent, so bilinear
  sampling never bleeds into a neighbour); the glyph is drawn into the inner `ceil(24 * s)`
  square, scaled by `ceil(24 * s) / 24`.
- Layout: glyphs in name order, row-major, 16 per row; the image is `16 * cell` wide and
  `rows * cell` high. `rect(name)` is the inner pixel rectangle `[x0, y0, x1, y1]`;
  `uv(name)` the same in 0..1 texture coordinates.
- Pixels are premultiplied RGBA8 (`pixels`), directly usable as
  `egui::ColorImage::from_rgba_premultiplied` (`to_color_image`). `IconAtlas::texture(ctx)`
  uploads it with linear filtering.
- Deterministic: the same scale gives byte-identical pixels.
- A glyph that fails to parse is a build error caught by the tests, never a runtime panic:
  `build` skips it (and logs a warning), so it then falls back to its letter.

### 25.4 Tests (`crates/ui/tests/icons.rs`)
Every node type and command resolves to a glyph that exists; the map is sorted and unique;
no unused file; every file has the shared form of §25.1 and; the atlas has
the documented size, every glyph's rectangle contains inked pixels and the padding is empty;
scale 2 doubles the cell; two builds are identical; the letter fallback; unknown ids; the
atlas converts to a `ColorImage` of the same size.

## 26. Content browser and recent files (`browser`, `content_browser`; brief 154)

Sources: own design from docs/roadmap/areas/L-ui.md; no reference-app observation exists, so
**the panel layout, the tab names, the folder structure of the library and the number of recent
files are parity-pending**. The presets are written for this project (simple primitive
assemblies saved with the native format, CORE.md section 6); no preset, thumbnail or folder
layout comes from any other application. Every action is a registry command (COMMANDS.md §6,
`browser.*`), so it works headless; the panel only computes arguments.

### 26.1 Library (`forge_command::browser`)
A preset is a native scene file (`.forge`). Its root nodes are what loading adds to the open
document. Kinds: `objects` only. A materials kind waits for material nodes (area J); no
material node type is registered today, so there is nothing to browse or load for it.
- Built-in presets are our own files in `crates/command/presets/objects/`, compiled in
  (`include_str!`). Ids: `builtin:<slug>`. Today: `pedestal`, `table`, `snowman`.
- User presets are files `<library>/objects/<slug>.forge`. `<library>` is `<data dir>/presets`
  (PLATFORM.md section 2) in the editor and none in a headless run unless the host sets it
  (`Browser::use_files`); tests use a temporary folder. Ids: `user:<slug>`.
- `list` returns the built-in presets, then the user presets, each group sorted by name
  (case-insensitive, then id). The name is the title-cased slug (`-` becomes a space). A user
  file that is not a `.forge` file is ignored; an unreadable folder lists as empty.
- A slug is the name lower-cased with every run of characters other than ASCII letters and
  digits replaced by one `-`, trimmed of `-`; an empty slug is refused.

### 26.2 Loading
`browser.load` reads the preset (user files through `file::load_with` with the document's
registry, so older format versions migrate) and copies every root with its subtree, tags,
parameters and tracks under the given parent (none: the top level), after the existing
children, in one undo step "Load Preset". Parameters that link to nodes inside the preset
are remapped to the copies; links to nodes outside it and the layer link `object.layer` are
dropped. Names are kept. The new roots become the selection. A preset with no roots, an
unknown id or a node type the registry lacks is an error and the document is unchanged.
Load warnings go to the status bar.

### 26.3 Saving and deleting user presets
`browser.save_preset` copies the given nodes (default: the selection; nodes inside another
listed node are skipped) into a new scene and writes it to `<library>/objects/<slug>.forge`.
Existing files are only replaced with `overwrite: true`. Not undoable (it touches no
document). Without a library folder it fails. `browser.delete_preset` removes a user preset
file; built-in ids are refused.

### 26.4 Recent files
`Browser.recent` holds absolute paths, newest first, at most `MAX_RECENT_FILES` (10,
parity-pending), no duplicates. `file.open`, `file.save` and `file.save_as` record the path
after they succeed (`browser.recent` itself never records). With a settings file the list is
loaded from and stored under the key `recent.files` (other keys are kept; a missing or
malformed key is an empty list). `browser.recent` returns `{files: [{index, path, name,
exists}]}`. `browser.open_recent {index}` runs `file.open` on that entry; a file that no longer
exists is removed from the list and the command fails. `browser.clear_recent` empties it.

### 26.5 Panel and menu
`Panel::ContentBrowser`, title "Content Browser", default area right bottom, closed by default;
the Window menu lists "Content Browser panel". Two tabs (panel state only): "Presets" and
"Recent". Presets: a search box (case-insensitive substring of the name), one row per preset
(a button showing the name, accessibility label `Load <name>`, tooltip with the source) running `browser.load`, and a "Save
selection as preset" row (name box + button, `browser.save_preset`) shown only when a library
folder exists. Recent: one button per file (shows the file name, accessibility label `Recent <file name>`, tooltip = path; a missing file is struck through) running
`browser.open_recent`, and a "Clear recent" button; "No recent files" when empty. The File menu
gets an "Open Recent" sub menu with the same entries (disabled when empty). Opening a recent file
with unsaved changes asks like File > Open does.

### 26.6 Tests
`crates/command` unit tests: slugs, names, sorting, search. `crates/app/tests/browser.rs`: every
built-in preset loads into an empty document and into a nested parent (parameters, tags and
tracks preserved, link remap, one undo step, undo and redo), save/list/load round trip in a temp
library, overwrite and delete rules, recent list order, limit, de-duplication, persistence in a
temp settings file next to foreign keys, `open_recent` (missing file removed), commands refuse
bad arguments. `crates/ui/tests/content_browser.rs` (egui_kittest): panel opens from the Window
menu, closed by default, clicking a preset loads it with one undo step, recent entries open a
file, Open Recent submenu, search filter.

## 27. Import/export options window (`app/files`; brief 172)

Sources: IO.md §3 (own design). The reference app's dialog layout is not observed: **the window
title, the order of the controls and the button names are parity-pending**.

- The File menu items keep their labels (Import OBJ..., Export OBJ..., Export Selected as
  OBJ...). Choosing one opens the file chooser (IO.md §2); when a file is chosen, the options
  window opens instead of running at once. Cancelling the chooser opens nothing. "Nothing to
  export" is reported before the chooser, as before.
- The window is an egui `Window` titled `Import OBJ Options` / `Export OBJ Options` (the
  format label), anchored at the centre, not resizable, shown above the docks until closed. It
  lists the path and one row per option descriptor (IO.md §3.2) in descriptor order:
  bool -> check box (label = descriptor label), int/float -> `DragValue`, enum -> combo box,
  string -> text edit. Buttons: `Import` / `Export` (runs `file.import` / `file.export` with all
  option values) and `Cancel`. Escape cancels.
- `Editor` methods (the UI and the tests use the same ones): `begin_import(format)`,
  `begin_export(format, selected)` (pick the file, open the window), `io_dialog()` (the pending
  state: format, path, selected, descriptors, values), `io_dialog_set(id, Value)`,
  `io_dialog_confirm()` (runs the command, closes), `io_dialog_cancel()`.
  `import_obj()`/`export_obj(selected)` stay: immediate, default options (shortcuts, tests).
- The commands `file.import_dialog` / `file.export_dialog` (UiRequest) call `begin_*`.
- Only one options window exists; starting another replaces it.
- Tests (`crates/ui/tests/io_dialog.rs`): scripted chooser answers; the window appears with the
  descriptors' controls (kittest); toggling "Write normals" and confirming writes a file without
  `vn`/`//`; Flip Z on import mirrors z; Cancel and a cancelled chooser change nothing; the request
  commands open the window; headless run of `file.import_dialog` is refused.
