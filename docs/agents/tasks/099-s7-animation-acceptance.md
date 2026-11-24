# 099 · S7 acceptance test and animation docs

Status: done · Area: G · Depends on: 090–096 · Observations: – · Touches: tests, docs

## Goal
The S7 scenario (roadmap README) runs as an automated test through the CLI; the manual has an animation chapter.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- docs/roadmap/README.md (S7)

## Design decisions (planner)
- Drive the test entirely through `forge` commands (013/014) so it doubles as an automation example.

## Done when
- [x] S7 is wired into the existing three-OS CI suite (macOS verified locally).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- **Native S7:** CLI creates and saves a cube scene, stretches its keys from 0/15/30
  to 0/30/60, edits broken weighted handles, eases just the outgoing speed and
  verifies loop-after by comparing frame 75 with frame 15. All assertions use
  positions from CLI OBJ exports, not direct access to stored tracks.
- **Manual:** expanded the Animation chapter with Attributes indicators, Dope Sheet
  retiming, Value/Speed editing, and a repeat-after automation example.
- **CI:** the new test is part of the existing workspace suite on Linux, macOS
  and Windows. Verified locally on macOS; no remote run is claimed here.
- **Files:** `apps/forge/tests/acceptance.rs`, ANIM §2.4a, and `docs/manual/animation.md`.
- **Gate:** `GATE: 2483 tests passed, 0 failed`; `GATE: all passed`.
