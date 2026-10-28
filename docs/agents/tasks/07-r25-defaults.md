# 07 · R25 defaults with version-aware parity tests

Status: blocked (needs O4 `docs/parity/r25-defaults.md` and the committed O1 cases) · Branch: `batch-08` · Planner: main session · Implementer: Claude Sonnet 5.5 (local, macOS)

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
- O1 already lists the expected differences: sphere segments 16 (R20 24), cylinder and
  torus segments and radii, extrude offset 100 (R20 20), boole type union and hide new
  edges on. O4 is the authority for exact values.
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
