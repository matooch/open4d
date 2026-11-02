# 087 · Keyframe rework: O16 rules (millisecond rounding at other fps; tangent at uneven monotone keys)

Status: done · Area: G · Depends on: – · Observations: O16 · Touches: anim

## Goal
The three keyframe cases O16 broke match: `fps-24`, `dense-quarter` and `mono-uneven`. Remove them from `KNOWN_MISMATCH`, and make every other keyframe case keep matching.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- O16: [../../parity/keyframes-3.md](../../parity/keyframes-3.md) and its golden cases
- docs/specs/ANIM.md §1 (brief 09 rules)
- `crates/anim/tests/keyframes_parity.rs` (the `KNOWN_MISMATCH` list)
- `tests/golden/keyframes/{fps-24,fps-25,dense-quarter,mono-uneven,mono-steep,mono-steep-rev,many-keys}`, plus `tangents.json` in mono-uneven and many-keys

## Design decisions (planner)
- Time rounding: brief 09 rounds the evaluation time to whole milliseconds. At 24 fps and at quarter frames that gives 2.7e-4 errors while 25 fps matches. Test hypotheses: rounding happens on a different time base (ticks per second, e.g. a common multiple of frame rates), or on frames rather than seconds, or not at all with a different solver. Use fps-24, fps-25, dense-quarter and the 30 fps cases together.
- Monotone middle key with uneven spans: O16 `tangents.json` for mono-uneven shows the key at 10 with dt ±3.333 (a third of the shorter span) and dv ±33.33, a slope of 10 per frame, which is the left segment's slope, not the central difference 2.5. many-keys shows more cases. Derive the slope rule (candidates: the slope of the shorter adjacent segment; a harmonic-mean slope; a clamped Catmull-Rom) and the handle length rule (min of the adjacent spans / 3?) from `tangents.json`. Then confirm with the samples.
- Prefer tangents read from `tangents.json` as ground truth for the auto-tangent rule, and use the samples to confirm the curve evaluation.

## Done when
- [ ] ANIM.md §1 is updated: the rules cite O16 cases, and the parity-pending markers are removed where O16 proves the rule.
- [ ] All 24 keyframe cases match. `KNOWN_MISMATCH` is empty, or each entry left has a documented reason after three hypotheses.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
New tangent modes (089).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: 24/24 keyframe cases exact (max error 2.3e-13), `KNOWN_MISMATCH` empty, `UNSUPPORTED` empty. Tests added in `crates/anim/tests/rules.rs` (mean slope at a monotone key; 1 s span symmetric at 24 fps); `KNOWN_MISMATCH` emptied.
- Rules derived: (1) slope at a monotone middle key is the mean of the two segment slopes, `mono-uneven` (5.5 from 10 and 1; fitted exactly, handle 50/9), clamp proven by `mono-steep` and `mono-steep-rev`. (2) Millisecond time rounding is right at 24/25/30 fps; the 2.7e-4 misses were exact-tie samples, decided by the stored handle lengths (multiples of 4e-7 s, read from `tangents.json`): proven by `fps-24`, `dense-quarter`, `two-keys`, `four-keys`. Parity-pending: the grid unit at spans that are not whole seconds, and the grid handles being used only for ties.
- Deviations: `tangents.json` is not ground truth for slopes (it shows a third of the left chord at every key, also at extremes); used only for handle lengths. Three hypotheses were not needed per case (time base and solver unchanged).
- Files: crates/anim/src/interp.rs, crates/anim/tests/{keyframes_parity,rules}.rs, docs/specs/ANIM.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open: an observation at a non-whole-second span and another fps (e.g. 24 fps, 10 frames) would settle the grid unit in seconds vs frames.
- Gate: see commit.
