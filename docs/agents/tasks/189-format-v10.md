# 189 · Native format v10 for list-valued parameters

Status: done · Area: M · Depends on: 028, 163 · Observations: – · Touches: core, io

## Goal
Brief 028 added `Value::Vec3List` (editable spline points and handles) inside format 9, so an
older build fails a v9 file that uses it with a parse error instead of `UnsupportedVersion`.
Bump the native format to 10 so version checks stay meaningful.

## Design decisions (planner)
- Version 10 = 9 plus list-valued parameter kinds (`Vec3List`, and the integer lists editable splines use). The 9 → 10 migration is the identity; a file is written as 10 only when it is written by this build (always).
- Files that use no new kind still load in builds that read 9 only if they were written as 9; this build always writes 10.
- Add a v10 fixture with an editable spline; keep the v1–v9 fixtures and their tests; update pinned-version tests to the new current version through one constant, not by hand in each test.
- CORE.md: the format table gets v10 and the 028 follow-up note is closed.

## Done when
- [x] Loading every existing fixture works; a v10 file read by a simulated v9 reader gives `UnsupportedVersion`.
- [x] Round-trip of a scene with an editable spline is bit-identical.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Files changed: crates/core/src/file.rs (VERSION 10, `migrate_9_to_10` identity, `load_as_reader` for simulating an older reader), crates/core/tests/{native_format,file,tracks,time}.rs and crates/nodes/tests/expression.rs (pinned `version: 9` now use `file::VERSION`; two future-version probes use 99), new fixtures `scene-v10*.forge` (with an editable spline) in crates/core/tests/fixtures and fuzz/corpus/scene_file, docs/specs/CORE.md (section 6.9, 028 note closed), docs/roadmap/BACKLOG.md, this brief.
- Tests added: v10 fixtures byte stable, v9 fixtures still load and resave as current, v9 reader refuses v10 (`UnsupportedVersion(10)`), editable spline round trip bit-identical (text and chunked). v1-v9 fixtures untouched.
- Deviations: none. Preset .forge files stay version 9 (they migrate on load).
- Gate: GATE: all passed
