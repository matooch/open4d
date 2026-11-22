# 106 · Tags: Target, Align to Spline, Vibrate, Look at Camera, Protection, Display, Compositing

Status: done · Area: H · Depends on: 088 · Observations: O27, O54 · Touches: core, rig

## Goal
The common expression and display tags.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- O27: [../../parity/animation-2.md](../../parity/animation-2.md) and its golden cases
- O54: [../../parity/tags-2.md](../../parity/tags-2.md) and its golden cases
- docs/parity/animation-2.md (O27), tags-2.md (O54)

## Design decisions (planner)
- Expression tags run in the tag phase of evaluation (111). Vibrate is seeded noise: statistical if the generator is not reproduced.
- Display/Compositing affect only viewport/render; O54 says whether OBJ export changes.

## Done when
- [x] O27/O54 tag cases pass.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- **Result:** 7/8 exact, 8/8 passing. `vibrate-tag` checks repeatability, bounded offsets
  and nontrivial movement instead of the exact reference noise. No unsupported recorded cases.
  Three app tests also check relative pulses, parented path placement and partial position locks.
- **Rules:** `target-tag` uses exact-time eased keys and aims +Z with zero bank.
  Both `align-to-spline` cases use collapsed-handle keys and a 50-chord-per-cubic length
  table; tangential orientation uses the cubic derivative. Worst error across those
  three cases is below `1e-12`. O54 pins camera aim and lock behavior plus unchanged OBJ
  output for Display and Compositing.
- **Placement:** the app registers and evaluates the tags, so the rig crate need not depend
  on generators. The rig crate exposes position locks to the transform command.
- **Files:** `crates/app/src/motion_tags.rs`, the app scheduler and registry,
  `crates/app/tests/motion_tags_parity.rs`, spline sampling, animation evaluation,
  the transform command, rig lock queries and editor icon mappings.
- **Follow-up:** viewport Display and render Compositing effects are not implemented;
  observe those directly along with non-default cameras, paths and unobserved tag options.
- **Gate:** 2431 Rust tests passed, 0 failed; 38 Python tests passed; cleanroom OK;
  golden check OK (1117 cases). `cargo-deny` was not installed. `GATE: all passed`.
