# 147 · Familiar default layout: command bar, tool palette, manager arrangement

Status: ready · Area: L · Depends on: 011 · Observations: G1 · Touches: ui

## Goal
The default window arrangement a user of the reference app expects: command bar on top, tool palette on the left, viewport centre, object manager top right, attributes bottom right, timeline at the bottom.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- GUI session G1 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G1
- docs/specs/UI.md
- crates/ui (egui_dock)

## Design decisions (planner)
- Layout defined in code (serialisable `Layout`). Panel order from G1 when answered; until then the arrangement above, marked parity-pending.
- Buttons call registry commands only.
- Never copy the reference layout files or icons.

## Done when
- [ ] egui_kittest snapshot of the default layout (our own baseline).
- [ ] Every button maps to a command id.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Icons (148).

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
