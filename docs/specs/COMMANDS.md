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
| `edit.undo` | - | `{done}` | no |
| `edit.redo` | - | `{done}` | no |
| `view.shading` | `mode` smooth, flat, wire, smooth_wire, flat_wire | null | no |
| `view.frame_all` | - | null | no |
| `view.toggle_expanded` | `node` | null | no |
| `mesh.make_editable` | `nodes?` | `{made, skipped}` | yes |
| `mesh.current_state_to_object` | `nodes?` | `{made, skipped}` | yes |
| `mesh.extrude` `mesh.extrude_inner` `mesh.bevel` | `node?`, `polys?`, `settings?`; bevel also `edges?`, `points?` (MODELING.md §0.1, §3) | `{node, points, polys, selected}` | yes |
| `mesh.subdivide` `mesh.triangulate` `mesh.untriangulate` `mesh.reverse_normals` `mesh.optimize` `mesh.weld` `mesh.melt` `mesh.dissolve` `mesh.split` `mesh.disconnect` `mesh.connect_points` `mesh.mirror` `mesh.center_axis` | `node?`, `polys?`, `settings?`; weld, dissolve and connect_points also `edges?`, `points?` (MODELING.md §0.1, §4) | `{node, points, polys, selected}`; split adds `created`, center_axis adds `center` | yes |
| `mesh.loop_cut` | `node?`, `edge` (id or point pair), `offsets?` or `count?` (MODELING.md §5.1) | `{node, points, polys, cut_edges}` | yes |
| `mesh.knife` | `node?`, `mode` line / plane / loop, `polys?`, `point` `normal` / `from` `to` `eye`\|`direction` `visible_only?` / `edge` `offset?` (MODELING.md §5.2) | `{node, points, polys, cut_edges}` | yes |
| `mesh.slide` | `node?`, `edges?`, `points?`, `settings?: {amount, toward}` (MODELING.md §5.3) | `{node, points, polys, selected}` | yes |
| `select.mode` | `mode` model, points, edges, polys | null | no |
| `select.all` `select.none` `select.invert` `select.grow` `select.shrink` `select.connected` | - | null | yes |
| `select.convert` | `to` points, edges, polys; `mode?` strict, tolerant (EDITABLE.md §2.8) | null | yes |
| `select.loop` `select.ring` | `across?` 0 or 1 (EDITABLE.md §2.8) | null | yes |
| `file.new` | - | null | no |
| `file.open` | `path` | `{warnings}` | no |
| `file.save` | `path?` (default: current file) | `{path}` | no |
| `file.save_as` | `path` | `{path}` | no |
| `file.import_obj` | `path` | `{nodes}` | yes |
| `file.export_obj` | `path`, `selected?` | `{objects}` | no |
| `key.record` | - | `{keys}` | yes |
| `key.delete_at_frame` | - | `{keys}` | yes |
| `key.set` | `node`, `track`, `frame`, `value` | null | yes |
| `key.jump` | `forward` | `{frame}` | no |
| `key.autokey` | `enabled?` (absent toggles) | `{enabled}` | no |
| `timeline.set_frame` | `frame` | `{frame}` | no |
| `timeline.step` | `delta` | `{frame}` | no |
| `timeline.play` / `timeline.stop` | - | null | no |
| `timeline.settings` | `fps`, `start`, `end` | null | yes |
| `transform.set` | `node`, `pos?`, `rot?` (degrees), `scale?` | null | yes |
| `param.set` | `node`, `param`, `value` | null | yes |

Rules (behaviour moved unchanged from the former `Command` enum, UI.md §5 and ANIM.md §2):

- `object.create`: see UI.md §5 (a Tag type attaches to the active node, errors without
  one; a Generator also gets a smoothing tag in the same step; the new node becomes the
  only selected one). Unknown type is `Failed`.
- `object.delete`: skips nodes inside another selected node's subtree; clears the selection.
- `edit.undo` / `edit.redo`: then `retain_existing` and clamp the timeline. `done` is false
  when there was nothing to undo.
- `mesh.*`: EDITABLE.md §1; skipped nodes (no mesh) are listed in `skipped` and in the
  status text. Empty selection is `Failed`.
- `select.*`: EDITABLE.md §2.5 (model mode selects objects).
- `file.*`: IO.md §2. `file.new`/`file.open` replace the document and reset selection,
  timeline and expansion; the dock layout stays. Errors read `Open failed: ...`,
  `Save failed: ...`, `Import failed: ...`; an OBJ import without polygons fails with
  `Import: no polygons found`; an export of nothing fails with `Nothing to export`.
  Dialogs and "save changes?" prompts stay in the editor: it picks a path, then runs
  the command. `file.export_obj` writes the scene **evaluated at the current frame**
  (`CommandContext::evaluated_scene`), like `App::export_obj`; an un-animated scene exports
  exactly its stored geometry.
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
- `param.set`: converts `value` by the parameter kind: bool, integer, float (a `Unit::Angle`
  float is degrees, `Unit::Percent` is percent), `[x,y,z]` vec3 (angle vec3 in degrees)
  and color, string/enum id, link (node id or null), index list (array of integers). Mesh
  parameters are `Failed("unsupported")`. Numbers are clamped into the hard range by the document (CORE.md §2); a type
  mismatch or unknown parameter is `Failed` and changes nothing.

## 7. Tests

`crates/command/tests/registry.rs`: every command has a unique well-formed id, a non-empty
label, an object schema, and its `sample_args` validate against that schema (a small
validator: required keys, no unknown keys, primitive types, enums, arrays) and parse into
the argument struct; duplicate ids are rejected; insertion order is kept; ten commands run
headlessly through a test `CommandContext` (create, rename, reparent, transform.set,
param.set, undo, redo, timeline.set_frame, select.mode, delete, plus errors), each one
undo step. `crates/ui/tests/*` run the same commands through `EditorState`.
