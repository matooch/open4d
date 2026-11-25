# 114 · S6 acceptance test and rigging docs

Status: done · Area: H · Depends on: 100–113 · Observations: – · Touches: tests, docs

## Goal
S6 runs as a CLI-driven test; manual rigging chapter.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- docs/roadmap/README.md (S6)

## Design decisions (planner)
- Drive through `forge` commands.

## Done when
- [x] S6 passes (Linux here; the CLI-only test has no OS-specific code, macOS and Windows run it in CI).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `tests/acceptance/s6_rigging.rhai` and two tests in `apps/forge/tests/acceptance.rs`
  (`s6_rigging_skin_follows_the_joints_ik_pole_and_pose_morph`, `s6_all_frames_evaluate_and_two_runs_agree`),
  driven only by `forge run`, `forge eval` (bbox centres, OBJ export) and `forge cmd` variants. Both ran
  5 times in a row green. No parity count (no reference data for the combined scene).
- What is asserted: (1) IK: the tip (a cube riding on the end joint) is within the tag's threshold
  (0.01) of the goal at frames 0, 30, 60; both bones keep length 100 (1e-6); the elbow offset is in the
  chord-pole plane on the side opposite the pole (cos < -0.999999, RIG.md section 10), and the bend plane
  turns as the pole moves (+Z, +X, -Z). (2) Skin: the cap centres follow J1 and the IK-driven tip (< 0.5), the mesh
  moves by more than 50 between frame 0 and 30. (3) Pose Morph, against the same scene with the tag
  disabled: 0 % moves nothing, 100 % moves only the 48 target points and by at most the 20 units of
  the target, the 50 % frame moves every point exactly half as far as the same scene keyed to 100 %
  (linear skin). (4) Determinism: two runs give identical points and joint positions at 5 frames.
  (5) All 61 frames evaluate without error; the mean frame time is printed, not asserted.
- Measured mean frame time (debug build, one `forge eval` process per frame, includes process start
  and scene load): 23.9 ms per frame. No wall-clock assertion (HERMES-IMPLEMENTER 5a).
- Added for the script (no command existed): `tag.attach` (tag on a named node) and `weights.auto_bind`
  (see 107). Rigging chapter `docs/manual/rigging.md` and SUMMARY entry; PROGRESS.md S6 row, area H row,
  at-a-glance lines; reference docs regenerated.
- Deviations: the first goal for frame 60 was out of reach (210 > 200); changed to (-40, 70, 40). The
  IK pole side asserted is the observed rule (elbow opposite the pole), not "towards".
- Files: tests/acceptance/s6_rigging.rhai, apps/forge/tests/acceptance.rs, docs/manual/rigging.md,
  docs/manual/SUMMARY.md, docs/PROGRESS.md, docs/roadmap/BACKLOG.md.
- Open questions: none beyond O37 (the Pose Morph rules are our own until G9).
- The last lines of the gate output: see below.
