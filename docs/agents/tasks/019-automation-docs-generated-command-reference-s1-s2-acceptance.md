# 019 · Automation docs, generated command reference, S1/S2 acceptance tests

Status: ready · Area: A · Depends on: 013, 015, 016 · Observations: – · Touches: docs, apps/forge

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
- [ ] `docs/manual/automation.md`: CLI, RPC, MCP, scripting quick starts.
- [ ] S1 (reduced) and S2 run in CI.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added; parity N/M (`UNSUPPORTED` / `KNOWN_MISMATCH` with reasons).
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
