# 138 · Physical camera: projection, sensor, ortho, DOF, motion blur

Status: done (partial: projection parity done, motion blur not) · Area: K · Depends on: 136, 047 · Observations: O57, O29 · Touches: render-cpu, core

## Goal
Camera projection matching the reference exactly (silhouettes within 1 px).

## Inputs
- Area design: [../../roadmap/areas/K-rendering.md](../../roadmap/areas/K-rendering.md)
- O57: [../../parity/camera.md](../../parity/camera.md) and its golden cases
- O29: [../../parity/scene-defaults.md](../../parity/scene-defaults.md) and its golden cases
- docs/parity/camera.md (O57)

## Design decisions (planner)
- Focal length + sensor width define FOV (horizontal fit; check O57 aspect-square). Film offset shifts the principal point. Ortho zoom rule from O57.

## Done when
- [x] All O57 cases (the 4 observed): silhouette and checker corners within 1 px.
- [x] The gate passes (per-crate checks; the full gate once at the end of the session) and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `camera_parity.rs` renders the 4 observed O57 cases through a `forge.camera` object and
  the render bridge: 4/4 silhouettes within 1 px (`focal-18`, `film-offset`, `ortho-front`,
  `ortho-top`), no UNSUPPORTED, no KNOWN_MISMATCH. The cases the form lists but that were not
  observed (`default`, `focal-100`, `aspect-square`, `sensor`, `dof`) are asserted absent. Also
  `camera_lens.rs` (3 tests): thin-lens rays and the circle-of-confusion blur size.
- Rules derived (RENDER.md R15): lens from FOV and sensor; film offset X +20 % moves the picture
  LEFT (principal point at w/2 - offset * w; Y sign parity-pending); axis projections ignore the
  camera rotation; orthographic width 1000 at zoom 1 (`ortho-front` IoU 1.0 at zero tolerance;
  `ortho-top` only fixes it to 1.6 %, its plane edge squares are black); the case files' camera
  rotation [45, -19.45, 0] is real, so the empty `default`/`focal-100`/`aspect-square` frames are
  correct pictures of a scene outside the frame (the form's guess of a misplaced plane is wrong;
  the plane is horizontal and edge-on, the visible shape is the cube).
  Parity-pending: ortho zoom and non-4:3 aspect, Y film offset sign, Back/Left/Right/Bottom.
- Deviations: DOF is implemented as a thin lens (own design, `RenderSettings::depth_of_field`,
  off by default, so existing renders are unchanged; no golden data since the Physical renderer
  could not be switched on by script). Motion blur is not implemented (needs the snapshot at two
  times and shutter semantics, none observed); shutter, lens distortion, vignetting and chromatic
  aberration parameters exist on the camera object and are ignored by the renderer.
- Files changed: crates/render-api (camera.rs, settings.rs, lib.rs), crates/render-cpu (engine.rs,
  tests/camera_parity.rs, tests/camera_lens.rs, Cargo.toml), crates/generators/src/scene_objects.rs,
  crates/render-bridge/src/scene.rs, crates/core/src/render_config.rs, command render.settings,
  apps/forge `--dof`, docs.
- Open questions: redo the observation of `dof` with a GUI session (Physical renderer), a
  `film-offset-y` case, an orthographic case with zoom != 1, a 240 x 240 render.
