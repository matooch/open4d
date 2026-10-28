# Keyframes and interpolation — observation (O2)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-28
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)); level B parity (numeric
samples, [PLAN.md](../PLAN.md)). Unblocks: brief 09 (keyframe tracks + interpolation).

Every case: new document at **30 fps**, project range 0–90, a default cube, keys on its
**position X** only (one track). The probe sets the keys listed, then reads the evaluated
position X at **every frame 0…90** plus the half frames 0.5, 14.5, 29.5 (if sub-frame
time can be set), and saves `samples.json`:

```json
{
  "kind": "track",
  "param": "pos_x",
  "fps": 30,
  "tolerance": 1e-4,
  "samples": [[0.0, 0.0], [1.0, 0.123], ...]
}
```

`samples` = `[frame, value]` pairs in frame order. Folder:
`tests/golden/keyframes/<case>/` with `samples.json` and a `case.toml` whose `[params]`
lists the keys as `keys = [[frame, value], ...]` plus the options set.

**Keep the default interpolation** unless a row says otherwise: the default curve is what
we need. O0 already saw it isn't linear (keys 0 → 0 and 30 → 300 gave 150.0004 at frame
15, where a symmetric ease would give exactly 150): record full precision (repr of the
float, no rounding) so that small asymmetry is visible. The app version lives in this file's
header (fill it in), not in `case.toml`.

## 1. Defaults (GUI or probe read-back)

| What | Value shown |
|---|---|
| Default interpolation of a new key (Preferences > Animation, and the key's own field) | Spline (read from a key made by the record command; Preferences not opened) |
| Default tangent mode / "auto tangents" setting and its clamp option, as labelled | key reports automatic-tangent mode 1; the label and the clamp option were not read (no scripting access to the labels) |
| Overshooting prevention / "zero length" / "zero angle" style options on a new key, as labelled | not read; the curves behave as if overshoot is prevented: the 0 → 100 → 0 and 0 → 100 → 150 cases never exceed 100 and 150 |
| Track before/after behaviour (what the value is before the first and after the last key) | Constant: the first key's value before it, the last key's value after it (before-after: 50 up to frame 20, 80 from frame 40) |
| Key time unit as shown (frames, seconds) | frames at 30 fps (project range 0–90); times were set as frame / 30 seconds, so half frames work |

## 2. Cases

| Case | Keys (frame → value) | Options changed | Notes |
|---|---|---|---|
| two-keys | 0 → 0, 30 → 100 | – (default interpolation) | the basic ease curve |
| three-keys | 0 → 0, 30 → 100, 60 → 0 | – | peak: does the curve overshoot 100? |
| three-keys-monotone | 0 → 0, 30 → 100, 60 → 150 | – | tangent at the middle key |
| uneven | 0 → 0, 10 → 100, 60 → 0 | – | tangent with uneven spacing |
| linear | 0 → 0, 30 → 100, 60 → 0 | interpolation Linear on all keys | |
| step | 0 → 0, 30 → 100, 60 → 0 | interpolation Step on all keys | which side of the key does the jump happen? |
| before-after | 20 → 50, 40 → 80 | – | sample 0…90 shows the out-of-range behaviour |
| rotation | 0 → 0°, 30 → 270° on rotation H | – | record values in degrees; one sample case for angle tracks |
| vector | position X, Y, Z each keyed 0 → 0, 30 → 100 | keyed with the "record" command, not per track | how many tracks does one record make? |

For `rotation` and `vector`, adapt `param` (`rot_h`, `pos_y`, …); `vector` saves one
`samples.json` per track as `samples-<param>.json`.

## 3. Behaviour notes
- Keys were made with the **Record Active Objects** command (the record button's command), after setting the time and the value, because keys added directly through the scripting interface get a different curve: for 0 → 100 over 30 frames the direct key reads 1.70 at frame 0.5, the recorded one 0.0857 (an ease). Only the recorded form is stored here. The single-track cases then deleted the other recorded tracks.
- One record on a cube makes **9 tracks**: Position X/Y/Z, Scale X/Y/Z, Rotation H/P/B, each with a key per recording (this is the answer to the `vector` row). The `vector` case keeps the three position tracks only, as `samples-pos_x.json`, `-pos_y`, `-pos_z`; they are identical.
- Shape of the default curve (spline): ease in and out. `two-keys` reads 0.0857 at frame 0.5, 47.45 at 14.5, 50.00014 at 15 (not exactly 50: tiny asymmetry, see O0) and 99.914 at 29.5. `three-keys` peaks at exactly 100 at the middle key with no overshoot and is symmetric (92.56 at frames 25 and 35); `three-keys-monotone` passes the middle key at 100 with a slope (102.47 at frame 31) and never exceeds 150; `uneven` reaches 100 at frame 10 and does not overshoot.
- `linear` and `step` were made by setting the interpolation on all keys after recording. Step: the value jumps **at the key's own frame** (frame 29.5 reads 0, frame 30 reads 100; frame 59 reads 100, frame 60 reads 0), i.e. each key's value holds from its frame until the next key.
- `rotation`: the rotation H track is keyed in degrees (0 → 270 over 30 frames, no wrap to -90); samples are the evaluated H in degrees, 270.0 at frame 30 and after.
- Samples are the object's evaluated local position X (or H) at each frame 0–90 and at 0.5, 14.5, 29.5, written as the floats the app returned. `tolerance` 1e-4.
- Not observed: autokey, the timeline's display of keys, shortcut keys, keyframe time snapping.
