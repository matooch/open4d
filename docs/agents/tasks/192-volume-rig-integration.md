# 192 · Volume and rig nodes in the editor: evaluation hook, commands, display

Status: ready · Area: F, H · Depends on: 081, 083, 100, 101, 112, 084 · Observations: – (integration; parity is already covered by the crate tests) · Touches: generators, app, command, ui, viewport, volume, rig

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
- [ ] `App::new_default()` registers the volume types; creating a Volume Builder with a cube child and a
      Volume Mesher above it through `object.create` / the RPC yields a mesh equal to `mesh_volume_node`.
- [ ] `joint.create_chain`, `joint.align`, `joint.mirror` run through the command registry (CLI, RPC, MCP),
      are undoable, and are in the generated reference.
- [ ] The Create menu shows the new types; `icons` tests pass.
- [ ] The editor draws joints, bones, weight markers and volume boxes (headless draw-list test).
- [ ] The gate passes, BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
