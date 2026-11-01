# 024 · Interactive cutting: Loop/Path Cut, Knife (line, plane, loop), Slide

Status: ready · Area: B · Depends on: 020, 023 · Observations: G3 · Touches: modeling, ui, viewport

## Goal
Cutting tools in the viewport: loop cuts with offset and count, the knife in three modes, and edge/point slide.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- GUI session G3 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- crates/viewport/src/picking.rs (edge picking)

## Design decisions (planner)
- Pure functions `loop_cut(mesh, start_edge, offsets: &[f64])`, `knife_line(mesh, plane_or_segment)` and `slide(mesh, sel, amount)`. The interactive layer handles the hover preview line and click to commit, and the HUD shows count and offset.
- Behaviour follows the public manual plus G3 answers when they arrive. Until then, own design, parity-pending.
- Commands `mesh.loop_cut {edge, offsets}` and `mesh.knife {mode, ...}` so the tools are scriptable.

## Done when
- [ ] MODELING.md §5.
- [ ] Unit tests on cubes and cylinders (exact counts and positions); UI tests for hover and commit.
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
