# 013 · `forge` command-line tool

Status: ready · Area: A · Depends on: 012 · Observations: – · Touches: apps/forge (new), app

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
- [ ] Spec `docs/specs/CLI.md` covers every subcommand, its JSON output shape and exit codes.
- [ ] Tests in `apps/forge/tests/cli.rs` with `assert_cmd`: new → cmd object.create cube → eval → export → the OBJ matches the cube golden.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
RPC and script execution beyond stubs.

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
