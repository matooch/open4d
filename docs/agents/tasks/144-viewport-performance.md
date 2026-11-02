# 144 · Viewport performance: GPU instancing, LOD, large meshes

Status: done · Area: K · Depends on: 143 · Observations: – · Touches: viewport

## Goal
Draw 100k clones and 10M-triangle meshes interactively.

## Inputs
- Area design: [../../roadmap/areas/K-rendering.md](../../roadmap/areas/K-rendering.md)

## Design decisions (planner)
- Instance buffers for clones (matrix + colour); mesh upload in chunks; frustum culling.

## Done when
- [x] Benchmarks recorded in S10 thresholds (VIEWPORT.md §10.10).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: done. No golden or parity data (rendering work); spec VIEWPORT.md §10. Tests added: `crates/viewport/tests/performance.rs` (22, pure: frustum, AABB/sphere tests, Arvo transform, layout, draw lists, LOD flag, revision cache, mesh/instance revisions, LOD proxy, screen size, instance picking incl. equality with picking the expanded mesh), `tests/instancing.rs` (13, lavapipe or `FORGE_REQUIRE_GPU=1`: instanced render equals the expanded render for 8 display modes, non-uniform scale (flat), two prototypes + world matrix, rig + shadows, MSAA, colour tint, hidden clones, frustum culling stats, mesh culling, chunked upload byte-identical, LOD swap, new baseline `instanced_rig_shadows.png`), `crates/generators/tests/cloner_instances.rs` (3), `crates/ui/tests/viewport_perf.rs` (4: draw items, deformer fallback, picking cloner + clone index, upload cache counts after edits). Benchmarks: `crates/viewport/benches/performance.rs` (criterion, `default-features = false`; deny.toml passes).
- Design: cloners draw one `MeshBuffers` per prototype plus an instance buffer (3 matrix rows + colour, 64 B); instances are Morton-ordered in chunks of 2048 with chunk boxes; `visible_ranges` is the draw list (frustum culled, merged ranges); instanced pipelines for fill, edges, shadow. Upload cache `RevisionCache<NodeId, _>` keyed by content hash (+ smoothing, selection for component targets); selection colour changes upload nothing. Chunked upload: 262 144 triangles per vertex buffer, generated slice by slice, chunk boxes culled. LOD hook: `Settings.lod_px` + `MeshBuffers::set_lod` / `InstanceSet::set_proto_lod` + vertex-clustering `lod_proxy`; off by default, no UI. Picking: `pick_instances` / `manipulate::pick_object` -> `PickHit { node, clone }`; `pick_node` unchanged.
- Deviations: (1) cloners with enabled deformer children are expanded, not instanced (deformers act on the combined mesh). (2) Smooth normals of non-uniformly scaled clones are transformed, not re-averaged (spec §10.1). (3) Shadow fit uses the union of instance boxes. (4) `Frame` gained a field `instanced` (all literals updated); `scene_bounds` (frame all) still uses the expanded geometry. (5) Clone colours tint the base colour in the viewport (own design, parity-pending G8). (6) LOD has no UI switch.
- Baselines: the 16 existing images are untouched and pass; one new baseline `instanced_rig_shadows.png` was added.
- Files changed: crates/viewport/{Cargo.toml,src/{lib,cache,cull,instancing,lod,picking,render}.rs,src/render.wgsl,benches/performance.rs,tests/{instancing,performance,display,render}.rs,tests/baselines/instanced_rig_shadows.png,examples/snapshot.rs}; crates/generators/{src/{cloner,lib}.rs,tests/cloner_instances.rs}; crates/ui/{src/{viewport_panel,manipulate,screenshot}.rs,tests/viewport_perf.rs}; Cargo.lock; docs/specs/VIEWPORT.md; docs/roadmap/BACKLOG.md; this brief.
- Open questions: real-GPU frame times (no GPU in the container); whether the 262 144-triangle and 2048-instance chunk sizes should be tuned on hardware; a UI switch for `lod_px` (brief 079 or later).
- Gate: `GATE: 1172 tests passed, 0 failed` / `advisories ok, bans ok, licenses ok, sources ok` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`.
