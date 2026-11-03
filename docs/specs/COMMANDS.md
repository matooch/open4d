# Commands (`crates/command`, package `forge-command`)

Source: our own design (docs/roadmap/areas/A-automation.md, DECISIONS D3); no reference
observation is involved. Command names follow public menu names so users can find them.
Nothing here is **parity-pending** except the key bindings, which stay as in UI.md §5.

Every user action is a named, typed command in one `CommandRegistry`. The editor menus,
shortcuts and panels run commands through it; later the CLI, RPC, MCP and scripting front
ends (briefs 013-016) call the same registry.

## 1. The handler trait

```rust
trait CommandHandler: Send + Sync {
    fn id(&self) -> &str;                  // dotted lowercase, see 2
    fn label(&self) -> &str;               // menu text, non-empty
    fn args_schema(&self) -> serde_json::Value;   // JSON Schema (draft 2020-12) of args
    fn run(&self, ctx: &mut dyn CommandContext, args: serde_json::Value)
        -> Result<serde_json::Value, CommandError>;
    fn undoable(&self) -> bool;            // true: a run may edit the document (one undo step)
    fn sample_args(&self) -> serde_json::Value;   // a valid example (tests, docs); default `{}`
}
```

`sample_args` is an addition to the brief's trait: the registry test needs one valid
invocation per command.

`CommandRegistry`: `register(Box<dyn CommandHandler>) -> Result<(), CommandError>` (rejects
a duplicate or malformed id), `get(id)`, `iter()` (insertion order), `run(ctx, id, args)`.
`run` fails with `CommandError::Unknown` for an unknown id. `register_fn::<A>(id, label,
undoable, fn(&mut dyn CommandContext, A) -> Result<Value, CommandError>)` builds a handler
whose arguments are the serde struct `A`; its schema comes from `schemars`.

Plugins: `trait CommandPlugin { fn register_commands(&self, &mut CommandRegistry)
-> Result<(), CommandError> { Ok(()) } }`. It lives in `forge-command`, not on
`forge_core::Plugin`, because `forge-command` depends on `forge-core` (a method on
`Plugin` would be a dependency cycle). `register_editor_commands` registers the built-in
set below.

`CommandError`: `Unknown(id)`, `InvalidArgs(msg)` (arguments do not fit the struct or are
out of range), `Failed(msg)` (the command ran and refused; nothing changed). `Display` is
the message only, so the editor can put it in the status bar.

## 2. Ids

Lowercase ASCII `a-z 0-9 _`, segments joined by `.`, at least two segments
(`domain.verb[_noun]`). Domains: `object edit view mesh select file key timeline transform
param`. Ids are stable: renaming one is a breaking change.

## 3. Context

`CommandContext` is what commands may touch; it never mentions egui. `EditorState`
implements it; `forge-app` will implement it headless.

- `doc()`, `doc_mut()`: the `Document` (the only way to change the scene).
- `selection()`, `selection_mut()`: `NodeSelection` (UI.md §1; now defined in
  `forge-command` and re-exported by `forge-ui`).
- `edit_mode()`, `set_edit_mode()`: `ModeArg` (`model|points|edges|polys`).
- `timeline()`, `timeline_mut()`: `Timeline` (also moved to `forge-command`), `autokey()`,
  `set_autokey()`.
- `evaluated_scene()`: the scene at the current frame (default: the stored scene).
- `set_status(String)`, and no-op defaults for view-only state: `set_shading(ShadingMode)`,
  `frame_all()`, `toggle_expanded(NodeId)`.
- Editor services that still live in `forge-ui` and are called through the trait:
  `select_components(SelectOp)`, `record_keys()`, `delete_keys()`, `jump_key(forward)`.
  They move into command-neutral code when `forge-app` needs them.
- Documents: `reset_document(Document, path, full)`, `file_path()`, `mark_saved(path)`.

## 4. Argument conventions

- Arguments are one JSON object, deserialized into a struct with
  `#[serde(deny_unknown_fields)]`; missing required fields and unknown keys are
  `InvalidArgs`.
- Node ids are JSON integers (`NodeId.0`).
- Units as shown to the user: angles in **degrees**, percent as shown (`50` is 50 %),
  distances in scene units. Stored values stay radians and fractions; commands convert
  (`param.set` by the parameter's `Unit`, `transform.set` for rotation).
- Rotation is `[h, p, b]`, scale and position are `[x, y, z]`.
- Optional `nodes` on object commands means "the current selection" when absent.
- Enum arguments are lowercase strings.
- The result is a JSON value, `null` when there is nothing to report.

## 5. Undo

A command that edits runs inside exactly one `Document::transact` (or `transact_merged`
for `timeline.settings`), so one successful run is at most one undo step and a failed run
changes nothing. The transaction label is the specific edit ("Create Cube", "Delete",
"Move"); the handler `label()` is the menu text. `undoable()` is true for every command that
can write to the document, including `select.*` (in a component mode it writes the polygon
object's selection param as one "Select" step, EDITABLE.md §2.5; in model mode it only
changes the selection). View, selection-mode, file and timeline-position commands never
touch history.

## 6. Commands

| id | arguments | result | undoable |
|---|---|---|---|
| `object.create` | `type` | `{node}` | yes |
| `object.delete` | `nodes?` | `{deleted: n}` | yes |
| `object.rename` | `node`, `name` | null | yes |
| `object.reparent` | `node`, `parent?` (null = root) | null | yes |
| `object.move` | `nodes?` (selection), `parent?` (null = root), `index?` (position among the parent's children without the moved nodes; absent = last) | `{moved}` | yes |
| `object.group` | `nodes?`, `name?` (default "Null") | `{node}` | yes |
| `object.create_cloner` | `nodes?` (selection), `name?` (default "Cloner") | `{node}` | yes |
| `object.visibility` | `nodes?`, `editor?`, `render?` each `default`\|`on`\|`off` (at least one) | null | yes |
| `object.enable` | `nodes?`, `enabled` | null | yes |
| `edit.undo` | - | `{done}` | no |
| `edit.redo` | - | `{done}` | no |
| `edit.history_jump` | `position` (steps applied, 0 ..= undo + redo steps) | `{position, undone, redone}` | no |
| `view.shading` | `mode` smooth, flat, wire, smooth_wire, flat_wire, quick, constant, hidden_line, bounding_box; `pane?` perspective, top, front, right (default: the active pane) | null | no |
| `view.frame_all` | - | null | no |
| `view.frame_selected` | - | null | no |
| `view.set_view` | `view` perspective, front, back, left, right, top, bottom; `pane?` perspective, top, front, right (default: the active pane) | null | no |
| `view.layout` | `layout?` single, four (absent toggles) | null | no |
| `view.maximize` | `pane?` perspective, top, front, right (default: the active pane) | null | no |
| `view.lighting` | `mode` headlight, rig | null | no |
| `view.shadows` | `enabled` | null | no |
| `view.antialias` | `enabled` | null | no |
| `view.clone_display` | `indices?`, `axes?` (at least one) | null | no |
| `view.set_camera` | `target?` [x,y,z] world, `distance?`, `yaw?`, `pitch?` (degrees), `projection?` perspective, orthographic, `fov?` (degrees), `height?` (orthographic) | null | no |
| `view.get_camera` | - | `{target, eye, distance, yaw, pitch, projection, fov?, height?}` | no |
| `view.gizmo_mode` | `mode` move, rotate, scale | null | no |
| `view.gizmo_space` | `space?` object, world (absent toggles) | null | no |
| `object.begin_rename` | - | null | no |
| `view.toggle_expanded` | `node` | null | no |
| `mesh.make_editable` | `nodes?` | `{made, skipped}` | yes |
| `mesh.current_state_to_object` | `nodes?` | `{made, skipped}` | yes |
| `mesh.extrude` `mesh.extrude_inner` `mesh.bevel` | `node?`, `polys?`, `settings?`; bevel also `edges?`, `points?` (MODELING.md §0.1, §3) | `{node, points, polys, selected}` | yes |
| `mesh.subdivide` `mesh.triangulate` `mesh.untriangulate` `mesh.reverse_normals` `mesh.optimize` `mesh.weld` `mesh.melt` `mesh.dissolve` `mesh.split` `mesh.disconnect` `mesh.connect_points` `mesh.mirror` `mesh.center_axis` | `node?`, `polys?`, `settings?`; weld, dissolve and connect_points also `edges?`, `points?` (MODELING.md §0.1, §4) | `{node, points, polys, selected}`; split adds `created`, center_axis adds `center` | yes |
| `mesh.loop_cut` | `node?`, `edge` (id or point pair), `offsets?` or `count?` (MODELING.md §5.1) | `{node, points, polys, cut_edges}` | yes |
| `mesh.knife` | `node?`, `mode` line / plane / loop, `polys?`, `point` `normal` / `from` `to` `eye`\|`direction` `visible_only?` / `edge` `offset?` (MODELING.md §5.2) | `{node, points, polys, cut_edges}` | yes |
| `mesh.slide` | `node?`, `edges?`, `points?`, `settings?: {amount, toward}` (MODELING.md §5.3) | `{node, points, polys, selected}` | yes |
| `mesh.bridge` | `node?`, `edges: [e, e]` or `polys: [p, q]`, `segments?` (MODELING.md §6.2) | `{node, points, polys, new_polys, new_points}` | yes |
| `mesh.close_hole` | `node?`, `edge` (id or point pair of a border edge) (MODELING.md §6.3) | `{node, points, polys, new_polys, new_points}` | yes |
| `mesh.stitch` | `node?`, `points: [[a, b]]` or `edges: [[e, e]]`, `position?` middle\|first (MODELING.md §6.4) | `{node, points, polys, merged}` | yes |
| `mesh.pen` | `node?`, `corners: [{point} \| {pos}]` (MODELING.md §6.5) | `{node, points, polys, new_polys, new_points}` | yes |
| `object.duplicate` | `nodes?`, `copies?`, `offset?`, `rotation_deg?`, `scale?`, `mode?` step\|total (MODELING.md §9.2) | `{created, count}` | yes |
| `object.arrange` | `nodes?`, `mode?` line\|grid\|circle, `spacing?`, `row_spacing?`, `columns?`, `radius?`, `plane?`, `start_angle_deg?` (MODELING.md §9.3) | `{arranged}` | yes |
| `object.randomize` | `nodes?`, `seed?`, `position?`, `rotation_deg?`, `scale?`, `uniform_scale?` (MODELING.md §9.4) | `{randomized}` | yes |
| `object.transfer` | `target`, `source?`, `position?`, `rotation?`, `scale?`, `space?` world\|local (MODELING.md §9.5) | `{node}` | yes |
| `mesh.array_command` | `node?`, `polys?`, `settings?: {copies, offset, rotation_deg, scale, mode}` (MODELING.md §9.6) | `{node, points, polys, selected}` | yes |
| `mesh.transform` | `node?`, `nodes?`, `space?` (EDITABLE.md §6.4), `points?`, `move?`, `rotate? {axis, angle}`, `scale?`, `pivot?`, `soft?`, `symmetry?` (EDITABLE.md §5.5) | `{node, moved, soft, symmetry}` | yes |
| `edit.soft_selection` | `enabled?`, `radius?`, `falloff?`, `distance?`, `curve?` (EDITABLE.md §5.1) | the soft selection settings | no |
| `edit.symmetry` | `enabled?`, `axis?`, `offset?`, `tolerance?` (EDITABLE.md §5.1) | the symmetry settings | no |
| `select.mode` | `mode` model, points, edges, polys | null | no |
| `select.all` `select.none` `select.invert` `select.grow` `select.shrink` `select.connected` | `nodes?` (polygon objects; the component targets when absent, EDITABLE.md §6.3) | null | yes |
| `select.convert` | `nodes?`, `to` points, edges, polys; `mode?` strict, tolerant (EDITABLE.md §2.8) | null | yes |
| `select.loop` `select.ring` | `nodes?`, `across?` 0 or 1 (EDITABLE.md §2.8) | null | yes |
| `graph.add_node` | `tag`, `type`, `values?`, `pos?` (NODES.md §6) | `{node}` | yes |
| `graph.remove` | `tag`, `node?`, `nodes?` | `{removed_links}` | yes |
| `graph.connect` | `tag`, `from_node`, `from_port`, `to_node`, `to_port`, `release_node?`, `release_port?` (type and cycle checked) | `{replaced}` | yes |
| `graph.move` | `tag`, `moves` (`{node, x, y}` list), `gesture?` | `{moved}` | yes (a gesture merges) |
| `graph.disconnect` | `tag`, `to_node`, `to_port` | null | yes |
| `graph.set_value` | `tag`, `node`, `port`, `value`, `gesture?` | null | yes (a gesture merges) |
| `graph.get` `graph.evaluate` | `tag` | graph / dry-run result | no |
| `graph.node_types` | - | `{types}` | no |
| `file.new` | - | null | no |
| `file.open` | `path` | `{warnings}` | no |
| `file.save` | `path?` (default: current file) | `{path}` | no |
| `file.save_as` | `path` | `{path}` | no |
| `file.open_dialog` `file.save_dialog` `file.save_as_dialog` | - | null | no |
| `ui.command_palette` | - | null | no |
| `file.autosave_now` | `wait?` (flush the background write) | `{path, queued}` | no |
| `file.import_obj` | `path` | `{nodes}` | yes |
| `file.export_obj` | `path`, `selected?` | `{objects}` | no |
| `key.record` | - | `{keys}` | yes |
| `key.delete_at_frame` | - | `{keys}` | yes |
| `key.set` | `node`, `track`, `frame`, `value` | null | yes |
| `key.jump` | `forward` | `{frame}` | no |
| `key.autokey` | `enabled?` (absent toggles) | `{enabled}` | no |
| `timeline.set_frame` | `frame` | `{frame}` | no |
| `timeline.step` | `delta` | `{frame}` | no |
| `timeline.play` / `timeline.stop` / `timeline.toggle_play` | - | null | no |
| `timeline.settings` | `fps`, `start`, `end` | null | yes |
| `timeline.set_fps` | `fps` (1-240): keys keep their seconds, ranges/markers/current frame convert (ANIM.md §2.9) | `{fps,start,end,frame}` | yes |
| `timeline.set_preview` | `start`, `end` (inside the project range) | null | yes |
| `timeline.clear_preview` | - | null | yes |
| `timeline.set_mode` | `mode`: `once`, `loop`, `ping_pong` | null | yes |
| `timeline.set_frame_dropping` | `enabled` (true: real-time, false: every frame) | null | yes |
| `timeline.set_display` | `format`: `frames`, `smpte`, `seconds` | null | yes |
| `timeline.info` | - | the timeline state (fps, range, preview, mode, display, frame, text, markers) | no |
| `marker.add` | `frame?` (default current), `name?`, `color?` (`#rrggbb`) | `{id,frame}` | yes |
| `marker.remove` | `id` | null | yes |
| `marker.rename` | `id`, `name` | null | yes |
| `marker.move` | `id`, `frame` (steps of one drag merge) | null | yes |
| `marker.set_color` | `id`, `color` | null | yes |
| `marker.list` | - | `{markers:[{id,frame,name,color}]}` | no |
| `marker.jump` | `forward` | `{frame,found}` | no |
| `transform.set` | `node`, `pos?`, `rot?` (degrees), `scale?` | null | yes |
| `object.set_axis` | `node?`, `pos?`, `rot?` (degrees), `scale?`, `space?` (`local`, `world`) (MANIPULATE.md §7.2) | `{nodes}` | yes |
| `object.center_axis` | `nodes?`, `to?` (`bounding_box`, `points`, `selection`, `children`) | `{nodes, centers}` | yes |
| `object.align_axis` | `nodes?`, `to` (`world`, `parent`, `normal`) | `{nodes}` | yes |
| `edit.axis_mode` | `enabled?` (toggles when absent) | `{enabled}` | no |
| `snap.set` | `enabled?`, `mode?`, `radius?`, `visible_only?`, `grid_spacing?`, `targets?`, `workplane?`, `quantize?` (MANIPULATE.md §6.1) | the snap settings | no |
| `snap.toggle` | `enabled?` | `{enabled}` | no |
| `snap.reset` | - | the snap settings | no |
| `snap.query` | `point`, `radius?`, `nodes?` (MANIPULATE.md §6.4) | `{hit}` | no |
| `param.set` | `node`, `param`, `value` | null | yes |
| `param.edit` | `param`, `value`, `nodes?`, `component?`, `gesture?` | `{changed}` | yes |

`object.move`: nodes listed below another listed node are dropped from the list (they move
with it); the rest keep the order given and end up consecutively at `index`; one undo step;
a cycle fails and changes nothing. `object.group` creates a `forge.null` under the first
node's parent at the first node's position (after the group's nodes are removed) and moves
the nodes into it in the order given (nested ones dropped as above), then selects the group.
`object.create_cloner` (brief 079, UI.md §16.2) creates a `forge.cloner` (with the smoothing
tag, like `object.create`) under the first top-level node's parent at its position, moves the
nodes (nested ones dropped, as `object.group`) into it in the order given and selects it, in one
undo step "Create Cloner". With no nodes it creates an empty cloner. `view.clone_display`
(UI.md §16.3) switches the viewport's clone index and axes overlay; view state only.
`object.visibility` / `object.enable` write the `object.*` params (TRANSFORM.md §2) of every
named node in one undo step; tags and non-hierarchy nodes are skipped.

Rules (behaviour moved unchanged from the former `Command` enum, UI.md §5 and ANIM.md §2):

- `object.create`: see UI.md §5 (a Tag type attaches to the active node, errors without
  one; a Generator also gets a smoothing tag in the same step; the new node becomes the
  only selected one). Unknown type is `Failed`.
- `object.delete`: skips nodes inside another selected node's subtree; clears the selection.
- `edit.undo` / `edit.redo`: then `retain_existing` and clamp the timeline. `done` is false
  when there was nothing to undo.
- `edit.history_jump` (brief 159, UI.md §18.6): undoes or redoes until exactly `position` steps
  are applied, then retain_existing and clamp the timeline like `edit.undo`. `position` above the
  number of undo plus redo steps is `InvalidArgs`. It is the history panel's click.
- `ui.command_palette` (brief 159, UI.md §18): a window action like the `file.*_dialog` ones.
- `mesh.*`: EDITABLE.md §1; skipped nodes (no mesh) are listed in `skipped` and in the
  status text. Empty selection is `Failed`.
- `select.*`: EDITABLE.md §2.5 (model mode selects objects).
- `file.autosave_now`: PLATFORM.md §7.8; needs the editor window (headless: `needs the editor window`).
- `file.*`: IO.md §2. `file.new`/`file.open` replace the document and reset selection,
  timeline and expansion; the dock layout stays. Errors read `Open failed: ...`,
  `Save failed: ...`, `Import failed: ...`; an OBJ import without polygons fails with
  `Import: no polygons found`; an export of nothing fails with `Nothing to export`.
  Dialogs and "save changes?" prompts stay in the editor: it picks a path, then runs
  the command. `file.export_obj` writes the scene **evaluated at the current frame**
  (`CommandContext::evaluated_scene`), like `App::export_obj`; an un-animated scene exports
  exactly its stored geometry.
- `view.gizmo_*`, `object.begin_rename`, `file.*_dialog` (brief 152, UI.md §14): window actions
  so that shortcuts are commands. The dialog ones and `begin_rename` call
  `CommandContext::request_ui(UiRequest)`, which the editor window serves after the command
  returns (file dialogs, unsaved-changes prompt, inline rename); headless contexts fail with
  `needs the editor window`. `view.gizmo_*` are view-only no-ops headless.
- `view.frame_all`, `view.frame_selected`, `view.set_view`, `view.set_camera`, `view.get_camera`
  (brief 153, VIEWPORT.md §7, UI.md §6.1): camera commands. They change view state only and
  are **not undoable** (`undoable` false, no document history step), like `view.shading`:
  the camera is not part of the document, so Ctrl+Z never moves it. Frame all fits every
  visible mesh object (nodes drawn by the viewport) of the evaluated scene; frame selected
  fits the selected nodes and their descendants (selected nodes without any geometry recentre on
  their world positions, keeping the distance); with nothing selected it frames all. With no geometry
  at all, `view.frame_all` resets the default camera as before. Both use the viewport's last
  aspect ratio. `view.set_camera` changes only the given fields and validates: `distance`
  and `height` > 0, `fov` in (1, 170) degrees, `pitch` clamped to the camera limit, finite
  numbers; errors are `Failed` and change nothing. `target` is in world coordinates; `yaw`
  and `pitch` are the camera's display-space angles in degrees (they equal the world angles
  in a right-handed world). `view.get_camera` reads the same fields back (`eye` in world
  space), so an agent can drive the camera and check it. Headless contexts (no viewport)
  ignore the setters and answer `view.get_camera` with null.
- `view.shading`, `view.set_view`, `view.layout`, `view.maximize`, `view.lighting`, `view.shadows`,
  `view.antialias` (brief 143, VIEWPORT.md §8-§9, UI.md §6.2): view state only, not undoable.
  `pane` selects one of the four viewport panes (`perspective`, `top`, `front`, `right`); absent it
  means the active pane. A command with a `pane` changes that pane's camera or shading and leaves the
  other panes and the active pane alone. `view.layout` without `layout` toggles; `view.maximize`
  toggles the pane's maximised state (a no-op in the single layout). `view.lighting`
  chooses the headlight or the default rig, `view.shadows` and `view.antialias` switch shadow maps
  and 4x MSAA. Headless contexts ignore all of them. `view.frame_*`, `view.set_camera` and
  `view.get_camera` act on the active pane.
- `key.*`, `timeline.*`: ANIM.md §2; `timeline.set_frame` and `step` clamp to the project
  range. `timeline.settings` merges steps like before (`timeline-settings` key).
- `key.set`: writes one Spline key (ANIM.md §1) on `track` (an animatable float parameter id
  or `<vec3 parameter>.<x|y|z>`, e.g. `transform.pos.x`) at `frame` (time `frame / fps`,
  any integer) with `value` in display units (angle tracks in degrees, percent as shown);
  an existing key at that time is replaced. One step "Set Key". Unknown node or track is
  `Failed`. Added for scripting (brief 016): `key.record` keys the *evaluated* value, so it
  cannot author different values at different frames.
- `transform.set`: sets only the given components of the object's pos/rot/scale in one step
  ("Set Transform"); it does not write animation keys (autokey is a gizmo/attribute
  panel feature; replaying a gizmo drag as a command comes with brief 012+).
- `object.set_axis`, `object.center_axis`, `object.align_axis` (brief 032): edit the axis (the
  transform) without moving the content, one transaction each (MANIPULATE.md §7). `snap.*` and
  `edit.axis_mode` change the editor's transform settings only (MANIPULATE.md §6, §7.4).
- `param.set`: converts `value` by the parameter kind: bool, integer, float (a `Unit::Angle`
  float is degrees, `Unit::Percent` is percent), `[x,y,z]` vec3 (angle vec3 in degrees)
  and color, string/enum id, link (node id or null), index list (array of integers). Mesh
  parameters are `Failed("unsupported")`. Numbers are clamped into the hard range by the document (CORE.md §2); a type
  mismatch or unknown parameter is `Failed` and changes nothing.

- `param.edit` (registered by forge-app, UI.md §12): the attribute manager's edit. Like
  `param.set` (display units, same conversion) but for several objects in one transaction
  ("Set <label>"); `nodes` absent means the selection; objects whose type lacks the
  parameter, or has it with another kind than the first object that has it, are skipped, and
  no match is `Failed`. It writes through the animation rules (autokey, rule 5, ANIM.md
  §2.4). `component` (0..2, vec3 only) makes `value` one number that replaces just that
  component of each object's current value. `gesture` merges edits with the same number and
  objects into one undo step (`drag:<ids>:<param>:<gesture>`).

## 7. Tests

`crates/command/tests/registry.rs`: every command has a unique well-formed id, a non-empty
label, an object schema, and its `sample_args` validate against that schema (a small
validator: required keys, no unknown keys, primitive types, enums, arrays) and parse into
the argument struct; duplicate ids are rejected; insertion order is kept; ten commands run
headlessly through a test `CommandContext` (create, rename, reparent, transform.set,
param.set, undo, redo, timeline.set_frame, select.mode, delete, plus errors), each one
undo step. `crates/ui/tests/*` run the same commands through `EditorState`.
