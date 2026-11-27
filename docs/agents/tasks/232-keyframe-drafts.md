# 232 · Keyframe drafts from every edit route, shown in the viewport

Status: in-progress (planner subagent) · Area: L, G · Depends on: 224, 097, 151 · Observations: owner note N10 (GUI-SESSIONS.md) · Touches: ui, app, anim, viewport

## Goal
On an animated property, with Auto-Key off, any edit (gizmo drag in the viewport, the Coordinates
panel, the inspector) changes the object **visibly** in the viewport and the inspector and marks
that property's key button as *modified*, but nothing is written to the animation until the user
clicks the key button (owner note N10). Today drafts exist only in the inspector (UI.md §12 "With
Auto-Key off …"), the viewport does not show them, and the gizmo and Coordinates paths write
straight through.

## Steps
1. **Spec:** UI.md §12 and ANIM.md, citing N10:
   - **One draft layer** per document view (`(node, param) → value`), sitting on top of the evaluated
     animation for display, the gizmo and the inspector. It is never saved, rendered or exported.
   - **When an edit becomes a draft:** a property with ≥ 1 key track, Auto-Key off. A property with no
     track still edits the document directly, as today. With Auto-Key on, the edit keys immediately,
     as today.
   - **Committing:** clicking the key button commits that row's draft as a key at the current frame
     (one undo step; vector rows commit all three components). "Key all" / Record commits every draft
     of the selection.
   - **Discarding** (parity-pending G5): changing the frame, undo, a document edit to the same param,
     or Escape in the viewport. The animated value returns.
   - **Display:** the key button shows the *modified* state (§12 indicator). The viewport shows the
     drafted pose. The object manager or HUD may show a small "modified" hint *(open)*.
2. Route the gizmo (MANIPULATE.md) and the Coordinates manager (§20) through the draft layer for
   tracked properties. Multi-selection drafts per object.
3. **Evaluation:** `App::evaluated` (or the viewport's scene build) applies the draft layer after
   animation. Behaviours and expressions see the drafted value while the user looks at it *(open:
   state it)*. Rendering and export ignore drafts.
4. **Tests:**
   - A gizmo move on a keyed object leaves the track unchanged, moves the object in the evaluated
     view, and marks the key button modified.
   - Clicking the key writes the key; undo removes it.
   - A frame change discards the draft.
   - A property without tracks still edits the document.
   - Auto-Key on still keys immediately.
   - The Coordinates panel takes the same path.
   - Render, export and save ignore drafts.

## Done when
- [ ] Move a keyed object with the gizmo: it moves, the key button shows modified, the track is unchanged; click the key and the key is stored (egui_kittest and app tests).
- [ ] The owner confirms it matches the reference (Report).
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
