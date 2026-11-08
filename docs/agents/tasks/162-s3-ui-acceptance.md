# 162 · S3 acceptance test and UI docs

Status: done · Area: L · Depends on: 147–153 · Observations: – · Touches: tests, docs

## Goal
S3 passes on all three OS; UI chapter in the manual.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- docs/roadmap/README.md (S3)

## Design decisions (planner)
- egui_kittest scenario script.

## Done when
- [x] S3 passes.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added: `crates/ui/tests/s3_chair.rs` (`model_a_chair_save_reopen_and_export`, spec UI.md §28). No parity numbers change (no GPU image check, so no tolerance).
- S3 as driven: Create > Cube (menu click), size/position by `param.set`/`transform.set`, Edit > Make Editable, Mesh > Extrude Inner / Extrude (leg), Mesh > Bevel on an edge (Alt+3), Mesh > Loop/Path Cut (hover + click), Ctrl+Z / Ctrl+Shift+Z, Symmetry and Subdivision nested in the object tree, evaluated mesh checks (mirrored about x = 0, subdivided, inside the cage), Ctrl+S, Ctrl+O in a second window (bytes and evaluated mesh identical), File > Export OBJ... with the options window, OBJ face count equals the evaluated polygons, import back, Ctrl+Z to an empty scene. Temp dir only; no plug-in or user dirs touched.
- Docs: `docs/manual/user-interface.md` (layout, managers, keys, navigation, themes) and `docs/manual/tutorial-chair.md`, both in SUMMARY.md; the manual structure tests pass.
- Parity-pending (no observation): menu arrangement and command bar/palette order (G1), key chords and navigation gestures (G2), attribute manager look and scrub modifiers (G4); the pointer-picking part of S3 is replaced by setting the stored selection (viewport picking needs the GPU, picking look is G8).
- Deviations: Symmetry and Subdivision are created with `object.create` instead of the Create menu. The Create > Generator submenu is longer than an 800 px window and does not scroll, so those entries are unreachable by pointer (found while writing the test; follow-up: scrollable or grouped Create submenus). The manual tells readers about the palette workaround.
- Files changed: crates/ui/tests/s3_chair.rs, docs/specs/UI.md, docs/manual/{user-interface,tutorial-chair}.md, docs/manual/SUMMARY.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions: G1 (menu and layout), G2 (keys), G4 (widgets) would let the labels and chords in the test and manual be confirmed. The "all three OS" part of S3 runs in CI (brief 173-176); only Linux was run here.
- Gate: `GATE: 1963 tests passed, 0 failed` ... `GATE: all passed` (clippy -D warnings clean).
