# 228 · Clone toolset at the top level

Status: ready · Area: L · Depends on: 079, 227 · Observations: owner notes (GUI-SESSIONS.md), G1/G2/G5 · Touches: ui

## Goal
The Cloner and the clone toolset are reached from the top level of the UI (owner note N4, G1 item 9): a menu bar menu and a command bar group with a drop-down (Cloner first; then Matrix, Fracture, Voronoi Fracture, Tracer, the effectors, the fields), each with an icon.

Sources: owner notes N1–N8 and GUI sessions G1/G2/G5 in [../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md), decision D22. Write the spec change in [../../specs/UI.md](../../specs/UI.md) first, citing the note (for example "owner note N1"). Where the exact layout is still open (G1 not yet answered), build it so the order and placement are data (a table in code or a TOML file), mark the spec rule `parity-pending G1`, and say so in the Report. Clean room: no screenshots, icons or layout files from the reference app.

## Steps
1. UI.md §16.2: the top-level menu and the command bar group (with its order from G1 item 9 when answered, otherwise the order above, `parity-pending G1`).
2. "Cloner from selection" and the effector-with-selected-cloner behaviour (UI.md §16) work from the new entries.
3. Tests: the menu lists every registered clone-toolset type once; creating from the command bar group makes the same object as the Create menu.

## Done when
- [ ] One click from the command bar creates a Cloner; the menu lists the whole toolset.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
