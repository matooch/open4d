# 205 · S4 acceptance test and motion-graphics docs

Status: ready · Area: E · Depends on: 066, 072, 074, 075, 136 · Observations: – · Touches: tests, docs

## Goal
The S4 scenario (roadmap README) runs as an automated test through the CLI: a text object
fractured into letters, cloned along a spline, effectors with a field falloff, animated over 90
frames and rendered as a 1080p image sequence. The manual's cloner chapter gets a motion-graphics
walkthrough built from the same script.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md) ("Done when": S4 passes)
- docs/roadmap/README.md (S4); docs/manual/cloner.md
- Pieces used: clone-toolset Text (075), Fracture (074), object-mode cloner on a spline (066),
  effector falloff via fields (072), keyframes (088), CPU render of a sequence (136, `forge render`).

## Design decisions (planner)
- Write the scene as a Rhai script in `tests/acceptance/s4_mograph.rhai` (like S1's
  `s1_cloner.rhai`) and drive it with `forge run` / `forge render`, so it doubles as an
  automation example.
- Scene: Text "FORGE" → Fracture (per letter) → Cloner in object mode on a Circle spline;
  a Plain effector with a Linear field falloff whose position is keyed from frame 0 to 90;
  a Random effector (fixed seed). 90 frames at the document frame rate.
- Assertions (all from CLI output, not internal state):
  1. Fracture yields one piece per letter (5); the cloner yields the expected clone count.
  2. Clone positions lie on the spline at frame 0 (within 1e-3) and differ between frame 0 and
     frame 90 only where the field covers them (the field sweep is visible).
  3. Two runs give byte-identical OBJ exports at frames 0, 45, 90 (determinism).
  4. `forge render` writes a 1920×1080 sequence; the CI test renders frames 0, 45 and 90 at
     1 sample and checks size, frame numbering and that the letters are visible (non-empty alpha).
     The full 91-frame render at default samples runs once locally; record its time in the Report.
- No reference comparison: this is an integration scenario (parity is covered by the area's
  own parity tests).

## Done when
- [ ] The S4 test passes on Linux, macOS and Windows CI within the existing test time budget.
- [ ] The manual has a motion-graphics walkthrough that uses the same script.
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
