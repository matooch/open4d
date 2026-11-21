# 027 · Selection tags, vertex map tags, restriction tags

Status: done (was waiting: O54) · Area: B · Depends on: 023 · Observations: O54 · Touches: core, generators, ui, modifiers

## Goal
Stored selections and vertex weights exist as tags, and deformers can be restricted by them.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O54: [../../parity/tags-2.md](../../parity/tags-2.md) and its golden cases
- docs/parity/tags-2.md (O54 point/polygon/edge selection and vertex map cases)

## Design decisions (planner)
- Tag node types `forge.selection_points|edges|polys` (IndexList param) and `forge.vertex_map` (`Value::FloatList` per point; add the Value variant). Commands `select.store` and `select.restore`.
- Restriction tag: names a selection or vertex map. Deformers multiply their effect by the weights (0/1 for selections). This hooks into `apply_deformer`.
- Topology changes (modeling tools) remap or clear the tags. Document the rule: indices are kept when the topology is unchanged, and tags are cleared otherwise.

## Done when
- [x] EDITABLE.md §7 (the brief's §6 number was taken by brief 034).
- [x] O54 selection and vertex map cases match (4/4).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Vertex maps from fields (062).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 4/4 O54 cases exact at level A (point-selection, polygon-selection, edge-selection, vertex-map-tag) in `crates/app/tests/tag_commands.rs` through `select.store`, `select.restore`, `vertexmap.create`; `UNSUPPORTED` and `KNOWN_MISMATCH` empty. Further tests: `crates/modifiers/tests/restriction.rs` (4), `crates/core/tests/materials.rs` (float lists, restriction weights), `tag_commands.rs` (store/restore ops and undo steps, vertex map set, restriction command, topology clearing). The other five tags-2 cases belong to briefs 106 and 110.
- Rules: stored selections are tags holding `indices`; a vertex map holds one weight per point (`FloatList`, 0..1); a restriction tag on a deformer names a point selection or vertex map tag of the deformed object, weights blend `p0 + w (p - p0)` (EDITABLE.md section 7). Parity-pending: all restriction rules (no observation), vertex map colour default, topology clearing.
- Deviations: tag ids are `forge.point_selection`, `forge.edge_selection` (plus the existing `forge.polygon_selection`) instead of the proposed `forge.selection_*`, to follow the existing tag; the restriction tag names its target by tag name (survives Make Editable) instead of a link; spec section is EDITABLE.md section 7; the file version bump to 12 is shared with brief 194 (CORE.md 6.11, `scene-v12*` fixtures, v11 fixture migration test); `select.restore` also switches the edit mode to the tag's kind. Explosion and Shatter ignore restriction tags (point count changes). Topology rule: `clear_stale_tags` is called by the cutting, building and modeling-tool writers (indices kept while the polygon lists and point count are unchanged).
- Files: crates/core (tags.rs, param.rs, file.rs, scene.rs, document.rs), crates/modifiers/src/lib.rs, crates/generators (clear_stale_tags), crates/app/src/tag_commands.rs, crates/command builtin writers, crates/ui (Select menu: Set Selection, Restore Selection; icon table, attribute read-only summary), docs/specs (CORE, EDITABLE, COMMANDS), docs/manual/reference.
- Open: a restriction tag observation (which target kinds, inversion, whether it multiplies fields); UI for editing vertex maps (a paint tool is out of scope).
