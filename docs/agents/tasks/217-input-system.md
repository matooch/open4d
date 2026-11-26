# 217 · Input system: devices, input map, injection

Status: ready · Area: P · Depends on: 216 · Observations: – (own design, D21) · Touches: input (new), ui, core, command, behaviour

## Goal
Live input from the keyboard, mouse, gamepads, MIDI and OSC, reached through named actions in
the document. Synthetic injection makes it testable without hardware. BEHAVIOURS.md §7.1–§7.4,
§7.7.

Spec: [../../specs/BEHAVIOURS.md](../../specs/BEHAVIOURS.md) (own design, D21; no parity data). Where the spec says *(open)*, decide, write the rule into the spec, and say so in the Report.

## Steps
1. `crates/input` (`forge-input`): the event type, the queue, and device threads for `gilrs`, `midir` (feature `midi`, on by default except where ALSA is missing in CI; document it) and `rosc`. Keyboard and mouse come from the viewport.
2. The input map in the document (§7.2), with the `input.*` commands, and an Input panel listing actions, bindings and live values (a "learn" button binds the next moved control).
3. The script API (§7.3) and event timing (§7.4).
4. `input.inject` and headless `play.start { headless, steps }` / `play.step` (only as much play-run machinery as injection needs; the editor play mode is 219).

## Done when
- [ ] The binding-resolution, deadzone/invert, timing and `pressed`/`released` tests of §14 [217] pass with injected events only (no hardware in CI).
- [ ] A manual check on the owner's Mac with a gamepad and a MIDI controller (Report: the devices and what was seen). Tick this when the owner confirms.
- [ ] `cargo deny` passes with the new crates, and the licences are listed in the release notices.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
