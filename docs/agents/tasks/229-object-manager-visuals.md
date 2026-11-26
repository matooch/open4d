# 229 · Object manager hierarchy cues and row separation

Status: done (parity-pending: G1 item 7 numbers) · Area: L · Depends on: 149 · Observations: owner notes (GUI-SESSIONS.md), G1/G2/G5 · Touches: ui

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

- Spec: UI.md §13.1a (new; the numbers and rules below), §13.1 item 1 and §13.3 amended.
- Row geometry (ours, `parity-pending G1` item 7; all in `tree::METRICS` and pure functions): row height 22 px with no gap between rows (was about 18 px plus 3 px spacing); indent 18 px per level (was 14); fold arrow cell 16 px wide at the start of the row's own column, a painted triangle (right = collapsed, down = expanded), replacing the `+` / `-` text buttons (accessible labels now "Expand: <name>" / "Collapse: <name>"); connector lines 1 px: a trunk at `x = 18 k + 8` per ancestor level, an elbow per child (vertical to the row centre, or to the bottom when a later sibling exists, then a horizontal to the arrow cell, or 14 px into the cell for a leaf); 1 px separator along every row's bottom edge; odd rows shaded. Colours derive from theme tokens (`tree::row_colours`): guide = weak_text 75 % into panel (>= 3:1 on both row shades in dark, light and high-contrast, tested), separator = weak_text 18 %, alt = faint 55 %.
- Drag and drop: `tree::drop_mark` gives Into (whole row filled at 25 % accent and outlined) or a 2 px line on the top/bottom edge with a 3 px dot at the indent of the depth the nodes land at (one level deeper below an expanded parent = first child). Rows touching also fixed a latent gap where the pointer between rows fell through to "end of the root list".
- Per-type icon from brief 227: not present yet (227 is `ready`); the spec says its 16 px cell goes between the arrow and the name and does not change the numbers.
- Tests (crates/ui/tests/object_manager.rs): `row_geometry_numbers_are_fixed`, `connectors_follow_the_visible_depth_sequence`, `the_three_level_tree_draws_these_connector_lines` (the "snapshot": exact segment list of a three-level tree, text-compared rather than an image, so fonts and GPUs cannot change it), `drop_marks_tell_between_rows_from_inside_a_row`, `row_colours_stay_readable_in_every_theme`, and the kittest `children_are_nested_connected_and_rows_are_separated_in_the_panel` (names indent exactly 18 px per level, rows one row height apart, fold arrows only on parents and they fold). The existing 26 object manager tests (drag tests included) pass unchanged. 5 runs green; clippy clean.
