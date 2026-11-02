# F-curves: tangents and behaviours — observation (O38)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
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

## Cases (keys 0 → 0 and 30 → 100 unless stated)

| Case | Change | Notes |
|---|---|---|
| user-tangent-steep | key 30: auto off, left tangent (−5 frames, −60) | |
| user-tangent-flat | key 0: auto off, right tangent (10 frames, 0) | |
| broken | key 30 (with a key 60 → 0 added): break tangents, left (−10, −20), right (10, 40) | |
| weighted-long | key 30: weighted on, left tangent (−20 frames, −50) | Check that the length matters |
| weighted-short | key 30: weighted on, left tangent (−2 frames, −5) | |
| zero-angle | three keys 0 → 0, 30 → 100, 60 → 150: middle key "zero angle" on | |
| zero-length | the same three keys: middle key "zero length" on | |
| clamp-off | keys 0 → 0, 30 → 100, 60 → 0: overshoot prevention / clamp off on the middle key | Does it overshoot now? |
| ease-in | the app's ease-in preset on key 30, if the scripting interface has one | Otherwise skip |
| linear-to-spline | key 0 Linear, key 30 Spline, key 60 → 0 Spline | Mixed segments |
| after-repeat | keys 0 → 0, 30 → 100; track after-behaviour Repeat | Sample to 90 |
| after-offset | the same; Offset Repeat | |
| after-oscillate | the same; Oscillate | |
| after-continue | the same; Continue | |
| before-repeat | keys 30 → 0, 60 → 100; before-behaviour Repeat | Sample from 0 |

## Notes
TODO
