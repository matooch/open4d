# 039 · Text spline (fonts)

Status: waiting: O48 · Area: C · Depends on: 038 · Observations: O48 · Touches: generators, text (new)

## Goal
A Text spline from font outlines, with height, alignment, spacing and lines.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O48: [../../parity/text.md](../../parity/text.md) and its golden cases
- docs/parity/text.md

## Design decisions (planner)
- New crate `crates/text` (`forge-text`) with `ttf-parser`. It finds fonts by family through the OS font directories, and bundles one open-licence font (DejaVu Sans) as the default.
- Glyph outlines (quadratic/cubic Béziers) become spline segments, then go through spline sampling.
- Parity: level B with the same font. If the font in a case is missing on CI, skip with a clear message, and keep a CI case that uses the bundled font plus our own expected output.

## Done when
- [ ] TEXT.md spec.
- [ ] O48 cases match where the font is available.
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
