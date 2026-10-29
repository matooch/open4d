# Keyframes II — observation (O12)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
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
- All 8 cases observed, no failures. Keys were made with the Record Active Objects command, as in O2. Each record made 9 tracks; only position X is sampled. Samples are at frames 0–90 plus the listed half frames, full float precision, `tolerance` 1e-4.
- **Tangents are readable** (each key has a left and a right tangent, time and value offsets). They are in `tangents.json` in `two-keys/` and `four-keys/`, as `[frame, value, left_dt, left_dv, right_dt, right_dv]`, dt in frames. Read-back: the first key has all-zero tangents. In `two-keys` the last key has dt ±9.999996 (a third of the 30-frame span, with a small float error) and dv ±33.333…, which is a third of the 100 rise. In `four-keys` the three later keys all have dt ±6.666672 (a third of the 20-frame spacing); dv is ±33.333… at the keys at 20 and 60 and ±16.666… at the key at 40 (a decrease of 50, so the right dv is negative). The left and right values of a key are mirror images. Whether the last key of `two-keys` really has non-zero tangents while the first has zero ones is as read; no rule is inferred here.
- `mixed-interp`: the middle key set to Linear. The segment **after** that key (30 → 60) became linear; the segment before it (0 → 30) kept the spline shape (frame 15 reads 50.00014, frame 29.5 reads 99.914). So a key's interpolation governs the segment that starts at that key. The linear segment reads 98.3 at 30.5 and 96.7 at 31 (a straight line from 100 at 30 to 0 at 60 would read 98.333 and 96.667); 50.0 at frame 45. The values are as the app returned them.
- `step-first`: only the first key set to Step. The first segment holds 0 up to frame 29.5 and jumps to 100 at frame 30; the second segment (30 → 60) stays a spline (99.914 at 30.5, 49.99986 at 45, 0.0857 at 59.5).
- `plateau`: the two equal keys (30 and 60, value 100) give exactly 100 at every frame between them; the curve does not overshoot.
- `negative`: the decreasing segment is the mirror of `two-keys` (−0.000286 at frame 15, the same tiny asymmetry as O2); −100 from frame 30 on.
- Not checked: the other tracks the record made (scale, rotation, position Y/Z); the sample JSON for `scale-1000` etc. was not compared by hand with a scaled `two-keys`; that comparison is for the implementer.
