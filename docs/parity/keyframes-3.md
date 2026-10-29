# Keyframes III — observation (O16)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [keyframes.md](keyframes.md) and [keyframes-2.md](keyframes-2.md): keys
made with the record command, sampled at every frame in the range plus the half frames
listed, at full precision. Folder: `tests/golden/keyframes/<case>/`, with
`observation = "docs/parity/keyframes-3.md"`.

Unblocks: confirming or breaking the parity-pending rules brief 09 derived from only a
few samples (docs/specs/ANIM.md §1):
- evaluation time rounded to whole milliseconds;
- the 19-step solver and its tie-break;
- handle length as the harmonic mean at middle keys;
- the slope at a monotone middle key;
- the overshoot clamp.

## Cases (position X unless stated)

| Case | fps | Keys (frame → value) | Extra frames | Tests which rule |
|---|---|---|---|---|
| mono-uneven | 30 | 0 → 0, 10 → 100, 60 → 150 | 9.5, 10.5, 35 | slope and handle at a monotone middle key with uneven spans |
| mono-steep | 30 | 0 → 0, 30 → 10, 60 → 300 | 29.5, 30.5 | overshoot clamp: shallow segment, then a steep one |
| mono-steep-rev | 30 | 0 → 0, 30 → 290, 60 → 300 | 29.5, 30.5 | overshoot clamp, the other way round |
| fps-24 | **24** | 0 → 0, 24 → 100 | 0.5, 11.5, 12, 23.5 | millisecond rounding at another fps (set the document to 24 fps before recording) |
| fps-25 | **25** | 0 → 0, 25 → 100 | 0.5, 12, 12.5, 24.5 | the same, at 25 fps |
| dense-quarter | 30 | 0 → 0, 30 → 100 | every quarter frame from 0 to 30 (121 samples) | time rounding and the solver grid at fine sample spacing |
| many-keys | 30 | 0 → 0, 7 → 40, 13 → −20, 29 → 80, 41 → 80, 60 → 0 | 6.5, 7.5, 12.5, 13.5, 40.5, 41.5 | a mix of extremes, a plateau and uneven spans |

Sample the full range of each case: 0–90 frames at 30 fps, 0–72 at 24 fps and 0–75 at
25 fps. `samples.json` holds `fps` as set.

Record `tangents.json` (as in O12) for `mono-uneven` and `many-keys`.

## Notes
TODO
