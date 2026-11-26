# 222 · `forge play` runtime and S11 acceptance

Status: ready · Area: P · Depends on: 219, 220 · Observations: – (own design, D21) · Touches: apps/forge, tests, docs

## Goal
The runtime window and the area's acceptance scenario S11 (README.md, P-behaviours.md):
an interactive scene built by a Luau script, driven by injected gamepad and MIDI input,
recorded, rendered deterministically. BEHAVIOURS.md §13, §10.

Spec: [../../specs/BEHAVIOURS.md](../../specs/BEHAVIOURS.md) (own design, D21; no parity data). Where the spec says *(open)*, decide, write the rule into the spec, and say so in the Report.

## Steps
1. `forge play` (§13).
2. The S11 test: build with Luau, then a headless play run with injected input for 10 s and recording, then render 0–240 twice byte-identically, scrub with no change, the document unchanged apart from the tracks, budgets from §10.
3. Manual chapter `docs/manual/behaviours.md`, and examples in `examples/behaviours/` (a spring follower, a MIDI-driven cloner, a gamepad-flown camera, a spawner).

## Done when
- [ ] The S11 test passes in CI (Linux and macOS).
- [ ] The play-mode fps budget is measured on the M5 Pro and recorded in PERFORMANCE.md.
- [ ] The manual chapter and the examples exist, and each example runs in a test.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
