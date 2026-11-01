# Headless application core (`crates/app`, package `forge-app`)

Source: our own design (docs/roadmap/areas/A-automation.md, DECISIONS D3); no reference
observation is involved. Nothing here is **parity-pending**.

`forge-app` is the application without a window: it loads and saves scenes, runs
commands, evaluates any frame, and exports OBJ. It depends on no egui, wgpu or viewport
crate, so it runs on a machine without a GPU. The editor (`forge-ui`) and every
automation front end (CLI, RPC, MCP, scripting; briefs 013-016) are built on it.

## 1. `App`

```rust
pub struct App {
    pub doc: Document,                    // the scene, undo history (registry: doc.scene().registry())
    pub commands: Arc<CommandRegistry>,   // every action (COMMANDS.md)
    pub selection: NodeSelection,         // object selection (UI.md §1)
    pub mode: EditMode,                   // Model | Points | Edges | Polys (EDITABLE.md §2.2)
    pub timeline: Timeline,               // current frame and playback; fps/range are scene settings
    pub autokey: bool,
    pub eval: Evaluated,                  // cache of the scene animated to a frame
    pub status: String,                   // last message (errors, load warnings)
    pub file_path: Option<String>,        // last saved or loaded file
    pub saved_revision: u64,              // scene revision at the last save or load (dirty marker)
}
```

`App` implements `forge_command::CommandContext`. View-only hooks (`set_shading`,
`frame_all`, `toggle_expanded`) keep their no-op defaults. Component selection, key
recording and key navigation live in `forge-app` (modules `select`, `animate`) because the
headless commands need them; `forge-ui` re-exports them (`components`, `animate`).

## 2. API

- `default_registry() -> Result<Registry, RegistryError>`: the built-in node types plus
  every plugin the editor installs (`Primitives`, `Deformers`). The editor, the CLI and
  tests share it; `apps/studio` no longer lists plugins.
- `default_command_registry() -> CommandRegistry`: the built-in editor commands.
- `App::new(Arc<Registry>) -> App`, `App::new_default() -> App` (default registry).
- `run(id, args: serde_json::Value) -> Result<Value, CommandError>`: runs one command
  through the registry (one undo step if it edits; COMMANDS.md §4).
- `open(path)`, `save(path)`: `file.open` / `file.save_as` with explicit paths.
- `frame(n) -> Arc<Scene>`: sets the current frame (clamped to the project range) and
  returns the evaluated scene; `evaluated()` returns the scene at the current frame.
- `eval_mesh(node, frame) -> Result<PolyMesh, AppError>`: the node's mesh with animation
  at `frame` and deformers applied, in **world space** (the node's world matrix applied
  to the points). Errors: `UnknownNode`, `NoGeometry` (the node produces no mesh),
  `Build(msg)` (the generator rejected its parameters).
- `export_obj(path, roots: Option<&[NodeId]>) -> Result<usize, AppError>`: writes the
  scene **evaluated at the current frame** (the whole scene, or the listed nodes and their
  descendants) as OBJ and returns the object count. `AppError::NothingToExport` if empty.
  (The `file.export_obj` command exports the stored, un-animated scene.)

Frames are integers; time is `frame / fps` (ANIM.md §2). `eval_mesh` does not change the
timeline.

## 3. Editor

`EditorState` holds `app: App` plus the UI-only state: dock, camera, shading, expanded
rows. It dereferences to `App`, so `state.doc`, `state.selection`, `state.timeline` and
the rest keep working. Its `CommandContext` forwards to `App`, and adds the view hooks.
`File > New` resets the `App` (keeping the command set) and the view state except the dock.

## 4. Tests (`crates/app/tests/app.rs`)

- Open a saved scene file and read it back.
- Run `object.create` and `param.set` through `App::run`; undo reverts each in one step.
- Evaluate frame 15 of an animated cube (keys on `transform.pos.x`) with `eval_mesh` and
  check the world-space points; the timeline is unchanged.
- `export_obj` at a frame, parsed back with `forge_parity_harness::parse_obj`; the points
  equal `eval_mesh`.
- Headless component selection (`select.all` in Points mode) and key recording.
- `App::new_default` registers primitives and deformers; the registry matches the editor's.
