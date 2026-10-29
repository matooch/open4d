# Phase 0 — Workspace, parity harness, golden data, clean-room check

Sources: this spec is project infrastructure, not a parity feature. It implements
docs/CLEANROOM.md (forbidden material, Maxon marks), docs/PARITY.md (golden layout,
diff metrics, tolerances) and the conventions in CLAUDE.md. No reference-app
observation is involved.

## 1. Workspace

- All crates inherit `version`, `edition = "2024"` and lints from `[workspace.package]` /
  `[workspace.lints]` in the root `Cargo.toml`.
- Package names are `forge-<dir>` (lib `forge_<dir>`). Reason: a package named `core`
  shadows the built-in `core` crate, and generic names (`io`, `ui`, `plugin`) collide
  with crates.io names.
- Directories carrying reference-app product names are renamed (Hard rule 2):
  `crates/mograph` → `crates/clone-engine`, `crates/render-redshift-bridge` →
  `crates/render-bridge`. Docs may keep naming the reference features they describe.
- Workspace lints: `clippy::undocumented_unsafe_blocks = "deny"` (enforces the
  `// SAFETY:` rule); `unsafe_op_in_unsafe_fn = "deny"`.
- Dev tools live under `tools/*` and are workspace members.

## 2. Clean-room check (`tools/cleanroom`, package `forge-cleanroom`)

Purpose: CI fails if forbidden trademarks/product names or reference-app file types
enter code, UI strings, crate names or file paths.

### Config: `tools/cleanroom/forbidden.txt`
Line-based; `#` starts a comment; blank lines ignored. Directives:

| Directive | Meaning |
|---|---|
| `term <text>` | Forbidden anywhere as a case-insensitive substring. `<text>` may contain spaces. |
| `word <text>` | Forbidden as a case-insensitive whole word (see boundaries). For short, collision-prone terms. |
| `ext <ext>` | Files with this extension (case-insensitive, no dot) are forbidden anywhere. |
| `exclude <path>` | Repo-relative path prefix (`/`-separated) skipped for **content and path** term checks. `ext` still applies. |

An unknown directive or a directive without an argument is a config error (exit 2).

### Word boundaries
A `word` match at `[s, e)` counts only if:
- **start**: `s == 0`, or the previous char is not ASCII alphanumeric, or the previous
  char is ASCII lowercase/digit and the matched text starts with an ASCII uppercase
  letter (camelCase join, e.g. `myXyz`).
- **end**: `e == len`, or the next char is not ASCII alphanumeric, or the next char is
  ASCII uppercase (CamelCase join, e.g. `XyzCloner`).

Effect: identifiers like `xyz_tool`, `XyzTool`, `load_xyz` match; lowercase hex like
`9a1xyzf0` does not.

### Scan
- Walks the repo root recursively; always skips `target/`, `.git/` and `.claude/` directories
  (`.claude/` holds local, untracked tool state such as agent worktrees — full copies of
  the repo whose docs legitimately name the reference app),
  and a `.git` *file* at the root (a git worktree's pointer to its metadata, which
  holds the checkout's absolute path).
- For each non-excluded file: checks the repo-relative path (`/`-separated) and every
  line of content (bytes decoded lossily as UTF-8) against all `term`/`word` entries.
- For every file (excluded or not): checks the extension against `ext` entries.
- Output, one per violation, sorted by path then line:
  `<path>:<line>:<col>: forbidden <kind> "<entry>"` (path hits use line 0, col 0;
  extension hits use `forbidden extension`). Columns are 1-based char indices.
- Exit code: 0 clean, 1 violations, 2 config/IO error.
- CLI: `forge-cleanroom [ROOT]` (default: current directory); config is read from
  `ROOT/tools/cleanroom/forbidden.txt`.
- `cargo test` also enforces it: an integration test scans the real repo.

Tests must not embed real forbidden terms (they would trip the scan); they use
synthetic configs.

## 3. Golden-data layout

```
tests/golden/<feature>/<case>/
  case.toml        # required manifest
  mesh.obj         # optional: exported reference geometry
  render.png       # optional (image loading lands with render work, Phase 10)
docs/parity/<feature>.md   # the observation the case is cited from
```

`case.toml` schema:

```toml
feature = "cube"                         # must equal the <feature> directory name
observation = "docs/parity/cube.md"      # repo-relative, must exist, must be under docs/parity/
description = "Default cube, 1 segment"  # optional
mesh = "mesh.obj"                        # optional, relative to the case dir; must exist

[params]                                 # optional; parameter values as set in the reference app
size_x = 200.0

[import]                                 # optional; exporter settings recorded by the observer
scale = 1.0                              # multiply golden positions (default 1.0)
flip_z = false                           # negate Z and reverse face winding (default false)

[tolerance]                              # optional
position = 1e-4                          # max abs per-component error (default 1e-4)
psnr = 40.0                              # min image PSNR in dB (default 40.0)
compare = "strict"                       # "strict" (compare_meshes, default) or "geometric" (§4.2)
```

Unknown keys are an error (catches typos). `observation` must not contain `..`. If
`mesh` is set, the file must exist and parse as OBJ. Validation problems are reported
as a list; a valid case has none. An empty `tests/golden/` is valid.

### 3.1 App version and the R20 defaults overlay

Golden cases come from two app versions; the case file does not say which (the manifest
denies unknown keys and golden files are read-only). The version is read from the case's
observation file: its header line starting `Status:` has `App version: <name> R<nn>…`, and
the first `R2` + digit token after `App version:` is the version (`R20` or `R25`). Anything
else (no `Status:` line, no `App version:`, another version, unreadable file) is an error
naming the observation file. `docs/parity/r25-defaults.md` and `docs/parity/r25-drift.md`
are R25; every other observation is R20.

Our node descriptors use the **R25** defaults (`docs/parity/r25-defaults.md`). A parity test
that builds a case from `[params]` must give an R20 case the R20 defaults: they live in
`tests/defaults/r20.toml`, one table per node type (our type id without `forge.`, e.g.
`[cylinder]`, `[circle_spline]`), holding **only** the params whose R20 default differs from
R25 (the `!` rows of that observation) in case-file units (degrees as shown, percent as
shown). A case's params are `overlay ∪ case.params`, the case winning; the overlay applies
to the case's own `[params]` (type = the case feature) and to each `[params.input*]` /
`[params.generator]` table (type = its `type` key). R25 cases get no overlay. Params of the
reference app that we have no key for (the isoparm subdivisions) are not in the file.

## 4. Parity harness (`crates/parity-harness`, package `forge-parity-harness`)

- `parse_obj(&str) -> Result<Mesh, ObjError>`: reads `v` (x y z; extra w ignored) and
  `f` (index forms `i`, `i/t`, `i//n`, `i/t/n`; 1-based; negative = relative to the
  current vertex count; ≥3 verts). Other statements (`vt vn g o s usemtl mtllib l`) and
  `#` comments ignored. Multiple objects merge into one mesh. Errors carry the line number.
- `Mesh { positions: Vec<[f64; 3]>, faces: Vec<Vec<u32>> }` (0-based indices).
- `apply_import(mesh, &Import)`: scale, then optional Z flip + winding reversal.
- `compare_meshes(expected, actual, &Tolerance) -> MeshReport`, checks in order:
  1. vertex count equal, 2. face count equal, 3. topology: face lists identical
     (same order, same vertex order — strict; reference ordering is part of parity
     because selections and vertex maps index into it),
  4. bbox: max abs diff of min/max corners ≤ `position`,
  5. per-vertex max abs component error ≤ `position` (only when counts match).
  `MeshReport::passed()` is true iff there are no failures; failures are human-readable.
  An empty mesh has bbox `None`; two empty meshes compare equal.
- `max_matrix_error(&[f64; 16], &[f64; 16]) -> f64`: max abs element difference.
- `psnr(expected: &[f32], actual: &[f32]) -> f64`: values in [0, 1];
  `10·log10(1 / MSE)`; `f64::INFINITY` when identical; panics on length mismatch or
  empty input.
- `discover_cases(golden_root) -> io::Result<Vec<GoldenCase>>`: every
  `<feature>/<case>/` directory, sorted; a case dir without `case.toml` still appears
  (so validation can flag it).
- `validate_case(repo_root, &GoldenCase) -> Vec<String>`: rules in §3.
- `AppVersion { R20, R25 }` and `app_version(repo_root, &CaseManifest) -> Result<AppVersion,
  String>`: the version rule of §3.1. `defaults_overlay(repo_root, AppVersion, node_type) ->
  BTreeMap<String, toml::Value>`: the `[node_type]` table of `tests/defaults/r20.toml` for
  R20 (empty if the table is missing), always empty for R25; panics with the file name if
  `r20.toml` is missing or malformed (it is test data).
- Integration test validates every case under the real `tests/golden/`.
- `mesh_info(&Mesh) -> MeshInfo { vertices, unused_vertices, faces, face_sizes:
  BTreeMap<usize, usize>, bbox: Option<([f64; 3], [f64; 3])> }`; `obj_extras(&str)
  -> ObjExtras { normals: usize, uvs: usize }` counts `vn` / `vt` statements.
  `Display` for `MeshInfo`:
  ```
  vertices: 8 (unused 0)
  faces: 6 (4-gons 6)
  bbox min: (-100, -100, -100)
  bbox max: (100, 100, 100)
  size: (200, 200, 200)
  centre: (0, 0, 0)
  ```
  Face sizes are listed ascending as `3-gons N, 4-gons N, …`; numbers use Rust's
  shortest `f64` formatting; an empty mesh prints `bbox: empty` instead of the
  three bbox lines.

- `parse_obj_corner_normals(src) -> Result<Vec<Vec<Option<[f64; 3]>>>, ObjError>`: per face,
  per corner, the `vn` the corner references (`i/t/n` or `i//n`; 1-based, negative =
  relative), `None` when the corner names no normal; out-of-range or zero indices are
  errors with the line number. Faces match `parse_obj` one to one.
- `max_normal_error(expected, actual: &[Vec<[f64; 3]>]) -> Option<f64>`: max abs
  component difference over all corners; `None` if the face/corner shapes differ.

### 4.1 `golden` CLI (observation helper)

- `golden info <file.obj>`: prints `mesh_info`, then `normals: N`, `uvs: N`.
  Exit 2 on unreadable/invalid files (message names the line).
- `golden check [repo_root]` (default `.`): validates every case under
  `<root>/tests/golden/`; prints each problem as `<feature>/<case>: <problem>` and
  exits 1 if any, else prints `ok: N cases`.
- The harness's own tests use a hand-written fixture repo in
  `crates/parity-harness/tests/fixtures/` (synthetic, not reference data).

### 4.2 Geometric comparison (`compare = "geometric"`)

For features whose reference numbering is algorithm-specific (booleans,
docs/specs/BOOLE.md), a case may opt in to comparing the *surface* instead of the
order. `compare_meshes` is unchanged; `Tolerance::compare` (`Compare::Strict` default,
`Compare::Geometric`) tells the feature's parity test which one to call.

`compare_geometric(expected, actual, &Tolerance) -> GeometricReport`, with
`t = position`:
1. **Points**: the distinct positions used by faces (positions within `t` are one
   point). Every expected point must have an actual point within `t` (max abs
   component) and vice versa; failures name the first missing point.
2. **Oriented planes**: each face contributes its Newell normal `N` (area `|N|/2`,
   unit normal `n`) and `d = n·c` (`c` its vertex centroid). Faces whose unit normals
   differ by ≤ `max(4t/√a, 1e-6)` per component (`a` the face's or plane's area) and whose `d` differ by ≤ `t` share a plane. Per
   plane: total area and area-weighted centroid (fan-triangle centroids). The two
   plane sets must match one to one; per plane, the area may differ by at most
   `t × P` (`P` = the expected faces' total perimeter in that plane) and the centroid by
   `t` plus `t × P / A`.
3. **Closedness**: after merging points as in 1, a mesh is closed iff every directed
   edge `u→v` has exactly one opposite `v→u` and no edge repeats. Both must agree.
4. **Volume** (closed meshes only): `Σ (p0 · (p1 × p2)) / 6` over fan triangles;
   `|ΔV| ≤ t × expected total area`.

`GeometricReport { points: (usize, usize), planes: (usize, usize), closed: (bool,
bool), volume: Option<(f64, f64)>, face_counts: (usize, usize), failures }` —
`face_counts` is informational (never a failure); `passed()` iff no failures.

## 5. Level-B samples (`samples.json`)

Sources: the observation forms `docs/parity/keyframes.md` (kind `track`),
`docs/parity/fields.md` (kind `field`) and `docs/parity/cloner.md` (kind `clones`),
and docs/PLAN.md "Parity levels" (level B: numeric samples instead of a mesh).

### 5.1 Files and formats
A case directory may hold `samples.json` and any number of `samples-*.json` files (the
keyframe `vector` case has `samples-pos_x.json`, `-pos_y`, `-pos_z` and no
`samples.json`). All belong to the case, sorted by file name. A file is one JSON object
whose `kind` selects the form; other keys are ignored.

| kind | Fields | Rust |
|---|---|---|
| `track` | `param` (string), `fps`, `tolerance`, `samples`: `[[frame, value], ...]` | `Samples::Track` |
| `field` | `tolerance`, `samples`: `[[[x, y, z], value], ...]` | `Samples::Field` |
| `clones` | `tolerance`, `clones`: `[{pos, x, y, z, color: [3 numbers], weight, visible: bool}, ...]` | `Samples::Clones` with `Clone` |

`parse_samples(&str) -> Result<Samples, String>`: unknown `kind`, a missing field or a
wrongly typed one is an error that names the field (`missing field "tolerance"`,
`samples[3]: expected 2 items, got 3`, `clones[0]: missing field "weight"`).
`GoldenCase::samples_files()` lists the file names; `GoldenCase::samples()` parses them
all into `(file name, Samples)`; the first bad file is an error prefixed with its name.

### 5.2 `golden check` rules
In addition to §3, per case:
- every samples file parses;
- a case without a mesh and without samples files is **not** a problem (tree-only or
  form-only cases, e.g. `make-editable/spline`, whose observation says splines do not
  export). `cases_without_output(&cases)` lists them as `<feature>/<case>`; `golden check`
  prints `note: <feature>/<case> has no mesh or samples file (tree or form only)` per case
  after the `ok: N cases` line and still exits 0; the repo golden test pins the exact list
  (`["make-editable/spline"]`), so a new output-less case is a deliberate test change;
- all numbers are finite;
- track frames strictly increase;
- field and clone tolerances are > 0.
`validate_samples(&Samples) -> Vec<String>` holds the semantic rules (parsing is
structural only); problems are printed as `<feature>/<case>: <file>: <problem>`.

### 5.3 `golden info <dir-or-file>`
- A `.obj` file: as §4.1. A `.json` file: `samples_info`, one `key: value` per line:
  `kind`, then for `track` `param`, `fps`, `samples` (count), `frames: first .. last`,
  `value min`, `value max`; for `field` `samples`, `value min`, `value max`; for `clones`
  `clones` (count), `invisible`, `position min`, `position max` (bbox of clone
  positions); finally `tolerance`.
- A directory: for every `*.obj` and samples file in it, sorted by name, a `== <file>`
  header line and that file's info, separated by a blank line.

### 5.4 Comparison
`compare_samples(expected, actual) -> SamplesReport { failure_count, failures (first 10),
max_error }`, `passed()` iff `failure_count == 0`.
- Kinds must be equal (`kind: expected track, got field`).
- Tracks: `param` equal and `fps` equal within 1e-9 (`param: expected "pos_x", got ...`,
  `fps: expected 30, got 24`).
- Track and field: equal sample counts; frames (or points, max abs component) equal
  within 1e-9 at every index, else the failure names the index and the two positions.
- Values: absolute error ≤ the **expected** file's `tolerance`. A failure names the
  frame or point, expected, actual and the error.
- Clones: equal clone count; per clone, `pos`, `x`, `y`, `z`, `color` (max abs component)
  and `weight` within the tolerance; `visible` exactly.
- Tolerances are never loosened here or in the files; the actual file's tolerance is
  ignored.

## 6. CI (`.github/workflows/ci.yml`, ubuntu-latest)

1. `cleanroom` job: `cargo run -p forge-cleanroom -- .` (fails fast on violations).
2. `check` job: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`.

## Done when
Clippy and tests pass locally, the cleanroom scan of the repo is clean, and the CI
workflow has run green on the hosted runner.
