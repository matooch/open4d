# 016 · Rhai scripting: engine, bindings, `forge run`, Script Manager panel

Status: ready · Area: A · Depends on: 012 · Observations: – · Touches: script (new), apps/forge, ui

## Goal
Users and agents can write scripts that build and edit scenes. Scripts run from the CLI and from a Script Manager panel in the editor.

## Inputs
- Area design: [../../roadmap/areas/A-automation.md](../../roadmap/areas/A-automation.md)
- DECISIONS D5
- brief 012 (`App`), brief 011 (commands)

## Design decisions (planner)
- New crate `crates/script` (`forge-script`) with `rhai` (sync, no std features we don't need).
- Bindings: `run(id, #{args})` → command; `scene()` queries (`nodes()`, `node(id).param(name)`, `children`, `find(name)`); maths types `vec3`, `deg(x)`; `frame(n)`; `print`; a seeded `rand(seed)`.
- Safety: no file system access except `export_*` commands with explicit paths; max operations limit; a timeout.
- `forge run script.rhai [--scene in.forge] [--save out.forge]`.
- Script Manager panel: a code editor (egui `TextEdit` with monospace; syntax colouring optional), Run, an output console, and a recent scripts list stored in the settings.
- Example scripts in `examples/scripts/` (cube grid, animated sphere). They also serve as tests.

## Done when
- [ ] Spec `docs/specs/SCRIPTING.md` with the binding reference.
- [ ] Tests run every example script headlessly and check the resulting scene.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Rhai plugin packages (N/184), script nodes (H/108).

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
