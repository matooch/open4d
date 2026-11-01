# 011 · Command registry and migration of the editor commands

Status: ready · Area: A · Depends on: – · Observations: – · Touches: command (new), ui, core

## Goal
Every user action becomes a named, typed, undoable command in one registry, and the editor runs all its actions through it. This is the base for the CLI, RPC, MCP and scripting (DECISIONS D3).

## Inputs
- Area design: [../../roadmap/areas/A-automation.md](../../roadmap/areas/A-automation.md)
- `crates/ui/src/commands.rs` (today's `Command` enum and `execute`), `crates/core/src/{document,registry}.rs`
- docs/specs/UI.md §5, CORE.md §2, §4

## Design decisions (planner)
- New crate `crates/command` (`forge-command`): `trait CommandHandler { fn id(&self) -> &str; fn label(&self) -> &str; fn args_schema(&self) -> serde_json::Value; fn run(&self, ctx: &mut dyn CommandContext, args: serde_json::Value) -> Result<serde_json::Value, CommandError>; fn undoable(&self) -> bool }`. `CommandRegistry` keeps handlers in insertion order. `Plugin` gains `register_commands` (default: none).
- `CommandContext` is a trait with access to the `Document`, selection, edit mode and timeline. It is implemented by `EditorState` (ui) and later by `forge-app` (headless), so commands never depend on egui.
- Arguments are typed with serde structs plus `schemars` for the schema. Ids are dotted lowercase: `object.create {type}`, `object.delete`, `object.rename`, `object.reparent`, `edit.undo`, `edit.redo`, `view.shading`, `view.frame_all`, `mesh.make_editable`, `mesh.current_state_to_object`, `select.mode`, `select.all|none|invert|grow|shrink`, `file.new|open|save|save_as|import_obj|export_obj`, `key.record`, `key.delete_at_frame`, `timeline.set_frame|play|stop|settings`, `transform.set` (an object's pos/rot/scale), `param.set {node, param, value}`.
- Migrate every variant of the existing `Command` enum and every menu item and shortcut to registry commands. The enum may stay as a thin wrapper while you migrate, but it must be gone by the end.
- Undo: a command that edits runs inside one `Document::transact` whose label is the command label, so every command is one undo step, as briefs 03–06 guarantee today.
- A test enumerates the registry and checks that every command has a unique id, a label and a valid JSON schema, and that a sample invocation of each command validates against the schema.

## Done when
- [ ] Spec `docs/specs/COMMANDS.md`: the trait, id naming rules, the argument conventions (units: degrees in arguments, radians inside; percent as shown) and the list of commands.
- [ ] Every existing UI test still passes. New tests cover the registry invariants and run 10 representative commands headlessly through `CommandContext`.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
The CLI, RPC and scripting front ends (briefs 013–016).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added; parity N/M (`UNSUPPORTED` / `KNOWN_MISMATCH` with reasons).
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
