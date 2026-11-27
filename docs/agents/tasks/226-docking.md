# 226 · Dockable panels (egui_dock)

Status: ready · Area: L · Depends on: 147 · Observations: owner notes (GUI-SESSIONS.md), G1/G2/G5 · Touches: ui, apps/studio

## Goal
Real docking: drag a panel's tab to another area, split areas, stack tabs, float a panel, and save and restore layouts (owner note N5). `egui_dock` 0.22 resolves with egui 0.37 (D22).

Sources: owner notes N1–N8 and GUI sessions G1/G2/G5 in [../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md), decision D22. Write the spec change in [../../specs/UI.md](../../specs/UI.md) first, citing the note (for example "owner note N1"). Where the exact layout is still open (G1 not yet answered), build it so the order and placement are data (a table in code or a TOML file), mark the spec rule `parity-pending G1`, and say so in the Report. Clean room: no screenshots, icons or layout files from the reference app.

## Steps
1. UI.md §2: replace the six fixed areas with an `egui_dock` tree. Keep `Panel` ids. The default layout is the current §11 layout (`parity-pending G1` until the owner's sitting).
2. Layout persistence: save in the settings file, restore at start, and add Window > Reset layout. A saved layout with an unknown panel loads without it (warn).
3. Keep every existing panel test working. The viewport keeps its wgpu callback when it moves, splits or is resized.
4. `cargo deny`: egui_dock's licence (MIT) and duplicate-version check.
5. Tests: drag-to-dock through the `egui_dock` API, split, close and reopen, layout save/restore round trip, an old settings file without a layout.

## Done when
- [ ] Panels can be dragged, split, tabbed and floated in the editor; layouts persist.
- [ ] Every UI test still passes; `cargo deny` passes.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
