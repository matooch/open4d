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

## 6. S10 real-time render frame (R8, brief 204)

Sources: [S10](../roadmap/README.md), [R8 and the reference machines](../roadmap/areas/R-realtime.md),
[D19](../roadmap/DECISIONS.md), and the existing S10 workload in
`crates/app/benches/s10.rs`. No reference-app data or renderer is involved.

- Use one deterministic snapshot for both tiers: a 1000×1000 quad surface
  (2,000,000 input triangles) and a 100×100 grid of clones of one shared cube
  mesh (120,000 more rendered triangles). Frame the grid and surface; use one
  shadow-casting light plus an environment. Do not downscale the scene, trim
  coverage, replace clones with unrelated meshes, or report a partial frame as S10.
- Time a complete 1920×1080 render, with no region, extra AOVs, or temporal
  antialiasing. Tier 1 uses the raster path; Tier 2 enables all four R24 effects.
  Verify the backend that supplied Tier 2 (Metal on M5 Pro; Vulkan on RTX 3090).
  A compute run is useful diagnostics but not a reference-machine measurement.
- Build the fixture and open the adapter before timing. Reuse the same engine
  across warmups and samples; report the median of each pass and of complete
  frames independently. The targets apply to complete warm frames: Tier 1
  ≤ 1000/60 ms, Tier 2 ≤ 1000/30 ms. Report measured shortfalls as such.
- Report setup/packing separately from the shadow atlas, G-buffer, each
  requested hybrid effect, lighting and output/readback. Pass timings are
  synchronized host wall times, not GPU-only timestamps; the uninstrumented
  render supplies the complete frame metric so extra profiling waits do not
  reduce its reported fps. End each measured GPU phase only after its submitted
  work completes. Profiling must produce the same image as the normal path.
- Compare like runs only: include the adapter, API, actual hybrid backend,
  workload version, resolution and sampling protocol in the baseline identity.
  Compare each corresponding median, including the complete frame. A current
  median above `baseline × 1.10` is a regression; exactly 10% is allowed.
  Missing baseline passes and incompatible run identities are errors, not
  skipped comparisons. Keep this same-machine check separate from the existing
  CI CPU budgets and from the absolute fps targets.
- Use two independent series per tier (uninstrumented frames, then profiled
  frames), each with its own engine, two warm-up frames and five timed frames.
  Output nanosecond medians as integers for comparison; use a caller-supplied
  machine label to distinguish hosts with identical adapter names.
- Device limits are part of the result: request sufficient limits where the
  adapter advertises them, and report unsupported workloads explicitly rather
  than attempting an invalid GPU allocation. The current flattened raster
  triangle table alone needs 373,120,000 bytes at this scale (176 × 2,120,000);
  default storage binding limits cannot be assumed sufficient.

Run on an idle reference machine, using a separate baseline and output file:

```sh
WGPU_BACKEND=metal cargo bench -p forge-render-rt --bench s10 -- m5-pro tier1 tier1-baseline.json
WGPU_BACKEND=metal cargo bench -p forge-render-rt --bench s10 -- m5-pro tier2 tier2-baseline.json
WGPU_BACKEND=metal cargo bench -p forge-render-rt --bench s10 -- m5-pro tier2 tier2-current.json tier2-baseline.json
```

Use `compute` in place of `tier2` to explicitly allow the compute backend for
diagnostics. The FPS target is reported separately; the optional baseline
check returns a nonzero status on incompatible identity, missing timing, or
regression and leaves the current JSON available for inspection. Hardware
Vulkan is pending brief 212; a Tier 2 run does not substitute compute.

### 6.1 Persistent scene data (brief 214)

Sources: [brief 214 and its S10 baseline](../agents/tasks/214-realtime-performance.md),
[CORE dirty evaluation and sharing](CORE.md), [wgpu buffer usages](https://docs.rs/wgpu/latest/wgpu/struct.BufferUsages.html).
These are internal performance rules; they do not change the renderer's reference behavior.

- Retain immutable geometry resources across renders on the same device. Separate mesh
  data (positions, triangle indices, normals and UVs), instance data (mesh reference,
  transform, material index and object ID), and frame inputs. Use the evaluated geometry
  identity, not the document revision alone: animation can change a transform at the same
  revision, and unrelated documents can have the same revision number. Shared immutable
  data can use pointer identity; mutable snapshot data needs a value comparison unless
  the producer supplies a trustworthy dirty marker.
- A camera, light, environment, material graph or render-settings edit does not invalidate
  mesh or instance resources. Per-frame validation must still catch invalid references,
  e.g. after removal of a material. A transform edit updates the affected instance data
  and RT top-level bounds; it does not rebuild unchanged mesh BLASes. Mesh edits invalidate
  the affected mesh and its dependent instance bounds. Additions, removal and reordering
  must not leave stale indices or IDs.
- First checkpoint: keep the existing raster triangle layout but reuse its uploaded
  buffer and precomputed shadow bounds while mesh and instance inputs match. This is
  whole-geometry invalidation, not per-mesh instancing. Keep packing semantics, draw order
  and all AOVs unchanged. Cached frames after edits must equal freshly rendered frames
  for the same snapshot, including object and material IDs. Test reuse with a work count
  or resource identity, not a wall-clock assertion. An exact match to previously validated
  mesh inputs can skip scanning mesh indices and positions a second time; recheck instance
  references and camera against the current frame. Validate changed geometry before upload.
- Native ray queries retain one BLAS and its input buffers per nonempty mesh slot. At
  each scene update, compare positions and triangle indices exactly in that slot;
  vertex attributes, revisions and instance edits do not replace matching BLASes.
  Build and upload only changed or newly added nonempty slots. Empty and removed
  slots keep no BLAS; reordering may rebuild moved meshes. Build the new TLAS with
  both reused and newly built handles, preserving snapshot instance and primitive
  IDs. Install replacements and retire old resources only after validation succeeds.
  `blas_builds` counts successful mesh-layout/geometry update batches (including
  initial setup and removals), not individual BLASes. Test actual handle/buffer
  reuse and query results across edits and a rejected update. This checkpoint does
  not make the Compute or raster geometry packing selective.
- Report unchanged-scene S10 per §6, with the same fixture and all four Tier 2 effects.
  Separately report camera-only setup (target <5 ms); do not use this phase timing as a
  substitute for frame throughput. Any Tier 2 estimator changes require their own R24
  contract and a rerun of the R26 quality matrix before claiming a performance win.

### 6.2 Demand-driven raster output (brief 214)

Sources: [R21.6 raster outputs](RENDER.md#r216-outputs-aovs-and-limits),
[R24 primary data](RENDER.md#r24-hybrid-ray-traced-effects), and the
[S10 checkpoint](../agents/tasks/214-realtime-performance.md#report-implementer-fills-in).

- Final output always reads the selected beauty attachment (lit or SSR). Read shadow
  only for Shadow. Read G0 for Depth, Normal, Position, ObjectId, MaterialId, Shadow
  or Emission, and G1 only for Normal or ObjectId. If hybrid already read primary
  attachments, use those arrays rather than copying them again. The hybrid input
  readback is independent of which output AOVs the caller requests.
- Read both light-split attachments for Diffuse, Specular or Emission, and the
  albedo attachment only for Albedo. Emission uses pre-postprocessing beauty.
- Beauty and each AOV must match the same render with all AOVs requested, including
  outside a crop. Preserve alpha metadata, statistics and the order and values of
  callback events. Gather only the postprocessed crop rows for the Tile event,
  and do not allocate a Tile payload if there is no callback. Beauty-only output
   does not iterate over AOV pixels.

### 6.3 Compiled raster programs (brief 214)

Sources: [R21 raster tier](RENDER.md#r21-real-time-raster-engine),
[wgpu pipeline layouts](https://docs.rs/wgpu/latest/wgpu/struct.PipelineLayoutDescriptor.html)
and the [S10 setup target](../agents/tasks/214-realtime-performance.md#steps-spec-first-measure-after-each-step-on-the-m5-pro-record-in-the-report).

- Keep one successfully compiled raster program per engine/device, shared by engine
  clones. Compare the complete assembled WGSL and the number of bitmap bindings; a
  change in either replaces the program and its bind group layouts. A failed
  compilation must not replace a valid entry.
- Assembling the current materials and resolving and binding their current bitmap
  views remain frame work. Identical WGSL does not imply identical texture content.
  Camera, light, resolution and pass selection are not keys when source and layout
  match. A frame may retain its program after another frame replaces the cache slot.
- Check program identity without timing assertions and compare warm and cold
  rendered output across material and bitmap edits. Measure full-size S10 and
  camera-only setup separately, as in §6.1.
