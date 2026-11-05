# 180 · Dependency-graph evaluation scheduler (parallel, dirty-only)

Status: done · Area: N · Depends on: 012 · Observations: – · Touches: core

## Goal
Evaluate only dirty subgraphs, in parallel where independent.

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)
- docs/specs/CORE.md
- crates/core/src/cache.rs

## Design decisions (planner)
- Explicit dependency edges (children, links, fields); rayon over independent nodes; results identical to serial evaluation (test).

## Done when
- [x] Serial vs parallel equality test; benchmark.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: spec `docs/specs/CORE.md` section 5b. New `forge_core::depgraph` (two-vertex dependency graph, bit-exact scene diff, deterministic cycle breaking), `forge_app::schedule::Scheduler` (dirty-only, rayon-parallel, `Threads::{Serial, Auto, Fixed}`; `App::scheduled(frame)`), parallel cloner expansion, and three time-reader hooks. Tests added: `crates/core/tests/depgraph.rs` (8), `crates/generators/tests/schedule_hooks.rs` (3), `crates/app/tests/schedule.rs` (13: serial vs 1/4/global-pool equality over 180 scenes rebuilt from the golden cases, a hand-built MoGraph scene and 12 seeded random scenes with random edits and frames; evaluation counts for edits; time dependence; expressions; cycles; fps change). Criterion bench `crates/app/benches/schedule.rs`. No parity cases (own design); all existing tests and the 471 golden cases are unchanged.
- Rules: `Result(n)` depends on `Frame(n)`, children, tags, link/link-list targets and the sibling deformers of a geometry-less parent; `Frame(n)` depends on `Frame(parent)`. A bit-exact diff of consecutive evaluated scenes seeds the dirty set (transform/object params seed `Frame`, others `Result`), closed over dependants in the new and old graph. Time: implicit readers (Random field/effector in a noise mode with animation speed) are seeds on every frame change; tracks and expression writes are found by the diff; a scene without tracks, readers or enabled expression tags does nothing on a frame change. Expressions stay one ordered stage. Cycles: DFS in ascending id order removes the edge closing each cycle and reports it.
- Benchmark (4 cores): 1 animated + 1000 static cubes, per frame: old full serial 28.9 ms, scheduler dirty-only 6.2 ms serial / 6.6 ms parallel, scheduler full evaluation 40.0 ms serial / 14.2 ms on 4 threads; 10k-clone cloner full evaluation 7.7 ms serial / 5.4 ms parallel (parallel expansion only).
- Deviations from the design decisions: (1) the dependency edges drive dirtiness, not evaluation order: every evaluator reads only the immutable evaluated scene and recurses into its own inputs, so independent evaluation roots are all dirty drawn nodes; there is no per-node result sharing between a parent and its inputs. (2) Dirtiness is found by a diff of the evaluated scenes rather than by revision numbers, because `with_params` stamps fresh revisions every pass. (3) The parity corpus is rebuilt from the golden case files for primitives, the modeling generators, six simple deformers and the cloner modes (180 scenes); the deformer-common hierarchies, falloff fields and effector cases are covered by the hand-built and random scenes instead of re-implementing their builders. (4) Effector application stays serial. (5) `ui::viewport_panel::drawn_nodes` now calls `forge_app::schedule::drawn_nodes`; the viewport itself does not use the scheduler yet (brief 098). (6) New dependency: rayon 1 (MIT/Apache-2.0) in forge-app and forge-generators; criterion as a dev-dependency of forge-app.
- Files changed: crates/core/src/{depgraph.rs,lib.rs}, crates/core/tests/depgraph.rs, crates/generators/{Cargo.toml,src/cloner.rs,src/lib.rs,tests/schedule_hooks.rs}, crates/fields/src/{random.rs,lib.rs}, crates/clone-engine/src/{random.rs,lib.rs}, crates/nodes/src/{expression.rs,lib.rs}, crates/app/{Cargo.toml,src/{schedule.rs,lib.rs},tests/schedule.rs,benches/schedule.rs}, crates/ui/src/viewport_panel.rs, docs/specs/CORE.md, this brief, docs/roadmap/BACKLOG.md, Cargo.lock.
- Open questions: incremental animation (no scene copy per pass) and viewport/instancing integration are brief 098; whether the Random effector's per-clone loop should be made parallel (needs the weights closure to be `Sync`).
- Gate: see the commit message.
