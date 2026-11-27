# 233 · Apply the owner's GUI session: key map, object manager rows, key indicators, window metrics

Status: in-progress (planner subagent) · Area: L · Depends on: 223, 229, 232 · Observations: GUI-SESSIONS.md "Owner's Cowork session, 2026-11-27" (G1, G2, G4, G5, G10) · Touches: ui, viewport

## Goal
The owner's GUI session answered G1, G2, G4, G5 and G10 with measured values. Replace our
`parity-pending` choices with the observed ones, cite each rule as "observed, G<n> item <k>", and
keep what the session left open as `parity-pending`.

## Steps
1. **Key map defaults (G2 item 1; UI.md §14.2):**
   - **Tools and selection:** Live Selection `9`, Rectangle Selection `0`, Move/Rotate/Scale `E`/`R`/`T` (already done).
   - **Undo:** `Mod+Z` / `Mod+Y`. Undo View / Redo View: `Mod+Shift+Z` / `Mod+Shift+Y`.
   - **Framing:** Frame geometry `H`, frame selected elements `S`, frame selected objects `O`.
   - **Views:** `F1` Perspective, `F2` Top, `F3` Right, `F4` Front, `F5` all views. The four-view panes are
     Perspective upper left, Top upper right, Right lower left, Front lower right.
   - **Rendering:** Render View `Mod+R`, Render to Picture Viewer `Shift+R`.
   - **Playback and recording:** Play forward / back / stop `F8` / `F6` / `F7`, Record `F9`, Autokey `Mod+F9`, Record
     animated `Alt+F9`. Next / previous frame `G` / `F`; next / previous key `Mod+G` / `Mod+F`.
   - **Object commands:** Make Editable `C`, Group `Alt+G`, Ungroup `Shift+G`, Delete `Backspace` and `Delete`, Select
     All / Deselect All `Mod+A` / `Mod+Shift+A`.
   - **Two-key sequences** (new in the key map: a first key arms a prefix, the second runs the command;
     the prefix times out or is cancelled by Escape; spec it):
     - Mesh: `M T` Extrude, `M W` Inset, `M S` Bevel, `M B` Bridge, and the other `M`-prefixed entries in the G10 menu table.
     - Cuts: `K K` Line Cut (knife), `K L` Loop/Path Cut, `K J` Plane Cut.
     - Selection and topology: `U S` Subdivide, `U O` Optimize, `U I` Invert, `U Y` Grow, `U K` Shrink, and the other `U`-prefixed
       entries in that table.
   - **Conflicts to move:** our `F2` rename and anything else on these keys. Rename moves to the object
     manager's double-click / menu entry (`parity-pending G1`).
   - **Modes:** the reference has no default mode shortcuts. Keep Alt+1..4 as an extra and say so in the spec.
   - **Shading:** still not observed. It stays on `Mod+Shift+digits`.
2. **Scroll zoom direction (G2 item 2):** scroll up zooms in. Check ours matches.
3. **Object manager rows (G1 items 2 and 7, UI.md §13.1a):**
   - **Geometry:** row pitch 20 px, child indent about 12 px.
   - **Disclosure:** a boxed minus (expanded) or boxed plus (collapsed) in the gutter, with a thin right-angle connector to the children.
   - **Selected rows:** names are drawn in the accent highlight (the reference uses amber; ours is our selection colour, D22).
   - **Row order:** gutter, type icon, name, layer glyph, two stacked visibility dots (upper = editor,
     lower = render; a click cycles default → on → off → default), the evaluation check (tick ↔ red X,
     only on types that have it; cameras show an active-camera target instead; nulls have none), then tags.
4. **Key indicators (G4 item 2, UI.md §12):** a diamond with four states:
   - hollow grey: no track;
   - solid key colour: a key at this frame;
   - solid *modified* colour: an unkeyed draft (232);
   - hollow key colour: animated, no key here.

   Our palette picks the colours (D22), but the key colour and the modified colour must be distinct hues.
5. **Number fields (G4 item 1):** left / right chevrons step one unit; double-click edits the text;
   Return commits. Scrub modifiers stay `parity-pending G4`.
6. **Attribute tabs (G1 item 3):** Shift-click a second tab shows both tab bodies stacked in one
   scroll area, the first-active one on top.
7. **Window metrics (G10 item 1, UI.md §29):** at 1920 × 1080 (1×):
   - **Rows:** title bar 28 px, document/layout row 24, command bar 42, viewport menu row 20, timeline 92, status bar 24.
   - **Columns:** left tool strip 43 px, object palette 43 px, managers column 402 px, split near 47 % of its height.
   - **Buttons:** command bar about 27 × 26, palette buttons 34 × 34.

   Record these in §29 (replacing the proportions) and make them the default layout's sizes and the
   theme's button metrics. The timeline ruler labels every 5 frames at that width (G10 A7). The status bar
   shows the active tool's help text (G10 A8).
8. **Tests:**
   - every new default binding;
   - the sequence prefix: arm, run, timeout, Escape, no conflict with single keys;
   - F2 is no longer rename;
   - the row metrics (20 / 12);
   - the boxed disclosure;
   - the visibility-dot cycle;
   - the four indicator states;
   - the chevron step;
   - Shift-click tab stacking;
   - the default layout sizes.

## Done when
- [ ] The key map, rows, indicators, chevrons, tab stacking and layout sizes follow the session, each cited in the spec.
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
