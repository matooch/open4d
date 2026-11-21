# 047 · Camera, Light, Environment, Floor and Sky as scene objects

Status: done · Area: C · Depends on: – · Observations: O29 · Touches: generators, core, viewport

## Goal
These scene objects exist as nodes with R25 parameters and defaults, show in the viewport, and are ready for the renderers.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O29: [../../parity/scene-defaults.md](../../parity/scene-defaults.md) and its golden cases
- docs/parity/scene-defaults.md §2–3

## Design decisions (planner)
- Node types `forge.camera` (focal, sensor, FOV, projection, film offset, clipping, DOF params), `forge.light` (type, colour, intensity, falloff, cone angles, area shape and size, shadow type), `forge.environment`, `forge.floor`, `forge.sky`.
- The viewport draws gizmos (camera frustum, light shapes). "Look through camera" sets the viewport camera.

## Done when
- [x] SCENE-OBJECTS.md spec with tables citing O29.
- [x] Tests: descriptor defaults equal O29; the viewport camera from a camera object gives the same projection matrix as the formula.
- [x] The gate passes (per-crate checks; the full gate once at the end of the session) and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Rendering (K).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added in generators (7: every camera default and every light default of all nine
  Type options read from the O29 tables, FOV readbacks, type mapping, rigid world matrix and
  heading sign), render-bridge (6), viewport (5), ui (4), cli (3 end-to-end renders) and core
  (render settings are a pure addition to v12). Parity: the O29 form has no golden folders; the
  default comparison is 36/36 camera rows and 40+ rows for each of the 9 light tables. Not
  parameters yet (listed in SCENE-OBJECTS.md S2, S3, S8): camera Stereoscopic, Composition
  Helpers, Spherical, DOF map and shader; light Visibility (volumetric), Photometric, Caustics,
  Noise, Lens Effects, Project include list.
- Rules derived: lens from FOV (O57 18/36/100 mm); axis projections ignore the node rotation
  (O57 `ortho-top`); orthographic width 1000 at zoom 1 (O57 `ortho-front`, parity-pending for
  other zoom); heading sign finding recorded in RENDER.md R10 and SCENE-OBJECTS.md S2.
  Parity-pending: Back/Left/Right/Bottom orientations, colour temperature curve, soft-shadow
  radius, cone angle meaning for spots.
- Deviations: one `forge.light` type for the nine Type options (O29 lists identical parameters
  for each); Create > Scene submenu instead of the Objects list; the render settings are stored as
  `Option<RenderConfig>` (None = nothing stored, so the CLI defaults stay 320 x 240) and need no
  version bump; new commands `render.settings` and `view.look_through_camera`.
- Files changed: crates/generators/src/scene_objects.rs (+ lib.rs), crates/core (render_config.rs,
  scene.rs, document.rs, file.rs, lib.rs), crates/render-bridge/src/scene.rs, crates/command
  builtin/render.rs, crates/viewport/src/scene_view.rs, crates/ui (commands.rs, app.rs,
  scene_overlay.rs, icon_table.rs, 4 icons), apps/forge/src/render.rs, docs.
- Open questions: observe the Floor, Sky and Environment objects' parameters; the camera zoom in
  orthographic views; the default light's direction and the light units (RENDER.md R14).
