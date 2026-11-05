# 053 · Explosion, Shatter, Wind deformers

Status: done · Area: D · Depends on: 050 · Observations: O19 · Touches: modifiers

## Goal
Explosion, Shatter and Wind match O19.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O19: [../../parity/deformers-3.md](../../parity/deformers-3.md) and its golden cases
- docs/parity/deformers-3.md

## Design decisions (planner)
- Explosion and Shatter work per polygon (polygons separate; the mesh topology splits). Wind is time-based through EvalCtx.

## Done when
- [x] MODIFIERS.md sections.
- [x] Cases match, or are pinned.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/modifiers/tests/destructive_parity.rs` added. Explosion 1/2 exact, Shatter 1/2 exact (the strength-0 `defaults`), Wind 0/1 exact. The three other cases are in `KNOWN_MISMATCH` (exact comparison asserted to fail) and checked by the same statistical invariants on the golden and on our mesh (MODIFIERS.md §11-13). `explosion-fx/frame-10` is O45 and not part of this brief.
- Rules derived: polygons split, each owns its points, order kept (`*/strength-50`); rigid motion + uniform scale 0.5 per polygon (Kabsch residual 0); Euler `Ry Rx Rz` angles uniform in +-37.5 degrees; Shatter centres to y = 0 with a triangular xz offset (+-50); Explosion `|d|` uniform to 750 with radial bias; Shatter polygon 7m+6 shares its rotation with Explosion polygon 6m+5 (same stream, 6 vs 7 draws per polygon); Wind: only z, `dz(x, y)`, 0 at x = 0, sign of x, below 1.5 amplitude. All single-observation laws (strength / speed / angle speed / end size / randomness scaling, the 0.75, the 15, the radial 1.5, the Shatter floor law, the Wind noise and x weight) are parity-pending.
- Deviations: Explosion/Shatter change topology, so `forge_modifiers::apply_deformer_mesh` was added and `forge_generators::evaluate` calls it (points-only deformers unchanged). The wind case toml lacks the `frame` / `fps` keys its description mentions; the test defaults to frame 10 at 30 fps.
- Files changed: crates/modifiers/{Cargo.toml, src/{lib,explosion,wind}.rs, tests/destructive_parity.rs}, crates/generators/src/lib.rs, crates/ui/{src/icon_table.rs, icons/{explosion,shatter,wind}.svg}, docs/specs/MODIFIERS.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions / follow-up cases: 1-polygon and 2-polygon planes at strength 25 / 100, End Size 100, Randomness 0, Speed and Angle Speed 0 / 200 (pin each law and the stream order); wind at frames 0 and 20, Flag off, Size 50 / 200, a one-row plane along x.
