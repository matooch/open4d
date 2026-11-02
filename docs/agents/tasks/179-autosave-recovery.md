# 179 · Autosave and recovery

Status: done · Area: N · Depends on: 175, 163 · Observations: – · Touches: app

## Goal
Periodic autosave and recovery after a crash.

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)

## Design decisions (planner)
- Write to a temp file then rename (atomic).

## Done when
- [x] Test: recovery offered after a simulated crash.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: spec `docs/specs/PLATFORM.md` §7 (+ UI.md §15, COMMANDS.md). Pure logic in `forge_platform::autosave` (settings, naming, schedule, atomic write, entries, pruning, session markers, background `Writer`, `ManualClock`): 17 unit tests. `crates/ui/tests/autosave.rs`: 8 integration tests (schedule with a manual clock, disabled setting, command without a directory, save/new/clean exit removal, crash then recover with identical scene bytes through the kittest dialog, original path kept and Discard, damaged file, live session not offered). Command registry tests extended.
- Policy: default every 300 s, only when the scene changed since the last save and since the last autosave; settings `autosave.{enabled,interval_seconds,max_files}` (defaults true/300/10) read once at start. Files in `<data>/autosave/` (or `FORGE_AUTOSAVE_DIR`): `<key>.forge.autosave` (native `save_with` bytes) + `.json` metadata; key `untitled-<session>` or `<stem>-<fnv1a64 of absolute path>`.
- Crash detection: `sessions/<id>.lock` held with `File::try_lock` for the process life; an existing unlocked marker means a crash, a locked one a running instance; clean exit deletes it.
- Recovery: window "Recover unsaved work" (Recover / Discard / Discard all); recover opens a dirty document with the original path, autosaves it again, then removes the old entry.
- Threading: scene serialised on the UI thread, bytes written (temp + fsync + rename) by one ordered writer thread; prune after each write.
- Commands: `file.autosave_now` (`wait?`), via new `CommandContext::autosave_now` (headless: "needs the editor window").
- Deviations: none from the brief. Follow-up: closing the window with a dirty document now asks Save / Don't Save / Cancel (UI.md §15.1; `crates/ui/tests/close.rs`). Pending crashed entries can be pruned by later writes beyond `max_files`. Settings are not reloaded at runtime.
- Files: crates/platform/{Cargo.toml,src/lib.rs,src/autosave.rs}, crates/command/src/{context.rs,builtin/file.rs}, crates/command/tests/registry.rs, crates/app/src/lib.rs, crates/ui/src/{autosave.rs,lib.rs,state.rs,commands.rs,app.rs,app/files.rs,app/recovery.rs}, crates/ui/tests/autosave.rs, docs/specs/{PLATFORM,UI,COMMANDS}.md, docs/manual/reference/*, BACKLOG, Cargo.lock.
- Open questions: none.
- Gate: `cleanroom: ok` ... `GATE: all passed`.
