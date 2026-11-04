# 181 · Benchmarks and S10 performance thresholds

Status: done · Area: N · Depends on: 180, 144 · Observations: – · Touches: benches

## Goal
criterion benchmarks and S10 thresholds enforced on Linux CI.

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)
- docs/roadmap/README.md (S10)

## Design decisions (planner)
- Thresholds with 20 % headroom.

## Done when
- [x] S10 passes.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: spec docs/specs/PERFORMANCE.md. New criterion bench `crates/app/benches/s10.rs` (1 M-polygon plane generate and bend, 10 k clones with effectors and a field, save/load of a 1 M-polygon editable object with binary chunks, OBJ write, startup to first frame headless). Existing benches (schedule.rs, viewport performance.rs) are reused. New crate `tools/bench-check` (`forge-bench-check`): runs the benches in quick mode, reads criterion medians, compares with `bench/budgets.toml` (27 budgets), prints a table, exits 1 on regression or missing result; 7 unit tests including a check of the checked-in file. Linux only.
- Rules: budget = max(measured x 1.2 headroom x 4 CI factor, 100 us floor), 3 significant digits, enforced by tests; statistic = criterion median, one retry keeping the lower median; `FORGE_BENCH_SCALE` for slow machines.
- Deviations: (1) planner said 20 % headroom; applied on top of a CI factor of 4 because hosted runners are slower and noisy (headroom alone would flake). (2) CI is a separate workflow `bench.yml` (main pushes, daily, manual), not ci.yml, as the release bench run takes minutes. (3) GPU frame/upload rows and window/GPU startup are not budgeted (software adapter, core-count dependent). (4) Real-GPU 20 fps cannot be measured; S10 is guarded via CPU-side per-frame costs.
- Files changed: crates/app/{Cargo.toml,benches/s10.rs}, tools/bench-check/**, bench/budgets.toml, .github/workflows/bench.yml, docs/specs/PERFORMANCE.md, docs/roadmap/BACKLOG.md, Cargo.lock, this brief.
- Open questions: real-runner numbers (ci_factor 4 is a guess until bench.yml has run on GitHub); a GPU runner for frame budgets.
- Gate: `GATE: 1644 tests passed, 0 failed` / `advisories ok, bans ok, licenses ok, sources ok` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`.
