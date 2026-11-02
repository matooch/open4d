# 079 · Clone UI: effector lists, clone display, GPU instancing hookup

Status: ready · Area: E · Depends on: 065, 144 · Observations: G4 · Touches: ui, viewport

## Goal
The cloner attribute UI (effector list with drag/drop and enable toggles), clone index display, and viewport instancing through K/144.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- GUI session G4 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G4 (list widgets)

## Design decisions (planner)
- Effector list is a link-list widget (shared with field lists, 150).
- Viewport draws clones with an instance buffer (matrices + colour) from 144; no geometry expansion for display.

## Done when
- [ ] egui_kittest test: add/reorder/disable an effector through the UI; viewport shows 10 000 clones at interactive rate on the CI software adapter.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

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
