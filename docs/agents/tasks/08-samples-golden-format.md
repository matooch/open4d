# 08 · `samples.json`: numeric golden cases in the harness

Status: done · Branch: the planner's session branch · Planner: main session · Implementer: Claude Sonnet 5.5

## Goal
The parity harness understands level-B golden cases. It parses a case's `samples.json`
(keyframe tracks, field values, clone data), validates it in `golden check`, summarises
it in `golden info`, and offers one comparison function that every level-B parity test
uses.

## Inputs
- `docs/PLAN.md` "Parity levels" (level B).
- The three formats as the observer wrote them:
  - `docs/parity/keyframes.md` and `tests/golden/keyframes/` (kind `track`; `vector` has
    three files named `samples-<param>.json`);
  - `docs/parity/fields.md` and `tests/golden/fields/` (kind `field`);
  - `docs/parity/cloner.md` and `tests/golden/cloner/` (kind `clones`; those cases also
    have `mesh.obj`).
- `crates/parity-harness/src/lib.rs` (`CaseManifest`, `GoldenCase`, `validate_case`,
  `mesh_info`), `crates/parity-harness/src/bin/golden.rs`, `docs/specs/INFRA.md` §3–4.

## Design decisions (planner)
1. **Types** in `forge-parity-harness`, module `samples`:
   ```rust
   pub enum Samples {
       Track { param: String, fps: f64, tolerance: f64, samples: Vec<(f64, f64)> },
       Field { tolerance: f64, samples: Vec<([f64; 3], f64)> },
       Clones { tolerance: f64, clones: Vec<Clone> },
   }
   pub struct Clone { pub pos: [f64; 3], pub x: [f64; 3], pub y: [f64; 3], pub z: [f64; 3],
                      pub color: [f64; 3], pub weight: f64, pub visible: bool }
   ```
   `parse_samples(&str) -> Result<Samples, String>` dispatches on `kind`. Unknown kinds
   and missing fields are errors that name the field.
2. **Discovery:** every file in a case directory matching `samples.json` or
   `samples-*.json` belongs to the case. `GoldenCase::samples() -> Result<Vec<(String, Samples)>, String>`
   returns them sorted by file name, where the `String` is the file name.
3. **`golden check`:**
   - every samples file must parse;
   - a case must have a mesh or at least one samples file (today a case with neither
     passes silently);
   - samples must be finite;
   - track samples must be in strictly increasing frame order;
   - field and clone tolerances must be > 0.
4. **`golden info <dir-or-file>`:** for a samples file, print the kind, the count, and
   the value range (min/max of the values, or the clone position bbox). For a case
   directory, print info for every mesh and samples file in it.
5. **Comparison:** `compare_samples(expected: &Samples, actual: &Samples) -> SamplesReport`
   with `passed()` and up to 10 failures. Each failure names the frame, point or clone
   index, the expected and actual values, and the error.
   - Both sides must have the same kind and the same sample positions; frames and points
     must match exactly, within 1e-9.
   - Values compare with the **expected** file's tolerance, absolute.
   - Clones compare position, axes, colour and weight with the tolerance, visibility
     exactly, and the clone count exactly.
   - Never loosen a tolerance here or in the files.
6. Spec: `docs/specs/INFRA.md` gets a new §5 "Level-B samples" covering the formats,
   check rules, info output and comparison rules, citing the three observation files.

## Hints (planner)
- `serde_json` is probably already in the lock file; add it to the harness if not.
- The keyframe `vector` case has three files and no `samples.json`; discovery must find
  them.
- The cloner `case.toml` files hold nested tables (`[params.effector]`, plus
  `[params.settings]` in modeling tools). They must keep parsing: `[params]` is already
  a free-form map, so leave it that way.

## Done when
- [ ] INFRA.md §5 written.
- [ ] Unit tests in `crates/parity-harness/tests/samples.rs`:
  - parsing each kind from a small inline string;
  - error messages for a bad kind, a missing field and an unsorted track;
  - `compare_samples` passing, failing on a value, failing on a sample position, and
    failing on a kind mismatch.
- [ ] A test that parses **every** real samples file under `tests/golden/` and asserts
      the count per kind. Today that is 11 track files (9 keyframe cases, `vector` with
      3), 24 field cases and 16 clone cases. Count them yourself and pin the numbers.
- [ ] `golden check` prints `ok: N cases` with the new rules. No current case may fail.
      If one does, report it: never edit golden data.
- [ ] `sh scripts/gate.sh` prints `GATE: all passed`.
- [ ] Committed with the trailer below; report written.

Commit trailer:
```
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01AF8235TxjbnG4zHfXN22Nb
```

## Out of scope
Building anything that produces samples (keyframes, fields and clones are later briefs),
and editing golden data.

## Report (implementer fills in)
- Blocker found: the brief's rule "a case needs a mesh or a samples file" failed on the tree-only
  case `make-editable/spline` (its case.toml: splines do not export). Planner ruling: not an
  error. `golden check` prints a `note:` line per such case after the ok line and exits 0;
  `cases_without_output` exposes the list and `repo_golden.rs` pins it to
  `["make-editable/spline"]`. The ruling also added: `compare_samples` for tracks requires equal
  `param` and `fps` (1e-9).
- Implemented: `crates/parity-harness/src/samples.rs` (types, `parse_samples`,
  `validate_samples`, `samples_info`, `compare_samples`), `GoldenCase::samples_files()` and
  `samples()`, `validate_case` samples rules, `has_no_output`/`cases_without_output`,
  `golden info <dir|file>`, INFRA.md new §5 (CI is now §6), `tests/samples.rs`, `serde_json`.
- Real samples files pinned: 11 track, 24 field, 16 clones; all parse and validate.
- Accepted deviations: `validate_samples` holds finite/order/tolerance rules while
  `parse_samples` is structural; track tolerance not checked > 0; synthetic test
  `minimal_manifest_is_valid` rewritten; `SamplesReport { failure_count, failures, max_error }`;
  `golden info` directory headers `== <file>`.
- Gate: see the final gate lines in the commit/hand-off message.
