# 051 · Squash & Stretch and Wrap deformers

Status: done · Area: D · Depends on: 050 · Observations: O9, O11 · Touches: modifiers

## Goal
Squash & Stretch and Wrap match their O9/O11 cases.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O9: [../../parity/deformers-2.md](../../parity/deformers-2.md) and its golden cases
- O11: [../../parity/followups-1.md](../../parity/followups-1.md) and its golden cases
- docs/parity/deformers-2.md, followups-1.md §3
- tests/golden/{squash-stretch,wrap}

## Design decisions (planner)
- Pure functions as in briefs 01–02. Squash: Top/Bottom/Center, factor, expand, curvature types (Square/Cubic/Quartic first; Custom/Spline Unsupported). Wrap: Cylindrical/Spherical, longitude/latitude ranges, movement, Scale Z, tension.

## Done when
- [x] MODIFIERS.md §7–8.
- [x] All cases match, or are pinned.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added (3 squash and 3 wrap unit tests, `fields` list check for both types, 6 + 5 golden cases in `deformers_parity.rs`). Parity 11/11 at level A (squash-stretch 6/6, wrap 5/5), plus the existing 64 (75/75); `UNSUPPORTED` and `KNOWN_MISMATCH` stay empty. Types `forge.squash_stretch`, `forge.wrap`, registered by `Deformers` (so `object.create` works headless), both with the `fields` list and `enabled`.
- Rules (MODIFIERS.md §7), each first hypothesis:
  - Squash: span scales by Factor between `bottom - 0.005` and `top + 0.005` (`factor`: y rows -50 -> -25; beyond the ends the shift is `100.005 * (f - 1)`, i.e. 50.0025, `factor-big` 250.0025); x/z scale `1 + e(1/f - 1)(1 - u^2)` with `u = y / 100.005` (`factor`: 175.0025, 100.01; `expand-0` +-100, `expand-200` +-300).
  - Wrap: `d = radius - z`, axis/centre at `z = radius`, `phi = lon - 270 deg`, `lon = mid + x/width * span` (`defaults`: (-212.132, -12.132); `width-90`: (-277.164, 85.195)); height unused cylindrically (`angles`); spherical latitude `theta = lat_mid + y/height * lat_span` (`spherical`: (-150, 212.132, 50)).
- Parity-pending: the 0.005 slack (additive vs factor 1.00005), Center pivot, Aspect and Smooth Start/End (not applied), Curvature < 100 % and Cubic/Quartic exponents (Bulge's family by analogy), Movement (shift of x), Scale Z (scales z in `d`), Tension (linear blend), other latitude and longitude ranges, non-default Width/Radius. Custom and Spline squash types leave the points unchanged (Unsupported; apply_deformer cannot fail).
- Deviations: param ids drop the `_percent` / `_deg` suffixes (house convention), the test translates the O9/O11 keys; no Fit/size box since the types have none; spec section §7 added, §8 left as is.
- Files changed: crates/modifiers/{src/lib.rs,src/common.rs,src/squash.rs,src/wrap.rs,tests/deformers_parity.rs,tests/falloff.rs}, docs/specs/MODIFIERS.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, docs/manual/reference (regenerated), this brief.
- Open data that would settle pending rules: Squash with Top 50 / Bottom -150 (the slack), Center 50 %, Aspect 50 %, Smooth Start/End 50 %, Curvature 50 %, Cubic and Quartic types, Power with Custom; Wrap with Movement 100, Scale Z 50 %, Tension 50 %, Latitude 0..90, Longitude 0..180, Width 200, Radius 100, Spherical with Height 200.
- Gate (last lines): `GATE: tests passed` ... `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`.
