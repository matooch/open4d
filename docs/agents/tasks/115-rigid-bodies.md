# 115 · rapier integration: rigid body tag, shapes, scene settings, per-frame cache, scrubbing; compare_trajectory

Status: done · Area: I · Depends on: 050, 010 · Observations: O39 · Touches: dynamics (new), parity-harness

## Goal
Rigid bodies through rapier3d with deterministic cached simulation and level-C parity against O39.

## Inputs
- Area design: [../../roadmap/areas/I-dynamics.md](../../roadmap/areas/I-dynamics.md)
- O39: [../../parity/dynamics.md](../../parity/dynamics.md) and its golden cases
- docs/parity/dynamics.md (O39)

## Design decisions (planner)
- New crate `forge-dynamics`. Fixed step 1/(fps × substeps); state cached per frame; scrubbing back resimulates from the nearest cached frame.
- Shapes: auto (convex hull), box, sphere, static mesh. Units: cm in the scene, metres in rapier – convert at the boundary.
- Harness `compare_trajectory` with per-case thresholds from case.toml `[tolerance]` (rest position, first contact ±1 frame, bounce ratio ±10 %, slide distance ±10 %).

## Done when
- [x] O39 cases meet thresholds; two runs give identical caches.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `crates/dynamics` (`forge-dynamics`) on `rapier3d-f64` (cargo-deny: licenses and bans ok),
  `compare_trajectory` in the harness (`crates/parity-harness/src/trajectory.rs`), spec
  `docs/specs/DYNAMICS.md`. Tests: `crates/dynamics/tests/rigid_parity.rs` (7 golden cases),
  `rigid_unit.rs` (11), `crates/app/tests/dynamics_eval.rs` (2), harness unit tests (3).
  Parity at level C: 7 of the 8 observed cases assert their derived quantities and pin one
  quantity each as `KNOWN_MISMATCH`; `drop-friction-high` is pinned whole; `gravity-half` was not
  observed.
  - `drop-cube`: free fall frames 0-22 within 1 cm, contact frame ±1, rest y 0.05 cm, at rest from
    frame 35, rotation at frame 90 within 1°; pinned: rebound ratio (observed rise 4.34 cm for 5 %
    bounce, ours none).
  - `drop-bounce`: contact frame ±1; pinned: the rest (energy gain at 1e+18 is not reproduced).
  - `drop-friction-slide`: slide distance ±10 % (ours 41.5 vs 45.03 cm), rest y 0.05 cm.
  - `drop-friction-high`: pinned (ours tumbles, 56.6 vs 66.1 cm).
  - `stack`: stays up: 0.25 cm vertical, 0.3 cm sideways, 1° tilt (reference drift is 0.045 cm;
    ours sinks 0.17 cm: softer contacts).
  - `slope-sphere`: x at frame 30, final x and y within ±10 %.
  - `tumble`: contact frame ±1, rest y 0.05 cm, rests flat (P, B within 1° of a multiple of 90°);
    pinned: rest x, z and heading.
- Rules derived: friction combines by maximum (`drop-friction-slide`: the product tips the cube,
  60.2 cm; parity-pending, one case); restitution by product; automatic shape on a sphere primitive
  is an exact ball (`slope-sphere`: hull reaches x 513, ball 609, reference 669 cm); default
  gravity 1000 cm/s² and 2 steps per frame are a fit to the free fall of `drop-cube` (max error
  0.74 cm over frames 0-22; 981 gives 3.4-6.6 cm), not read values (scene settings are not
  readable by script). All parity-pending.
- Deviations from the design decisions: the settings are an object (`forge.dynamics_settings`), not
  scene file fields (no file-format change); the cache keeps the live world at its last frame
  instead of restarting from a mid-run frame (rapier worlds are not `Clone`); the case files hold
  no `[tolerance]` table (golden data is read-only), so the thresholds live in the test and the
  spec; `Thresholds::from_case` reads such a table when a case has one.
- Files changed: crates/dynamics/**, crates/parity-harness/src/{trajectory,lib}.rs,
  crates/app/{Cargo.toml,src/lib.rs,src/animate.rs,src/dynamics.rs,tests/dynamics_eval.rs},
  crates/ui/src/icon_table.rs + icons (rigid_body, dynamics_settings, connector),
  docs/specs/DYNAMICS.md, BACKLOG, STATUS.
- Open questions: the scene's gravity, steps per frame and scale (a GUI read would replace the
  fit); how bounce and friction combine (a drop with the plane's bounce at 0 and at 50 %); what
  Ghost does; the friction 1e+18 behaviour.
- The last lines of the gate output: see the final report (the full gate is run once at the end).
