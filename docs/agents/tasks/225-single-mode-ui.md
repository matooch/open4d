# 225 · One place to switch model / point / edge / polygon mode

Status: ready · Area: L · Depends on: 147, 032 · Observations: owner notes (GUI-SESSIONS.md), G1/G2/G5 · Touches: ui

## Goal
Remove the duplicate component-mode interface, keeping one, placed where the reference has it (owner note N6, G1 item 8).

Sources: owner notes N1–N8 and GUI sessions G1/G2/G5 in [../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md), decision D22. Write the spec change in [../../specs/UI.md](../../specs/UI.md) first, citing the note (for example "owner note N1"). Where the exact layout is still open (G1 not yet answered), build it so the order and placement are data (a table in code or a TOML file), mark the spec rule `parity-pending G1`, and say so in the Report. Clean room: no screenshots, icons or layout files from the reference app.

## Steps
1. Find both interfaces (for example the tool palette and a toolbar or menu) and list them in the Report. Keep the one that matches N6 and G1 item 8. Until G1 is answered, keep the left tool palette's mode buttons and remove the other.
2. Keyboard bindings for the modes stay (key map).
3. Tests: exactly one widget per mode exists (a kittest query); mode switching still works from the palette and the keys.

## Done when
- [ ] Only one mode interface is left in the default layout; the tests pass.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
