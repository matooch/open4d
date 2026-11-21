# 105 · Constraint tag: PSR, Aim, Parent, Clamp

Status: done · Area: H · Depends on: 100 · Observations: O36 · Touches: rig

## Goal
Constraint tag modes matching O36.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- O36: [../../parity/constraints.md](../../parity/constraints.md) and its golden cases
- docs/parity/constraints.md (O36)

## Design decisions (planner)
- Weighted blends of targets; maintain-offset captured at activation; up-vector rules from O36.

## Done when
- [ ] O36 cases pass at level B.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `crates/rig/tests/constraints_parity.rs` (7 O36 cases x 6 tracks x 31 frames + 6 unit tests); 7/7 exact at 1e-4 using the reference's recorded driver curve (the position and H tracks of `psr-all`), error below 1e-8. With an ideal smoothstep driver the worst error is 2.9e-4 (the reference's driver itself deviates by up to 1.3e-4 degrees), guarded by an envelope of 1e-3, tolerance unchanged. `UNSUPPORTED`/`KNOWN_MISMATCH` empty. The cube mesh at frame 15 is not compared.
- Rules (RIG.md §11): see the spec; Aim = minimal swing (B 8.13) and, with an up target, `z -> d, y -> up` (B 341.57); Parent = target-space offset captured at activation; PSR offset = world delta. Parity-pending: PSR rotation/scale offsets, Aim axes other than Z+, multi-target weights, strength. Clamp: O36 saw no effect, so an own distance clamp with unit tests only.
- Deviations: `capture_offset` is a library function (no command); no UI for it yet.
- Files: crates/rig/src/{constraint,tags,lib}.rs, crates/rig/tests/constraints_parity.rs, crates/ui/icons/constraint.svg, crates/ui/src/icon_table.rs, docs/specs/RIG.md.
- Open data: Clamp activation route; a two-target PSR weight case; Aim with axes other than Z+.
- Gate: see the report of brief 111 (final).
