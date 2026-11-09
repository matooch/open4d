# 095 · Motion paths and onion skin

Status: done · Area: G · Depends on: 010, 143 · Observations: – · Touches: viewport

## Goal
Draw the world-space path of the selected object over a frame range, and ghosted copies (onion skin).

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)

## Design decisions (planner)
- Evaluate scene_at over the range in a background job; cache invalidated by track edits.

## Done when
- [x] Viewport shows a path for an animated cube; test the path sampling function.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added: crates/anim/tests/motion.rs (6), crates/viewport/tests/motion.rs (7) and display.rs (2 GPU, baseline `motion_path.png`), crates/app/tests/motion.rs (5), crates/ui/tests/motion_view.rs (6). No parity cases (own design, no observation): 0/0.
- Rules derived: none from observations. Own design in ANIM.md section 2b and VIEWPORT.md section 12: path = world origin sampled per frame (`scene_at`-equivalent via the scheduler's `animate`, so expression tags count), key squares from `transform.*` tracks of the node and its ancestors, past/future colouring, static objects have no path, ghosts as wire (bounding box over 5000 polygons) faded by distance. Parity-pending G5: every colour, size, default count and whether the reference shows frame ticks or a per-object flag (a new G5 item "motion trajectory and ghosting display" could settle it).
- Deviations from the design decisions: the "background job" is one worker thread behind a keyed cache (`AsyncCache`); a stale result stays on screen while the new key computes, results are only used for their own key (scene revision, node count, nodes, range or current frame, settings), so undo and scrubbing are exact once ready. The toggles are view state (like `view.clone_display`), not document edits, so they have no undo step; nothing is saved.
- Files changed: crates/anim (motion.rs, lib.rs, Cargo.toml, tests/motion.rs), crates/viewport (motion.rs, lib.rs, curves.rs, tests/motion.rs, tests/display.rs, baseline), crates/app (motion.rs, lib.rs, tests/motion.rs), crates/command (state.rs, context.rs, lib.rs, builtin/object.rs), crates/ui (motion_view.rs, panes.rs, state.rs, commands.rs, app.rs, viewport_panel.rs, screenshot.rs, icon_table.rs, icons/motion_path.svg, icons/onion_skin.svg, tests/motion_view.rs), docs (specs ANIM, VIEWPORT, COMMANDS, generated reference, BACKLOG, G-animation, this brief).
- Open questions: G5 display rules above; whether paths should also show for unselected animated objects.
- Gate: 1962 tests passed, 0 failed; fmt, clippy -D warnings, cargo-deny, cleanroom and parity (716 cases) ok - "GATE: all passed".
