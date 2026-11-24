# 206 · S5 acceptance test and volume docs

Status: ready · Area: F · Depends on: 081, 082, 083, 003 · Observations: – · Touches: tests, docs

## Goal
The S5 scenario (roadmap README) runs as an automated test through the CLI: the union of three
spheres and a spline tube in a volume builder, smoothed, meshed and made editable. The manual
gets a volumes chapter built from the same script.

## Inputs
- Area design: [../../roadmap/areas/F-volumes.md](../../roadmap/areas/F-volumes.md) ("Done when": S5 passes)
- docs/roadmap/README.md (S5); docs/specs/VOLUME.md
- Pieces used: volume builder with object list and spline radius (081), smooth filter (082),
  mesher (083), Make Editable (003), native save/reopen, OBJ export (004).

## Design decisions (planner)
- Scene as a Rhai script in `tests/acceptance/s5_volume.rhai`, driven with `forge run`.
- Scene: three spheres (radius 100, centres on a line 120 apart) and a Helix or Circle spline
  with a tube radius, all children of a Volume Builder in union mode; a Gaussian smooth layer;
  a Volume Mesher on top. Then Make Editable, save, reopen, export OBJ.
- Assertions (from CLI output):
  1. After Make Editable the scene holds one polygon object and no volume objects.
  2. The mesh is closed and manifold (every edge has two faces) with no degenerate faces.
  3. Its bounding box contains every sphere and the tube, within one voxel size.
  4. Its enclosed volume is within 5 % of a reference computed in the test from a fine
     analytic SDF of the same union (the tolerance comes from voxel size; state the derivation).
  5. Save → reopen → export gives the same OBJ bytes; two runs are byte-identical.
- No reference comparison: parity is covered by the 080–086 parity tests.

## Done when
- [ ] The S5 test passes on Linux, macOS and Windows CI.
- [ ] The manual has a volumes chapter that uses the same script.
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
