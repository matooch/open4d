# 019 · Automation docs, generated command reference, S1/S2 acceptance tests

Status: done (S1 partially pending: cloner + random effector 065/069, glTF 164, PNG render 136/139/146) · Area: A · Depends on: 013, 015, 016 · Observations: – · Touches: docs, apps/forge

## Goal
The automation surface is documented and proven by the S1 and S2 acceptance scenarios running in CI.

## Inputs
- Area design: [../../roadmap/areas/A-automation.md](../../roadmap/areas/A-automation.md)
- docs/roadmap/README.md (S1, S2)

## Design decisions (planner)
- `forge commands --markdown` writes `docs/manual/reference/commands.md`. A CI check fails if the committed file is stale.
- S1 as `tests/acceptance/s1.rhai` plus a Rust test that runs it with `forge run` and checks the glTF/PNG outputs. Until K and M exist, PNG and glTF are stubs and the test checks the OBJ; mark that clearly and re-enable the full test in 146/164.
- S2 as a test that starts `forge-mcp --headless`, creates a sphere, keys position X at 0 and 30, and reads the evaluated position at 15.

## Done when
- [x] `docs/manual/automation.md`: CLI, RPC, MCP, scripting quick starts.
- [x] S1 (reduced) and S2 run in CI.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: tests added: `apps/forge/tests/docs.rs` (4: committed reference equals generated, no stray files, deterministic and complete, `forge docs`/`--check`/`commands --markdown`), `apps/forge/tests/acceptance.rs` (4 passing S1 tests, 3 ignored pending tests), `crates/mcp/tests/stdio.rs` (+2 S2: running editor and headless), `crates/rpc/tests/reference.rs` (1), unit tests `forge-rpc` docs (3) and `forge-script` api (2), Python `test_acceptance.py` (1, S2 through forge-py). No parity data (N/A).
- Generated reference (`docs/manual/reference/`: commands.md, cli.md, rpc.md, mcp.md, scripting.md): `forge docs --out docs/manual/reference` (or `forge commands --markdown` for the command page on stdout). Sources: command registry (ids, labels, JSON Schema tables, examples); clap definition (CLI); `forge_rpc::docs` tables; headless `forge-mcp` `tools/list`; `forge_script::api::API` table. Staleness test: `cargo test -p forge-cli --test docs`. Drift guards for the hand-kept tables: RPC source scan plus a live-server call of every method; script table compared with the engine's registered functions (name, arity).
- S1 (`tests/acceptance/s1.rhai`): passing: script builds a cube with an animated bend deformer, validate, frame 0 equals the cube golden, frame 30 evaluated and exported as OBJ (bent, between frame 0 and 60), `export` equals `eval --export`, deterministic. Pending (ignored tests naming the brief): cloner with random effector (065, 069), glTF export (164), PNG render (136, 139, 146); a live test pins `export --format gltf` and `render` as stubs so their arrival is noticed.
- S2: passing through MCP against a running RPC server (`forge-mcp --port` with token), against `forge-mcp --headless`, and through forge-py: sphere created, position X keyed at 0 and 30, timeline play/stop, evaluated position at frame 15 (150 +/- 0.01 relative to frame 0). Not covered: real playback ticking (needs the window's clock; the host reports the `playing` flag only).
- Deviations: (1) `forge commands --markdown` prints to stdout instead of writing `docs/manual/reference/commands.md` itself; `forge docs --out` writes. Reference lives in `docs/manual/reference/` (brief named `docs/manual/reference/commands.md` too, plus three more pages). (2) S1 is a reduced scenario (bend animated by `key.set`, OBJ in place of glTF/PNG). (3) S2 "running editor" is `forge_rpc::Server` around an `App` (the editor minus the window). (4) `forge-script` gets a dev-only `rhai/metadata` feature for the drift test. (5) Added `forge_script::BLOCKED_COMMANDS`, `forge_rpc::docs`, `forge_script::api` public items.
- Files changed: apps/forge/{Cargo.toml,src/lib.rs,src/docs.rs,tests/docs.rs,tests/acceptance.rs}, crates/rpc/{src/lib.rs,src/docs.rs,tests/reference.rs}, crates/script/{Cargo.toml,src/lib.rs,src/host.rs,src/api.rs}, crates/mcp/tests/stdio.rs, python/forge_py/tests/test_acceptance.py, tests/acceptance/s1.rhai, docs/manual/{automation.md,reference/*.md}, docs/specs/{CLI,RPC,SCRIPTING}.md, docs/roadmap/areas/A-automation.md, README.md, Cargo.lock, this brief, BACKLOG.md.
- Open questions: none blocking; once 065/069/164/136 land, fill the three ignored S1 tests and drop the stub-pinning test.
- The last lines of the gate output:
  ```
  GATE: 564 tests passed, 0 failed
  Ran 38 tests ... OK (python)
  cleanroom: ok
  ok: 471 cases
  GATE: all passed
  ```
