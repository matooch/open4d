# Keyframes II — observation (O12)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted, the same as [keyframes.md](keyframes.md) (O2): keys made with the
**record** command, 30 fps, sampled at every frame 0–90 plus the half frames listed, at
full precision. Folder: `tests/golden/keyframes/<case>/`, with
`observation = "docs/parity/keyframes-2.md"`. Unblocks: pinning the spline tangent rule
of brief 09.

## Cases

| Case | Keys (frame → value), position X | Extra half frames | Notes |
|---|---|---|---|
| scale-1000 | 0 → 0, 30 → 1000 | 0.5, 14.5, 29.5 | Does the curve shape scale with the value? |
| long-span | 0 → 0, 90 → 100 | 0.5, 44.5, 89.5 | Does the shape depend on the span? |
| four-keys | 0 → 0, 20 → 100, 40 → 50, 60 → 150 | 19.5, 20.5, 39.5, 40.5 | Mixed extreme and middle keys |
| zigzag-uneven | 0 → 0, 5 → 100, 45 → 0, 50 → 100 | 4.5, 5.5, 44.5, 45.5 | Short segments next to long ones |
| plateau | 0 → 0, 30 → 100, 60 → 100, 90 → 0 | 30.5, 59.5 | Two equal keys |
| negative | 0 → 100, 30 → −100 | 15 | Decreasing segment |
| mixed-interp | 0 → 0, 30 → 100, 60 → 0; the middle key set to Linear after recording | 29.5, 30.5 | Which side of a key does its interpolation govern? |
| step-first | 0 → 0, 30 → 100, 60 → 0; only the **first** key set to Step | 29.5, 30.5, 59.5 | Same question, for Step |

If any key's tangent can be read through the scripting interface (left and right
tangent: time and value offsets), record them for `two-keys` from O2, rebuilt, and for
`four-keys`, in a `tangents.json`:

```json
{ "keys": [[frame, value, left_dt, left_dv, right_dt, right_dv], ...] }
```

Use frames for the dt values, if that is how they read. If the tangents can't be read,
write that here.

## Notes
TODO
