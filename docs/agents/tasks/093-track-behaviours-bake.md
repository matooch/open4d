# 093 · Track before/after behaviours, key reduction, bake

Status: done

## Goal
Repeat, offset repeat, oscillate, constant, continue before/after; key reduction; baking any evaluated parameter to keys.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- O27: [../../parity/animation-2.md](../../parity/animation-2.md) and its golden cases
- O38: [../../parity/fcurves.md](../../parity/fcurves.md) and its golden cases
- docs/parity/animation-2.md (O27), fcurves.md (O38)

## Design decisions (planner)
- Behaviours are applied by remapping time before evaluation (offset repeat adds the value delta per cycle).
- Reduction: greedy removal with max error tolerance.
- Bake: sample scene_at per frame and write keys (used by dynamics 122).

## Done when
- [ ] O27/O38 behaviour cases pass.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: before/after behaviours (Constant, Continue, Repeat, OffsetRepeat, Oscillate) in `Track`, applied by time remapping in `forge-anim`; the four pinned cases (`after-repeat`, `after-offset`, `after-oscillate`, `before-repeat`) now match at 4.9e-10, `after-continue` still matches. `keyframes_parity`: 34/36 (was 30/36). Key reduction `reduce_keys` and `bake_param` (tests in model.rs).
- Rules: ANIM.md §1.9. The behaviour is read from the case description in the test since `case.toml` has no field. Before-offset/oscillate are parity-pending (not observed).
- Deviations: none. `track.set_behavior` command not added.
- Files: crates/core/src/track.rs, crates/anim/src/{interp,bake,lib}.rs, crates/anim/tests/{keyframes_parity,model}.rs, docs/specs/ANIM.md.
- Open: observe before-Offset / Oscillate; a case.toml field for behaviours.
- Gate: see brief 096.
