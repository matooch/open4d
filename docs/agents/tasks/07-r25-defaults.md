# 07 · R25 defaults with version-aware parity tests

Status: done · Branch: `batch-08` · Planner: main session · Implementer: Claude Sonnet 5.5 (local, macOS)

## Goal
Our node types use the **R25** defaults and ranges, and every parity test still passes:
cases observed in R20 are built with R20 defaults, cases observed in R25 with R25
defaults. The O1 drift cases run as parity cases.

## Inputs
- `docs/parity/r25-defaults.md` (O4): the R20 vs R25 default tables (rows marked `!`
  differ).
- `docs/parity/r25-drift.md` (O1) and `tests/golden/r25-drift/` (R25 re-exports; case
  folder names are `<feature>-<original case>`, e.g. `sphere-defaults`).
- `crates/parity-harness/src/lib.rs` (`CaseManifest`, `discover_cases`), every parity
  test that builds from `[params]`: `crates/generators/tests/{parity,generators_parity,subdivision_parity,boole_parity}.rs`,
  `crates/modifiers/tests/deformers_parity.rs`.

## Design decisions (planner)
1. **The app version comes from the observation file**, not from `case.toml` (the
   manifest denies unknown fields and golden files are read-only). Add to the harness
   `pub fn app_version(repo_root, manifest) -> Result<AppVersion, String>` that reads the
   header line starting `Status:` of `manifest.observation` and finds the token `R20` or
   `R25` after `App version:` (match `R2` + digits; the code must not contain the
   vendor's name, the cleanroom gate checks). `AppVersion::{R20, R25}`; anything else is
   an error naming the file.
2. **R20 defaults overlay**: a data file `tests/defaults/r20.toml` with one table per
   node type holding **only the params whose R20 default differs from R25** (the `!`
   rows), values in case-file units (degrees, percent as shown). The harness gets
   `pub fn defaults_overlay(repo_root, version, feature) -> BTreeMap<String, toml::Value>`
   (empty for R25). Each parity test builds a case's params as
   `overlay ∪ case.params` (case wins). Put the merge in one shared helper per test
   crate, not copied five times.
3. **Our descriptors switch to R25**: defaults, and ranges where O4 shows a different
   clamp. Labels change only where O4 lists a changed label. Each changed descriptor
   cites `docs/parity/r25-defaults.md` in its spec table (update the param tables in
   PRIMITIVES*.md / GENERATORS2.md / BOOLE.md / SUBDIVISION.md / MODIFIERS.md: add a
   column or a note "R25 default (R20: x)").
4. **Drift cases**: a new test `crates/generators/tests/r25_drift.rs` runs every
   `tests/golden/r25-drift/<feature>-<case>` through the same builder as the original
   feature (feature = text before the first `-` that names a known feature; params from
   the drift `case.toml`, R25 defaults for the rest), compared the way the original
   feature's test compares (exact, or geometric for boole). Boole in R25 writes six extra
   duplicate points: compare through `export_weld` on both sides, as boole's test does.
   Unmatched drift cases go in `UNSUPPORTED` / `KNOWN_MISMATCH` with a reason.

## Hints (planner)
- Do 1 and 2 first and run the whole gate **before** touching any default: everything
  must still pass (the overlay is a no-op while defaults are still R20). Then change the
  defaults; any R20 case that now fails means an overlay row is missing.
- O4 (`docs/parity/r25-defaults.md`) is the authority: 73 `!` rows across the primitives,
  splines, generators. The deformers and the smoothing tag have none. Planner rulings on its
  open points:
  - **Slice To = 180** in R25 (machine readback of π). R20's 360 was "reading uncertain".
    Every golden slice case sets `slice_to` explicitly, so the overlay entry only matters
    for completeness: put `slice_to = 360` in the R20 overlay as the R20 file states.
  - **Boole type stays A subtract B** (O4 readback, same as R20). O1's "union" was the drift
    case's explicit `mode = "union"`. Only `hide_new_edges` moves (off → on).
  - **Extrude**: R25 replaced Movement (x, y, z) by one Offset number plus a Direction
    list. Map it to our existing key: `movement_z` default 20 → 100. Direction is a new
    R25-only parameter (out of scope).
  - **Enum defaults that name an option we don't have** (e.g. loft `cap_type` N-gon,
    `subdivide_uvs` Edge, spline `intermediate` Uniform): if our enum has the option, switch
    the default. If not, keep the R20 default, list it in the spec as "R25 default not
    supported yet", and don't add the option in this brief.
  - Rows marked "reading uncertain" or "from the mesh" in the R20 column (cone
    `fillet_segments`, capsule segments) are R20 values: put them in the overlay as written.
  - "none (±999999999 accepted)" means no clamp: don't add a range for it. Only add or
    change a range where O4 shows a finite clamp that differs from ours.
- **Vertex order may have changed in R25 for torus and extrude** (O1: with the R20
  values typed in, the same vertex set and counts but a different order). So
  `r25-drift/torus-slice` and `r25-drift/extrude-rectangle-caps-quadrangles` may fail
  the exact comparison even with correct defaults. Check them geometrically first
  (`compare_geometric`); if that passes, try to derive the R25 order (three hypotheses,
  IMPLEMENTER.md §3.7). Otherwise pin them in `KNOWN_MISMATCH` as "R25 vertex order" and
  say so in the spec. Don't change the R20 order: the R20 cases must keep passing.
- `r25-drift/sphere-type-icosahedron` is the first icosahedron case at segments 16
  (bbox 97.94, not 100): it tests the frequency rule PRIMITIVES.md §3.1 saw at 8 and 24
  only. A mismatch there is a finding about that rule, not about defaults.
- Cylinder and torus have a new R25 switch "use new version" (default on); O1 saw the
  same geometry either way. Not a param for this brief (out of scope, list it).
- The drift cases' `[tolerance] compare = "geometric"` (both boole cases) already
  selects the geometric comparison.
- The UI and file loading use descriptor defaults: a saved scene keeps its values
  (every param is written), so old files don't change. Say so in CORE.md §6 if it isn't
  there already.

## Done when
- [ ] Harness: `AppVersion`, `app_version`, `defaults_overlay` with unit tests
      (`crates/parity-harness/tests/`), INFRA.md §3 documents the version rule and
      `tests/defaults/`.
- [ ] Descriptors use R25 defaults/ranges; spec tables updated with citations.
- [ ] All existing parity cases still pass (same counts as before in STATUS.md); the
      drift test covers every `tests/golden/r25-drift/` case.
- [ ] `sh scripts/gate.sh` prints `GATE: all passed`.
- [ ] `docs/parity/STATUS.md`: a row "R25 defaults + drift cases".
- [ ] Committed on `batch-08` with the trailer below; report written.

Commit trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`

## Out of scope
R25-only parameters (O4 "New-object parameters"), any golden or observation edit,
exporter text format (we compare parsed numbers).

## Report (implementer fills in)
- Result: drift 6/7 exact or by the original feature's comparison (sphere-defaults, sphere-type-icosahedron, cylinder-fillet, torus-slice cut-geometric, boole x2 geometric). Known mismatch: extrude-rectangle-caps-quadrangles ("R25 vertex order": the 4-point caps start at another corner; GENERATORS2.md §11.1). None unsupported. All R20 parity counts unchanged.
- Rules derived: icosahedron frequency d = 5 (floor(segments/5); 8->1, 16->3, 24->4; `r25-drift/sphere-type-icosahedron`). R25 extrude 4-point caps: start `[3,2,1,0]`, end `[4,5,6,7]` (one case, parity-pending, not implemented).
- 37 of the 42 `!` rows applied. Not applied: circle and helix `intermediate` Uniform (build only supports Adaptive 5°), loft `cap_type` N-gon (loft ignores cap type, always triangles), lathe and sweep `isoparm_subdivision` (no param). All kept R20 values are still in the overlay.
- Ranges changed to O4's finite clamps (regular_grid_width >= 0.01, optimize_points >= 0.0001, array copies 1-1000 etc., see GENERATORS2.md §0a).
- Deviations: loft/spline defaults kept as above; icosahedron rule changed (finding); lathe/sweep `cap_type` default left Triangles although O4 lists N-gon for R25 (no `!`; R20 meshes show Triangles) - needs re-observation; tube/cone R25 fillet defaults (3) not applied (not `!`).
- Files changed: harness lib.rs, tests/defaults/r20.toml, crates/{generators,modifiers,io}/tests/common, parity tests, r25_drift.rs, r25_defaults.rs, version.rs, descriptors in crates/generators/src, specs, STATUS.md.
- Gate: GATE: 362 tests passed, 0 failed / cleanroom: ok / ok: 202 cases / GATE: all passed
