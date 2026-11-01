# 050 · Bend 3-axis gating fix + EvalCtx through evaluation

Status: ready · Area: D · Depends on: – · Observations: – · Touches: modifiers, generators, core

## Goal
Two things land together: (1) the three pinned bend mismatches pass (`within_box` gates on x, y and z, plus the suppression rule); (2) evaluation receives time, fps and seed, so later deformers and effectors can use them.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- MEMORY.md (bend follow-ups: rules proven by bend/mode-box-narrow{,-x,-z})
- docs/parity/bend.md, MODIFIERS.md §0
- crates/modifiers/tests/deformers_parity.rs (KNOWN_MISMATCH)

## Design decisions (planner)
- `within_box`: a point deforms only if |x| ≤ sx/2, |y| ≤ sy/2 and |z| ≤ sz/2. The whole deformation is suppressed when the box spans the parent's x/z extent in neither axis (object-relative threshold; see MEMORY.md for the exact proof cases).
- `EvalCtx { time: f64, fps: f64, seed: u64 }` is threaded through `forge_generators::evaluate` and `apply_deformer`. `forge_anim::scene_at` callers pass the frame time. Default ctx = frame 0, 30 fps.

## Done when
- [ ] MODIFIERS.md §0 updated; the 3 cases leave KNOWN_MISMATCH.
- [ ] All callers compile with the ctx; no behaviour change elsewhere (the gate proves it).
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
