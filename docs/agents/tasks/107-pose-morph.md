# 107 · Pose Morph tag

Status: ready · Area: H · Depends on: 088 · Observations: O37 (not capturable by script; own design per D20) · Touches: rig

## Goal
Point morph targets with strengths, absolute/relative modes, matching O37.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- O37: [../../parity/pose-morph.md](../../parity/pose-morph.md) and its golden cases
- docs/parity/pose-morph.md (O37)

## Design decisions (planner)
- **Own design (D20), parity-pending.** O37 has no golden data: the reference app does not let a
  script set the morph strength. Build from the public manual and the standard blend, write the
  rules in the spec (docs/specs/RIG.md, a new Pose Morph section), and test against the spec.
  A GUI-assisted capture (GUI-SESSIONS.md G9, O37) checks it later.
- Point morph, relative mode: p = base + Σ s_i (target_i − base), strengths may exceed 100 % and go
  negative unless the manual says otherwise. Absolute mode: p = base + Σ s_i (target_i − base)
  with targets stored as absolute positions; write the exact difference between the modes in
  the spec from the manual.
- Strengths are animatable parameters (keyed like any other track, brief 088), so S6 can key them.
- Works on polygon objects and on the bind mesh under a skin (morph before skinning, the usual
  order; spec it).
- Position/rotation/scale (PSR) morph of the object itself: include if the manual describes it
  simply, else leave it out and say so in the Report.

## Done when
- [ ] Spec-derived tests: base, single target at 0/50/100/150 %, two targets mixed, absolute vs relative, keyed strength sampled at frames, morph under a skin.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

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
