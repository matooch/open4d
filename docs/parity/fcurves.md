# F-curves: tangents and behaviours — observation (O38)

Status: observed · App version: Cinema 4D R25 (build 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-20
Method: as [keyframes.md](keyframes.md): 30 fps, keys made with the record command on a
cube's position X, then **changed through the key's scripting interface** as the row
says. Sample every frame 0–90 plus the half frames 14.5 and 29.5.

Output:
- `tests/golden/keyframes/<case>/samples.json` (kind `track`, param `pos_x`);
- `tangents.json` of the edited keys, as `[frame, value, left_dt, left_dv, right_dt, right_dv]`;
- `observation = "docs/parity/fcurves.md"`.

Unblocks: G/089, 093.

## Key flags (fill once)
List every per-key setting the scripting interface exposes, with its default on a recorded
key: interpolation, auto tangents, clamp, zero angle, zero length, break tangents, lock
tangent angles, lock tangent lengths, weighted tangent, and so on.

| Setting | Our key | Default on a recorded key | Options |
|---|---|---|---|
| Interpolation | yes | 1 = Spline | 1 Spline, 2 Linear, 3 Step, 4 Dummy (the interpolation setting's four options) |
| Auto tangents | yes | 1 (on) | 0 / 1 |
| Clamp, i.e. remove overshooting | yes | 0 (off) | 0 / 1 |
| Remove overshooting, auto | yes | 1 | 0 / 1 |
| Break tangents | yes | 0 | 0 / 1 |
| Lock tangent angles | yes | 0 | 0 / 1 |
| Lock tangent lengths | yes | 0 | 0 / 1 |
| Keep visual angle | yes | 0 | 0 / 1 |
| Weighted tangent | yes | 0 | 0 / 1 |
| Automatic weighting | yes | 0 | 0 / 1 |
| Tangent preset | yes | 2 | numeric slot only; the interface exposes no names |
| Angle | yes | 1 | numeric slot only; no names |
| Quaternion interpolation | yes | 2 | numeric slot only; no names |
| Mute | yes | 0 | 0 / 1 |
| Breakdown | yes | 0 | numeric |
| Lock time / lock value | yes | 0 / 0 | 0 / 1 |
| Key time / key value | yes | key time / key value | settable |
| Left tangent (time and value) | yes | -9.999996 frames / -33.333333 | settable |
| Right tangent (time and value) | yes | 9.999996 frames / 33.333333 | settable |
| Zero angle | no | not exposed | - |
| Zero length | no | not exposed | - |

## Cases (keys 0 → 0 and 30 → 100 unless stated)

| Case | Change | Notes |
|---|---|---|
| user-tangent-steep | key 30: auto off, left tangent (−5 frames, −60) | value at frame 15 = 22.0046; the tangent is stored exactly as set |
| user-tangent-flat | key 0: auto off, right tangent (10 frames, 0) | value at frame 15 = 49.9999; the start of key 0 is flat |
| broken | key 30 (with a key 60 → 0 added): break tangents, left (−10, −20), right (10, 40) | break flag reads back 1; value at frame 15 = 42.5001 and at frame 45 = 50.0001, so the two sides are independent |
| weighted-long | key 30: weighted on, left tangent (−20 frames, −50) | value at frame 15 = 49.3544; compared with weighted-short the tangent length clearly matters |
| weighted-short | key 30: weighted on, left tangent (−2 frames, −5) | value at frame 15 = 36.3583; the length matters (13.00 below weighted-long at the same tangent value) |
| zero-angle | three keys 0 → 0, 30 → 100, 60 → 150: middle key "zero angle" on | not observed: the key's scripting description has no zero-angle parameter (see Notes) |
| zero-length | the same three keys: middle key "zero length" on | not observed: the key's scripting description has no zero-length parameter (see Notes) |
| clamp-off | keys 0 → 0, 30 → 100, 60 → 0: overshoot prevention / clamp off on the middle key | no: clamp is off by default (the setting reads 0), and with it explicitly off all 93 samples still stay inside [0, 100] |
| ease-in | the app's ease-in preset on key 30, if the scripting interface has one | not observed: the only preset slot is the numeric tangent preset (default 2), which exposes no preset names, so an ease-in preset cannot be selected by name |
| linear-to-spline | key 0 Linear, key 30 Spline, key 60 → 0 Spline | key 0 reads back interpolation 2 with all four tangent values 0; keys 30 and 60 stay Spline (1) |
| after-repeat | keys 0 → 0, 30 → 100; track after-behaviour Repeat | repeats every 30 frames: 50.0001 at frames 45 and 75; at the exact boundary frame 90 it holds the last key value 100 |
| after-offset | the same; Offset Repeat | adds 100 per cycle: 150.0001 at 45, 200 at 60, 250.0001 at 75, 300 at 90 |
| after-oscillate | the same; Oscillate | mirrors every cycle: 50.0001 at 45, 0 at 60, 50.0001 at 75, 100 at 90 |
| after-continue | the same; Continue | stays at 100 after the last key: the last key's auto tangent is flat, so there is no slope to continue |
| before-repeat | keys 30 → 0, 60 → 100; before-behaviour Repeat | repeats backwards: 50.0001 at frame 15; at the exact boundary frame 0 it holds the last value of the shifted cycle, 100 |

## Follow-up cases (planner, after the O38 parity repair)

Redo these two cases as new case folders (keep the old ones):

| Case | Redo as | What was missing |
|---|---|---|
| linear-to-spline | linear-to-spline-2 | `case.toml` must record each key's interpolation (e.g. `interpolation_by_key = ["Linear", "Spline"]`), not only the description |
| broken | broken-2 | The read-back right tangent (10, 40) does not match the sampled curve, which follows the automatic flat tangent. Set the broken tangent, **read it back after setting**, evaluate the samples, and record both. If they still disagree, say so in the row |

## Notes

Case data: `tests/golden/keyframes/<case>/samples.json` (kind track, param pos_x, fps 30,
tolerance 0.0001) plus `tangents.json` for every edited key. Each sample file holds 93 frames:
0-90 every frame and 14.5, 29.5. All 12 observed cases pass `golden -- check` (938 cases).

How the changes were made (scripting interface of the reference app):
- auto tangents off: the key's Auto Tangents setting set to 0; break tangent: the Break Tangents setting = 1;
  weighted tangent: the Weighted Tangent setting = 1; clamp: the Clamp setting; interpolation: the key's interpolation setter.
- tangents: the key's left-tangent time setter with a time of -5 frames at 30 fps and its left-tangent value setter with -60.0
  (same for right). The curve has to be passed as the first argument; the binding rejects the
  one-argument form. Tangent lengths are stored in seconds, so the frame values in
  `tangents.json` are the seconds times 30.
- track after/before behaviour: the track's after and before behaviour settings with
  2 = Continue, 3 = Repeat, 4 = Offset Repeat, 5 = Oscillate.
- the keys themselves were made with the record command (the record-key command) after
  setting the cube's position X, as the form asks.

Not exposed by the scripting interface (cases marked not observed above): zero angle and zero
length. The key description holds three further settings that its parameter listing does not show;
setting them returns False and they cannot be read, so the two settings are GUI-only.
The tangent preset slot and the angle slot are numeric enums whose value
names are not exposed, so no preset can be selected by name.

Exact-boundary behaviour, worth knowing for the solver: with Repeat and Oscillate the value at a
frame that falls exactly on a cycle boundary is taken from the end of the cycle, not the start
(frame 90 in after-repeat = 100, not 0; frame 0 in before-repeat = 100).

