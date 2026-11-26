# Behaviours, live input and play mode (area P)

Source: **our own design** (owner request 2026-11-26, decision D21,
[../roadmap/areas/P-behaviours.md](../roadmap/areas/P-behaviours.md)). The reference app has no
equivalent, so nothing here is parity data. Luau is documented at <https://luau.org>, and its
sandboxing guide at <https://luau.org/sandbox>. The briefs that implement each section are in
brackets. Rules marked *(open)* are left for the implementing brief to decide, and it records
the choice here.

## 1. Languages and crates [215, 221]

- **Luau** through `mlua` with the `luau` and `vendored` features (MIT). New crate
  `crates/luau` (package `forge-luau`). Rhai (D5) stays for the existing expression node,
  script node and plugin packages; nothing is migrated.
- **Rust** behaviours implement the `Behaviour` trait (§4.4) from the new crate
  `crates/behaviour` (package `forge-behaviour`), which also holds the run loop, the cache and
  the Luau adapter.
- File extension `.luau`. Project scripts live in `<project>/scripts/`. A behaviour may also hold
  its source inline in the tag (§3).

## 2. Luau sandbox [215]

Every Luau state is created with `Lua::sandbox(true)`: the standard libraries are read-only and
each script gets its own global table.

| Rule | Value |
|---|---|
| Libraries | `base` (without `loadstring`, `getfenv`/`setfenv`, `collectgarbage` beyond `"count"`), `string`, `table`, `math`, `bit32`, `utf8`, `buffer`, `vector`. **No** `os`, `io`, `debug` or `ffi`. |
| `require` | Only `"<name>"` resolved inside `<project>/scripts/` (`.luau`, no `..`, no absolute paths), cached per state |
| `math.random` | Replaced by the seeded SplitMix64 generator of SCRIPTING.md §3. `math.randomseed` sets the state's seed. `rand(seed)` works the same as in Rhai |
| Clock | No wall clock. `time()` and `frame()` come from the run (§5). Play mode adds `realtime()` (§6), which is excluded from timeline mode |
| Memory | 256 MiB per state (`Lua::set_memory_limit`). Exceeding it raises a `Limit` error |
| CPU | Interrupt every 1,024 instructions, checking a per-call budget: builder script 10 s total; `start` 1 s; each `update` 50 ms (timeline mode) or 20 ms (play mode). Exceeding the budget raises a `Limit` error and disables the behaviour for the run |
| Errors | `ScriptError { kind: Syntax \| Runtime \| Limit, line, message }`, the same shape as SCRIPTING.md §1 |

Floating point: Luau numbers are IEEE doubles. `math.sin` and the other `math` functions call the
platform C library, whose last bits can differ between OSes. So cross-OS tests compare at 1e-9
relative, while tests on the same OS compare exactly. The `fmath` table (`fmath.sin`,
`fmath.cos`, `fmath.exp`, …) gives the workspace's own deterministic maths when bit-identical
results across OSes matter *(open: whether to ship it in 215 or later)*.

## 3. The behaviour tag [216]

Node type `forge.behaviour` (category Tag). It goes on any object. Params:

| Param | Type | Default | Rule |
|---|---|---|---|
| `enabled` | bool | true | |
| `language` | enum `luau`, `rust` | `luau` | |
| `script` | string | `""` | Project-relative path under `scripts/`, or empty |
| `source` | string (multiline) | `""` | Inline source, used when `script` is empty |
| `rust_type` | string | `""` | Registered Rust behaviour id, when `language` is `rust` |
| `priority` | int | 0 | Execution order (§5.2) |
| `seed` | int | 0 | Seeds the state's `math.random` |
| exposed params | — | — | Declared by the script (§3.1) |

### 3.1 Exposed params

A script declares its inspector params in a top-level `params` table, read once at load:

```lua
params = {
  stiffness = { kind = "float", default = 40, min = 0, max = 500, label = "Stiffness" },
  target    = { kind = "link", default = nil, label = "Target" },   -- object link
  mode      = { kind = "enum", default = "spring", options = { "spring", "follow" } },
}
```

Kinds and rules are those of the plugin params (PLUGINS.md §2), plus `link`, a node reference.
The params become data-driven descriptors on the tag instance, under ids `p.<name>`, so the UI,
undo, animation (they can be keyed) and serialization all follow (CLAUDE.md). Changing the
`params` table keeps the values of params whose name and kind still match, and drops the rest.

## 4. Lifecycle [216, 221]

### 4.1 Luau functions

All functions are optional. `ctx` is the behaviour's context (§4.3).

| Function | When |
|---|---|
| `start(ctx)` | Once, at the run's first frame, before any `update` |
| `update(ctx, dt)` | Every step. `dt` is the fixed step in seconds (1 / fps / `substeps`) |
| `late_update(ctx, dt)` | Every step, after every behaviour's `update` |
| `on_input(ctx, event)` | Play mode only, once per input event, before the step that consumes it (§7.4) |
| `stop(ctx)` | When the run ends (play stopped, or the cache dropped). No outputs are written |

State is ordinary Luau: script-level locals and the `ctx.state` table persist for the whole
run. A new run starts from a fresh Luau state.

### 4.2 Steps

- `substeps` is a scene setting, `behaviour.substeps`, default 1, range 1–16.
- Timeline mode: frame `f` runs `substeps` steps with `time()` advancing by `dt` each step.
- Play mode: §6.

### 4.3 `ctx`

| Member | Meaning |
|---|---|
| `ctx.self` | The host object (an `Object` handle) |
| `ctx.params` | The exposed param values at this frame, animated values included |
| `ctx.state` | A table that persists for the run |
| `ctx.input` | The input API (§7.3) |
| `ctx.find(path_or_name)` | `Object` handle or `nil` |
| `ctx.spawn(template, opts)` / `ctx.despawn(obj)` | Runtime objects (§8) |
| `ctx.log(...)` | Writes to the Console panel and the run log |

An `Object` handle reads evaluated values (`obj.position`, `obj.rotation`, `obj.scale`,
`obj:param(id)`, `obj.children`, `obj.world`) and writes outputs (`obj.position = vector(…)`,
`obj:set(id, value)`). Writes go to the behaviour output layer (§5.3), **never** to the document.

### 4.4 Rust behaviours

```rust
pub trait Behaviour: Send {
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), BehaviourError> { Ok(()) }
    fn update(&mut self, ctx: &mut Ctx, dt: f64) -> Result<(), BehaviourError>;
    fn late_update(&mut self, _ctx: &mut Ctx, _dt: f64) -> Result<(), BehaviourError> { Ok(()) }
    fn on_input(&mut self, _ctx: &mut Ctx, _e: &InputEvent) -> Result<(), BehaviourError> { Ok(()) }
    fn stop(&mut self, _ctx: &mut Ctx) {}
}
```

Rust behaviours register like node types, with an id, a factory and a param descriptor list
(`registry.register_behaviour(...)`). They use the same `Ctx` as Luau (the Luau adapter is
itself a `Behaviour`). Loading native code at run time, from a dynamic library over a versioned
C ABI, is brief 221 part 2 *(open: the ABI shape)*. It must keep the workspace's `unsafe` rule.

## 5. Evaluation [216]

### 5.1 Where behaviours run

Per frame, after animation and before expressions:

`animation → behaviours → expression tags → generators → deformers → dynamics`

So a behaviour can set a cloner's count or an effector's strength, and the generators see it.
A behaviour reads **animated** values (keys and earlier behaviours), not generator output. It can
read the previous frame's generator output via `obj:previous_mesh_bounds()` *(open: whether more
is needed)*.

### 5.2 Order

Behaviours run sorted by `priority` (ascending), then in tree order. `late_update` runs in the
same order after all `update` calls.

### 5.3 Output layer

Writes during a step go to the run's output layer: a map `(node, param) → value` plus
transforms. It sits on top of the animated values, and expression tags apply after it. At each
frame end, the layer is the frame's **output snapshot**. Writing a param that a later behaviour
in the same step also writes: the later write wins. Writes to the document (commands) are
refused inside `start`/`update` with the error `behaviours cannot run commands; use a builder
script`.

### 5.4 Timeline mode and the cache (`BehaviourCache`)

The rule of DYNAMICS.md §5 applies:

- The run starts at the scene's start frame (`timeline.start`, default 0). `start` runs, then
  the steps of the start frame.
- The cache holds one output snapshot per frame. Asking for a later frame steps forward frame by
  frame from the last cached frame. An earlier frame reads the cache (nothing re-runs). A frame
  before the start has no behaviour output.
- Any document edit that changes the scene revision drops the cache and ends the run (`stop`).
  The next evaluation starts a new run from the start frame. So the output at frame `f` depends
  only on the document, never on how the timeline was scrubbed.
- Input in timeline mode comes only from input tracks (§7.5).
- Rendering (`forge render`, the render queue) evaluates through the same cache, so a sequence is
  byte-identical between runs, OSes aside (§2).

### 5.5 Errors

A behaviour error stops that behaviour for the run. Its outputs from earlier frames stay cached.
The error shows on the tag (diagnostic, as for expression tags) and in the Console. Other
behaviours continue.

## 6. Play mode [219]

- **Play** (editor toolbar, shortcut *(open)*, command `play.start`) begins a live run from the
  current frame: its `start` uses that frame's animated values. The document is not changed while
  playing, except by recording (§7.6).
- **Clock:** a fixed-step accumulator. Each display frame adds the real elapsed time (clamped to
  0.25 s), then runs as many steps of `dt = 1 / fps / substeps` as fit. The timeline frame
  follows (`frame = start + steps / substeps`). If more than 8 steps are due in one display
  frame, the extra time is dropped and a "behind" counter increments.
- **Rendering** uses the viewport's Render mode (real-time tiers, REALTIME.md). Play mode doesn't
  pick the tier.
- **Loop:** with the timeline's loop on, reaching the end frame restarts the run from the start
  frame (`stop`, fresh state, `start`).
- **Stop** (`play.stop`, Esc in the viewport) ends the run, calls `stop`, despawns runtime objects
  and returns to the frame where play began. The timeline cache is not reused.
- **Hot reload:** saving a script while playing restarts that behaviour's run (new state,
  `start`). If the script defines `on_reload(ctx, old_state)`, that is called instead of `start`,
  with the old `ctx.state` table copied in.
- **Input focus:** while playing with the viewport focused, keys go to the input system first,
  and only Esc and the play shortcut reach the editor.

## 7. Input [217, 218]

### 7.1 Devices (`crates/input`, package `forge-input`)

| Device | Library | Events |
|---|---|---|
| Keyboard, mouse | the editor window (egui/winit) | key down/up (key name), mouse move, buttons, wheel |
| Gamepad | `gilrs` (MIT/Apache-2.0) | buttons, axes (−1..1), connect/disconnect, by pad index |
| MIDI | `midir` (MIT), behind the feature `midi` (on Linux it needs ALSA) | note on/off (note, velocity 0..1), CC (channel, controller, 0..1), pitch bend (−1..1) |
| OSC | `rosc` (MIT/Apache-2.0) over UDP, port setting, default 9000 | address and float/int/bool args |
| Synthetic | `input.inject` (§7.7) | any of the above |

All events carry a monotonic timestamp from the input thread. Device threads push to one queue,
which the run drains at each step.

### 7.2 Input map (stored in the document)

Scripts use **actions**, not devices, so a scene works with any controller. An action has an id,
a kind (`button`, `axis`, `vector2`) and bindings, for example:

```
throttle (axis):  gamepad.left_y   | midi.cc(1, 7)   | key W/S as -1/+1
jump (button):    gamepad.south    | key Space      | midi.note(1, 36)
look (vector2):   gamepad.right_stick | mouse.delta (scale 0.01)
```

Each binding has optional `scale`, `deadzone` (axes, default 0.1 on gamepad sticks) and
`invert`. The map is a document setting edited in the Input panel. Commands: `input.add_action`,
`input.bind`, `input.unbind`, `input.remove_action`. The combined value of an action is the
binding with the largest absolute value at that moment.

### 7.3 Script API

`ctx.input:axis("throttle")`, `ctx.input:vector("look")`, `ctx.input:held("jump")`,
`ctx.input:pressed("jump")` (true on the step it went down), `ctx.input:released(...)`. Raw access
(`ctx.input:raw("midi.cc(1,7)")`) exists for quick experiments, but it is **not recorded**, and
in timeline mode it reads 0.

### 7.4 Event timing

In play mode, events are assigned to the first step whose time is at or after the event's
timestamp. `on_input` runs for each event before that step's `update`, and action values change
at that step. In each step, `pressed` is true if the action went down since the previous step,
even if it also went up again.

### 7.5 Input tracks

Each action can have an **input track** in the document: a sampled track at the step rate.
Axes and vectors are floats (held between samples, no interpolation). Buttons are step keys.
It is shown and edited in the dope sheet like an animation track. In timeline mode, action
values come only from input tracks, and an action with no track reads 0 / not held.

### 7.6 Recording

**Record** while playing writes every action's values per step into its input track, replacing
the track over the recorded frame range. When play stops, the recording becomes one undo step
(`input.record` command result). After that, the same scene in timeline mode reproduces the live
run's actions exactly. Its outputs match exactly when every behaviour is deterministic (no
`realtime()` and no raw input).

### 7.7 Synthetic input (tests and agents)

`input.inject { events: [{ at_step | at_time, device, control, value }] }` queues events into a
live run, or into a headless play run (`play.start { headless: true, steps: N }`). This is how
tests, CI and MCP agents drive interactive scenes without hardware.

### 7.8 No-code wiring

The node expression graph (NODES.md) gets an `input.action` node (output: `value` float,
`vector` Vec3, `held` bool) that reads the action from the input track (timeline mode) or the
live value (play mode). So a knob can drive a param through an expression tag without a script,
the way the reference app's node graph links object params.

## 8. Runtime objects [219]

`ctx.spawn(template, { position=…, parent=… })` instantiates a copy of a template object (any
object in the document, usually hidden) into the run's **runtime layer**. These objects evaluate
and render like document objects, but are not stored in the document, have no undo, and are not
listed in the Object Manager except in a "Runtime" group while playing. `ctx.despawn(obj)` removes
one. In timeline mode the runtime layer is part of the per-frame snapshot, so spawned objects
scrub too. Limit: 10,000 runtime objects per run, beyond which `spawn` raises a `Limit` error.
**Bake** (`behaviour.bake { frames }`) turns a frame range's runtime objects and outputs into
ordinary objects and keys (one undo step).

## 9. Builder scripts (scene building in code) [215]

A Luau builder script runs once (`forge run build.luau`, the Script Manager, RPC/MCP
`script.run { language: "luau" }`). It has the same reach as Rhai's SCRIPTING.md §3, translated
to Lua: `run(id, args)`, `create(type, args)`, `set_param`, `set_transform`, `frame(n)`,
`scene()` queries, `print`. It is one undo step, rolled back completely on error, and limited by
the same caps as SCRIPTING.md §5 (operations become instruction-interrupt counts). Every Rhai
example under `examples/scripts/` gets a Luau twin with the same result (a test).

## 10. Budgets (S11)

| Measure | Budget | Where measured |
|---|---|---|
| Luau `update` for a behaviour that writes 1,000 clone transforms through the bulk API (§11) | ≤ 2 ms per step (M5 Pro, release build) | `cargo bench -p forge-behaviour --bench s11` |
| Same with per-object handles (1,000 `obj.position =` writes) | ≤ 8 ms (documented, not the primary path) | same |
| Play mode, the S11 scene, Tier 1 | ≥ 60 fps | the S10 bench harness, with behaviours on |
| Timeline cache memory | ≤ 64 bytes per written value per frame | test |
| Input latency (event → step that consumes it) | ≤ 1 step + 1 display frame | test with injected timestamps |

## 11. Bulk API and effector [220]

- `ctx:clones(cloner)` returns a view of the cloner's evaluated clones (count, positions,
  rotations, scales, colours, `index`), in **buffers** (Luau `buffer`) or as `vector` arrays.
  `view:write_positions(buf)` (and rotations/scales/colours) writes a whole array in one call.
  The writes apply as a **behaviour effector stage** on that cloner, after its own effectors
  *(open: before or after fields)*.
- `forge.luau_effector`: an effector whose per-frame function is Luau
  (`effect(ctx, clones, dt)`), stateful across frames like a behaviour and cached the same way.
  Field falloff applies to its output like any effector (MOGRAPH.md).

## 12. Agents and automation [215–222]

Commands (CLI, RPC, MCP, D3): `behaviour.add`, `behaviour.set_source`, `behaviour.state` (read
`ctx.state` of a live run as JSON), `play.start`, `play.stop`, `play.step { n }` (headless),
`input.inject`, `input.*` map commands, `behaviour.bake`. The MCP tool list exposes them, with
the error messages of §2 and §5.5, so an agent can write a behaviour, play it with synthetic
input, read the state and fix the script without a display.

## 13. Runtime (`forge play`) [222]

`forge play scene.f3d [--fullscreen] [--tier 1|2|3] [--record out.f3d]` opens a window with only
the viewport, in play mode, with the scene's input map. There's no editor UI. Esc quits. Packaging a
scene as a standalone app is out of scope for v1.0.

## 14. Tests (per brief, in its crate)

- Sandbox: each refused library and function, `require` escapes, memory and CPU limits, error
  lines [215].
- Builder parity: each Rhai example and its Luau twin give the same scene [215].
- Lifecycle order and the step count per frame with substeps; `start` once; `stop` on edit [216].
- Determinism: frames 0–N evaluated forward, then scrubbed randomly, equal exactly. A document
  edit restarts the run. Two runs are byte-identical [216].
- Output layer: priority order, last write wins, commands refused, expressions apply after [216].
- Input: binding resolution (largest absolute value, deadzone, invert), injection timing,
  `pressed`/`released` [217]. Record, then timeline replay equals the live outputs exactly [218].
- Play: the fixed-step accumulator with a fake clock (8-step cap, behind counter), stop restores
  the frame and despawns, hot reload and `on_reload` [219].
- Bulk API and the budgets of §10 [220]. A Rust behaviour and its Luau equivalent give the same
  outputs [221]. S11 [222].
