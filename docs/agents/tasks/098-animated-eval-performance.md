# 098 · Animated evaluation performance (dependency-aware re-evaluation)

Status: done · Area: G · Depends on: 180 · Observations: – · Touches: core, anim

## Goal
Playback re-evaluates only animated nodes and their dependants.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)

## Design decisions (planner)
- Use the scheduler from 180; mark nodes with tracks as time-dependent; propagate dirtiness through the dependency graph per frame.

## Done when
- [x] Benchmark: a scene with 1 animated cube and 1000 static ones plays at ≥ 10× the full-evaluation rate.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: spec `docs/specs/CORE.md` section 5b (change set, sharing, outputs for drawing, new benchmark table) and `docs/specs/VIEWPORT.md` section 10.8 (scheduled drawing). Playback of 1 animated cube + 1000 static: 192 us per frame serial / 101 us on 4 threads, against 29.0 ms for the old full evaluation (151x / 287x; target 10x) and 6.2 / 6.6 ms for brief 180. Tests added: `crates/app/tests/schedule.rs` (one more test, and every scheduler now runs with `with_validation(true)`: each change-set pass is checked against the bit-exact scene diff; path assertions), `crates/core/tests/depgraph.rs` (node sharing), `crates/ui/tests/scheduled_viewport.rs` (2: scheduled draw items equal `scene_draw_items_at` bit for bit over 11 frames incl. instanced cloner, selection; slot cache rebuild counts). No parity cases (own design); all 471 golden cases and brief 180's equality tests are unchanged and green.
- Design: (1) Change set: tracks (`forge_anim::overrides` over the cached list of track nodes) and expression tags (`forge_nodes::apply_expressions_in`, which returns every written (node, param)) report what they touched; when only the time moved, seeds come from comparing those pairs (previous and current pass) bit-exactly, no scene diff; the diff remains for edits/first pass and as the validator. (2) Sharing: `Scene.nodes` is `BTreeMap<NodeId, Arc<Node>>`, copy on write; `Scene::apply_params` writes in place; `schedule::animate` clones the scene (one pointer per node) and copies only written nodes; `Evaluated` uses the same function. (3) The scheduler also keeps node lists per document revision, updates time readers only for changed nodes, reuses the graph, the drawn list (unless a dirty node or its children can change drawn state) and the output map (clone of pointers).
- Scheduler users: the editor viewport (`App::scheduled_draw` -> `ViewportPanel` `SlotCache`: only changed nodes are rebuilt/hashed/uploaded; the 144 upload cache stays underneath), `viewport.screenshot` (`render_png`), `Evaluated` (the editor's evaluated scene: shares the animate stage). `App::scheduled` is unchanged.
- Deviations: (1) OBJ export stays on `export_scene` over the `Evaluated` scene: it exports hidden objects and evaluates with `EvalCtx::default()`, so scheduler outputs (drawn set, frame context) would not be bit-identical; it benefits only from the cheaper animate stage. (2) While a live tool preview is shown the viewport draws from scratch (the scheduler does not know the preview mesh). (3) `NodeOutput` lost `PartialEq` (it now may hold `ClonerInstances`); a second scheduler (`draw_scheduler`) serves instanced output so 180's expanded-mesh outputs and tests are untouched. (4) Full evaluation (first pass) is still 36 ms serial vs 29 ms for the old path (graph build, bookkeeping), 11.9 ms on 4 threads. (5) `Frame` gained `generation` and `drawn`, `Stats` gained `path`.
- Files changed: crates/core/src/scene.rs, crates/core/tests/depgraph.rs, crates/anim/src/{lib,scene_at}.rs, crates/nodes/src/{lib,expression}.rs, crates/app/src/{lib,animate,schedule}.rs, crates/app/tests/schedule.rs, crates/ui/src/{viewport_panel,app,screenshot}.rs, crates/ui/tests/scheduled_viewport.rs, docs/specs/{CORE,VIEWPORT}.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions: a frame-sequence export through the scheduler (needs an agreed export context: hidden objects, time); real-GPU frame times of the slot cache; first-pass graph build cost.
- Gate: see the commit message.
