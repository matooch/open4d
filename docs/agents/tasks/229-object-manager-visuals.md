# 229 · Object manager hierarchy cues and row separation

Status: ready · Area: L · Depends on: 149 · Observations: owner notes (GUI-SESSIONS.md), G1/G2/G5 · Touches: ui

## Goal
Make the tree readable: clear parent/child cues (indent, lines joining parent and children, fold arrows) and more separation between rows (owner note N7, G1 item 7).

Sources: owner notes N1–N8 and GUI sessions G1/G2/G5 in [../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md), decision D22. Write the spec change in [../../specs/UI.md](../../specs/UI.md) first, citing the note (for example "owner note N1"). Where the exact layout is still open (G1 not yet answered), build it so the order and placement are data (a table in code or a TOML file), mark the spec rule `parity-pending G1`, and say so in the Report. Clean room: no screenshots, icons or layout files from the reference app.

## Steps
1. UI.md §13: row geometry (row height, indent per level, connector lines, fold arrow, per-type icon from 227, alternating row shade or separator lines), exact numbers from G1 item 7 when answered, otherwise ours, `parity-pending G1`. Both themes.
2. Drag and drop shows the drop position (between rows versus onto a row as a child) clearly.
3. Pure functions for the geometry (`tree::row_layout`), unit-tested; one kittest snapshot of a three-level tree.

## Done when
- [ ] Children are visibly nested and connected; rows are separated; the tests pass.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
