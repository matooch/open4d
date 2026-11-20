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
| Interpolation (id 924) | yes | 1 = Spline | 1 Spline, 2 Linear, 3 Step, 4 Dummy (c4d.CINTERPOLATION_*) |
| Auto tangents (id 919) | yes | 1 (on) | 0 / 1 |
| Clamp, i.e. remove overshooting (id 923) | yes | 0 (off) | 0 / 1 |
| Remove overshooting, auto (id 1245) | yes | 1 | 0 / 1 |
| Break tangents (id 915) | yes | 0 | 0 / 1 |
| Lock tangent angles (id 917) | yes | 0 | 0 / 1 |
| Lock tangent lengths (id 918) | yes | 0 | 0 / 1 |
| Keep visual angle (id 927) | yes | 0 | 0 / 1 |
| Weighted tangent (id 1244) | yes | 0 | 0 / 1 |
| Automatic weighting (id 1246) | yes | 0 | 0 / 1 |
| Tangent preset (id 1247) | yes | 2 | numeric slot only; the interface exposes no names |
| Angle (id 1240) | yes | 1 | numeric slot only; no names |
| Quaternion interpolation (id 1248) | yes | 2 | numeric slot only; no names |
| Mute (id 902) | yes | 0 | 0 / 1 |
| Breakdown (id 1234) | yes | 0 | numeric |
| Lock time (id 925) / lock value (id 926) | yes | 0 / 0 | 0 / 1 |
| Key time (id 901) / key value (id 910) | yes | key time / key value | settable |
| Left tangent (id 911 time, 913 value) | yes | -9.999996 frames / -33.333333 | settable |
| Right tangent (id 912 time, 914 value) | yes | 9.999996 frames / 33.333333 | settable |
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
| clamp-off | keys 0 → 0, 30 → 100, 60 → 0: overshoot prevention / clamp off on the middle key | no: clamp is off by default (id 923 = 0), and with it explicitly off all 93 samples still stay inside [0, 100] |
| ease-in | the app's ease-in preset on key 30, if the scripting interface has one | not observed: the only preset slot is the numeric tangent preset (id 1247, default 2), which exposes no preset names, so an ease-in preset cannot be selected by name |
| linear-to-spline | key 0 Linear, key 30 Spline, key 60 → 0 Spline | key 0 reads back interpolation 2 with all four tangent values 0; keys 30 and 60 stay Spline (1) |
| after-repeat | keys 0 → 0, 30 → 100; track after-behaviour Repeat | repeats every 30 frames: 50.0001 at frames 45 and 75; at the exact boundary frame 90 it holds the last key value 100 |
| after-offset | the same; Offset Repeat | adds 100 per cycle: 150.0001 at 45, 200 at 60, 250.0001 at 75, 300 at 90 |
| after-oscillate | the same; Oscillate | mirrors every cycle: 50.0001 at 45, 0 at 60, 50.0001 at 75, 100 at 90 |
| after-continue | the same; Continue | stays at 100 after the last key: the last key's auto tangent is flat, so there is no slope to continue |
| before-repeat | keys 30 → 0, 60 → 100; before-behaviour Repeat | repeats backwards: 50.0001 at frame 15; at the exact boundary frame 0 it holds the last value of the shifted cycle, 100 |

## Notes

Case data: `tests/golden/keyframes/<case>/samples.json` (kind track, param pos_x, fps 30,
tolerance 0.0001) plus `tangents.json` for every edited key. Each sample file holds 93 frames:
0-90 every frame and 14.5, 29.5. All 12 observed cases pass `golden -- check` (938 cases).

How the changes were made (scripting interface of the reference app):
- auto tangents off: `key.SetParameter(descid(919), 0)`; break tangent: id 915 = 1;
  weighted tangent: id 1244 = 1; clamp: id 923; interpolation: `key.SetInterpolation(curve, mode)`.
- tangents: `key.SetTimeLeft(curve, BaseTime(-5, 30))` and `key.SetValueLeft(curve, -60.0)`
  (same for right). The curve has to be passed as the first argument; the binding rejects the
  one-argument form. Tangent lengths are stored in seconds, so the frame values in
  `tangents.json` are the seconds times 30.
- track after/before behaviour: `track.SetParameter(descid(911 or 910), mode)` with
  2 = Continue, 3 = Repeat, 4 = Offset Repeat, 5 = Oscillate.
- the keys themselves were made with the record command (`c4d.CallCommand(12410)`) after
  setting the cube's position X, as the form asks.

Not exposed by the scripting interface (cases marked not observed above): zero angle and zero
length. The key description has ids 916, 920 and 921, but `GetDescription` does not list them;
`SetParameter` on them returns False and they cannot be read, so the two settings are GUI-only.
The tangent preset slot (id 1247) and the angle slot (id 1240) are numeric enums whose value
names are not exposed, so no preset can be selected by name.

Exact-boundary behaviour, worth knowing for the solver: with Repeat and Oscillate the value at a
frame that falls exactly on a cycle boundary is taken from the end of the cycle, not the start
(frame 90 in after-repeat = 100, not 0; frame 0 in before-repeat = 100).

