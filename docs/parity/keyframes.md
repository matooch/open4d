# Keyframes and interpolation — observation (O2)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)); level B parity (numeric
samples, [PLAN.md](../PLAN.md)). Unblocks: brief 08 (keyframe tracks + interpolation).

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

## 1. Defaults (GUI or probe read-back)

| What | Value shown |
|---|---|
| Default interpolation of a new key (Preferences > Animation, and the key's own field) | |
| Default tangent mode / "auto tangents" setting and its clamp option, as labelled | |
| Overshooting prevention / "zero length" / "zero angle" style options on a new key, as labelled | |
| Track before/after behaviour (what the value is before the first and after the last key) | |
| Key time unit as shown (frames, seconds) | |

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
How a key is made in the UI (record button, the dot next to a parameter, shortcut), what
autokey does, what the timeline shows for keys: TODO
