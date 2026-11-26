# Area P · Behaviours, live input and play mode

Owner's request (2026-11-26): go beyond what the reference app's scripting and node graph
offer. Scene objects get **behaviour scripts** with a start/update lifecycle (like a game
engine's component scripts) that keep state across frames. A behaviour reacts to **live inputs**
(keyboard, mouse, gamepad, MIDI controller, OSC), so a scene can be an interactive
motion-graphics piece or a small runtime. The same API lets people and AI agents build scenes in
code. Languages: **Luau** (sandboxed Lua) for scripts, **Rust** for heavy behaviours (decision
D21).

This is **our own design**: the reference app has no equivalent, so there is no parity data.
Every rule is specified in [../../specs/BEHAVIOURS.md](../../specs/BEHAVIOURS.md) and tested
against that spec.

## Is it possible?

Yes. Most of the parts already exist:

- **Stepping state frame by frame and scrubbing it** is solved: rigid bodies and particles step
  forward from a start frame and cache every frame (DYNAMICS.md §5, §9.7). Behaviours reuse
  that rule, so a timeline with behaviours scrubs and renders the same way every time.
- **Overrides that don't touch the document** already exist: expression tags write evaluated
  values on top of the animation without creating undo steps (NODES.md §5). Behaviour output
  goes through the same layer.
- **Every action is a command** (D3), so a Luau builder script gets the whole application:
  primitives, generators, MoGraph, materials, rendering, export.
- **Real-time rendering**: Tier 1 runs S10 at 100 fps on the M5 Pro (brief 214), which is
  enough for play mode.
- **Input libraries** are permissively licensed pure Rust (or thin wrappers): `gilrs` for
  gamepads, `midir` for MIDI, `rosc` for OSC. Keyboard and mouse come from the editor window.
- **Luau** builds from source on all three OS (`mlua` with the `luau` feature, vendored). Its
  sandbox, interrupt hook and memory limit are designed for untrusted game scripts.

## Design in one picture

```
 inputs ─► input map (named actions) ─┬─► live values (play mode)
 (keys, pad, MIDI, OSC, injected)     └─► input tracks (recorded, keyed, deterministic)
                                                   │
 animation ─► behaviours (Luau / Rust: start, update) ─► expressions ─► generators ─► deformers ─► dynamics ─► render
                  │  state per run, outputs cached per frame
                  └─ writes: transforms, params, spawned runtime objects (evaluation layer, never undo steps)
```

Two run modes, one script:

1. **Timeline mode (deterministic).** The behaviour starts at the scene's start frame and steps
   one fixed frame at a time. Inputs come only from **input tracks**. Outputs are cached per frame,
   so scrubbing, network rendering and `forge render` all give the same image.
2. **Play mode (live).** A real-time clock, live devices and the real-time renderer. You can
   **record** a performance: the input events become input tracks, and the recorded timeline
   then renders deterministically in timeline mode. This is how an interactive piece becomes
   a final 1080p sequence.

Scene building in code is the third use: a **builder script** (Luau, like today's Rhai scripts)
runs once, issues commands and is one undo step.

## Scenario S11 (acceptance)

An interactive motion-graphics scene built only by a Luau builder script. A cloner with 1,000
clones has a behaviour that reacts to a gamepad stick and MIDI CC knobs (injected as synthetic
events in the test), with state carried from frame to frame (a spring with velocity). The test:

- plays it live for 10 s and records the inputs to tracks;
- renders frames 0–240 of the recording, twice, byte-identically;
- scrubs back and forth with no change;
- stops play mode, leaving the document as it was apart from the recorded tracks;
- meets the S11 budgets in BEHAVIOURS.md §10 (behaviour update ≤ 2 ms per frame for 1,000 clones; play mode ≥ 60 fps at Tier 1 on the M5 Pro).

## Briefs

| Brief | What |
|---|---|
| 215 | Luau runtime (`forge-luau`): sandbox, limits, command bindings, builder scripts, `forge run *.luau` |
| 216 | Behaviour tag, lifecycle, timeline mode, per-frame output cache, exposed params |
| 217 | Input system (`forge-input`): devices, input map, synthetic injection, `input.*` commands |
| 218 | Input tracks: record, play back, dope sheet, Input node for no-code wiring |
| 219 | Play mode in the editor: live clock, fixed step, spawn/despawn, hot reload, script editor panel |
| 220 | Bulk clone and field API, Luau effector, performance budgets |
| 221 | Rust behaviours: `Behaviour` trait SDK, built-in registration, native dynamic loading |
| 222 | `forge play` runtime, S11 acceptance test, manual and examples |
