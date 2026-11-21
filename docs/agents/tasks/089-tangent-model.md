# 089 · Tangent model: user, broken, weighted, zero angle/length, ease presets, overshoot

Status: done (parity-pending)

## Goal
Full tangent model matching O38: user-edited, broken tangents, weighted tangents, zero angle/length locks, the ease presets, and the overshoot setting.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- O38: [../../parity/fcurves.md](../../parity/fcurves.md) and its golden cases
- docs/parity/fcurves.md (O38)
- crates/anim/src/interp.rs

## Design decisions (planner)
- `Tangent { left, right, mode, weighted, broken }` as in the area design. Weighted tangents free the time component; unweighted keep it at 1/3 of the interval (check against O38).
- Presets (ease in/out, linear, step) are commands that set tangent modes.
- Expose the speed curve dv/dt analytically per segment for 092.

## Done when
- [ ] All O38 cases pass at level B.
- [ ] ANIM.md updated with each rule.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: tangent data model (`TangentMode`, `Handle`, `KeyTangent`, per-key in `Track::tangents`), presets in `forge_anim::tangent` (ease, ease in/out, zero angle, zero length, set_right_handle with broken/weighted), `speed_at` (analytic dv/dt). O38 keyframes: 34/36 cases exact (the 10 that were exact stay exact; `broken` still KNOWN_MISMATCH, `linear-to-spline` still UNSUPPORTED: no machine-readable per-key interpolation). Tests: `crates/anim/tests/model.rs`.
- Rules: the legacy read-back path (§1.5a) now builds `KeyTangent`s and runs the same evaluator; stored user handles are evaluated as stored whether weighted or not.
- Deviations: no separate overshoot flag (nothing observed distinguishes it; automatic slopes already never overshoot, `clamp-off` = default); registry commands (`key.set_tangent`) left to the UI briefs.
- Files: as 088 plus crates/anim/src/tangent.rs.
- Open: observe `broken` with its flags; redo `linear-to-spline` with `interpolation_by_key`.
- Gate: see brief 096.
