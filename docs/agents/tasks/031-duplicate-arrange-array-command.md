# 031 · Duplicate, Arrange, Randomize, Transfer, Array command

Status: done (parity-pending: no data; G3 and the array-command probe) · Area: B · Depends on: 011 · Observations: G3 · Touches: ui, modeling

## Goal
Object arrangement commands exist and are scriptable.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- GUI session G3 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- docs/parity/modeling-tools-2.md (array-command note: field mapping TODO)

## Design decisions (planner)
- Commands `object.duplicate {count, offset, rotation}`, `object.arrange`, `object.randomize {seed, ranges}`, `object.transfer {target}` and `mesh.array_command`. All are own design, parity-pending, and deterministic with a seed.

## Done when
- [x] COMMANDS.md updated; unit tests.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added; parity 0/0 (no golden data exists for these commands; `UNSUPPORTED` / `KNOWN_MISMATCH` do not exist). Tests: `crates/modeling/tests/arrange.rs` (8: step rule both modes, array order/closedness/winding/rotation/cap, arrange line/grid/circle, randomize draws, descriptor defaults), `crates/app/tests/arrange_commands.rs` (12: one undo step each, deep copy with tags/children/link remap, determinism and node independence of Randomize, world/local Transfer under a rotated parent, argument errors, schemas), `crates/ui/tests/arrange.rs` (5: tool list, Apply is one undo step, Transfer target from the selection, Array preview and apply, Mesh menu and panel), one Rhai script in `crates/script/tests/script.rs`.
- Commands: `object.duplicate {nodes?, copies, offset, rotation_deg, scale, mode step|total}`, `object.arrange {nodes?, mode line|grid|circle, spacing, row_spacing, columns, radius, plane, start_angle_deg}`, `object.randomize {nodes?, seed, position, rotation_deg, scale, uniform_scale}`, `object.transfer {target, source?, position, rotation, scale, space world|local}`, `mesh.array_command {node?, polys?, settings: {copies, offset, rotation_deg, scale, mode}}`. The settings come from `ToolDesc` descriptors (`array_descs`, `object_descs`), so schema, panel, defaults and clamping share one definition. Rules: MODELING.md §9.1-9.6.
- Rules (all **parity-pending**, own design; no case proves any): step/total fraction rule (`k` or `k/copies`, scale factor `max(0.001, 1 + f(S-1))`, linear like the observed cloner), copies inserted after the source with `<name>.N` names, deep copy with link remap and no copy of the root's transform tracks, line/grid/circle formulas, SplitMix64 draws keyed by `seed ^ node_id * golden` (a node's result does not depend on the others), world Transfer through the parent's inverse world matrix, Array copies appended copy after copy in the same corner order, result selection = the copies' polygons. The only observation (modeling-tools-2 `array-command`) was dropped as unmappable, so nothing is matched at level A.
- Deviations from the design decisions: `object.arrange` etc. take their numbers as top-level arguments (the object commands) while `mesh.array_command` uses `settings` (the tool framework contract); the brief's `ranges` for Randomize are the `position`, `rotation_deg`, `scale` vectors. The Mesh menu (there is no separate Tools menu) lists the five tools after the topology tools. Transfer's panel `target` is a node id (-1 = the first selected object other than the active one).
- Files changed: `crates/modeling/src/{arrange.rs,desc.rs,lib.rs}`, `crates/command/src/builtin/{arrange.rs,mod.rs,object.rs,tools.rs}`, `crates/ui/src/{tools.rs,app.rs}`, tests above, `docs/specs/{MODELING,COMMANDS,UI}.md`, `docs/manual/reference/*` (regenerated), `docs/parity/STATUS.md`, BACKLOG.md, this brief.
- Open data that would settle the pending rules: (1) whether the reference Array command's "copies" counts the original (the probe's 24 points for a cube with 3 copies suggests 3 instances in total) and the dialog's field labels for offset, rotation and scale (the probe's copies came out scaled and rotated); (2) the Array command's mode semantics (per copy vs total) and whether it acts on the whole object or the selection; (3) a GUI export of Duplicate with a position/rotation/scale offset (G3) to confirm accumulation per copy and the naming; (4) Arrange layouts and Randomize ranges/distribution with a fixed seed from the GUI (the reference RNG cannot be matched without such data); (5) Transfer's exact meaning in the reference (what is transferred, and whether it is world or local).
- The last lines of the gate output: `GATE: 1343 tests passed, 0 failed` ... `cleanroom: ok`, `ok: 471 cases`, `GATE: all passed`.
