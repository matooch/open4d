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

`create_menu(registry) -> Vec<(String, Vec<(type_id, label)>)>`: placeable
node types (hierarchy + Tag) whose `in_create_menu` is true, grouped by
`NodeType::menu_path` and ordered by `NodeType::menu_order`, then label.
Groups follow the reference order: Object, Spline, Generator, Deformer, Field,
Volume, Camera, Light, Tag. The Clone toolset remains a separate submenu (see below).
Types with `in_create_menu == false` (the polygon object) are left out.

Key bindings are data in the key map (§14.2, observed in G2 item 1 where listed there): Ctrl+Z undo,
Ctrl+Y redo, Ctrl+A select all, Ctrl+Shift+A deselect all (Cmd on macOS),
Backspace or Delete delete selected. Rename has no key (§13.4); shading is on Ctrl+Shift+1 ..
(ours).

## 6. Viewport panel (`viewport_panel`)

Renders with forge-viewport's `Renderer` on the eframe wgpu device into an offscreen
`Rgba8Unorm` texture registered with egui as-is, so palette values display as
authored (matching the snapshot PNGs). Recreated on resize. Draws the grid and every
generator node (`forge_generators::build`, placed by `Scene::world_matrix`); selected
nodes use the selection colour. GPU meshes are rebuilt only when `Scene::revision()` or
the selection changes; nodes whose build is unsupported are skipped. Without a wgpu render state (tests) it shows a
"Viewport unavailable" label.

### 6.1 Navigation (brief 153; revised by brief 223 and brief 233, owner note N1, decision D22)

Camera maths is pure and lives in `forge_viewport::nav` (VIEWPORT.md §7). The panel only
turns pointer input into calls. The hold keys come from **owner note N1** (GUI-SESSIONS.md,
answers G2); the wheel and Alt gestures stay our own design, parity-pending G2.
`nav_action(input) -> NavAction` with
`NavInput { primary, middle, secondary, alt, held: Option<NavMode>, drag, scroll, cursor }`:

| Input | Action |
|---|---|
| wheel `s` (points) at pointer `c` | `ZoomAt(0.999^s, c)`: zoom toward the cursor, `s > 0` zooms in (scroll up zooms in, scroll down reverses it: observed, G2 item 2) |
| hold `3` + primary drag, or Alt + primary drag | `Orbit(dx, dy)` |
| hold `1` + primary drag, or middle drag (with or without Alt), or Alt + middle | `Pan(dx, dy)` |
| hold `2` + primary drag, or Alt + secondary drag | `Dolly(dx)` (zoom), about the view centre |
| anything else | `None` |

Wheel takes precedence over drags in the same frame. `NavMode` = `Pan | Dolly | Orbit` for the
held keys; the key table is data (`NavMode::KEYS`: `Num1` Pan, `Num2` Dolly, `Num3` Orbit).
Rules (owner note N1):

- The digits are **plain hold modifiers**, read from keyboard state, not chords: a digit
  counts as held only when no Mod/Alt is down, no text field has the keyboard, and the pointer
  is over the viewport pane (or a drag that began there is still running). A digit held while
  the pointer is over another panel does nothing.
- **A tap with no drag does nothing**: the digits are not in the key map, so pressing and
  releasing one changes nothing (no shading, no camera, no selection). Only primary-button
  motion while the digit is down moves the camera.
- While Alt or a hold key is down the viewport does no selection, rubber band or gizmo
  interaction (`NavInput::navigating`).
- The chords bare `1`, `2`, `3` are **reserved** (`keymap::is_reserved`): the key map editor
  refuses to bind them and a settings file entry on them is dropped with a warning (§14.3).
  `4` .. `9` are not navigation keys and stay bindable.
- The earlier own-design hold keys X / C / V are gone; X, C and V are free.

Navigation works even without a wgpu render state (it is evaluated before the "Viewport
unavailable" early return), so it is testable with egui_kittest.

The viewport stores its aspect ratio (`EditorState.view_aspect`, width / height, default 16/9)
each frame; the frame commands use it.

Key-map bindings (UI.md §14.2) for the commands of COMMANDS.md §6 (observed, G2 item 1): `H` =
`view.frame_all` (Frame Geometry), `S` and `O` = `view.frame_selected` (the reference has `S`
for the selected elements and `O` for the selected objects; ours has no component framing, so
both frame the selected objects), `F1` Perspective, `F2` Top, `F3` Right, `F4` Front =
`view.set_view` on the active pane (the reference shows that view; in the four-view layout ours
sets the active pane's view, **parity-pending G2**), `F5` = `view.layout` (all views, toggling
back to one: the toggle back is ours). `Mod+1` .. `Mod+7` = `view.set_view` perspective, front,
right, top, back, left, bottom stay as our extras. View > menu entries: Frame all, Frame
selected, and a "View" submenu with the seven views, each showing its chord. The wheel
direction above is checked by the kittest wheel test (`the wheel zooms toward the cursor`).

Tests (crates/ui/tests/navigation.rs): the input mapping of every gesture and hold key, `apply_nav`, the key table;
`view.frame_all` / `view.frame_selected` fit the bounds (aspect-aware), nothing selected frames
all, a selected null recentres, an empty scene resets; `view.set_view`, `view.set_camera`
(partial update, validation, pitch clamp) and `view.get_camera`; camera commands add no undo
step and Ctrl+Z leaves the camera alone; egui_kittest (no GPU): Alt + left/middle/right drags
orbit / pan / dolly, held `3` / `1` / `2` turn a plain drag into orbit / pan / dolly (and not
without the key), a digit held with the pointer outside the viewport does nothing, a digit tap
changes nothing, no selection while a digit is held, the wheel zooms toward the cursor, `H`,
`S`, `Mod+4`, `Mod+1` run their commands.

Undo: camera moves, frames and view switches are **not** undo steps (the camera is view state,
not document state; COMMANDS.md §6). Gestures and `view.*` commands never touch the history.

Component modes (EDITABLE.md §2): `EditorState.mode` is switched in **one place** (owner note N6,
decision D22): the four "Mode" buttons at the top of the left tool palette (§11: Objects, Pts,
Edg, Pol; the selected one is drawn selected) and the Alt+1 .. Alt+4 keys of the key map (§14.2).
The viewport header used to repeat the four buttons as "Model", "Points", "Edges", "Polygons";
that duplicate is removed (brief 225). Which one stays follows G1 item 8, still unanswered:
**parity-pending G1**, our choice is the palette, which the owner's note names as the place of
the tools. The header keeps the Soft / symmetry bar, shown while a component mode is on. With a component target, left
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
  VIEWPORT.md §9 in the viewport area, with a 2 px gap; the active pane has a state (mint) outline.
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

### 6.3 Display mode "Render" (brief 199)

`EditorState.display: DisplayMode::{Shaded, Render}` (default `Shaded`) and
`EditorState.post: PostSettings` are **UI state**: the viewport header's "Render" toggle button
and the Post menu beside it ("SSAO", "SSR", "Bloom", "TAA", "Tone map") set them directly, so they
push no undo entry and do not dirty the document. In Render mode a pane shows
the real-time tier's frame of its own camera (RENDER.md R22.7); the gizmo, HUD and selection
overlays are left out, and a missing adapter keeps the shaded image with a "Render unavailable"
note. Switching back to Shaded shows the shaded panes exactly as before.

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
  strip, draws a time-coloured band along the bottom and two handles (labels "Preview start handle" and
  "Preview end handle"; dragging runs `timeline.set_preview`, the handles cannot cross). Keys M
  and Alt+Comma / Alt+Period (ANIM.md §2.7).
- `Timeline { current: 0, playing: false }` is editor state; start, end and fps live in
  the scene (`SceneSettings`, default 0, 90, 30). `set_current` clamps to the range.
- The Attributes panel draws a marker left of every animatable row: filled diamond =
  key on this frame, hollow = animated without a key here (ANIM.md §2.5).

## 8. Theme (`theme`)

Three themes (own palettes, §24.1): dark (default), light, high contrast. The text below is
the dark one.

Our own dark palette (sRGB; role tokens, §24.1, owner note N11): window `#171B21`, panel `#20262E`,
viewport `#252C35`, input `#181D24`, divider `#343D49`, text `#E8EDF2`, weak text `#A7B2C0`;
**state** (mint) `#73D6BC` marks the active tool, mode, toggle, layout tab and the playhead;
**selection** (amber) `#F3B36B` marks what is selected everywhere; **record** (red) `#F17A80` marks
recording and errors; component selection `#38C6E8` (selected points, edges and polygons of the
component target, EDITABLE.md §2.6). There is no single `accent` any more. `apply(&egui::Context)`.
Requirement: text on panel and on window has WCAG contrast ≥ 7, weak text ≥ 4.5, text on a state
or selection fill ≥ 4.5.

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
`Sizes { left_width 150, right_width 402, right_top_fraction 0.47, bottom_height 92 }` are
initial sizes of the resizable areas (egui remembers later drags). Right width, split and
timeline height are the reference's measured numbers (observed, G10 item 1; §29.1, `layout::WINDOW`).
`left_width` stays 150 because our tool palette still holds labelled buttons; the reference's strip
is 43 px (`WINDOW.tool_strip`) and brief 227 sets it when the buttons become icons. The panels
grow to the widest content, so a tab or a number field that is wider than the managers column
pushes it out: the number field of §12 is sized to keep three of them in 402 px.

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
`select.mode` and `view.shading` buttons show as selected for the current value. The Mode group
is the only mode switch in the default layout (§6.1, owner note N6): no other panel, bar or menu
repeats it; a test counts the widgets per mode. A failed
command goes to the status bar like any other.

Snapping and axis mode exist as commands (`snap.*`, `edit.axis_mode`, MANIPULATE.md §6–7) and as Edit menu entries; they are not palette buttons yet.
Pointer-driven tools (cuts, bridge, pen) stay in the Mesh menu, as do the arrangement tools
(Array, Duplicate, Arrange, Randomize, Transfer; MODELING.md §9.7), which have a settings panel and no
viewport interaction.

## 12. Attribute manager (`attributes`, `widgets`; brief 150, revised by brief 233)

Sources: own design from docs/roadmap/areas/L-ui.md; GUI session G4 items 1 and 2 and G1 item 3
(docs/parity/GUI-SESSIONS.md, "Owner's Cowork session, 2026-11-27") observed the number field, the
key diamond and the stacking of two tabs. What stays **parity-pending G4**: the scrub modifiers
(the session did not test continuous scrubbing or fine / coarse modifiers), the widget look
beyond what is written below. Every row is derived from a `ParamDesc`; nothing is hand-laid per
type.

Tabs and groups: `UiHint.group` is `"Tab/Group"` (split at the first `/`) or just `"Group"`
(tab `Basic`; empty group is `Parameters`). `AttributeView.tabs()` lists tabs in
first-appearance order, `groups_of(tab)` the groups of one; a tab bar is drawn only with two
or more tabs, groups are collapsible. The shown tab is editor state (`attr_tab`); a tab
that no longer exists falls back to the first.

**Stacked tabs** (observed, G1 item 3: Shift-clicking the Light's Details with General active
showed both tab bodies in one vertical scroll area, General first, Details below). The shown
tabs are `attributes::TabSelection { active, stacked }` (editor state, not saved). A plain click
on a tab shows only that tab. A Shift-click on another tab adds its body below the shown ones;
the one that was active first stays on top and the others follow in click order, all in one
scroll area with a separator line between the bodies. The tab buttons of every shown tab are drawn
selected. A second Shift-click on a shown tab removes it (ours, **parity-pending G1**; removing the
top one promotes the next, and the last shown tab cannot be removed). Tabs that the new selection
does not have drop out; with none left the first tab shows.

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
Values are shown and edited in display units (degrees, percent, UI.md §4).

**Number field** (`widgets::number`, shared by the float, int and vector widgets; observed, G4
item 1: "a dark numeric box with left/right chevrons"; the right chevron took Size X from 200 cm to
201 cm and the left chevron back; double-clicking the value allowed direct text entry and Return
committed it): a dark value box with a left and a right chevron (12 px cells, a 56 px box; the
three of a vector must fit the managers column, §29). A click on a chevron steps the value by one
unit (`scrub::chevron_step`): 1 for whole numbers and for distances, angles and percentages in
their display unit, 0.1 for other floats (**parity-pending G4**: "one unit" of a unit-less float is
unobserved). The step stays inside the parameter's range and is one undo step per click. A
**double-click** on the value opens a text edit with the value selected for typing; **Return
commits** (a value that does not parse, or is unchanged, changes nothing), Escape cancels, losing
the focus commits (ours). A drag on the value scrubs by the base step per pixel with the
modifiers above (the single-click-types rule of the earlier DragValue is gone). An accessibility
tree exposes a spin button with its value and the chevrons as buttons `Increase <row>` /
`Decrease <row>` (vectors: `Increase X <row>`); assistive technology can also set, increase and
decrease the value.

Multi-edit: the rows are those of the active object whose id and kind exist in every other
selected object's type. A row is `mixed` when values differ (vec3: per component, `mixed_axes`).
A mixed number shows "mixed", a checkbox is indeterminate, a drop-down shows "(mixed)", a
string is empty with a hint, a colour has a "mixed" note, a list shows the active object's.
Other widgets show the active object's value. A set applies to all selected objects in one
undo step; a vec3 component edit keeps each object's other components.

Key indicators (097, toggle by brief 224, owner note N3; look observed by brief 233, G4 item 2):
every row whose descriptor supports key tracks has a 12 px diamond button left of the label. It
has **four states** (`key_indicator::KeyState`), told apart by fill and colour:

| State | Look (observed) | Ours (theme tokens) |
|---|---|---|
| unkeyed: no track | hollow grey diamond | outline `weak_text`, no fill |
| keyed: a key at this frame | solid red | solid `key` (the time colour, mint) |
| changed: an unkeyed draft (§12.1) | solid amber / orange | solid `modified` (violet) |
| animated: a track, no key at this frame | hollow red | outline `key`, no fill |

The reference showed the amber diamond on a keyed frame after an unrecorded edit and after
reselecting the object, so a draft wins over a key at this frame; scrubbing to another frame
discards it and shows the hollow `key` one. Colours are our own (D22; brief 234, owner note N11):
`Theme.key` is the time colour (mint) and `Theme.modified` a violet in every theme, **not amber**:
amber is selection, and a draft must not read as selected. Their hues differ by at least 30
degrees, `modified` also differs from `selection` by at least 30 degrees, and both keep 3:1 against
the panel (tested), so the two solid states are never mistaken; fill (solid or hollow) carries the
rest in greyscale. The accessible label is
`<row> unkeyed | keyed | changed | animated`. What a click does depends on the state
(`RowKeys::click`):

| State | Click |
|---|---|
| unkeyed (no track) | adds a key of the row's current value (`key.set_parameter`; a Vec3 keys all three components) |
| animated (a track, no key at this frame, so also a frame between keys) | adds a key here |
| changed (uncommitted edit; a key may or may not exist here) | adds or updates the key with the edited value |
| keyed (a key at this frame on any of the row's tracks, no uncommitted edit) | **removes** the keys of the row here through `key.delete`, one undo step; a Vec3 loses every component key at this frame together |

The state is computed from the scene every frame, so the button shows the new state in the frame
after the click, with no further input. Removing
the last key of a track removes the track (the `key.delete` rule, ANIM.md "key.delete {keys}"); the row then shows
unkeyed again. **Parity-pending G5:** the reference may keep an empty track; ours never has one
(`Track::validate` refuses an empty track). The context menu is unchanged: "Add / Update Key on
Parameter", "Delete Key on Parameter" (enabled when a key is here), and the previous / next
keyed frame entries.

The timeline's "Record" button (the key-all button of the selection, ANIM.md §2.3) toggles the
same way: when every recordable selected node already has a key on all 9 record tracks at the
current frame, the button is drawn selected and a click removes exactly those keys (`key.delete`,
one undo step, other parameters' keys are kept); otherwise it runs `key.record`. The `K` key and
the Animate menu's "Record Active Objects" stay add / update. "Delete Keys" is unchanged.
Vector and colour rows key their three component tracks together. This is an own
design based on [G4](../parity/GUI-SESSIONS.md#g4--attribute-manager-widgets-and-field-lists)
and is **parity-pending**.

### 12.1 Keyframe drafts (brief 232; owner note N10)

Source: owner note N10 ([GUI-SESSIONS.md](../parity/GUI-SESSIONS.md), owner notes table): on a
property that has a key track, with Auto-Key off, changing the value by any route (the gizmo,
the Coordinates panel, the inspector) shows the new value in the viewport and the inspector,
marks the key button as modified, and does not store it; clicking the key button stores it as a
key at the current frame. Everything else below is our own rule and **parity-pending G5**.
This replaces the earlier inspector-only draft.

**One draft layer** (`forge_app::drafts::DraftLayer`, held by `App`): `(node, param) -> value`.
It is the only mechanism: the inspector, the gizmo and the Coordinates panel all write into it.
A draft is the whole parameter value (a Vec3 is one draft, never a component).

- **When an edit is a draft:** Auto-Key is off and the node has at least one key track on the
  parameter (any component). Auto-Key on keys at once (ANIM.md §2.4). A parameter without a track
  edits the document as before. A multi-object edit decides per object: the animated ones get
  drafts, the others are written in one undo step (the drafts survive that step). A vector edit
  of a partly animated parameter drafts the whole vector; the commit keys all three components
  (as the key button already did).
- **Routes.** Gizmo (`apply_drag_edit`) and Coordinates (`edit_results` +
  `apply_transform_edits`, the same transforms `transform.edit` computes) stage Position,
  Rotation and Scale drafts. The inspector row (`param.edit`'s value rules, `draft_value`) stages
  the row. Scripted and agent edits (`param.edit`, `transform.edit`, `transform.set`) are not
  interactive routes and keep ANIM.md §2.4 rule 5 (they write the key).
- **Display.** `App::evaluated()` is the animation with the layer on top: the viewport
  (`scheduled_draw`), picking, snapping, the gizmo, the Attributes and Coordinates panels read
  it. A drag that starts on a drafted object starts from the drafted pose. The key button shows
  the *changed* state (warning fill, §12 indicators). No other "modified" hint is shown (the
  object manager and HUD stay as they are; parity-pending G5).
- **Behaviours and expressions** see the drafted value while it is shown (our choice,
  parity-pending G5): the layer replaces the parameter's tracks in the scene that is evaluated
  for display, so tags, dependants and deformers read the drafted value.
- **Never in the output.** `App::eval`, `App::scheduled`, `eval_mesh`, export, render, `save`
  and every command read the animation only; the layer is not saved, not undoable, not part of
  the document revision.
- **Committing.** The key button (and the "Add / Update Key" menu entry) of a changed row keys
  the draft of every selected object at the current frame through `key.set_parameter`, one undo
  step; the other rows' drafts stay. The timeline Record button (and `key.record`, K, the
  Animate menu) is the key-all: it keys the drafted value of Position, Rotation and Scale and
  also keys the selection's drafts of other parameters whole, in one step "Record". A selection
  with a draft never takes Record's toggle-off branch (uncommitted edit, as the button rule).
- **Discarding.** A draft is dropped when its scope changes: the current frame (observed, G4 item
  2: scrubbing to another frame re-evaluates and the unkeyed edit is gone), the document (any edit,
  undo, redo, load; our rule, parity-pending G5), Auto-Key, or Escape pressed over the viewport.
  A **selection change keeps** the drafts (observed, G4 item 2: selecting another object and
  reselecting kept the edited value and its *modified* indicator). Setting a draft back to the animated value removes it. Dropping is lazy and
  needs no hook: each read compares the scope.

Tests: `crates/ui/tests/keyframe_drafts.rs` (gizmo drag leaves the track unchanged, moves the
shown object and marks the button; the key click is one undo step and undo removes it; frame
change, undo and document edit discard, a selection change keeps it; dragging back removes the draft; unanimated
edits write the document; Auto-Key keys at once; the Coordinates panel and the inspector use the
same layer; a mixed selection; Record keys drafts; render, export and save ignore drafts; Escape).

Not in this brief: lock/unlock, history back/forward, the field-list widget.

## 13. Object manager (`tree`, `object_manager`; brief 149, revised by briefs 229 and 233)

Sources: own design from docs/roadmap/areas/L-ui.md, replaced where GUI session G1 items 2 and 7
(docs/parity/GUI-SESSIONS.md, "Owner's Cowork session, 2026-11-27") observed the reference: the
row order, the dot click cycle, what the evaluation cell shows, the row pitch, the indent and
the boxed disclosure. What stays **parity-pending G1**: the drop zones and the in-progress drop
markers (the session's drag completed before a capture), the context menu items, the lower dot's
rendered result, and the context list. Look and labels are ours (D22).
All edits are registry commands (COMMANDS.md §6); the panel only computes arguments, so an
agent gets the same behaviour. Each user gesture is one undo step.

### 13.1 Columns (left to right; the order is observed, G1 item 2)

1. Gutter: a boxed minus (expanded) or boxed plus (collapsed), only for rows with children, with
   thin connector lines to the children, indented 12 px per depth (§13.1a). A click folds.
2. Type icon: brief 227's icon goes here. Until it lands, the weak type label stands in for
   it, drawn after the name.
3. Name, renamed in place (§13.4). **Selected rows draw the name in the selection colour**
   (observed: amber; ours is the theme's `selection`, the same amber as the viewport outline, D22,
   owner note N11) over a faint selection fill, with a 2 px selection bar at the row's left edge so
   the row reads in greyscale.
4. Layer glyph (a small bar in the layer's colour; §22.2), only for objects on a layer.
5. Two **stacked visibility dots** in one column: the upper one is the editor visibility, the
   lower one the render visibility. Each is its own 14 x 10 px click target.
6. The type-dependent evaluation cell (`tree::eval_cell`): a **check** (`EvalCell::Check`) on
   most types, an **active-camera target** on cameras (`EvalCell::ActiveCamera`), **nothing**
   on nulls (`EvalCell::None`), as observed for the Light (check), the camera and the null. Which
   other types lack the check is **parity-pending G1**: ours is every type but the null.
7. Tags: one weak `[label]` per tag on the host (observed: small tag glyphs further right).

Columns 4 to 7 sit right aligned in that order, the name column takes the rest.

Dot state (`object.editor_vis`, `object.render_vis`, `Vis`): `Default` (grey: follows the
parent, root = on), `On` (green, forced on even under an Off parent), `Off` (red). **A click
cycles Default -> On -> Off -> Default** (observed, G1 item 2: two clicks on the Cube's upper
dot first kept it visible, then hid it, and a third restored the default), and runs
`object.visibility` for the clicked row (or for the whole selection when the row is selected).
The evaluation check is `object.enabled`; a click runs `object.enable` with the inverted value
(observed for the Light: the check became a red X and the object stopped evaluating; ours draws
an empty box when off). The camera's target runs `view.look_through_camera` for the row, our
stand-in for the reference's active camera (**parity-pending G1**; the target does not show which
camera is active). A node is shown in the viewport iff it and every ancestor are enabled and its
resolved editor visibility (`forge_core::visibility`) is on. Render visibility is stored and
resolved the same way (`render_visible`) but honouring it in the render bridge is a follow-up
(the renderer does not read it yet).
Accessibility labels (used by the tests): `Editor visibility: <name>`, `Render visibility:
<name>`, `Enabled: <name>`, `Active camera: <name>`.

### 13.1a Row geometry and hierarchy cues (brief 229; numbers observed by brief 233, G1 item 7)

Owner note N7 asks that a child is clearly shown as a child and that rows are separated more.
G1 item 7 measured the reference: **row pitch 20 px** (row centres at y 112, 132, 152, 172),
**child indent about 12 px** (between the left edges of a parent's and a child's icon), and a
**small boxed minus** in the gutter of an expanded parent with a **thin right-angle connector**
to its child; a collapsed branch shows a boxed plus. The numbers below follow; they live in one
place, `tree::METRICS` and the pure functions `tree::row_layout`, `tree::disclosure_box`,
`tree::disclosure_glyph`, `tree::connectors`, `tree::connector_segments`, `tree::drop_mark` and
`tree::row_colours`. They replace brief 229's 22 px rows, 18 px indent and painted triangles.
The separator, the alternating shade, the colours and the drop marks of brief 229 are ours and
stay (**parity-pending G1**).

| Quantity | Value |
|---|---|
| Row height | 20 px (observed) |
| Gap between rows | 0 px: rows touch, so every y in the list belongs to a row (drop zones, §13.3) |
| Indent per depth level | 12 px (observed, "about"); the row of depth `d` starts at `x = 12 d` |
| Gutter cell | 12 px wide, at the start of the row's own column, `x = 12 d .. 12 d + 12`, full row height as the click target |
| Disclosure box | a 10 x 10 px square outline centred in the gutter cell and in the row; a 6 px horizontal bar inside it when expanded (minus), plus a 6 px vertical bar when collapsed (plus); only on rows with children |
| Connector lines | 1 px, colour `guide` |
| Separator | 1 px line along the bottom edge of every row, colour `separator` |
| Alternating shade | rows with an odd index in the visible list are filled with `alt`; a selected row is filled with `selected` (the selection colour at 16 % into the panel colour) |

Connectors (`connectors(rows)` gives, per row, `trunks: Vec<bool>` of length `depth`, from the
visible depth sequence alone): an ancestor at depth `k` has a **trunk**, a vertical line at
`x = 12 k + 6` (the centre of that ancestor's gutter cell and box), which runs down through
the rows of its subtree. Row `i` of depth `d > 0`: for `k < d - 1`, `trunks[k]` is true when a
later row of depth `k + 1` follows before the list returns to a depth `<= k`, and a full-height
vertical is drawn at the trunk of `k`; the **elbow** is at `k = d - 1`: a vertical from the row's
top to its vertical centre, continuing to the bottom when `trunks[d - 1]` is true (a later
sibling exists), then a horizontal at the vertical centre from the trunk to the left edge of the
row's disclosure box (`x = 12 d + 1`) for a row with children, or to `x = 12 d + 8` for a leaf.
A row with children that is expanded also draws a **stub** on its own trunk from the bottom of
its box to the row's bottom edge, so the line continues into its children. Root rows have no
elbow. Rows of the search result (§13.5) use the same rules over their own depth sequence; a
layer with the manager off (§22) does not appear, its children move up one level, the rules read
depths only.

Colours come from the theme tokens (`row_colours(theme)`), so all three themes follow
(§24.1): `guide` is `weak_text` mixed 75 % into `panel`, `separator` is `weak_text` mixed 18 % into
`panel`, `alt` is `faint` mixed 55 % into `panel`, `selected` is `selection` mixed 16 % into `panel`.
The guide keeps at least 3:1 contrast against both `panel` and `alt` in dark, light and
high-contrast (tested), and the selection-coloured name keeps 3:1 on `selected`.

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
shows what the drop does (`tree::drop_mark`; ours, **parity-pending G1**: the session's drag
completed before a capture could be taken): **Into** fills the whole hovered row with the state
colour (mint) at 25 % alpha and outlines it 1.5 px (the row becomes the parent); **Before / After**
draw a 2 px line across the row's bottom or top edge with a filled 3 px dot at its left end,
placed at the indent of the depth the nodes would land at (`Before`: the row's depth; `After`:
the row's depth, or one deeper for an expanded row with children, whose first child it becomes).
So a line with the dot at the row's own indent means "next to this row", a dot one level deeper
means "first child", and a filled row means "inside". Dragging is disabled while a
search filter is active. Moving keeps local transforms, like `object.reparent`.

### 13.4 Rename

Double-click (or the context menu item "Rename"; there is no key, `F2` is the Top view, §14.2,
**parity-pending G1**) opens a text field on the row; Enter or focus loss
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
cycles, ranges, filter, context items, visibility resolution (core), the row numbers (20 / 12),
the boxed disclosure, the connector snapshot, the evaluation cell per type, the dot cycle,
the selection-coloured name. egui_kittest: reparent
by drag, reorder by drag, dots and tick (each with undo), rename, search, context menu
actions, a drag onto an attribute link field, the stacked dots (one column, editor above
render, the check to their right), no check on nulls and a target on cameras.


## 14. Key map (`keymap`, `keymap_editor`; brief 152, revised by brief 233)

Sources: GUI session G2 item 1 (docs/parity/GUI-SESSIONS.md, "Owner's Cowork session,
2026-11-27"): the owner read the reference app's Command Manager and menus, so every default
below marked **observed** is the reference's. What the session did not cover (shading, the
`Alt+digit` mode keys, `Mod+1 .. 7` views, `Mod+M`, `Q`, `L`, `Shift+S`, `Shift+M`, the marker
jumps, `Mod+Shift+P`, `Space`'s viewport scope) stays **ours, parity-pending G2**. Owner notes N1
(bare 1 / 2 / 3 are navigation hold keys) and N2 (E / R / T) still hold; N2 is observed too.
Nothing is taken from the reference app's key files or presets.

### 14.1 Model

A `Chord { command, alt, shift, key }` is one key press with modifiers. `command` means
egui's `Modifiers::COMMAND`: Cmd on macOS, Ctrl elsewhere. A `Binding { chord, then, command,
args, scope }` runs the registry command `command` with the JSON object `args`; `then` is
`None` for a single key and the second key of a **two-key sequence** (§14.7), in which `chord`
is the prefix key. `scope` is
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
A `chord` with a space is a two-key sequence (`"M T"`, §14.7). Default bindings. Rows marked
**observed** are G2 item 1; the others are ours (parity-pending G2):

| Keys | Command (args) | Source |
|---|---|---|
| Mod+O / Mod+S / Mod+Shift+S | `file.open_dialog` / `file.save_dialog` / `file.save_as_dialog` | ours |
| Mod+Z / Mod+Y | `edit.undo` / `edit.redo` | observed |
| Backspace, Delete | `object.delete` | observed |
| Mod+A / Mod+Shift+A | `select.all` / `select.none` | observed |
| U I / U Y / U K | `select.invert` / `select.grow` / `select.shrink` | observed |
| U W / U L / U B | `select.connected` / `select.loop` / `select.ring` | observed (Select menu) |
| Alt+1 / Alt+2 / Alt+3 / Alt+4 | `select.mode` model / points / edges / polys | ours, an extra: the reference has no default mode keys (G2 item 1) |
| C | `mesh.make_editable` | observed |
| Alt+G | `object.group` | observed |
| Mod+Shift+P | `ui.command_palette` (brief 159, §18) | ours |
| Mod+Shift+1 .. 5 | `view.shading` smooth, flat, wire, smooth_wire, flat_wire | ours (G2 did not observe shading; owner note N1 moved it off the digits) |
| Mod+Shift+6 .. 9 | `view.shading` quick, constant, hidden_line, bounding_box (brief 143) | ours |
| F1 / F2 / F3 / F4 | `view.set_view` perspective / top / right / front (§6.1) | observed |
| F5 | `view.layout` (toggle single / four; §6.2) | observed (opens four panes); the toggle back is ours |
| Mod+M | `view.maximize` (toggle the active pane; brief 143) | ours |
| H / S / O | `view.frame_all` / `view.frame_selected` / `view.frame_selected` | observed; S and O both frame objects |
| Mod+1 .. Mod+7 | `view.set_view` perspective, front, right, top, back, left, bottom | ours, extras |
| E / R / T | `view.gizmo_mode` move / rotate / scale (owner note N2; W is free) | observed |
| Q | `view.gizmo_space` (toggle) | ours |
| Shift+S / L | `snap.toggle` / `edit.axis_mode` (brief 032) | ours |
| F8 / F7 | `timeline.play` / `timeline.stop` | observed |
| F9 / Mod+F9 | `key.record` / `key.autokey` | observed |
| G / F | `timeline.step` +1 / -1 (next / previous frame) | observed |
| Mod+G / Mod+F | `key.jump` forward true / false (next / previous key) | observed |
| Space (viewport scope) | `timeline.toggle_play` | the reference also plays with Space in the timeline (observed); the viewport scope is ours |
| Shift+M | `marker.add` (brief 094; bare M is the mesh prefix, so it moved) | ours |
| Alt+Comma / Alt+Period | `marker.jump` backward / forward (brief 094) | ours |
| M T / M W / M S / M B | `mesh.extrude` / `mesh.extrude_inner` (Inset) / `mesh.bevel` / `mesh.bridge` | observed |
| M Q / M P / M N / M H | `mesh.weld` / `mesh.stitch` / `mesh.dissolve` / `mesh.mirror` | observed (Mesh menu tables, G10) |
| M M / M O | `mesh.connect_points` / `mesh.slide` | observed (G10) |
| U S / U O | `mesh.subdivide` / `mesh.optimize` | observed |
| U Z / U P / U D / U U / U R | `mesh.melt` / `mesh.split` / `mesh.disconnect` / `mesh.untriangulate` / `mesh.reverse_normals` | observed (G10) |

**Replaced by brief 233** (the earlier own-design choices): `Mod+Shift+Z` redo (now Undo View, no
command yet), `F2` rename (rename is the object manager's double-click or menu entry, §13.4;
`object.begin_rename` has no default key, **parity-pending G1**), `Mod+I` invert (now `U I`),
`Mod+Shift+E` make editable (now `C`), `F4` layout toggle (now `F5`; `F4` is the Front view),
`K` record and `Shift+K` autokey (now `F9` and `Mod+F9`; `K` is free for the Line Cut family),
`,` / `.` and `Shift+,` / `Shift+.` (now `F` / `G` and `Mod+F` / `Mod+G`), `M` marker (now
`Shift+M`).

**No command yet** (observed in G2 item 1, nothing bound until a command exists; the
default-map test pins each as unbound): Live Selection `9`, Rectangle Selection `0` (tool
selection has no command), Undo View / Redo View `Mod+Shift+Z` / `Mod+Shift+Y`, Render View
`Mod+R`, Render to Picture Viewer `Shift+R`, Play backwards `F6` (`timeline.play` has no
direction), Record Animated `Alt+F9`, Ungroup `Shift+G`, and the pointer tools Line Cut `K K`,
Loop/Path Cut `K L`, Plane Cut `K J`, Polygon Pen `M E`, Close Polygon Hole `M D` (their commands
need geometry arguments). Also unbound for lack of a command: `M A` Create Point, `M X`, `M Y`,
`M R`, `M F`, `M G`, `M C`, `M I`, `M U`, `M V`, `M Z`, `U C`, `U H`, `U T`, `U G`, `U E`, `U A`,
`U J`, `U Q`, `U F`, `U M`, `U N`, `U V`. The Spline menu repeats several sequences (`M A`, `U S`,
`U O`, `M Q`, `M H`, `U D`, `U P`, `M I`, `M U`, `K K`) for spline objects; the key map has no
selection-dependent dispatch, so the Mesh commands are bound (**parity-pending G2**). "Toggle
Active View" (middle mouse or Page Up) is the viewport's own gesture, not a binding. The reference
also shows the frame commands with an `Alt+` second chord (`Alt+H`, `Alt+S`, `Alt+O`); ours has
one chord each.

The default map has no bare digit binding, and no single key on a prefix key (§14.7).

### 14.3 User overrides (settings file)

The shared settings file of RPC.md §2 (`forge_platform::settings_path()`: `$FORGE_SETTINGS`, else the per-OS config
directory, PLATFORM.md §1-2) holds
an optional object `keymap`, keyed by canonical or parseable chord text, or by a two-key
sequence (the two chords separated by a space, §14.7):

```json
{"keymap": {"Mod+Shift+K": {"command": "key.autokey", "args": {}, "scope": "global"},
            "M X": {"command": "mesh.extrude"},
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

**Migration (brief 223, owner notes N1 and N2).** Files written before the change hold only
the user's differences from the old defaults, so they stay valid: other bindings are kept
untouched. Two things need care. (1) An entry on a bare `1`, `2` or `3` chord (any command, or
`null`) cannot be honoured, the chords being navigation keys now (§6.1): it is dropped with
the warning "keymap '1': 1, 2 and 3 are navigation keys; the binding was dropped (shading is on
Mod+Shift+1 ..)" and `KeyMapState::use_settings_file` removes exactly those entries from the
file, so the warning appears **once**; every other entry, including unparsable ones, stays in
the file. (2) An entry on `4` .. `9`, `W`, `E`, `R` or `T` is the user's own choice and replaces the
new default on that chord like any override. Defaults the user never touched simply follow the
new table.

### 14.4 Conflicts

Two bindings conflict when they have the same keys and their scopes overlap (`global`
overlaps everything; `viewport` overlaps `viewport` and `global`), or when one is a single key
and the other a sequence that starts with it (§14.7). `KeyMap::conflicts()`
lists them (`Conflict { chord, then, bindings }`: `then` is `None` for the single key and the
sequences it starts); the default map has none (tested). Dispatch with a conflict is deterministic:
the first matching binding in map order runs, and exactly one command runs per key press.
Dispatch is matched exactly on `Mod`, `Alt`, `Shift` (so Mod+A and Mod+Shift+A need no
ordering) and the matching key event is consumed. Nothing runs while a text field has the
keyboard or while the editor is capturing a chord.

`bind` removes the bindings that conflict with the new one and returns them. The editor
never does that silently: it shows the conflicting command and asks to Replace or Cancel.

### 14.5 Key map editor (Window > Key map...)

A floating window (state in `Editor`, not in the layout). Top: a search field (matches
command id, label, or the keys' text, case-insensitive; `m t` finds the sequence M T), a "Reset all" button, a count of
conflicts and unknown commands, and the warnings of the last load. Below, a scrolling list,
one row per binding plus one row for each registry command that has no binding: command
label, id (weak), args summary, the chord as a button. Clicking the chord button starts
capture ("Press keys..."): the next key press with its modifiers becomes the chord; Escape
cancels. If the chord is taken by another binding the window shows "<chord> is bound to
<label>" with Replace and Cancel; Replace rebinds and removes the other (for a single key
on a prefix key, every sequence it starts; §14.7). Each row has
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

Brief 233 adds: the default table above (every observed binding, the "no command yet" keys
pinned unbound), each replaced default checked, F2 is no longer rename, and the sequence rules
of §14.7 (arm, run, time-out, Escape, no conflict with single keys, overrides, the editor's
capture).

### 14.7 Two-key sequences (brief 233; observed, G2 item 1: "M then a letter means two keypresses in sequence")

The reference binds many commands to a prefix key followed by a second key (`M T` Extrude, `U S`
Subdivide, `K L` Loop/Path Cut). The rest of this section (timing, cancel, conflicts, editor,
file syntax) is ours: the session did not measure them (**parity-pending G2**).

- **Model.** `Binding.then: Option<Chord>`; the keys of a binding are `chord` or `chord then`.
  The text form is the chords joined by white space, `M T`, `Mod+K Mod+S`; spaces next to a `+`
  belong to the chord (`Mod + A`). More than two keys, an empty text or an unknown key is an
  error (`parse_keys`, `keys_text` print the canonical form). Used in `default.toml`
  (`chord = "M T"`), in the settings file's `keymap` keys and in the editor. Bare `1`, `2`, `3`
  (§6.1) are refused as the first or the second key.
- **Prefix key.** A key press that is the first key of at least one sequence binding active in
  the current scope **arms** the prefix: the press is consumed and nothing runs. State
  (`KeyMapState.armed: Option<Armed { chord, at }>`) is editor state, not document state, never
  saved. While armed, the status bar shows the prefix in the platform spelling and an ellipsis
  (`M ...`).
- **Second key.** The next key press, with its modifiers matched exactly like a single key,
  runs the binding `chord then` (scope as for singles; one command per press) and disarms.
  Any other key disarms the prefix and is then handled as an ordinary fresh press: it may run
  a single-key binding or arm another prefix (`M` then `U` arms `U`). Modifier keys alone are not
  key presses and change nothing.
- **Time-out.** The prefix lapses 1.5 s (`SEQUENCE_TIMEOUT`) after it was pressed, measured on
  the input clock; a key after that is a fresh press (`M`, wait, `T` is Scale). The window
  repaints once so the status text clears.
- **Escape** while armed disarms and is consumed (nothing runs; a later Escape is ordinary). A
  text field taking the keyboard, an open palette or an active key capture also drop the prefix.
- **Single keys on a prefix.** A key that starts a sequence cannot also run a single binding: a
  single binding and a sequence starting with its chord conflict (§14.4). If such a map is
  loaded anyway the prefix wins (the single key never runs), which keeps dispatch
  deterministic. The default map has none: bare `M` marker became `Shift+M`, bare `K` record
  became `F9`. `M T` and `M W` do not conflict; `M T` and `U T` do not conflict; two bindings
  on the same two keys do.
- **Binding over a collision** (`KeyMap::bind`, the editor's Replace, a settings file entry):
  the new binding removes every binding it collides with. A single key on `M` therefore removes
  all `M` sequences, a sequence on `M T` removes a single `M`, and the same two keys replace
  the old sequence. A `null` entry unbinds exactly its own keys (`"M": null` leaves `M T`).
  `KeyMap::overrides` writes canonical sequence text (`"M T"`) and stays the minimal inverse of
  the merge.
- **Key map editor (§14.5).** A row shows a sequence as its chords in the platform spelling,
  separated by a space; the search matches its canonical or shown text. Capture takes the
  first key press and then waits up to 1.5 s: a second press makes a two-key sequence at once;
  with none the single key is bound when the time is up (the window repaints for it; the
  button shows `M ...` meanwhile); Escape cancels at either step; a second press after the time
  limit is dropped and the first key stands alone. Taken keys ask Replace / Cancel naming the
  bindings that would go (§14.4). A bare navigation digit ends the capture with the message of §6.1.
- **Menus and the palette** show the keys of the first binding of a command as the same text
  (`Binding::display`; the palette's `Entry.keys` is the canonical text).

Tests (crates/ui/tests/keymap.rs): text parse and print (spaces, aliases, three keys refused);
arm then run; a prefix alone runs nothing; modifiers must match; time-out at 1.5 s; Escape
cancels and is consumed; a foreign key disarms and acts fresh; prefix and single key conflict
and the dispatch rule; bind-over replaces; settings-file syntax and the round trip through
`overrides()`; navigation digits refused; kittest: the sequence runs in the window, the
prefix alone does nothing, the status bar shows it, Escape drops it; the editor's capture
(sequence, time-out single key, late second key, Escape, Replace over a prefix, refused digit).

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
no UI change). The remaining submenus are driven by `create_menu` and grouped by
`NodeType::menu_path`: Object, Spline, Generator, Deformer, Field, Volume, Camera, Light,
Tag. Each submenu scrolls when it is taller than the window (its height is capped at the
window height minus 48 px), so every entry stays reachable by pointer (S3, `s3_chair`).

The exact grouping and wording are **parity-pending G1** until the reference app's Create
menu is observed.

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

### 16.6 Field list folders and field display (brief 064)

Sources: own design from docs/roadmap/areas/D-deformers-fields.md; GUI session G4 is
unanswered, so **the columns, the folder wording, the gizmo and preview looks are
parity-pending G4**. The table of layers (enable box, name, blend, strength, move, remove, add
drop-down, drop target) is 16.1; this section adds the folder and the viewport display.

- **Folder.** A `LinkRows` list that accepts fields has a **Folder** button (`Add Folder`).
  It creates a Group field (`forge.field.group`, FIELDS.md F7: a field whose own rows are a nested
  list) with `object.create`, makes the owner the active object again and appends a row for the
  group (first blend mode, 100 %, enabled) with `param.edit`: two undo steps (create, then
  list edit). Selecting the group shows its own `fields` list, which is edited with the same widget.
  `Edit.create` carries the type id from the widget to the editor.
- **Gizmos.** `field_overlay::shown_fields` collects the fields to draw for the selection: the
  selected fields and the enabled rows of the `fields` list of the selected deformers, effectors
  and groups (groups opened, each field once). `field_gizmo` gives world-space line strips of the
  shape (`local_gizmo` in the field's space, then its world matrix): Spherical three circles of
  `radius`; Box the twelve edges of the box of half sizes `size`; Linear the two planes at `-length` and
  `+length` along its direction and the axis between; Cylinder two circles at `-height`, `+height` and four
  side lines (the surface of F6 is at |axial| = height); Cone the base circle at `-height / 2` and two
  lines to the apex at `+height / 2`. Other field types have no gizmo. The editor paints the strips in the
  selection colour; strips are cut where they pass behind the camera (`project_strip`).
- **Preview.** View > **Field Preview** (`Views.field_preview`, off by default, not undoable, not a command) paints one
  dot per point of the selected meshes and of the parents of the selected nodes (at most `MAX_DOTS` = 5000 dots, points behind the
  camera or outside the pane skipped), coloured by the value of the first selected owner's list (a
  selected field is a list of itself): `value_colour` is blue at 0, white at 0.5, red at 1 (clamped).
  The value comes from the same `FieldList::sample` the deformers and effectors use, at the world position.
- Tests: `crates/ui/tests/field_display.rs` (pure gizmo and preview functions, the folder button with undo,
  the Field Preview menu entry). Not tested headless: the painted pixels.

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
With Auto-Key off, an edit of an object that has a Position, Rotation or Scale track stages a
keyframe draft instead of running `transform.edit` (§12.1, owner note N10); every other object of
the selection is edited by the same maths as before.

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

### 24.1 Theme tokens (one module, `theme`; revised by brief 234, owner note N11)
`ThemeKind { Dark, Light, HighContrast }`, names `dark`, `light`, `high-contrast`
(`name()`, `from_name()`, `ThemeKind::ALL`). `Theme::of(kind)` returns the token set; the UI
takes its colours from it. Colour **roles** replace the single accent (the reference layout stays,
D22; the proposal's palette is ours, source: design reference `WORKSPACE-DESIGN.md`, "Dark token"
table, and owner note N11):

| role | tokens | marks |
|---|---|---|
| surfaces | `window`, `panel`, `viewport`, `input`, `faint`, `divider` | shell, managers, canvas backdrop, recessed controls (text fields, drag values), hover/stripe fill, panel edges |
| text | `text`, `weak_text` | controls and values; units and secondary labels |
| **state** (mint) | `state`, `state_text` | the active tool, mode, checked toggle, active attribute tab, active layout tab (underline), links, "current" markers; `state_text` is the text on a `state` fill |
| **time** (mint) | `time` (equal to `state` in every theme, tested) | the playhead and current-frame marker (timeline, dope sheet, F-curve), the timeline's key ticks, preview-range band and handles, key diamonds at or around the playhead (`key` is `time`) |
| **selection** (amber) | `selection`, `selection_text` | selected objects (viewport outline, VIEWPORT.md §16), selected rows and names in the object manager and other lists, selected keys in the dope sheet and F-curve (drawn larger as well), selected nodes and UV points, box-select rectangles; `selection_text` is the text on a `selection` fill |
| **record** (red) | `record`, `error` (equal, tested) | auto-key recording and errors; always paired with a label or shape |
| **focus** | `focus` | keyboard focus ring, IME composition underline |
| **component** | `component_selection` | selected points, edges and polygons of the component target (distinct from the amber object selection) |
| **key** | `key` (= `time`), `modified` | the key diamond: key at this frame and animated-elsewhere use `key`; a draft uses `modified` |

The *modified* hue is a **violet**, so it never reads as "selected" (amber), as the playhead and keys
(mint) or as an error (red); its hue differs from `key` and from `selection` by at least 30 degrees
(tested; §12).

| token | dark | light | high contrast |
|---|---|---|---|
| window | `#171B21` | `#EEF1F5` | `#000000` |
| panel | `#20262E` | `#E6EAF0` | `#000000` |
| viewport | `#252C35` | `#D9DFE7` | `#0A0A0A` |
| input | `#181D24` | `#FFFFFF` | `#000000` |
| faint | `#2A323C` | `#D8DEE6` | `#1A1A1A` |
| divider | `#343D49` | `#B4BEC9` | `#BFBFBF` |
| text | `#E8EDF2` | `#161B22` | `#FFFFFF` |
| weak text | `#A7B2C0` | `#465162` | `#D8D8D8` |
| state / time / key | `#73D6BC` | `#0B6B57` | `#5CFFD8` |
| state text | `#0E221C` | `#FFFFFF` | `#000000` |
| selection | `#F3B36B` | `#8F4C00` | `#FFC266` |
| selection text | `#2A1805` | `#FFFFFF` | `#000000` |
| record / error | `#F17A80` | `#B3202E` | `#FF8A90` |
| modified | `#B79CF2` | `#6B3FC4` | `#D2B0FF` |
| component selection | `#38C6E8` | `#006C99` | `#00E5FF` |
| focus | `#5AA9FF` | `#0B57D0` | `#5CE1FF` |

The light and high-contrast values are derived from the dark ones: every role colour keeps its hue
and is darkened (light) or brightened (high contrast) until it reaches the contrast below.
Contrast (WCAG ratio, tested for every theme): text on panel, window, viewport and input >= 7 (high
contrast >= 15); weak text on panel and viewport >= 4.5; `state_text` on `state` and
`selection_text` on `selection` >= 4.5; `state`, `selection`, `record` and `modified` on panel,
viewport and input >= 4.5 (text drawn in a role colour is then readable); `focus` on panel >= 3.
**Greyscale:** colour is never the only cue. Dark mint and amber have almost the same luminance,
so each role carries a shape, outline or text as well (checked by the role audit below): state is
a filled or checked control or an underline, selection is an outline, a row bar or a larger key,
time is a line, record has a label.
`Theme::visuals()` builds from egui's dark visuals (light visuals for Light): `selection.bg_fill`
is `state` and `selection.stroke` is `state_text` (egui's checked buttons, selectable labels and
sliders are mode and tab states); `hyperlink_color` is `state`; `extreme_bg_color` is `input`;
`widgets.noninteractive.bg_stroke` is `divider`; the active/focused widget outline is a 2 point
stroke in `focus` and the IME composition underlines use `focus`. Lists whose rows are selected
things (nodes, joints, UV points) draw them with `Theme::selection_scope`, which sets
`selection.bg_fill` to `selection` and `selection.stroke` to `selection_text` for the closure.
`Theme::apply(ctx)` installs visuals and fonts and stores the theme in the context;
`Theme::current(&Context)` reads it back (dark before the first apply). `Theme::apply_visuals` does
the same without reinstalling fonts (used when switching).

**Role audit (`theme::role_audit() -> &[RoleUse]`, tested).** Every use of a role colour in
`crates/ui` and `crates/viewport` maps to exactly one `Role` (`State`, `Time`, `Selection`, `Record`,
`Focus`, `Component`). `RoleUse { role, widget, cue }` lists the widgets per role and the
non-colour `Cue` that keeps the state legible in greyscale (`Fill`, `Outline`, `Underline`, `Line`,
`Size`, `Text`). The test checks that every role has widgets, no entry is colour only, and that no
source file under `crates/ui/src` or `crates/viewport/src` reads a token named `accent` (the token no
longer exists). Fixed own colours that are not theme roles are named in the audit as such: the
console WARN level (amber text, always with the word WARN), port-type colours of the node editor,
the object palette's category hues (§29.2) and the clone axis marks.

### 24.2 Preferences (`prefs::UiPrefs`)
`UiPrefs { theme: ThemeKind, scale: f32 }`, default dark and 1.0. `scale` is the UI scale
factor, clamped to `0.5..=3.0` (`MIN_SCALE`, `MAX_SCALE`), multiplied with the OS scale
(`pixels_per_point = native * scale`, egui `set_zoom_factor`). Persistence: the `ui` key of the
settings file (`{"ui": {"theme": "light", "scale": 1.5}}`), other keys kept, written on every
change; `use_settings_file(path)` loads; an unknown theme name or a non-finite or out-of-range
scale falls back per field to the default (lenient, like the key map). `EditorState.prefs`
holds it.

### 24.2a Preferences window (brief 155)

Sources: [area L](../roadmap/areas/L-ui.md), [O29 §§1, 5](../parity/scene-defaults.md),
[PLATFORM §7](PLATFORM.md) and [RPC §2](RPC.md). O29 is form-only; it records a new
project's centimetre scale and 1280×720 render size, Cubic filter at width 2 and
Default Light on. It explicitly excludes the separate user preferences. The
Preferences window and its other defaults are native, not reference parity.

**Edit → Preferences** opens a draft form backed by `ParamDesc` descriptors.
Apply validates and saves the draft; Cancel or closing discards it. Changes are
not scene edits. Write the per-user JSON atomically and keep unrelated settings.

| Controls | Storage and effect |
|---|---|
| Theme, UI scale | Existing `ui.theme` and `ui.scale`; immediate |
| Distance unit (mm, cm, m, in) | `ui.distance_unit` (default cm); convert display/edit values in Attributes and Coordinates, not document or command units |
| Autosave enabled, interval seconds | Existing `autosave` keys; immediate, retention unchanged |
| Editor render defaults (opt-in), width, height, samples, depth, filter and default light | `render_defaults`; applies to new Picture Viewer/IPR renders of scenes without stored render settings; CLI defaults unchanged |
| RPC port and token | Existing `rpc` keys; masked token field, secure temporary file on Unix, restart required, environment overrides still win |

Default per-user RPC remains loopback and cannot enable insecure mode from this
form. An empty token means no saved token, not unauthenticated access. Test a
fresh editor loading saved values from a temporary settings file; O29 defaults
are checked directly because the observation created no golden cases.

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
`crates/ui/tests/accessibility.rs`: token tables, role equalities (time = state = key, record =
error), contrast of every role per theme, the modified hue, the role audit, names round
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

## 28. S3 acceptance: model a chair (`crates/ui/tests/s3_chair.rs`; brief 162)

Sources: scenario S3 of docs/roadmap/README.md; every control used is specified in §5, §11–14,
§20, §27 and in MODELING.md, EDITABLE.md, GENERATORS2.md (Symmetry), SUBDIVISION.md and IO.md.
This section adds no behaviour. Nothing here relies on a reference-app UI rule that has no
observation except those the earlier sections already mark parity-pending: the Create and
Mesh menu arrangement and the tool palette (G1), the key chords (G2), the attribute manager
look (G4). A reference session that answers G1/G2/G4 may rename menu entries or move keys; the
test then changes its labels and chords, not its assertions.

The test drives one `Editor` in an `egui_kittest` window (1280 x 800, no GPU) the way a person
would: menus and buttons are clicked by label, shortcuts are key presses, the file chooser is
an injected `Dialogs`. Picking in the viewport needs the GPU, so a selection is set on the
object (`selection.polys`, `selection.edges`) as in the other UI tests. All files go to a temp
directory. The scenario draws no picture, so there is no image comparison and no tolerance.

Steps and what is asserted after each:

1. Create > Generator > Cube; size 200 x 40 x 200 (`param.set`) and position x = 100
   (`transform.set`): half a seat whose inner side lies on the plane x = 0. The title shows `*`.
2. Edit > Make Editable: the active object is a `forge.polygon` with 8 points and 6 polygons.
3. Leg: select the bottom polygon; Mesh > Extrude Inner, offset 60, Apply (12/10); Mesh >
   Extrude, offset 180, Apply (16/14). Each Apply is one undo step; the lowest point is at
   y = -200; Done closes the tool.
4. Bevel: Alt+3 (edge mode), select the outer top edge of the seat, Mesh > Bevel, offset 8,
   Apply: the point and polygon counts grow.
5. Loop cut: Mesh > Loop/Path Cut, hover an edge, click: counts grow, one undo step.
6. Ctrl+Z removes the cut, Ctrl+Y brings it back.
7. Symmetry (mirror plane ZY, default) is created and the polygon object moved under it;
   Subdivision is created and Symmetry moved under it (object manager hierarchy: Subdivision >
   Symmetry > polygon object). The evaluated Symmetry mesh is symmetric about x = 0 (bounds sum
   to 0) and spans both halves; the evaluated Subdivision mesh has more than four times the
   polygons, is still symmetric and lies inside the bounds of the cage.
8. Ctrl+S (injected path) writes `chair.forge`, the title becomes `chair.forge - Forge3D` and the
   dirty mark clears. A second window opens it with Ctrl+O: the saved bytes equal the first
   window's, the Subdivision mesh evaluates equal, the history is empty.
9. File > Export OBJ...: the options window appears and nothing is written until its Export
   button is pressed. The file holds one object with as many faces as the evaluated Subdivision
   mesh and no more points than it. File > Import OBJ (`import_obj_path`) in an empty scene
   gives one polygon object with that face count.
10. Ctrl+Z until the history is empty leaves an empty scene (dirty) and the directory holds
    exactly `chair.forge` and `chair.obj`.

Known gap found while writing the test: the Create > Generator list is longer than an 800 px
window and the menu does not scroll, so Symmetry and Subdivision (alphabetically late) cannot
be reached with the pointer. The test creates them with `object.create`, the command the menu
item runs. Making the menu scrollable (or grouping the list) is a follow-up to briefs 049/147.

## 29. Reference layout and density (D22; owner screenshot 2026-11-26; metrics observed, G10, brief 233)

Sources: a screenshot of the reference app's Standard layout on an empty document, supplied by the
owner (D22), which the planner read and wrote down in words; and the owner's Cowork session of
2026-11-27 (GUI-SESSIONS.md, G10 item 1 and A2 to A8), which measured the window at 1920 x 1080
(scale 1) in pixels. The image is not in the repository, and no icon was traced. **The numbers of
§29.1a replace the earlier proportions** (which were taken from a 2000 x 790 capture). This
section is the target for briefs 223-229 and 231. Where an earlier section disagrees, this one
wins for layout and density, and the earlier sections for behaviour.

### 29.1a Measured sizes (observed, G10 item 1; `layout::WINDOW`, `layout::regions`, `theme::BUTTONS`)

At 1920 x 1080, scale 1 (a 1520-point horizontal resize reduced the captured width by exactly 1520
px, so the capture is 1x), origin at the window's top-left:

| Region | Observed |
|---|---|
| Title bar (native), document / layout row | y 0-28, y 28-52 (24) |
| Command bar, viewport menu row | y 52-94 (42), y 94-114 (20) |
| Left tool strip | x 0-43 (43 wide), from the command bar down to the bottom of the viewport |
| Viewport canvas | x 43-1472, y 114-962: about 1429 x 848 |
| Object palette | x 1473-1516 (43 wide) |
| Managers column (Objects / Takes above Attributes / Layers) | x 1517-1919 (402 wide); the split sits near y 500, about 47 % of the height |
| Timeline | x 43-1473, y 963-1055 (92) |
| Status bar | y 1055-1079 (24) |
| Buttons | command bar about 27 x 26, vertical palettes 34 x 34 |

`layout::WINDOW` holds the numbers (rows 28 / 24 / 42 / 20 / 92 / 24, columns 43 / 43 / 402, the
split 0.47, a 1 px separator between regions: three across, two down); `layout::regions(w, h)`
lays the regions out and reproduces the table above at 1920 x 1080 (tested). The default layout's
`Sizes` read them (§11): right width 402, split 0.47, timeline 92; the document row, the command
bar and the status bar use them as their panel heights (24 minimum, 42 minimum, 24 exact).
`theme::BUTTONS` holds the button sizes, used as the minimum size of the command bar's and the
palette's buttons (27 x 26 and 34 x 34; our text-labelled buttons are wider). **Parity-pending
G10:** our tool column is still 150 wide with labelled buttons (43 once the icons of brief 227
land), the object palette strip (43) comes with brief 228, the title bar is the window system's,
and the viewport menu row (20) comes with brief 227. The timeline ruler labels every 5 frames at
this width (`timeline::ruler_step(90, 1413) = 5`, label spacing at least 40 px: observed, G10 A7;
this replaces the "every 2 frames" read from the earlier screenshot). The status bar shows the
active tool's help text (G10 A8), here `tools::help_text` for the transform tool and the selection
mode, with errors replacing it.

### 29.1 Window structure, top to bottom

1. **Title bar:** the document name and the layout name, centred ("Untitled 1 - Main").
2. **Document and layout row:** on the left, undo/redo, then **document tabs** (one per open
   document, with a close cross, then "+"). On the right, the **layout switcher**: a row of text tabs
   (Standard, Model, Sculpt, UV Edit, Paint, Groom, Track, Script, Nodes, then "+"), the active one
   underlined, then a "New Layouts" toggle. Our names are ours (§21 workspaces), and we keep this
   placement and form.
3. **Main area, left to right:**
   - **Tool strip** (43 px wide, one icon column, from the command bar down): search at the top, then
     selection tools, then **Move, Rotate, Scale** (the active tool has a state fill, mint), then the other
     transform tools, then sculpt/paint-style brushes. Groups are separated by a small gap, not lines.
     Hovering shows the tool name next to the strip (for example "Move" with its icon).
   - **Viewport column** (about 1429 px wide at 1920, the rest of the width):
     - *Command bar* (one row, icons only): on the left, a make-editable style button, the **X / Y / Z axis
       locks** and the coordinate-system toggle. In the centre, the **component modes** group (model,
       mesh, points, edges, polygons, and the related UV and texture modes; the active one is
       filled), then the **snapping** group (two buttons), then the **workplane / grid** group (two),
       then two more toggle pairs. On the right, the **render** group (render view, render to
       picture viewer, render settings). Groups are separated by gaps.
     - *Viewport menu bar* (one row, text): a hamburger, then View, Cameras, Display, Options,
       Filter, Panel. On the right, four small navigation icons (pan, zoom, orbit, switch/maximise
       view). These work as drag handles: dragging one navigates.
     - *Viewport:* the view name top-left ("Perspective"), the camera name centred at the top
       ("Default Camera" with a small icon), a small axis triad top-right, "Grid Spacing : 50 cm"
       bottom-right. Outside the render frame the view is darker (the film-gate shading of
       VIEWPORT.md). There's a flat grid, and the world axes are drawn as long thin red, green and blue lines.
     - *Timeline* (92 px high), under the viewport: a row of controls, a ruler, and a
       range row. Controls: a key button on the left; in the centre, go to start, previous key,
       previous frame, play, next frame, next key, go to end; the loop and sound toggles; the
       current frame field ("0 F"); record and autokey (the autokey button turns red), the keying
       options; the key-type toggles (position, rotation, scale, parameter, point level). The ruler
       is labelled every 5 frames at 1920 px (§29.1a), and the playhead is a time (mint) marker. The range row
       has min, preview start, preview end and max fields ("0 F" … "90 F").
   - **Object palette strip** (43 px wide, one icon column, between the viewport and the
     managers): **creation categories**, each a drop-down group, colour-coded by category: object
     selection/null, **primitives (blue)**, splines/text, **generators (green)**, **the clone
     toolset (green: the Cloner sits here, top level, one click; owner note N4)**, **deformers
     (purple)**, **fields (purple)**, environment, camera, light. Then a separate tool at the bottom.
   - **Managers column** (402 px wide):
     - *Object manager* (top, about 47 % of the column height): tabs "Objects | Takes", then a menu
       row (hamburger, File, Edit, View, Object, Tags, Bookmarks) with search, filter and other icons on the
       right.
     - *Attribute manager* (bottom, about 53 %): tabs "Attributes | Layers", then a menu row
       (hamburger, Mode, Edit, User Data) with back/forward, up, search, lock and new-window icons on
       the right. Then a header row with the object's icon and name, and a preset drop-down
       ("Default") on the right. Then **tab buttons** (§29.3).
4. **Status bar:** one thin row at the bottom (a hamburger, then the active tool's help text; §29.1a).

### 29.2 Density and look

- **Dense:** small text, single-row bars, icon-only command bar and strips, and small gaps between
  groups instead of separator lines. The viewport gets almost all the space.
- **Flat dark theme:** charcoal panels, slightly darker splitters, no bevels or drop shadows.
  Text and icons are light grey. **Role colours** (brief 234, owner note N11; §24.1): the **state** fill (mint) marks the active
  tool, mode, attribute tab and timeline toggles, and the active layout tab is underlined in it; the
  **time** colour (mint) is the playhead; the **selection** colour (amber) marks selected things
  everywhere (objects in the viewport and rows in the managers, keys, nodes); **record** (red) marks
  recording and errors. Amber never marks a mode and mint never marks a selection.
- **Icons:** monochrome light grey, except the object palette's category colours (primitives,
  generators/clone toolset, deformers/fields each have their own hue). Our hues and icon art are our own
  (D22). The rule is one hue per category, used for that category's icons everywhere (object
  manager rows, menus).

### 29.3 Attribute manager layout (with §12 and brief 231)

- **Tabs are small toggle buttons that wrap onto several rows** when they don't fit (the Project
  example shows two rows: Project, an exchange-plug-in tab, Info, Dynamics, XRefs / To Do, Animation, Nodes).
  The active tab is state-filled (mint). Some tabs have a small leading icon.
- Under the tabs: the tab's title in bold ("Project"), then rows. **Collapsible groups** have a
  ▾ header ("Time", "Execution", "Display").
- **Rows:** a right-aligned label column, then the value. Number fields have small step arrows on
  both sides (‹ value ›) and show their unit ("0 F", "100 %"). Drop-downs show a ▾. Checkboxes are
  small state-filled (mint) squares. A slider can follow a percent field. A colour swatch has a picker
  button.
- **Two-column groups:** short related fields sit side by side, each with its own label
  (FPS | Project Time; Min Time | Max Time; the Execution checkboxes in two columns). Use this
  when a group's fields are short. It is per group, data-driven (a `columns: 2` UI hint).
- A `›` before a row means it expands to show sub-parameters (View Clipping, Input Color Profile).

### 29.4 What this changes (brief mapping)

| Rule | Brief |
|---|---|
| Tool strip left, Move/Rotate/Scale with state fill, hover name | 227 (icons, layout table), 223 (E/R/T) |
| Command bar groups: axis locks, component modes (the one mode UI), snapping, workplane, render | 225 (one mode UI), 227 |
| Viewport menu bar and the four navigation handle icons | 227 (handles drag = navigate, with 1/2/3, 223) |
| Object palette strip with colour-coded creation groups, Cloner one click | 228 |
| Managers column: Objects/Takes over Attributes/Layers, menu rows | 226 (docking default layout), 229 |
| Timeline row composition | 227 |
| Attribute tabs as wrapping toggle buttons, two-column groups, step arrows | 231 (and §12) |
| Document tabs and the layout switcher row | 227, 226 (layouts = §21 workspaces) |
