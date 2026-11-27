# 227 · Icon set and reference-style menus, command bar and tool palette

Status: ready · Area: L · Depends on: 147, 225 · Observations: owner notes (GUI-SESSIONS.md), G1/G2/G5 · Touches: ui, packaging

## Goal
Buttons show icons for their action and sit where the reference puts them (owner note N8, G1 items 1, 4, 5).

Sources: owner notes N1–N8 and GUI sessions G1/G2/G5 in [../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md), decision D22. Write the spec change in [../../specs/UI.md](../../specs/UI.md) first, citing the note (for example "owner note N1"). Where the exact layout is still open (G1 not yet answered), build it so the order and placement are data (a table in code or a TOML file), mark the spec rule `parity-pending G1`, and say so in the Report. Clean room: no screenshots, icons or layout files from the reference app.

## Steps
1. **Icons:** an original SVG icon set in `crates/ui/icons/` (one consistent style, 16 and 24 px, light and dark themes), rendered through `egui_extras` with SVG (resvg). Generic glyphs (folder, trash, eye, lock) may come from a permissively licensed icon library (record its licence in `deny.toml` and the release notices). Never trace or copy the reference's icons. Each command in the registry gets an optional icon id, and the command bar, palette and menus show it.
2. **Layout as data:** the menu bar, command bar groups and tool palette order live in one table (`crates/ui/layout/default.toml`), so G1's answers are a data edit. Fill it from G1 where answered, otherwise keep the current order marked `parity-pending G1`.
3. Tooltips: the command label plus its shortcut.
4. Tests: every command-bar and palette entry has an icon; the layout table loads; an unknown command id in the table is an error listing it; snapshot test of the command bar at 1x/2x (our own art only).

## Done when
- [ ] Command bar and tool palette show icons with tooltips; menus and groups come from the layout table.
- [ ] The owner checks the layout against the reference (Report).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
