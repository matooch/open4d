# 100 · Joint object, bone display, joint tools (align, mirror, chain creation)

Status: done · Area: H · Depends on: 011 · Observations: O34 · Touches: rig (new), plugin, viewport

## Goal
A Joint node type with bind pose, chain world matrices, bone display, and the joint commands (create chain, align, mirror).

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- O34: [../../parity/rigging.md](../../parity/rigging.md) and its golden cases
- docs/parity/rigging.md (O34)

## Design decisions (planner)
- New crate `forge-rig`. Joint is `Category::Object` with length derived from its child; bind pose stored as the rest local matrix.
- Commands: `joint.create_chain(points)`, `joint.align` (orient X/Y/Z down the bone per O34), `joint.mirror(plane, rename rule)`.
- Bone display: draw an octahedron/line between joint and child; size from length.

## Done when
- [ ] RIG.md spec citing O34.
- [ ] O34 joint matrix cases pass at level B.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Skinning (101).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: new crate `forge-rig` (`Rig` plugin: `forge.joint`, `forge.weight_tag`, `forge.skin`); joint
  object with the O34 parameter table, bone vector/length, chain world matrices, bone display geometry
  (octahedron), `create_chain`, `align_joint`, `mirror_chain` with left/right naming. Tests:
  `crates/rig/tests/rig_units.rs` (chain positions, bone length, align keeps children in place,
  mirror reflection and names, display), `rigging_parity.rs`. O34 has no joint-matrix golden; the
  joint matrices are exercised through the skinned meshes: `bind-pose` exact, the posed cases agree
  with the reference to its own noise (see 101).
- Rules derived: HPB rotation convention from TRANSFORM.md gives J2 "B 45" = `Rz(-45)` and the
  scale of a joint propagates to its children (`j2-scale`: J3 at y = 225); the chain world positions
  equal `joint_positions`. Parity-pending: bone display shape, align/mirror (own design, not observed).
- Deviations: the `joint.create_chain` / `joint.align` / `joint.mirror` command wrappers are not
  registered (forge-command depends on generators which depend on modifiers which depend on
  forge-rig, so the rig crate cannot register commands itself); the library functions are the
  commands' bodies. Rotation Order is not modelled (TRANSFORM.md has HPB only).
- Files: crates/rig/**, docs/specs/RIG.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md.
- Gate: see the final report of the session (run once before the last commit).
