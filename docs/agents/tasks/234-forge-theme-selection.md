# 234 · Forge theme: mint state accent, amber selection everywhere

Status: ready (start after 233) · Area: L, K · Depends on: 233 · Observations: owner note N11 (GUI-SESSIONS.md); design reference: branch `design/forge3d-workspace`, `docs/specs/WORKSPACE-DESIGN.md` (palette and viewport sections only) · Touches: ui, viewport

## Goal
Owner note N11: adopt the proposal's palette. **Mint** marks active tools, modes, toggles, the active
workspace tab and the playhead. **Amber** marks selection everywhere: selected objects get an amber
outline/wireframe in the viewport, and selected rows, keys and so on use the same amber, so there is a
clear hierarchy of what is selected. **Red** marks recording and errors. The layout stays the
reference's (D22, UI.md §29, brief 233); only colour roles and the selection display change.

## Steps
1. **Spec (UI.md §24 themes, §29.2):** replace the single `accent` with role tokens.
   - **Dark theme values**, from the proposal:
     - window `#171B21`, panel `#20262E`
     - viewport `#252C35`, input `#181D24`
     - divider `#343D49`
     - text `#E8EDF2` / `#A7B2C0`
     - active/state (mint) `#73D6BC`
     - selection (amber) `#F3B36B`
     - recording/error `#F17A80`
   - **Light and high-contrast:** derive each role so text reaches 4.5:1 and states read in greyscale
     (shape, outline or text as well as colour).
   - **Key diamond (232/233):** key-here, key-elsewhere and no-track use the time/state colours. The *modified*
     state keeps its own distinct hue (not amber, so it doesn't read as "selected"; state the choice).
2. **Role audit:** every use of the old accent in `crates/ui` and `crates/viewport` maps to one role
   (tool/state, selection, time, recording, focus). A test lists widgets per role.
3. **Viewport selection display (VIEWPORT.md):**
   - **Look:** selected objects get an amber silhouette outline (screen-space, a few px, drawn over
     shaded modes) and, in wireframe or "lines" display, amber edges. The active object is brighter or
     thicker than the rest of the selection. Component selection (points, edges, polygons) stays distinct.
   - **Coverage:** it must work with instancing (clones), picking, every display mode and the real-time Render mode.
   - **Cost:** profile the extra pass and record it in PERFORMANCE.md.
4. **Same amber elsewhere:**
   - object manager: selected names and rows (233's rows);
   - selected keys in the timeline, dope sheet and F-curve;
   - the selected gizmo axis keeps its own axis colours.
5. **Grid:** fade minor lines with distance and depth-test the grid against opaque geometry (from
   the proposal; VIEWPORT.md).
6. **Tests:**
   - token contrast per theme;
   - every accent use has a role;
   - a viewport readback where the selected object's outline pixels are amber and an unselected
     object's are not, on lavapipe;
   - instanced selection;
   - grid occlusion.

## Done when
- [ ] Mint/amber/red roles in all three themes, with no plain `accent` left; amber selection outline in the viewport and amber selection in the managers and editors.
- [ ] The owner checks the look in a Mac build (Report).
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
