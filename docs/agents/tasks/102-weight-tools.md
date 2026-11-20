# 102 · Weight tools: paint brush, weight manager, normalise/mirror/smooth/prune

Status: done (parity-pending G6) · Area: H · Depends on: 101, 150 · Observations: G6 · Touches: rig, ui, viewport

## Goal
Paint weights interactively and edit them in a weight manager.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- GUI session G6 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G6

## Design decisions (planner)
- Brush modes add/subtract/set/smooth with strength and falloff; each stroke is one undoable command. Tools are commands usable from the CLI.

## Done when
- [x] Unit tests for normalise/mirror/smooth/prune; egui_kittest for a paint stroke.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 8 rig unit tests, 3 command tests, 3 egui_kittest tests; no parity data (G6 unanswered).
- Rules: RIG.md section 9 (brush modes, falloff, normalise rule, smooth); all parity-pending.
- Files: crates/rig/src/paint.rs, crates/app/src/weight_commands.rs, crates/ui/src/weight_manager.rs, dock/app/state hooks, tests, RIG.md, reference docs.
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
