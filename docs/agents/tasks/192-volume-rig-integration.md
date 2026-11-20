# 192 · Volume and rig nodes in the editor: evaluation hook, commands, display

Status: done · Area: F, H · Depends on: 081, 083, 100, 101, 112, 084 · Observations: – (integration; parity is already covered by the crate tests) · Touches: generators, app, command, ui, viewport, volume, rig

## Goal
Briefs 080–086 and 100–113 built `forge-volume` and `forge-rig` as tested libraries, but a user can't
use them in the editor:
- `forge.volume_builder`, `forge.volume_mesher`, `forge.volume_filter` and `forge.volume` are not
  registered in the app, because `forge_generators::evaluate` can't call `forge-volume`
  (volume depends on generators: a cycle).
- The joint commands (`joint.create_chain`, `joint.align`, `joint.mirror`) aren't registered, because
  `forge-rig` can't depend on `forge-command` (command → generators → modifiers → rig).
- The draw lists in `forge_viewport::rig` and `forge_viewport::volume`, and the volume
  `PreviewWorker`, are not called by the editor.

## Design decisions (planner)
- **Evaluation hook.** Add a registry-level evaluator hook in `forge-generators` (or `forge-core`):
  a plugin can register `fn(&Scene, NodeId, &EvalCtx) -> Result<Option<PolyMesh>, BuildError>` for its
  type ids, and `evaluate` dispatches to it before its own match. Install the volume evaluator from
  `VolumePlugin`. No crate may gain a dependency that creates a cycle. Write the hook into
  GENERATORS.md (or the core spec) first.
- **Commands.** Register the joint commands from the crate that already sits above both
  (`forge-app` or `forge-command`'s builtin set, through a small wrapper), regenerate the command
  reference (`cargo run -p forge-cli -- docs --out docs/manual/reference`).
- **Registration and UI.** Install `VolumePlugin` in the default app registry, add icons for the four
  volume types (own SVGs, same style as `crates/ui/icons`), put Volume Builder / Mesher / Filter in
  the Create menu next to the generators, and the Joint, Skin and Weight tag under a Character group.
- **Display.** Draw the joint and bone lists and the selected joint's weight markers, and the volume
  box and preview, through the existing viewport line pass. A headless test checks that the draw lists
  are produced for a scene with a joint chain and a volume builder.

## Done when
- [x] `App::new_default()` registers the volume types; creating a Volume Builder with a cube child and a
      Volume Mesher above it through `object.create` / the RPC yields a mesh equal to `mesh_volume_node`.
- [x] `joint.create_chain`, `joint.align`, `joint.mirror` run through the command registry (CLI, RPC, MCP),
      are undoable, and are in the generated reference.
- [x] The Create menu shows the new types; `icons` tests pass.
- [x] The editor draws joints, bones, weight markers and volume boxes (headless draw-list test).
- [x] The gate passes, BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: all five boxes done. The evaluation hook already existed (`Registry::set_evaluator`, PLUGINS.md §8:
  `forge_generators::evaluate` dispatches to a plugin evaluator before its own match, no crate cycle) and
  `VolumePlugin` sets the mesher's; the missing step was installing `VolumePlugin` in `default_registry`
  (forge-app gained the `forge-volume` dependency). Builder and Filter have no evaluator (no geometry of their own).
- Commands: `joint.create_chain`, `joint.align`, `joint.mirror` in `crates/app/src/rig_commands.rs` (forge-app is
  the crate above both rig and command), registered by `default_command_registry`, one undo step each, in
  `docs/manual/reference`. Spec RIG.md §7, COMMANDS.md.
- UI: Create > Generator lists Volume Builder / Filter / Mesher (registry driven); new Create > Character submenu
  (`character_menu`: Joint, Skin, Weight, Align Joints, Mirror Joints; those types are skipped in their category
  submenus). Three new SVG icons (`volume_*`) and icon-table rows for them and the joint commands; the Joint, Skin
  and Weight icons already existed.
- Display: `forge_ui::scene_overlay` (`SceneOverlay::collect`, `VolumeViews`) feeds joints and bones, the weight
  squares of selected joints and the box of each mesher through the pane's existing line set (live panel and
  screenshots). Volume boxes come from `PreviewWorker` (one per mesher, never blocks). Spec VIEWPORT.md §13.4, §14.
- Tests: `crates/app/tests/rig_volume.rs` (4), `crates/ui/tests/scene_overlay.rs` (6, headless), icons suite.
- Not done (parity-pending, own design): slice markers and the coarse preview mesh are not drawn; weights show as
  squares, not a colour surface. Generators as clone / volume inputs is brief 048 (the builder's cube child is still
  drawn as its own object until then).
