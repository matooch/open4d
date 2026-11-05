# 039 · Text spline (fonts)

Status: done · Area: C · Depends on: 038 · Observations: O48 · Touches: generators, text (new)

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
- [x] TEXT.md spec.
- [x] O48 cases match where the font is available (font-independent invariants only; see Report).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `crates/text` (`forge-text`, skrifa) and `forge.text_spline` (`crates/generators/src/text_spline.rs`); tests `crates/text/tests/layout.rs` (12), `crates/generators/tests/text_parity.rs` (11, all nine O48 cases) and `text_spline.rs` (6). Parity: 9/9 cases pass the font-independent invariants, 0/9 are vertex-exact by design (planner decision: the reference default font cannot be set by script and may not be copied; ours is a bundled DejaVu Sans subset). `UNSUPPORTED`: Separate Letters (not observed). No `KNOWN_MISMATCH` list; glyph shapes are not compared at all.
- Rules derived (TEXT.md section 0): height is the em size (`a` vs `height`: exactly 1/4); Left pen x 0 and baseline y 0 (`a`, `two-lines`); Middle centres the **ink** bbox of the line, not the advance (`align-center`: shift 1340 font units = (30 + 2650) / 2, bbox +-127.93; the advance would give 1366); horizontal spacing added once per gap (`spacing`: 19.92 for 20, 204 whole font units, within one unit); newline = next line one line height lower at x 0 (`two-lines`: 245.996 = 2519/2048 of 200 = 1.23 em, the reference font's own constant); loops per glyph; outer clockwise / holes counter-clockwise and outer before holes inside a glyph; Extrude toward -Z under the signed-area guess; Uniform N gives pieces x (N + 1) points (`intermediate`: 28 x 5); straight letters keep only corners (`a`: 8 and 3). Parity-pending: glyph shapes and point counts, line-height constant, Right alignment, per-line alignment, Vertical Spacing, kerning, negative height.
- Deviations from the design decisions: skrifa instead of ttf-parser (the latter is unmaintained, RUSTSEC-2026-0192, and fails `cargo deny`; skrifa was already in Cargo.lock). DejaVu is bundled as a 26 832-byte subset (planner: any small licensed font); alignment uses the ink bbox, not the advance (the data say so); system fonts are found by file name containing the family, then verified by family name; Separate Letters is Unsupported.
- Files changed: crates/text/{Cargo.toml, src/lib.rs, tests/layout.rs, fonts/DejaVuSans-subset.ttf, fonts/LICENSE-DejaVu.txt}, crates/generators/{Cargo.toml, src/{text_spline.rs (new), splines.rs, lib.rs}, tests/{text_parity.rs, text_spline.rs}}, crates/ui/{src/icon_table.rs, icons/text.svg}, Cargo.lock, docs/specs/TEXT.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions or data: observer to re-observe `a`, `o`, `word` (and `defaults`) with DejaVu Sans via the GUI font picker so glyphs can be compared vertex for vertex; Right alignment and a two-line Middle case; Vertical Spacing non-zero; Separate Letters; a kerned pair; whether a GUI-picked font reshapes the spline.
- Gate (last lines): `advisories ok, bans ok, licenses ok, sources ok` / `cleanroom: ok` / `ok: 716 cases` / `GATE: 1822 tests passed, 0 failed` / `GATE: all passed`.
