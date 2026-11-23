# 078 · Clone selection and weight tags, clone cache

Status: done (parity-pending: O50 tag and cache captures) · Area: E · Depends on: 065, 027 · Observations: O50 · Touches: clone-engine

## Goal
Tags that restrict effectors to selected clones, store per-clone weights, and bake clone data.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O50: [../../parity/clone-toolset-3.md](../../parity/clone-toolset-3.md) and its golden cases
- docs/parity/clone-toolset-3.md

## Design decisions (planner)
- Clone selection tag: a set of clone indices; effectors with that selection only act on those.
- Weight tag: per-index weights feeding CloneData.weights.
- Cache: bake CloneData per frame into the document (binary blob in the native file, 163).

## Done when
- [ ] O50 tag cases pass (pending: usable reference scenes).
- [x] A baked cache plays back identically.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- **Parity pending:** O50 lists `cloner/selection-tag` and `cloner/weight-tag` as not
  observed. The tags read back correctly in the probe but had no effect; neither has
  a golden fixture. No reference cache capture exists either. No parity is claimed.
- **Native coverage:** tests assert host-local selection, empty and unresolved masks,
  selected and weighted Plain displacements, undo/redo, cached viewport instances and
  geometry, native binary file round trip, rebaking, and all `CloneData` channels
  across frames with different sample counts.
- **Design:** the three tags are registry nodes. `cloner.bake` samples the animated
  scene and stores a versioned `FloatList` using the existing native chunk format.
  The bake is exposed through the command registry, not a dedicated editor panel.
- **Follow-up:** capture GUI-built selection/weight tag scenes and a before/after
  bake sample. On integration with 071, keep selection masking around the expanded
  effector dispatcher and keep initial weight seeding in Delay replay.
- **Files:** MOGRAPH §O5; clone-engine `tags`, `cache`, `effector`, and Plain;
  generator `cloner`; app `clone_commands`, scheduler, and `clone_tags_cache` tests;
  UI icon mappings and generated command reference.
- **Gate:** `GATE: 2453 tests passed, 0 failed`; `GATE: all passed`.
