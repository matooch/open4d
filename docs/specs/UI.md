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
dock: Dock, camera: Camera, shading: Shading, timeline: Timeline, status: String }`,
built with `EditorState::new(Arc<Registry>)`.

`NodeSelection { nodes: BTreeSet<NodeId>, active: Option<NodeId>, anchor: Option<NodeId> }`:
- `click(id)`: select only `id` (active and anchor = id).
- `toggle(id)` (Ctrl-click): add/remove; active = id if added, else the largest
  remaining id or `None`; anchor = id.
- `range(id, rows)` (Shift-click): select every visible row between the anchor and
  `id` inclusive (row order), active = id; without an anchor behaves like `click`.
- `retain_existing(scene)`: drops deleted nodes (active/anchor too).

## 2. Dock model (`dock`)

Areas: `Left, RightTop, RightBottom, Bottom, Center`. Panels: `Viewport, ObjectTree,
Attributes, Timeline`. Each area holds an ordered tab list and an active index; each
panel appears at most once in the whole dock.
- `Dock::default()` (our layout): Center [Viewport], RightTop [ObjectTree],
  RightBottom [Attributes], Bottom [Timeline], Left [].
- `area_of(panel)`, `tabs(area)`, `active(area) -> Option<Panel>`.
- `move_panel(panel, to)`: removes it from its area (fixing that area's active index:
  stays on the same panel if possible, else clamps), appends to `to`, makes it
  active there. Moving to its own area is a no-op.
- `close(panel)`, `open(panel)`: reopen in its default area (no-op if present) and
  make it active. `set_active(area, i)` (ignored if out of range).
- An area with no tabs is not drawn. Left/right/bottom areas are resizable.

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

Edits: `apply_edit(doc, id, param, value, gesture)`; `Gesture::Commit` → one
`transact("Set <label>")`; `Gesture::Drag(n)` → `transact_merged` keyed
`"drag:<id>:<param>:<n>"`, so one continuous drag is one undo step. Validation
errors are returned (and shown in `status`) and change nothing. Name edits go through
`rename`.

## 5. Commands (`commands`)

`Command::{Create(type_id), DeleteSelected, Undo, Redo, Rename(id, name),
Reparent(id, Option<NodeId>), ToggleExpanded(id), SetShading(Shading),
FrameAll, MakeEditable, CurrentStateToObject}`; `execute(&mut EditorState, Command) -> Result<(), String>`.
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
- `FrameAll`: no geometry yet → resets the camera to default (placeholder until
  Phase 5).

`create_menu(registry) -> Vec<(Category, Vec<(type_id, label)>)>`: placeable
categories only (hierarchy + Tag), in `Category` order, types sorted by label; types
with `in_create_menu == false` (the polygon object) are left out.

Key bindings (**parity-pending**, ours): Ctrl+Z undo, Ctrl+Shift+Z / Ctrl+Y redo,
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

## 7. Timeline placeholder (`timeline`)

`Timeline { start: 0, end: 90, current: 0, fps: 30, playing: false }`
(**parity-pending** defaults). `tick()` advances `current` by one frame when playing,
wrapping from `end` to `start`; `set_current(f)` clamps to `[start, end]`;
`set_range(a, b)` orders the pair and clamps `current`. The panel shows play/pause,
a frame slider and the range; nothing is animated yet (Phase 11).

## 8. Theme (`theme`)

Our own dark palette (sRGB): window `#1E2024`, panel `#25282D`, faint
`#2E3238`, text `#D6D9DE`, weak text `#9AA0A8`, accent `#E08A2E` (selection), accent
text `#1A1A1A`, error `#E5534B`. `apply(&egui::Context)`. Requirement: text on panel
and on window has WCAG contrast ≥ 7, weak text ≥ 4.5, accent text on accent ≥ 4.5.

## 9. App (`app`, `apps/studio`)

`forge_ui::run()` opens a window titled "Forge3D" (eframe, wgpu backend) with a menu
bar (File: New; Edit: Undo/Redo/Delete/Make Editable/Current State to Object; Create: §5 menu; View: shading; Window:
open/move panels, reset layout), the dock (§2) and a status bar. `apps/studio`
calls it with `Registry::with_builtins()`. Window menu: one toggle per panel labelled
"<Panel> panel", a "Move <Panel>" submenu per panel with "<Panel> to <area>" items,
and "Reset layout".

UI tests (egui_kittest, no GPU): create via the Create menu, select in the tree,
edit an attribute, undo, and move a panel via the Window menu.
