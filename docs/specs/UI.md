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
node selection, the mode or the target changes.

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
| 1 .. 5 | `view.shading` smooth, flat, wire, smooth_wire, flat_wire |
| 6 .. 9 | `view.shading` quick, constant, hidden_line, bounding_box (brief 143) |
| F4 | `view.layout` (toggle single / four; brief 143) |
| Mod+M | `view.maximize` (toggle the active pane; brief 143) |
| W / E / R | `view.gizmo_mode` move / rotate / scale |
| Q | `view.gizmo_space` (toggle) |
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
all `LinkList` parameters drawn by `widgets::link_list` (UI.md §12). One row per entry:

`[x] name   [strength]   ^  v  x`

- **Enable box** (`Enable <name>`): shown when the entry node has an `enabled` parameter
  (effectors, deformers) or else the object state `object.enabled` (fields, TRANSFORM.md §2).
  It edits that parameter **on the entry node**, not on the list's owner. A disabled entry
  keeps its place in the list and is skipped when the list is applied (effector: no
  effect, MOGRAPH.md E1; field: the layer is left out of the `FieldList`, FIELDS.md F2).
- **Strength** (`Strength <name>`, percent): shown when the entry node has the `layer_strength`
  parameter (every field node, FIELDS.md; brief 061 stored a row's layer strength there). It
  edits `layer_strength` of the entry node. Own design, parity-pending: the strength belongs
  to the node, so a field used by two lists has one strength in both (real per-row storage
  comes with brief 059).
- `^` / `v` (`Move <name> up/down`) reorder: the new list order is the order the effectors
  or layers apply in (effectors in list order, MOGRAPH.md E1; layers top to bottom).
- `x` (`Remove <name>`) removes the entry from the list only; the node stays in the scene.
- The "Add..." drop-down lists the objects the parameter accepts that are not in the list; it
  is also the **drop target** for an object dragged from the object manager (the dragged
  node is appended; a node already in the list is ignored).

Every change is one `param.edit` command: a list edit on the list's owner (all selected
objects of that type), an enable or strength edit on the entry node. `Edit` carries an
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
  brings it back; the same for a field row; the strength box edits `layer_strength` and the
  clone result follows.
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
