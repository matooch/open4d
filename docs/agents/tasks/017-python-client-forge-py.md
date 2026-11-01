# 017 · Python client `forge-py`

Status: done · Area: A · Depends on: 014 · Observations: – · Touches: python/forge_py (new)

## Goal
Python users and tools drive Forge3D over RPC with a small, pure-Python package. Nothing is embedded.

## Inputs
- Area design: [../../roadmap/areas/A-automation.md](../../roadmap/areas/A-automation.md)
- brief 014 (protocol)

## Design decisions (planner)
- Folder `python/forge_py/`: a `pyproject.toml` (hatchling), the stdlib `socket` and `json` only, and `forge_py.Client(port, token)` with `run(id, **args)`, `scene()`, `node(id)`, `eval_mesh(node, frame)`.
- CI: the Linux job runs a Python test that starts `forge-studio --headless --rpc` (or the in-process test server binary) and runs the client tests.

## Done when
- [x] README in the package with examples.
- [x] Tests pass in CI with the system Python 3.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Embedding Python (v2).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 29 Python `unittest` tests (21 against a real server, 8 server-free incl. a fake socket server); spec in RPC.md section 9. No parity involved (our own design), so no `UNSUPPORTED` / `KNOWN_MISMATCH`.
- Rules derived: none parity-related. Client rules are in RPC.md 9.1 (generated `object_create(type=...)` methods from `command.list`, local `args_schema` check raising `TypeError`, `RpcError`/`Unauthorized`/`ConnectionFailed`, response id checked).
- Deviations: (1) no headless server existed (`forge serve` is not implemented and `forge-studio` is not built here), so I added a small binary `forge-rpc-serve` in `crates/rpc` (RPC.md 9.2), which the brief allows as the "in-process test server binary"; it is the server the tests, gate and CI use. (2) Package layout is `python/forge_py/forge_py/` (flat, hatchling `packages = ["forge_py"]`). (3) Extra explicit methods beyond the brief: `version`, `commands`, `timeline`, `call`; `eval_mesh` returns a `Mesh` object with `bbox()`.
- Files changed: `python/forge_py/**` (new), `crates/rpc/src/bin/forge-rpc-serve.rs` (new), `docs/specs/RPC.md`, `scripts/gate.sh` (Python step, skipped with a message without python3), `.github/workflows/ci.yml` (Linux `python-client` job), `.gitignore`, this brief, BACKLOG.md.
- Open questions: the user-facing `forge serve` (A-automation scope) could replace `forge-rpc-serve` later; the tests honour `$FORGE_RPC_SERVE`.
- Gate output: see the commit message / final report.
