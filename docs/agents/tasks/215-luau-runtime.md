# 215 · Luau runtime and builder scripts

Status: ready · Area: P · Depends on: – · Observations: – (own design, D21) · Touches: luau (new), script, app, command, rpc, mcp, apps/forge

## Goal
A sandboxed Luau runtime that can do everything a Rhai script can (SCRIPTING.md §3), so people
and agents can build scenes in Lua. BEHAVIOURS.md §1, §2, §9.

Spec: [../../specs/BEHAVIOURS.md](../../specs/BEHAVIOURS.md) (own design, D21; no parity data). Where the spec says *(open)*, decide, write the rule into the spec, and say so in the Report.

## Steps
1. Add `crates/luau` (`forge-luau`) on `mlua` with `luau` + `vendored`. Check `cargo deny` (MIT) and that the vendored build works on all three CI OSes.
2. The sandbox of §2: libraries, `require` from `<project>/scripts/`, the seeded `math.random`, memory limit, instruction-interrupt budgets, `ScriptError` with line numbers. Decide the `fmath` *(open)* question.
3. Bindings of §9 (the SCRIPTING.md §3 surface in Lua style), with undo as one step and rollback on error.
4. Hosts: `forge run file.luau` (pick the language by extension), Script Manager and Console language switch, RPC/MCP `script.run { language }`.
5. A Luau twin for every `examples/scripts/*.rhai`, and a test that each pair builds the same scene.

## Done when
- [ ] Every sandbox rule of §2 has a test (refused library or function, `require` escape, memory limit, CPU limit, error line).
- [ ] The Rhai/Luau example pairs give identical scenes; `forge run x.luau` works in the CLI tests.
- [ ] CI is green on Linux and macOS (Windows: the known brief 210 jobs aside), and `cargo deny` passes.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
