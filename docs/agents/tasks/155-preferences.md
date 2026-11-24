# 155 · Preferences window and settings storage

Status: done · Area: L · Depends on: 175 · Observations: O29 · Touches: ui

## Goal
Preferences (units, UI scale, theme, autosave, render defaults, RPC port/token) stored per user.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)
- O29: [../../parity/scene-defaults.md](../../parity/scene-defaults.md) and its golden cases
- docs/parity/scene-defaults.md

## Design decisions (planner)
- Settings struct with descriptors (same as params).

## Done when
- [x] Settings persist across restarts (test with a temp dir).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: two tests added (`crates/ui/tests/preferences.rs`): a restart round trip and an Apply/Cancel window test. O29 is form-only, so the new-project defaults are asserted directly (centimetre scale, 1280×720, Cubic at width 2, Default Light on) and no golden parity case changes; parity N/A.
- Rules derived: per-user JSON keeps unrelated keys on save and reload (the test seeds `keymap` and `rpc.other` and checks both survive); writes are atomic and 0600 on Unix; a rejected draft (height 0) leaves the persisted file untouched; Apply saves and updates autosave/render defaults, Cancel and the close button discard the draft, and neither touches the scene revision.
- Deviations from the design decisions: distance is converted in Attributes and Coordinates as specified, but the unit is shown once as a static "Units: cm" label in the Coordinates header instead of a per-field suffix. A per-field suffix made the resizable right panel grow while a drag changed the digits, which shifted the viewport mid-drag and broke `a_snapped_gizmo_drag_lands_on_the_target_vertex`; the static label keeps the panel width stable.
- Files changed: `crates/ui/src/prefs.rs` (settings, descriptors, draft form, atomic save), `crates/ui/src/app.rs` (Preferences window, menu item, apply/restart hooks, distance-aware coordinates), `crates/ui/src/widgets/{mod,float,vector}.rs` (distance display conversion), `crates/ui/src/picture_viewer.rs` and `crates/ui/src/interactive_render.rs` (editor render defaults), `crates/ui/Cargo.toml`, `docs/specs/UI.md` §24.2a, `docs/roadmap/BACKLOG.md`.
- Open questions: whether the render defaults should also seed the CLI when no scene render config exists; the spec currently keeps CLI defaults unchanged.
- The last lines of the gate output: `GATE: 2499 tests passed, 0 failed` and `GATE: all passed`.
