# 111 · Evaluation order: tag priorities, expressions before generators

Status: done (O36 ordering case not recorded: own unit tests) · Area: H · Depends on: 104, 105, 108 · Observations: O36 · Touches: core

## Goal
Implement and document the per-frame order: tracks, expressions/tags by priority, generators/deformers, dynamics.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- O36: [../../parity/constraints.md](../../parity/constraints.md) and its golden cases
- docs/specs/CORE.md
- docs/parity/constraints.md (constraint driven by IK case)

## Design decisions (planner)
- Tag priority value per tag (phase + number) as in the reference manual.

## Done when
- [ ] O36 ordering case passes; CORE.md updated.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `crates/rig/tests/tag_order.rs` (4), `crates/app/tests/rig_tags.rs` (1). O36 recorded no IK-then-constraint case, so there is no parity number; the rule is our own (group, number, document order) and stays parity-pending.
- Rules (RIG.md §12, CORE.md §5a): expression, IK and constraint tags sorted by (priority group, priority, document order) in `forge_app::schedule::animate`, after tracks and before generators; a constraint before an IK moves the IK goal in the same frame, the reverse order gives the stale goal.
- Deviations: `dynamics` and `generators` groups only order tags among themselves (no dynamics stage yet; geometry is evaluated lazily after all tags).
- Files: crates/app/src/schedule.rs, crates/rig/src/tags.rs, crates/app/tests/rig_tags.rs, docs/specs/CORE.md, docs/specs/RIG.md.
- Open questions: the reference's default priority per tag type.
