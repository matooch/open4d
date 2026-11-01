# 163 · Native format: migrations, binary encoding, external references

Status: done · Area: M · Depends on: – · Observations: – · Touches: core, io

## Goal
A versioned native file with migrations, a compact binary encoding for large arrays, and external file references.

## Inputs
- Area design: [../../roadmap/areas/M-io.md](../../roadmap/areas/M-io.md)
- crates/core/src/file.rs

## Design decisions (planner)
- Header with format version; migration functions per version step; meshes/caches as binary chunks; relative paths for references.

## Done when
- [x] Old test files load through migrations; fuzz target added (182).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: 14 tests in `crates/core/tests/native_format.rs`, 2 unit tests in `file.rs`; no parity cases (no golden data). Fixtures `crates/core/tests/fixtures/scene-v1..v5.forge` (hand-written as each old build wrote them, never regenerate) plus `scene-v6.forge` and `scene-v6-chunked.forge` (pin the current bytes; regenerate only with a version bump via `FORGE_WRITE_FIXTURES=1`). Every fixture loads to the same logical scene limited to what its version could store.
- Rules (CORE.md §6.1-6.5): file = RON document, optionally followed by `\n%%FORGE-CHUNKS 1\n` and a chunk section (`count u32`, then `kind u8, len u64, FNV-1a u64, payload`). Mesh and index list values of at least `CHUNK_MIN_BYTES` = 1024 payload bytes (42 points / 256 indices) become `MeshChunk(i)` / `IndexListChunk(i)` in the text; everything else, and every small scene, stays text. f64 bits are exact (binary raw bits; text via ron's shortest round trip, tested with -0.0, subnormals, MAX). Version 6 declares link lists (068) and the `object.*` params (149) that were written under version 5; `MIGRATIONS[v-1]` is the step v to v+1, run in order; steps 1 to 5 are all no-ops today (additive changes absorbed by serde defaults), the engine and tests exist for the first real one. File reference params: `ParamDesc::file_path` (`UiHint.file_ref`), stored relative to the scene folder on save (`SaveOptions.base_dir`), resolved on load (`LoadOptions.base_dir`), optional missing-file warning, `external_refs(&Scene)`.
- Fuzz entry: `forge_core::file::load_bytes(registry, &[u8])`, pure and total; counts are checked against the remaining bytes before allocating; a seeded mutation test stands in until brief 182.
- Deviations: (1) No MessagePack/zstd (M-io.md mentioned it): our own chunk layout, no new dependency; a compression wrapper can come later. (2) No `Value` kind exists yet for per-point data or caches, so only meshes and index lists use chunks; kinds 3+ are reserved. (3) No node type uses a file reference param yet (textures arrive with the material briefs); tested with a test type. (4) `save(&Scene) -> String` stays the text-only form; the `file.save` / `file.open` commands use `save_with` / `load_with` with the file's folder as base. (5) Existing tests changed only where the version number is asserted: `crates/core/tests/{file,tracks}.rs` (`version: 5` to `version: 6`; the "future version" cases 6 to 7). Version 0 is now `UnsupportedVersion`.
- Files: crates/core/src/{file,param}.rs, crates/command/src/builtin/file.rs, crates/core/tests/{native_format,file,tracks}.rs, crates/core/tests/fixtures/*, docs/specs/CORE.md, docs/roadmap/areas/M-io.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions: when brief 179 (autosave) lands, whether it wants the chunked form always; whether the UI needs a "collect files" command on top of `external_refs`.
- Gate: `GATE: 939 tests passed, 0 failed` ... `cleanroom: ok` ... `GATE: all passed`.
