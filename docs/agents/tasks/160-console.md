# 160 · Console panel (log + Rhai REPL)

Status: done · Area: L · Depends on: 016 · Observations: – · Touches: ui

## Goal
A console showing the log and a Rhai REPL.

## Inputs
- Area design: [../../roadmap/areas/L-ui.md](../../roadmap/areas/L-ui.md)

## Design decisions (planner)
- tracing subscriber to an in-memory ring buffer.

## Done when
- [x] REPL evaluates a command.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: spec `docs/specs/UI.md` §19 (+ `SCRIPTING.md` §6). Tests added: 7 unit in `forge_ui::console` (level parsing, level filter, continuation lines, search, copy text, REPL history up/down/draft, caps), 10 egui_kittest in `crates/ui/tests/console.rs` (closed by default and opened from the Window menu, fed log lines, level drop-down, search, Copy, Run with two commands = one undo step, Enter and Shift+Enter, focus kept for the next entry, failing entry rolled back with the sandbox refusing `edit.undo`, Up/Down history, Clear). No parity cases (own design).
- Rules: log = `forge_platform::logging::recent_lines()` (injectable via `ConsoleState::log_source`), level = second token of the tracing line, continuation lines inherit the previous level, search is a case-insensitive substring. REPL = `forge_script::run_script` with default limits on the editor context: one entry is one `Run Script` undo step, a failure is rolled back; each entry has a fresh scope.
- Deviations: no new registry command (the REPL goes through `run_script`, which runs registry commands); variables do not persist between entries; the log tail is the 200-line in-memory ring of brief 178 and cannot be cleared; Up/Down recall only while the text is single-line; history entries are in memory only.
- Files changed: crates/ui/src/{console.rs,state.rs,dock.rs,app.rs,lib.rs}, crates/ui/tests/console.rs, docs/specs/{UI,SCRIPTING}.md, BACKLOG, this brief.
- Open questions: persist REPL history in the settings file? Variables across entries (a persistent Rhai scope) would need a change in forge-script.
- Gate: see the commit message / final report.
