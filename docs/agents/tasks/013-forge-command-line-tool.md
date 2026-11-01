# 013 · `forge` command-line tool

Status: done · Area: A · Depends on: 012 · Observations: – · Touches: apps/forge (new), app

## Goal
A `forge` binary for headless work and agents: scene info, validation, evaluation, export, running commands and scripts, and talking to a running editor.

## Inputs
- Area design: [../../roadmap/areas/A-automation.md](../../roadmap/areas/A-automation.md)
- brief 012
- `crates/parity-harness/src/bin/golden.rs` (an existing small CLI to copy style from)

## Design decisions (planner)
- Binary `apps/forge` using `clap` (derive). Subcommands:
- - `new <path>`: an empty scene.
- - `info <scene> [--json]`: the node tree, types and key params.
- - `validate <scene>`: load report warnings; exit 1 on errors.
- - `eval <scene> --frame N [--node id] --export out.obj`.
- - `export <scene> --format obj --out path [--selected ids]`.
- - `cmd <scene> <command-id> [--args JSON] [--save]`: runs one command on the scene file and saves it.
- - `run <script.rhai> [--scene s]`: available once 016 exists; until then, a stub with a clear error.
- - `commands [--json]`: lists the registry with schemas.
- - `rpc <method> [--params JSON]`: talks to a running editor (needs 014; stub until then).
- - `render`: a stub until K.
- `--json` gives machine output on stdout and human messages on stderr. Exit codes: 0 ok, 1 user error, 2 scene load error, 3 internal error.
- The studio binary also accepts `--headless <forge args>` and forwards to the same code, so one installed binary does both.

## Done when
- [x] Spec `docs/specs/CLI.md` covers every subcommand, its JSON output shape and exit codes.
- [x] Tests in `apps/forge/tests/cli.rs` with `assert_cmd`: new → cmd object.create cube → eval → export → the OBJ matches the cube golden.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
RPC and script execution beyond stubs.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added: `apps/forge/tests/cli.rs` (5: new -> cmd object.create --save -> eval --export and `export`, both equal to `tests/golden/cube/defaults` under `compare_meshes`; info/validate/commands; animated cube exports the evaluated frame (x offset 0 vs 150 at frame 0 vs 15) and `info` lists the track; `cmd` without `--save` leaves the file unchanged; exit codes 1/2 incl. stubs, bad JSON, unknown node, missing/corrupt scene, `new` overwrite guard) and `crates/app/tests/app.rs::export_obj_command_writes_the_evaluated_frame`. No parity data of its own (N/A).
- Rules: spec docs/specs/CLI.md. Subcommands: `new <path> [--force]`, `info <scene>`, `validate <scene>`, `eval <scene> [--frame N] [--node id] [--export out.obj]`, `export <scene> --format obj --out path [--selected ids] [--frame N]`, `cmd <scene> <id> [--args JSON] [--save]`, `commands`, stubs `run`, `rpc`, `render`; global `--json`. Library `forge_cli::run` is shared by the `forge` binary and `forge-studio --headless <args>`.
- Planner decision applied: `file.export_obj` now exports the scene evaluated at the current frame, via a new `CommandContext::evaluated_scene()` (default: the stored scene; `App` and `EditorState` return the evaluated one). COMMANDS.md and APP.md updated; an un-animated scene exports exactly as before (all existing export tests pass unchanged).
- Deviations from the design decisions:
  - `validate`: a file that cannot be loaded is exit 2 (the load-error code); exit 1 is for a scene that loads but has a node whose generator rejects its parameters. Load warnings never fail.
  - `cmd` without `--save` is a dry run (file untouched); the brief's "saves it" is the `--save` flag.
  - `export`/`eval` take `--frame` (default 0, clamped); the brief's `export` had none. `new` refuses to overwrite without `--force` (addition).
  - `--headless` forwarding is not covered by an automated test (the studio binary is GUI); checked by hand.
  - Package is `forge-cli` (binary `forge`, library `forge_cli`) in `apps/forge`.
- Files changed: apps/forge/** (new), apps/studio/{src/main.rs,Cargo.toml}, Cargo.toml, Cargo.lock, crates/command/src/{context.rs,builtin/file.rs}, crates/app/{src/lib.rs,tests/app.rs}, crates/ui/src/commands.rs, docs/specs/{CLI.md (new),COMMANDS.md,APP.md}, this brief, docs/roadmap/BACKLOG.md.
- Open questions: none. Selection is not persisted in scene files, so `cmd` commands that default to the selection need explicit `nodes`.
