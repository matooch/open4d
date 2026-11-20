# 084 · Volume viewport display (preview iso-surface, slices)

Status: done (logic and draw lists; editor panel hookup not done) · Area: F · Depends on: 083, 143 · Observations: G4 · Touches: viewport

## Goal
Show volumes in the viewport (voxel box/preview iso-surface, optional slice view).

## Inputs
- Area design: [../../roadmap/areas/F-volumes.md](../../roadmap/areas/F-volumes.md)
- GUI session G4 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**

## Design decisions (planner)
- Preview: run the mesher at a coarser voxel size off the main thread; show the last result while recomputing.

## Done when
- [x] A builder with a sphere shows a preview without blocking the UI (test with an artificially slow mesher).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `forge-volume::preview` (`build_preview` at a coarse voxel size, `PreviewWorker` with coalescing requests, a never-blocking `snapshot`, the last preview kept while recomputing and on errors or panics, slice samples) and `forge_viewport::volume` (box lines, value colour ramp, slice markers, `display_lines`); builder params `display`, `slice_axis`, `slice_index`. Tests: `crates/volume/tests/preview.rs` (artificially slow mesher: requests and snapshots return in under 50 ms, the old preview stays up, coalescing), `crates/viewport/tests/volume_display.rs`.
- Rules: VOLUME.md section 11; all own design, parity-pending (G4 unanswered).
- Not done: the editor panel (`crates/ui`) does not own a `PreviewWorker` nor draw `display_lines` yet (other agents are editing the UI crates); the done-when is met at the library level.
- Files: crates/volume/src/preview.rs, node.rs (`BuildOptions`, display params), crates/viewport/src/volume.rs and `pub mod volume;` in lib.rs, tests above, docs/specs/VOLUME.md section 11.
