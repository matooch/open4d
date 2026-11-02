# 178 · Crash handler and logging

Status: done · Area: N · Depends on: 175 · Observations: – · Touches: app

## Goal
Panic hook writing a crash report; rotating tracing log.

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)

## Design decisions (planner)
- Report: backtrace, last 50 commands, versions.

## Done when
- [x] Test: forced panic writes a report.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: spec `docs/specs/PLATFORM.md` §8 (+ UI.md §15.2, manual `automation.md`). Tests added: `forge_platform::logging` (6 unit: dir, level precedence, clamps, size rotation keeping the newest files, append/never-split, tail), `forge_platform::crash` (7 unit: report sections, unknown fields, secret scrubbing, no overwrite, hook sources, unseen/seen bookkeeping, dir), `forge_command::history` (5 unit: ring of 50, token redaction, truncation, global), `platform/tests/crash_hook.rs` (child process panics: report written, previous hook chained, token scrubbed; two panics give two reports), `platform/tests/logging_init.rs`, `app/tests/crash_report.rs` (child runs 121 real commands then panics: report holds the last 50 in order, log tail, redactions), `ui/tests/crash_notice.rs` (4 egui_kittest tests). No parity cases.
- Logging: `tracing` + `tracing-subscriber` (fmt, no ANSI) writing to `<data>/logs/forge.log` (`FORGE_LOG_DIR` overrides), own size rotation (`forge.log.1..N`, defaults 1 MiB x 5, settings `log.max_bytes/max_files`), level from `FORGE_LOG`, then `log.level`, then `info`; stderr in debug builds or `FORGE_LOG_STDERR=1`; never stdout. Converted: editor start notices, screenshot failure, RPC server start/auth rejection/handler panic, rpc-serve and mcp start errors (still echoed on stderr where documented), per-command debug/warn in the registry. CLI output (`forge`) unchanged by design (its stderr lines are the user contract in CLI.md).
- Crash report: `<data>/crashes/crash-<session>-<n>.txt` (`FORGE_CRASH_DIR`): time, app+version, session, OS/arch, GPU (wgpu adapter name/backend/driver, set by the editor), thread, panic message and location, backtrace, last 50 commands, last 200 log lines; the RPC token is scrubbed everywhere. Hook installed by `forge_app::diagnostics::install` in forge-studio, forge-rpc-serve and forge-mcp; it chains to the previous hook. The session id is the one the autosave marker uses (`crash::process_session`).
- Command ring buffer: `forge_command::history` (50 entries, filled in `CommandRegistry::run`, keys containing `token` redacted, args cut at 2000 chars).
- Next start: window "Forge3D crashed last time" (`forge_ui::crash_notice`) lists unseen reports (`.seen` file in the crash folder), buttons Open folder (xdg-open/open/explorer) and Dismiss, and points to the recovery window when it has entries.
- Deviations: own size-based rotating writer instead of `tracing-appender` (daily rotation cannot be tested with a clock and it adds a `time` dependency tree); panics caught by the RPC server (`catch_unwind`) also write a report because the hook runs before unwinding; the documented `rpc: listening` banner stays on stderr next to the log line.
- Files changed: crates/platform/{Cargo.toml,src/{lib,logging,crash}.rs,tests/*}, crates/command/{Cargo.toml,src/{lib,registry,history}.rs}, crates/app/{Cargo.toml,src/{lib,diagnostics}.rs,tests/crash_report.rs}, crates/ui/{Cargo.toml,src/{lib,state,autosave,crash_notice}.rs,src/app/recovery.rs,tests/crash_notice.rs}, crates/rpc/{Cargo.toml,src/server.rs,src/bin/forge-rpc-serve.rs}, crates/mcp/{Cargo.toml,src/main.rs}, apps/studio/{Cargo.toml,src/main.rs}, docs/specs/{PLATFORM,UI}.md, docs/manual/automation.md, BACKLOG, this brief, Cargo.lock.
- Open questions: none.
- Gate: see the commit message / final report.
