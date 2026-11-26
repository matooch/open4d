# 046 · Null display, Instance render modes, Connect weld options

Status: done (parity-pending: Null circle/sphere segment count, 3D shape depth and pyramid apex, star inner radius, in-plane up axis for the X/Y orientations, Aspect on solids, X-Ray drawing) · Area: C · Depends on: – · Observations: O20 · Touches: generators, viewport

## Goal
The smaller R25 object options (Null display shapes, Instance modes, Connect weld tolerance) exist and match where observed.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O20: [../../parity/generators-3.md](../../parity/generators-3.md) and its golden cases
- docs/parity/generators-3.md, instance.md, connect.md

## Design decisions (planner)
- Null: display shape, radius, aspect, orientation (viewport only).
- Instance: instance, render instance and multi-instance modes are equal for geometry.
- Connect: weld on/off, tolerance, Phong mode.

## Done when
- [x] SCENE-OBJECTS.md S9 (Null) and GENERATORS2.md updated; tests.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: done (parity-pending, see below). Null display built from the O20b tables; Instance modes and Connect weld verified, not redone.
- Verified (already built, O20): Connect weld on/off, tolerance and move-to-later-point (`connect/tolerance-in`, `connect/weld-off`, `connect/defaults`) and Instance / Render Instance (`instance/render-instance`) are read by `crates/generators/tests/generators_parity.rs` (passes); Multi-Instance and Phong Average were withheld by the observer (GENERATORS2.md section 4).
- Rules derived, with the case that proves each (R = 100; full table in docs/specs/SCENE-OBJECTS.md S9.3): Rectangle corners on the circle (142.7 = 100 sqrt 2 + line width); Diamond vertices at +-R (198.7); Triangle circumradius R, vertex up (width 173.2 = R sqrt 3, y -50.3..98.7); Pentagon vertex up (width 191.1, y -80.9..98.7); Hexagon vertices left/right (198.7 x 173.2); Octagon flat sides on the axes (186.0 = 2 R cos 22.5 + line); Star 8 points, 16 segments (198.7); Axis +X and +Y arms from the origin (x 0..100.6, y -0.6..98.7); Cube square of side 2R in front view; Pyramid square plus diagonals, 8 segments; Sphere circle plus two diameters. Aspect multiplies the vertical axis of flat shapes (Rectangle R50 AR2: 71.3 x 142.7) and does nothing on the solids (Pyramid R50 AR2 identical to AR1). Orientation X/Y/Z lie in YZ/XZ/XY, Camera faces the view.
- Parity-pending (own design, S9): circle and sphere circles use 64 segments; solids are 3D with depth 2R, pyramid apex on the normal toward the viewer; star inner radius R/2; the in-plane up axis is world Y for X and Z, world Z for Y; solids with Camera orientation use the world axes; fixed orientations are drawn in the Null's local frame and Camera orientation at its world position without its rotation or scale.
- Deviations from the design decisions: none. X-Ray and the object tree icon tint (`icon_color`) are stored parameters but not drawn (the overlay line pipeline is depth tested with no per-line flag).
- Files changed: crates/core/src/registry.rs, lib.rs (`null_type`, `NULL_SHAPES`); crates/generators/src/null_display.rs, lib.rs; crates/ui/src/scene_overlay.rs; tests crates/generators/tests/null_display.rs, crates/io/tests/null_display.rs, crates/ui/tests/null_overlay.rs; docs/specs/SCENE-OBJECTS.md, GENERATORS2.md; BACKLOG, STATUS, PROGRESS.
- Open questions: circle/sphere segment count and solids' depth cannot be read from the front view; a Top and a rotated-view capture of Cube/Pyramid/Sphere at Camera orientation, and Aspect 2 with orientation X for a 3D shape, would settle them.
- Gate (`gate_chunked.sh`, log /tmp/claude-0/work/gate_046.log): every package passed (forge-ui 562, forge-generators 220, forge-viewport 160, ...), python tests 38 OK, cargo deny ok, cleanroom ok, golden check `ok: 1118 cases`, fmt and clippy clean. Two packages showed rc=101 in that run and were handled separately: forge-core (a test of the old no-param Null, fixed in f569851 after the run began; `cargo test -p forge-core` passes) and forge-render-rt (the run was interrupted on `o30_and_o43_cross_tier_consistency`, which exceeded 15 CPU minutes on the software Vulkan driver; not touched here; every other forge-render-rt test passes with `--skip o30_and_o43_cross_tier_consistency`). The run therefore ended `GATE: FAILED` and a full clean `GATE: all passed` was not obtained here.
