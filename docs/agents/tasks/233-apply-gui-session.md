# 233 · Apply the owner's GUI session: key map, object manager rows, key indicators, window metrics

Status: done (parity-pending: G1 drop markers and evaluation-cell types, G2 sequence rules, G4 scrub modifiers) · Area: L · Depends on: 223, 229, 232 · Observations: GUI-SESSIONS.md "Owner's Cowork session, 2026-11-27" (G1, G2, G4, G5, G10) · Touches: ui, viewport

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

- **Spec first:** UI.md §6.1 (scroll, F1-F5 views, framing), §11 (sizes), §12 (number field, four key states, stacked tabs), §13 / §13.1 / §13.1a (rows, order, cycle, metrics, evaluation cell), §14 (observed default table with a source per row, replaced defaults, "no command yet" list, new §14.7 two-key sequences), §29 (new §29.1a measured sizes replacing the proportions), ANIM.md §2.7, VIEWPORT.md §9 (quad order observed). Rows cite "observed, G2 item 1" and so on; what the session left open says `parity-pending` with our choice.
- **Key map:** all G2 item 1 actions that have an argument-free command are bound (see UI.md §14.2). Two-key sequences: `Binding.then`, text `"M T"`, `KeyMap::step` (arm / run / foreign key = fresh press / Escape cancels / 1.5 s time-out), a prefix cannot also be a single key (conflict, prefix wins), bind-over replaces collisions, settings file keys `"M T"`, editor capture (second key within 1.5 s or the single key at the time-out), status bar shows the armed prefix.
- **No command yet (left unbound, pinned by a test):** Live Selection 9, Rectangle Selection 0, Undo View / Redo View (Mod+Shift+Z / Mod+Shift+Y), Render View Mod+R, Render to Picture Viewer Shift+R, Play backwards F6, Record Animated Alt+F9, Ungroup Shift+G, Line Cut K K, Loop/Path Cut K L, Plane Cut K J, Polygon Pen M E, Close Polygon Hole M D, and the other table entries without a command.
- **Rows:** 20 px, 12 px indent, 10 px boxed plus / minus with connectors and a stub, order gutter / name (type label stands in for the icon of brief 227) / layer glyph / two stacked dots / evaluation cell / tags, cycle default -> on -> off -> default (`Vis::next` in core), check only on non-nulls, active-camera target on cameras, selected names in the accent.
- **Indicator, fields, tabs, metrics:** `KeyState` with `Theme.key` (red) and `Theme.modified` (amber; hue gap tested); `widgets::number` with chevrons (one unit), double-click edit, Return commits, accesskit set / step; `TabSelection` stacking; `layout::WINDOW` / `regions` / `theme::BUTTONS`, default `Sizes` 402 / 0.47 / 92 (left width stays 150 until icons), `timeline::ruler_step`, status bar help text.
- **Tests changed (replaced defaults):** keymap.rs (default table, redo / Mod+Shift+Z, F2 rename, Mod+I, K, editor capture tests now settle the 1.5 s capture window), animate.rs (K / `.` / `,` / Shift+K), panes.rs (F4 -> F5), time.rs (M -> Shift+M), s3_chair.rs (Mod+Shift+Z -> Mod+Y), object_manager.rs (22 px / 18 px / 16 px numbers, the connector snapshot, the dot cycle, the tick tests now use a non-null), layout.rs ("Ready" label), ui.rs (window 1000 high: the taller command bar pushes the attribute rows down).
- **Tests added:** keymap.rs (observed keys run, unbound-until-a-command list, sequence text, arm and run, time-out and Escape, prefix conflicts, bind-over, settings syntax and round trip, kittest sequence in the window, editor capture); object_manager.rs (boxed disclosure, stacked dots, evaluation cell per type, accent name); attribute_manager.rs (chevron step and range, typed text, chevrons and double-click edit in the panel, four key states per theme, tab stacking pure and in the panel); layout.rs (regions at 1920 x 1080, default sizes and button metrics, bars in the window, status help, ruler every 5 frames).
