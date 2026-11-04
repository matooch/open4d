# Performance budgets (S10) — brief 181

Sources: project requirement S10 (docs/roadmap/README.md); the benchmark numbers recorded in
docs/specs/CORE.md §5b (briefs 180/098) and docs/specs/VIEWPORT.md §10.10 (brief 144); criterion
(public documentation). No reference-app behaviour is involved.

## 1. S10 and what covers it

S10: "open a scene with 1 M polygons and 10 k clones, stay interactive (>= 20 fps viewport on a
mid-range GPU)". Twenty fps is 50 ms per frame. We cannot measure a real GPU in CI, so S10 is
guarded through the pieces that run on the CPU every frame or on every open/edit. (The 30-minute
fuzzed session of S10 is brief 182.)

| S10 element | Benchmarks (criterion ids) | File |
|---|---|---|
| 1 M-polygon generator, deformer | `s10_generate/plane_1M_polys`, `..._bend` | crates/app/benches/s10.rs |
| generator/clone/field/effector evaluation at scale (10 k clones) | `s10_clones/fields_10k_{serial,parallel}`, `cloner_10k_clones/{serial,parallel}` | s10.rs, schedule.rs |
| save / load with binary chunks (1 M polygons) | `s10_file/{save,load}_1M_polys_chunked` | s10.rs |
| OBJ export | `s10_export/obj_write_1M_polys` | s10.rs |
| open a big scene to the first frame, headless | `s10_startup/open_1M_polys_to_first_frame`, `empty_app_to_first_frame` | s10.rs |
| playback (scheduler) | `animated_cube_plus_1000_static/*` | crates/app/benches/schedule.rs |
| viewport draw list, instance layout, cache keys, picking, 1 M-triangle content hash | `instances/*`, `mesh/revision_1M_tris` | crates/viewport/benches/performance.rs |

Not budgeted: the GPU rows of performance.rs (upload and frame) run on the runner's software
adapter and depend on its core count; they stay informational (`cargo bench -p forge-viewport`).
"Startup to first frame" covers the headless part (application core, file open, first scheduled
draw data); window and GPU setup are not measured.

## 2. Budget rule

`bench/budgets.toml` lists, per benchmark, `measured_us` (median on the reference machine, the
development container: 4 cores, llvmpipe) and `budget_us`:

`budget_us = max(measured_us * headroom * ci_factor, floor_us)`, rounded up to 3 significant digits,
with `headroom = 1.2` (brief 181: 20 %), `ci_factor = 4` (hosted runners are slower and noisier) and
`floor_us = 100` (below this the timer and scheduler noise dominates, e.g. the draw list at 0.1 us).

The statistic is the **median** of criterion's samples (10 to 100 per benchmark; `estimates.json`
`median.point_estimate`). `forge-bench-check` retries a failing pass once and keeps the lower median
per benchmark. `FORGE_BENCH_SCALE` (or `--scale`) multiplies all budgets for an unusually slow machine.
Every budget is also far inside S10: the per-frame rows (draw list, cache keys, picking, scheduler
playback) are at most a few ms against the 50 ms frame.

## 3. Runner and CI

`cargo run --release -p forge-bench-check` (from the repository root, Linux only: it prints
"skipped" elsewhere unless `--force`) runs `cargo bench` for `forge-app` (`schedule`, `s10`) and
`forge-viewport` (`performance`, CPU rows only) with `--warm-up-time 1 --measurement-time 2`, reads
`target/criterion/*/new/estimates.json`, prints a table and exits 1 when a median exceeds its budget
or a budgeted benchmark did not run. Options: `--no-run` (check existing results), `--markdown`,
`--suggest` (budget-file text for the current medians, following the rule), `--scale F`, `--retries N`,
`--budgets PATH`. Unit tests (`tools/bench-check/tests/check.rs`) pin the rule, the comparison, and
that the checked-in file keeps >= 20 % headroom and is no looser than the rule, so a budget cannot be
quietly relaxed: raising one means changing `ci_factor` or re-measuring, in a visible diff.

CI decision: a separate workflow, `.github/workflows/bench.yml` (Linux, pushes to main, daily, manual),
not the per-commit `ci.yml`: release builds of the benches plus about ten minutes of running are too
slow for every push. The fast, always-on part is the unit tests of the checker in `cargo test`.

New dependencies: none beyond those already in the workspace (`serde`, `serde_json`, `toml`;
`criterion` as before, `default-features = false`); `cargo deny check` passes.

## 4. Measured table

Development container, release build, `cargo run --release -p forge-bench-check`. "used" is median
over budget.

| benchmark | median | reference | budget | used | result |
|---|---|---|---|---|---|
| `s10_generate/plane_1M_polys` | 67.91 ms | 67.91 ms | 326.00 ms | 21 % | ok |
| `s10_generate/plane_1M_polys_bend` | 57.96 ms | 57.96 ms | 279.00 ms | 21 % | ok |
| `s10_clones/fields_10k_serial` | 32.98 ms | 32.98 ms | 159.00 ms | 21 % | ok |
| `s10_clones/fields_10k_parallel` | 29.89 ms | 29.89 ms | 144.00 ms | 21 % | ok |
| `s10_file/save_1M_polys_chunked` | 105.81 ms | 105.81 ms | 508.00 ms | 21 % | ok |
| `s10_file/load_1M_polys_chunked` | 175.89 ms | 175.89 ms | 845.00 ms | 21 % | ok |
| `s10_export/obj_write_1M_polys` | 1.57 s | 1.57 s | 7.55 s | 21 % | ok |
| `s10_startup/empty_app_to_first_frame` | 569.47 us | 569.47 us | 2.74 ms | 21 % | ok |
| `s10_startup/open_1M_polys_to_first_frame` | 252.58 ms | 252.58 ms | 1.22 s | 21 % | ok |
| `animated_cube_plus_1000_static/full_serial_every_frame` | 20.41 ms | 20.41 ms | 98.00 ms | 21 % | ok |
| `animated_cube_plus_1000_static/dirty_only_serial` | 164.13 us | 164.13 us | 788.00 us | 21 % | ok |
| `animated_cube_plus_1000_static/dirty_only_parallel` | 97.15 us | 97.15 us | 467.00 us | 21 % | ok |
| `animated_cube_plus_1000_static/full_scheduler_serial` | 33.21 ms | 33.21 ms | 160.00 ms | 21 % | ok |
| `animated_cube_plus_1000_static/full_scheduler_parallel` | 11.29 ms | 11.29 ms | 54.30 ms | 21 % | ok |
| `cloner_10k_clones/serial` | 7.00 ms | 7.00 ms | 33.70 ms | 21 % | ok |
| `cloner_10k_clones/parallel` | 5.11 ms | 5.11 ms | 24.60 ms | 21 % | ok |
| `instances/build_layout/10000` | 2.62 ms | 2.62 ms | 12.60 ms | 21 % | ok |
| `instances/build_layout/100000` | 38.78 ms | 38.78 ms | 187.00 ms | 21 % | ok |
| `instances/revision/10000` | 416.63 us | 416.63 us | 2.00 ms | 21 % | ok |
| `instances/revision/100000` | 4.09 ms | 4.09 ms | 19.70 ms | 21 % | ok |
| `instances/draw_list_all/10000` | 0.10 us | 0.10 us | 100.00 us | 0 % | ok |
| `instances/draw_list_all/100000` | 0.92 us | 0.92 us | 100.00 us | 1 % | ok |
| `instances/draw_list_close/10000` | 0.09 us | 0.09 us | 100.00 us | 0 % | ok |
| `instances/draw_list_close/100000` | 0.46 us | 0.46 us | 100.00 us | 0 % | ok |
| `instances/pick/10000` | 687.04 us | 687.04 us | 3.30 ms | 21 % | ok |
| `instances/pick/100000` | 7.18 ms | 7.18 ms | 34.50 ms | 21 % | ok |
| `mesh/revision_1M_tris` | 6.62 ms | 6.62 ms | 31.80 ms | 21 % | ok |
bench-check: all 27 benchmarks within budget (scale 1)

Informational (not budgeted), from VIEWPORT.md §10.10: instanced frame at 320 x 240 on llvmpipe 25 ms
(10 k clones); 1 M-triangle `MeshBuffers` upload about 1 s (once per content change).

## 5. Regenerating

1. `cargo run --release -p forge-bench-check -- --suggest` runs the benches and prints new
   `[[budget]]` blocks; paste them into `bench/budgets.toml` only for a deliberate re-measurement
   (a faster or slower machine, a new benchmark) and say why in the commit message.
2. `cargo run --release -p forge-bench-check -- --no-run --markdown` prints the table above.
3. Adding a benchmark: add the bench, add its id to `bench/budgets.toml` (the runner fails on a
   budgeted id that did not run), re-run step 1.
