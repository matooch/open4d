# 110 · User data on any node

Status: done · Area: H · Depends on: 011, 150 · Observations: O54 · Touches: core, ui

## Goal
Add custom parameters (float, int, bool, vector, link, string) to any node.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- O54: [../../parity/tags-2.md](../../parity/tags-2.md) and its golden cases
- docs/parity/tags-2.md (user-data notes)

## Design decisions (planner)
- User data is extra ParamDescs stored on the node; UI/undo/animation/serialization derive as usual.

## Done when
- [x] User data animates and survives save/load.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

Status: done

Gate: all passed, 2429 tests (sh scripts/gate.sh).

Added per-node user data as `Node.user_data: Vec<ParamDesc>` with reserved `user_data.`
parameter ids. `Scene::param_desc` searches type params then user data, so `set_param`,
`set_track`, animation overrides and the attribute panel all treat user-data params like
declared ones. `Tx::add_user_data` appends a descriptor, auto-assigns `user_data.N`, and
initialises the value; undo removes the last descriptor and its value/tracks. The native
scene file carries the descriptors in `NodeRecord.user_data` (version 13); old files load
with an empty list. Added `user_data.add` command supporting float, int, bool, vec3, link
and string kinds; updated `param.edit` and the attribute panel to resolve user data.

Tests: core user-data create/set/save/load/animation/undo, app command API, UI attribute
panel row, and the O54 golden case `tests/golden/tags-2/user-data` (1/1 exact: float
"amount" descriptor with default 1.0 and range 0..10, link "target" pointing at the cube,
both values read back; exported cube mesh matches `mesh.obj`). STATUS.md updated.

Files changed: crates/core/src/{scene.rs,document.rs,file.rs,param.rs,registry.rs,
user_data.rs,lib.rs}, crates/core/tests/{user_data.rs,native_format.rs}, crates/core/tests/
fixtures/scene-v13*.forge, crates/app/src/{user_data_commands.rs,param_edit.rs,lib.rs},
crates/app/tests/user_data_commands.rs, crates/ui/src/{attributes.rs,icon_table.rs},
crates/ui/tests/attribute_manager.rs, docs/specs/CORE.md, docs/manual/reference/*,
docs/roadmap/BACKLOG.md.

Open questions: removal/editing of user-data descriptors is not in this brief; command
for it can be added when observations need it.

Co-Authored-By: kimi-k2.7-code <kimi-k2.7-code@nousresearch.com>
