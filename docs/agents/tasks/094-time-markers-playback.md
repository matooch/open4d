# 094 · Time: markers, preview range, playback modes, fps conversion, time display

Status: done (parity-pending G5) · Area: G · Depends on: 010 · Observations: G5 · Touches: core, ui

## Goal
Document time settings: fps, min/max, preview range, markers, playback modes (once, loop, ping-pong), time display formats.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- GUI session G5 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- GUI-SESSIONS.md G5 §4

## Design decisions (planner)
- Changing fps keeps keys at the same seconds (O16 rounding rules from 087).

## Done when
- [x] Unit tests for fps conversion; markers serialize.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 29 tests added, no parity cases (G5 unanswered, everything here is **parity-pending G5**):
  `crates/core/tests/time.rs` (10: ms rounding, `convert_frame`, display formats, keys keep
  their seconds, validation/undo/clip, jump target, save/load round trip, v6 -> v7 defaults,
  invalid block warning), `crates/command/tests/playback.rs` (7: modes, injected clock, frame
  dropping vs every frame, preview range, Once restart, re-anchor), `crates/ui/tests/time.rs`
  (11: commands headless, `set_fps` conversion, drag merge, play commands; egui_kittest for the
  preview checkbox and handles, marker add/click/drag/delete/prev/next, M and Alt+,/. keys, label
  per display), plus `native_format.rs` (v6 fixtures still load, v7 fixtures pin the bytes).
- Rules (ANIM.md §2.6a, §2.9, CORE.md §6.6): `TimeConfig` in the scene (preview range, mode,
  drop_frames, display, markers) next to `SceneSettings` (kept unchanged so no existing literal
  broke); file format 7, step 6 -> 7 is a no-op (serde defaults). Playback range = preview if set
  else project range; `Playback::frame_after(from, steps)` is pure (Once stops on the last frame,
  Loop wraps, Ping-Pong shows each end once); real-time steps = floor(elapsed * fps), every-frame
  takes at most one step per 1/fps without catch-up; Once at the end restarts at the start on play.
  fps conversion: keys are in seconds and never move; frame data (range, preview, markers, current
  frame) converts via the 087 millisecond rule `ms = round_half_away(f*1000/old)`, new frame
  `floor(ms*new/1000 + 1/2)` in integer arithmetic. Display: frames, SMPTE non-drop `HH:MM:SS:FF`,
  seconds `1.400 s`.
- Commands: `timeline.set_fps|set_preview|clear_preview|set_mode|set_frame_dropping|set_display|info`,
  `marker.add|remove|rename|move|set_color|list|jump`; keys M, Alt+Comma, Alt+Period; RPC
  `timeline.get` now also reports preview, mode, drop_frames, display, text, markers.
- Deviations: (1) the UI controls are appended to the two existing button rows and the marker
  lane lives inside the unchanged 30 pt strip, because an existing view-menu test
  (`panes.rs`) depends on the panel height; (2) existing tests changed only for the format
  version (`version: 6` -> `7`, the "future version" cases 7 -> 8, the default key table in
  `keymap.rs` got the three new rows); (3) `Tx::set_settings_and_time` was added so a fps
  conversion is one valid edit.
- Files: crates/core/src/{time,scene,document,file,lib}.rs, crates/command/src/{timeline,lib}.rs,
  crates/command/src/builtin/{timeline,mod}.rs, crates/rpc/src/{host,docs}.rs,
  crates/ui/src/{timeline,app}.rs, crates/ui/keymap/default.toml, tests (above, plus
  crates/core/tests/{file,tracks}.rs, fixtures scene-v7*.forge), docs/specs/{ANIM,CORE,COMMANDS,
  UI,RPC}.md, docs/manual/reference/*, docs/roadmap/{BACKLOG.md,areas/G-animation.md}, this brief.
- Open questions (G5 §4): where the reference sets preview range, markers and playback modes;
  whether markers carry ranges/comments; what the reference does with frames when fps changes
  (keys at the same seconds is the planner's decision, the frame fields are our rule); SMPTE
  drop-frame at 29.97 (fps here is an integer 1..240).
- Gate: `GATE: 1130 tests passed, 0 failed` ... `cleanroom: ok` ... `ok: 471 cases` ... `GATE: all passed`.
