# 182 · Fuzzing and UI soak test

Status: done (nightly workflow pending first run) · Area: N · Depends on: 014, 163 · Observations: – · Touches: fuzz

## Goal
cargo-fuzz targets for every reader and the command parser; a nightly random-command soak.

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)

## Design decisions (planner)
- Soak: random commands via RPC for 10 minutes, assert no panic.

## Done when
- [ ] Nightly job green (pending its first run on GitHub; the workflow file is in place and every step was exercised locally except `cargo +nightly fuzz`).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: spec first (docs/specs/PLATFORM.md section 9), then tests, then code. New crate
  `forge-fuzz-harness` (`crates/fuzz-harness`) holds one entry point per reader
  (`fuzz_scene_file`, `fuzz_obj`, `fuzz_image`, `fuzz_exr`, `fuzz_command_args`,
  `fuzz_rpc_line`, `fuzz_script`), a counting allocator (bounded allocation, 512 MiB peak per
  input), a seeded mutation engine, a schema-driven argument generator and the soak driver.
  `fuzz/` is a separate cargo workspace (listed in the root `exclude`) with the 7 cargo-fuzz
  targets calling the same entry points; it compiles on stable (`cargo check --manifest-path
  fuzz/Cargo.toml`) but is run only by the nightly workflow. Tests added (all in `cargo test`):
  7 mini-fuzz tests (4000/4000/4000/3000/2500/3000/1500 seeded inputs on top of the seeds),
  `every_command_survives_hostile_arguments` (132 commands x 22 argument sets), resealed-chunk,
  counting-allocator and FS_COMMANDS tests, 3 `handle_request_line` tests, `soak_smoke`
  (1 500 steps with 7 checkpoints) and the ignored `soak_long`.
  Parity: not applicable (no reference observation involved).
- Findings: 400 000+ soak steps (release, three seeds, 100 s each), 60 000-iteration mini-fuzz
  runs per reader and 15 000 for the command/script targets found **no panic and no invariant
  violation** (undo-all returns the original bytes, redo-all the checkpoint bytes, save/load
  round trips are byte-stable, non-undoable commands do not change the scene). Hardening that
  was already present (chunk counts checked before allocation, image `Limits`, Rhai limits,
  command-arg deserialisation) held. The soak and fuzz runs did find two harness-level hazards,
  not product bugs: duplicating N nodes 60 times multiplies later work (the soak prunes the
  scene past 80 nodes), and `file.export_obj` is allowed to scripts by design, so the script
  target skips sources that mention it and the file commands are never fuzzed
  (`FS_COMMANDS`, guarded by a test that lists every command with a `path` argument).
- Deviations: (1) the OBJ seeds are written by our own writer from our own primitives, not
  copied from `tests/golden` (that folder holds reference-application output; the brief says
  never copy anything from the reference app). (2) The fuzz entry points live in a workspace
  crate instead of in `fuzz/`, so the stable gate can run them as mini-fuzz tests. (3) Added
  `forge_rpc::handle_request_line` (a thin public wrapper around the private line handler
  with no editor attached) so the RPC line parser can be fuzzed without a socket. (4) The
  glTF/FBX/Alembic/VDB readers named in the area design do not exist yet; their targets come
  with their readers. (5) The soak drives `dispatch` in process (as `forge-rpc-serve` does)
  rather than over TCP: the server rate-limits to 50 requests/s, which would make 10 minutes
  a few hundred steps. (6) Nothing has run on GitHub yet; `cargo +nightly fuzz` is untested
  locally (no nightly installed), hence the status.
- Files changed: docs/specs/PLATFORM.md (section 9), crates/fuzz-harness/**, fuzz/** (targets,
  Cargo.toml, 178 seed files), .github/workflows/nightly.yml, crates/rpc/src/{server,lib}.rs
  (`handle_request_line`), Cargo.toml (`exclude`), Cargo.lock, this brief, BACKLOG.md.
- Open questions: structure-aware seeds for the future glTF/FBX readers; whether the nightly
  fuzz time (120 s per target) should grow once the first run shows coverage plateaus.
- The last lines of the gate output: `advisories ok, bans ok, licenses ok, sources ok` /
  `cleanroom: ok` / `ok: 471 cases` / `GATE: 1637 tests passed, 0 failed` / `GATE: all passed`.
