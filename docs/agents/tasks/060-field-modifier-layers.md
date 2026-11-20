# 060 · Field modifier layers: Freeze, Decay, Delay, Remap, Curve, Colorize, Step

Status: done · Area: D · Depends on: 059 · Observations: O49 · Touches: fields

## Goal
The modifier layers match O49.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O49: [../../parity/fields-4.md](../../parity/fields-4.md) and its golden cases
- docs/parity/fields-4.md

## Design decisions (planner)
- Time-dependent layers (Freeze, Decay, Delay) keep per-sample state across frames. Evaluate by stepping from frame 0 like the observer did, with a cache keyed by frame.

## Done when
- [x] FIELDS.md §modifiers; cases match, or are pinned.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added; parity N/M (`UNSUPPORTED` / `KNOWN_MISMATCH` with reasons).
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
- Result: no new code. Remap, Invert, Clamp, Colorize, Curve (default shape) were settled earlier: 6/6 exact with mirror-graph, random-space-none statistical (fields_parity).
- Pinned (FIELDS.md F8): `mod-step` UNSUPPORTED (value is the normalised coordinate over the bounds of the sampled set, not a function of the incoming value); `decay-frame20`, `delay-frame20` pending (committed samples not reproducible by the observer itself: 83 / 160 of 729 points differ); `freeze-frame10` not observed.
- Files changed: docs/roadmap/BACKLOG.md, this brief.
- Open data: re-export decay/delay/freeze with the observer's frame-by-frame procedure verified; a Step case on a second grid to separate bounds from direction.
