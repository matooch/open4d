# Animation — keyframe tracks and interpolation (`crates/anim`, data in `crates/core`)

Sources: docs/parity/keyframes.md (O2, 9 cases), docs/parity/keyframes-2.md (O12, 8
cases) and docs/parity/keyframes-3.md (O16, 7 cases), golden folder
`tests/golden/keyframes/` (24 cases, 26 sample files, level B, tolerance 1e-4). Every rule below cites the case that shows it. The exact curve was
derived from those samples; nothing was read from the reference app beyond them.

## 1. Tracks and their evaluation

### 1.1 Data model (`forge-core::track`)
- `Interp { Spline (default), Linear, Step }`.
- `Key { time: f64 /* seconds */, value: f64, interp: Interp }`. `interp` governs the
  segment that **starts** at this key (`mixed-interp`: the Linear middle key made 30→60
  linear and left 0→30 a spline; `step-first`: only 0→30 steps).
- `Track { keys: Vec<Key> }`: sorted by time, no two keys at the same time, finite,
  at least one key (`Track::validate` → `TrackError::{Empty, NotFinite, Unsorted}`).
- Values are in **parameter units** (radians for angles). Frames are `frame / fps`
  seconds (the files use 30 fps).

### 1.2 Addressing
A node has `tracks: BTreeMap<String, Track>` keyed by a *track key*:
`"<param id>"` for an animatable Float parameter, `"<param id>.<x|y|z>"` for one
component of an animatable Vec3 parameter (`"transform.pos.x"`, `"transform.rot.x"` =
rotation H). `parse_target(key, find)` resolves a key against a type's descriptors; a key
that names anything else (missing, non-animatable, Int/Bool/Colour/..., bad axis) is
`TrackError::BadTarget`. Param ids may contain dots, so a full match against a Float
parameter wins before the last `.x|y|z` is split off. Int, Bool, Enum, Colour, Link and
mesh point tracks are addressed as in §1.8. Vec3 tracks need not cover all three components; a component without a
track keeps the stored value.

### 1.3 Edits
`Tx::set_track(id, key, Option<Track>)` (None removes), `Tx::set_key(id, key, Key)`
(creates the track, replaces a key at exactly the same time), `Tx::remove_key(id, key,
time)` (removes the track with its last key; a missing track or key is a no-op). All
validate first (`EditError::Track`), are undoable, and stamp the node's revision.

### 1.4 Evaluation time
`value_at(track, time_seconds)`:
1. **Time resolution** (cases `two-keys`, `long-span`, `scale-1000`, `negative`,
   `mixed-interp`; confirmed at 24, 25 and 30 fps by `fps-24`, `fps-25`,
   `dense-quarter`): the reference evaluates at times rounded to whole milliseconds,
   `t = round(time · 1000) / 1000`. Frame 1 at 30 fps reads as 0.990 frames, frame 2 as
   2.010. This is a ±0.01 frame time error the samples show on every curve (up to 0.05
   in value on a 0→100 rise), so it is reproduced; it is the same for all interpolations
   (`mixed-interp` Linear: 96.7 at frame 31, not 96.667).
2. **Keys** keep their exact times; only the evaluation time is rounded. A rounded time
   that falls just before a key it rounds to counts as exactly that key (`uneven`
   frame 10: 0.333 s rounds from the key at 0.3333.. s and reads 100 exactly). A rounded
   time just after a key stays after it (`zigzag-uneven` frame 5 reads 99.99963,
   `before-after` frame 20 reads 50.0000226).
3. **Out of range** (`before-after`): `t <= first.time` gives the first value, `t >=
   last.time` the last value (constant extension).
4. **Exactly on a key** the value is the key's (`three-keys-monotone` frame 30 reads
   100.0 for every interpolation).
5. Otherwise the segment is `(a, b)` with `a.time < t < b.time` and `a.interp` decides:
   - **Step** (`step`, `step-first`): `a.value`. The jump happens at the key's own time
     (29.5 → 0, 30 → 100).
   - **Linear** (`linear`, `mixed-interp`): `a.value + (b.value − a.value) · (t − a.time) /
     (b.time − a.time)`.
   - **Spline**: §1.5.
6. **Rotation** (`rotation`): a rotation component is a plain float track; no angle
   wrapping (0 → 270° passes 135° at frame 15).

### 1.5 Spline segment
Each segment is a cubic Bézier in a curve parameter `s ∈ [0, 1]`, with `x(s)` for time
and `y(s)` for value, control points
`P0 = (a.t, a.v)`, `P1 = (a.t + h_a, a.v + m_a·h_a)`,
`P2 = (b.t − h_b, b.v − m_b·h_b)`, `P3 = (b.t, b.v)`. `h` is a handle length in time and
`m` a slope in value per time. The sample is `y(s)` for the `s` with `x(s) = t`.

**Handle lengths** (`two-keys`, `four-keys`, `uneven`, `zigzag-uneven`). At the first key
`h = (t₁ − t₀)/3`, at the last `h = (tₙ − tₙ₋₁)/3`. At a middle key both sides use the
**harmonic mean of the two adjacent spans, divided by three**: `h = 2·l·r / (l + r) / 3`.
For equal spans that is `span/3` (the tangent read-back in `tangents.json`: 9.999996 of
30 frames, 6.666672 of 20); `uneven` (spans 10 and 50: fitted 5.556 = 50/9) and
`zigzag-uneven` (5 and 40) need the harmonic form: a third of the shorter span or of the
longer one does not fit.

**Slopes** (`two-keys`, `three-keys*`, `plateau`, `four-keys`, `uneven`). The first and
last keys have slope 0 (an ease). A middle key with `(v − v_prev)·(v_next − v) ≤ 0` (a
local extreme or a plateau edge) has slope 0: `three-keys` peaks at exactly 100,
`plateau` holds 100 between its equal keys, `four-keys` has a flat valley. Any other
middle key (monotone) has the **mean of the two segment slopes**
`m = ((v − v_prev)/l + (v_next − v)/r) / 2`, limited so that neither handle's value
offset exceeds its segment's rise: `|m|·h ≤ min(|v − v_prev|, |v_next − v|)` (no
overshoot). `mono-uneven` (spans 10 and 50 frames, rises 100 and 50) fits slope 5.5 per
frame exactly with handle 50/9, which is the mean of 10 and 1; the central difference
2.5 (brief 09) was wrong for uneven spans and is the same as the mean for equal spans
(`three-keys-monotone`). The clamp is proven by `mono-steep` (mean 5, limit 1) and
`mono-steep-rev`. The `tangents.json` read-back of `mono-uneven` and `many-keys` was
**not** used as ground truth for slopes: it shows a third of the left chord at every
key (also at extremes, whose evaluated slope is 0), so it is a display value; its
handle lengths are used only for tie breaking below.

Handle positions do not depend on the neighbours' interpolation (`mixed-interp`).

**Solving `x(s) = t`** is part of the observed behaviour, not just an implementation
detail (cases `two-keys`, `long-span`, `scale-1000`, `negative`, `four-keys`): the
reference finds `s` by bisection with a fixed number of halvings and returns the middle
of the final interval. With 19 halvings the value matches every sample to ≤ 3·10⁻¹⁰
(18 or 20 halvings leave errors of 1.4·10⁻⁴ and 7·10⁻⁵). The result is a sawtooth in the
time domain of 2⁻¹⁹ of the segment (5.7·10⁻⁵ frames for 30 frames), which is why
`two-keys` reads 50.000143 at frame 15 instead of 50. `x(mid) < t` goes up.

**Ties** (O16: `fps-24`, `dense-quarter`, `four-keys`, `two-keys`). When `x(mid) = t`
exactly (`|x(mid) − t| ≤ 10⁻¹² · span`; at 24 fps frames 3, 6, 9, 15, 18, 21 are
whole-millisecond times at dyadic fractions of the 1 s span) the direction is set by
the stored handle lengths, not by the curve position. The read-back `tangents.json`
shows handle times that are whole multiples of 4·10⁻⁷ s, a hair off a third (9.999996
of 30 frames, 6.666672 of 20, 2.333328 of 7, 2.000004 of 6, 5.333328 of 16, 3.999996 of
12, 6.333336 of 19: all equal `round(third / 4e-7 s) · 4e-7 s`). With handle error `d`
(grid handle = third − `d`), `x(s) = span·s − 3·d·s(1−s)(1−2s)`, so a shorter handle
(`d > 0`) puts the tie below `t` for `s < ½` (go up) and above it for `s > ½` (go
down); a longer one does the opposite. 1 s spans have shorter handles (fps-24 ties:
s 0.125, 0.25, 0.375 up; 0.625, 0.75, 0.875 down; `two-keys`), a 20-frame span a longer
one (`four-keys` frame 15, s 0.75, up; frame 45, s 0.25, down). The rule: decide a tie
with `x` built from the grid handles; if that is a tie too (the centre of a symmetric
curve) go up. The grid handles are used **only** for this decision: sampling `y` and
non-tie decisions use exact thirds (using the grid everywhere leaves errors of 4·10⁻⁶
in `three-keys-monotone`, while exact thirds match to 10⁻¹⁴). This replaces brief 09's
`mid ≥ 0.5` tie rule. **Parity-pending**: the grid unit is inferred in seconds (all
O16 spans are whole seconds at 24 and 30 fps), so a non-whole-second span at another
fps might differ.

**Time rounding** stays at whole milliseconds at every fps: `fps-25` and `fps-24`
match with it once ties follow the handles (the 2.7·10⁻⁴ misses of brief 09 were all
tie samples, not time-rounding errors; `dense-quarter`, `fps-24`).

### 1.5a User tangents and exact-time sampling (O38, `docs/parity/fcurves.md`, 12 cases)
Source: the twelve `keyframes` cases whose `observation` is `fcurves.md`.

**Sampling.** These samples are taken at the exact frame time: `two-keys`-style 0 → 100
reads 0.32593453 at frame 1 and 1.2740717 at frame 2 (exact `t = 1/30`, `2/30` s), where the
earlier O2 files read 0.3195 and 1.2866 (the 0.990 and 2.010 frame times of §1.4.1). The
spline, bisection and tie rules of §1.5 are unchanged. `forge_anim::value_at_exact(track,
user, t)` evaluates without the millisecond rounding; `value_at` (document time) keeps it.

**User tangents.** `tangents.json` rows are `[frame, value, left_dt, left_dv, right_dt,
right_dv]` (frames, handle end relative to the key; left `dt` is negative). A user tangent
replaces the automatic handle of that side: the Bézier control point is `(key.t + dt/fps,
key.v + dv)`. Handle length and value offset are both used as stored (`user-tangent-steep`:
left (−5, −60); `weighted-long` (−20, −50); `weighted-short` (−2, −5); `user-tangent-flat`)
whether or not the weighted flag is set: the curve depends only on the stored handle end.
A read-back side is **not** a user tangent (it stays automatic, with the exact thirds of §1.5)
when it is (a) equal to the automatic handle (within 2 grid steps in time, 1e-6 relative in
value; `user-tangent-flat` key 0), (b) the display tangent the reference reads back on keys
it did not edit: a third of the span from the previous key and a third of that chord,
mirrored on the two sides (`after-*`, `clamp-off`: (9.999996 frames, ±33.33) at key 30,
exactly the spec §1.5 "display value"), or (c) of zero length (`after-*` key 0 reads all
zeros but evaluates as the automatic ease). Anything else replaces that side.
**Tie handles and the grid** (parity-pending, `weighted-long`, `weighted-short`): in a
segment that has an edited handle, the remaining automatic handle of the segment sits on
the handle grid (key 0 right = 9.999996 frames) for the curve itself, not just for ties; with
exact thirds `weighted-long` frame 9 misses by 1.8e-4 (a flipped bisection step). Ties use the
stored read-back lengths where the file has them (`user-tangent-flat` frame 15). These
cases match to the 1e-4 tolerance: `user-tangent-steep`, `user-tangent-flat`,
`weighted-long`, `weighted-short`.

**Other cases.**
- `clamp-off`: keys 0 → 100 → 0 behave exactly like `three-keys` (clamp is off by default and
  a local extreme has slope 0 anyway). Matches.
- `after-continue`: the last key's automatic tangent is flat, so Continue equals the constant
  extension of §1.4.3. Matches.
- `after-repeat`, `after-offset`, `after-oscillate`, `before-repeat` match since §1.9 (the
  behaviour is read from the case description: `case.toml` has no field for it).
- `linear-to-spline-2` (redo of `linear-to-spline`, `case.toml` records
  `interpolation_by_key = ["Linear", "Spline", "Spline"]`): matches. The old `linear-to-spline` stays
  **UNSUPPORTED** (its `case.toml` has no per-key interpolation; the redo supersedes it).
  The redo files hold the value of the rounded whole frame at the half-frame rows (14.5 reads 50.0
  = frame 15, 29.5 reads 100 = frame 30); the test reads those rows at the rounded frame
  (`WHOLE_FRAME_SAMPLING`: `linear-to-spline-2`, `broken-2`; the original recordings sampled the true
  fractional time).
- **Broken tangents** (`broken`, `broken-2`; read-back of `broken-2` equals what was set: left
  (−10, −20), right (10, 40), auto 0, break 1, so the read-back is trustworthy and the evaluated curve
  still follows the automatic right side: 65 would be the value at frame 45 with the stored right handle,
  the samples read 50.0001). Rule (parity-pending, two cases, same numbers): **a user handle whose end
  value lies outside the value range of its own segment overshoots and is evaluated as the automatic
  tangent of that side**. Key 30 right handle ends at 100 + 40 = 140, outside [0, 100] of the segment
  to key 60 (automatic, flat); the left handle ends at 80, inside [0, 100] (used as stored, 42.5001 at 15).
  No other O38 case has a handle end outside its segment's range, so all of them stay as stored.
  Both `broken` and `broken-2` match; the `KNOWN_MISMATCH` pin is removed. The rule is applied in the
  read-back path (`value_at_exact` with user tangents); the stored-tangent model of §1.5b keeps
  evaluating `User` handles as stored (nothing observed for the editing path).

### 1.5b Tangent model, presets, speed (brief 089)
Data: `Track::tangents` (empty = every key automatic, else one `KeyTangent` per key) with
`KeyTangent { left, right: Handle { mode, dt, dv }, weighted, broken }`. `TangentMode::Auto` is the
ease of §1.5, `Chord` the straight line of §1.8, `User` the stored handle end (`dt` seconds, negative
on the left, `dv` value offset), evaluated exactly as stored whether or not `weighted` is set
(§1.5a: the curve depends only on the stored handle end). `broken` and `weighted` are editing
flags. The legacy `value_at_exact(track, user, t)` read-back path classifies each side as Auto or
User with the §1.5a rules and then uses the same evaluator, so all twelve O38 cases keep their
result (`keyframes_parity`: 39 of 40 match, `linear-to-spline` unsupported).
Presets (`forge_anim::tangent`, own design, the observations fix only evaluation):
- `ease`: both handles flat (`dv = 0`), a third of the adjacent span long; `ease_in` / `ease_out`
  flatten one side only.
- `zero_angle`: both handles flat, lengths kept (automatic sides get the automatic length).
- `zero_length`: both handle ends on the key.
- `set_right_handle(dt, dv)`: unweighted handles keep a third of the span as length and take the
  slope `dv / dt`; weighted ones take `dt` as length; an unbroken key mirrors the slope onto the left
  handle, a broken one leaves it.
- **Overshoot prevention** (O38 `clamp`): the automatic slope already never overshoots (§1.5) and
  `clamp-off` equals the default, so there is no separate flag; **UNSUPPORTED** as an option
  (nothing observed distinguishes it).
- **Speed**: `speed_at(track, t)` is `dv/dt`: 0 for Step and outside the keys, the chord slope for
  Linear, `(dy/ds) / (dx/ds)` at the solved curve parameter for a spline segment (an ease peaks at
  1.5 times the mean speed). Used by the speed graph (brief 092).
- **Not observed / not done**: `track.set_behavior` and
  `key.set_tangent` registry commands are not added (they are brief 090/091's UI work).

### 1.6 Animated scenes (`forge-anim::scene_at`)
`scene_at(&Scene, time) -> Scene` copies the scene and sets every tracked parameter to
`value_at(track, time)`. Float tracks set a Float, component tracks set one component of
a Vec3 on top of the stored (or already animated) value. Values go through
`Scene::with_params`, which validates like `Tx::set_param` (type, range clamp); an
override that fails (the node type no longer has the param) is skipped. The copy is not
an undoable edit; every changed node gets a fresh revision above all earlier ones, so one
`EvalCache` serves scene copies made for different times.

### 1.7 File format (`docs/specs/CORE.md` §6)
Version 4 writes `tracks: { "<track key>": (keys: [(time: .., value: .., interp: Spline)]) }`
per node, sorted by key; versions 1–3 still load without tracks. A track that does not
validate against its node's type is dropped with a warning on load (kept as-is on unknown
types).

### 1.8 Tracks for every value type, point-level animation (O53, `docs/parity/animation-3.md`, brief 088)
Sources: `tests/golden/animation-3/` (7 cases) and the O53 notes. `forge_core::parse_track_target`
resolves a track key to a `TrackTarget`; `parse_target` (Float and Vec3 components) is unchanged.
Key values stay `f64`; the target decides how a value is read (this replaces the planned
`KeyValue` enum, so existing `Key` literals keep compiling):

| Parameter | Track key | Key value | Evaluation |
|---|---|---|---|
| Float | `<id>` | the value | the curve (§1.4, §1.5) |
| Vec3 | `<id>.x|y|z` | the component | the curve |
| Colour | `<id>.r|g|b` | the channel | the curve, clamped to >= 0 (`color-track`: linear 1 -> 0 and 0 -> 1) |
| Int | `<id>` | the number | the curve **floored** after adding 1e-4 (`int-track`) |
| Bool | `<id>` | 0 / 1 | step: key at or before the time, value >= 0.5 is true |
| Enum | `<id>` | option index | step, index into the descriptor's options |
| Link | `<id>` | node id, 0 = none | step |
| Mesh | `<id>.points` | unused | point-level, below |

- **Int** (`int-track`, keys 1 @ 0 and 10 @ 30): 1 at frames 0-3, 2 at 4-6 ... 10 from 30. That is
  `floor(1 + 0.3 f)`: the value holds and steps at the integer crossings of the straight
  line. The 1e-4 added before flooring absorbs the solver's 2^-19 sawtooth (frame 10 is exactly 4).
- **Colour** and **Float**/**Vec3** keys of the O27 and O53 sessions evaluate as a **straight
  line**, not the ease of O2 (`color-track` 0.5 @ 15, 0.966666 @ 1; `euler-cross` 158.666546 @ 1;
  `tie-span-24` 10 per frame; `tie-span-30` 100/7 per frame; also `animation-2/param-track`).
  Their curve is a spline segment whose handles lie on the chord: handle length = a third of the
  span rounded to the 4e-7 s grid, handle value offset = chord / span * length. Model:
  `TangentMode::Chord` (§1.5a gives the tangent model; `Key::new` stays `Auto`, as O2/O12/O16/O38
  keys ease). The handle grid follows seconds, not frames: the same frame span gives the same
  curve at 24 and at 30 fps (`tie-span-24`, `tie-span-30`).
  Sampling is at exact frame times (like §1.5a), so `value_at_exact`.
- **Bool, enum and link tracks are not observable** in the reference as scripted (`bool-track`,
  `enum-track`, `link-track` not observed in O53: the parameter keeps its base value). They are
  implemented as steps by design (parity-pending, no golden data).
- **Point-level animation** (`pla` not observed): a track on `<mesh param>.points` holds a whole
  point array per key (`Track::points`, one array per key, same point count; `set_points_key`).
  `forge_anim::points_at`: constant outside the keys, the previous array for a Step key, else a
  linear blend of the two neighbouring arrays by time fraction. `scene_at` replaces the points of
  the mesh parameter and keeps its polygons; an override whose point count differs from the mesh is
  skipped. Parity-pending.
- **Generator parameter** (`generator-param`): a Bend's strength track is an ordinary Float track.
  `scene_at` at frame 15 gives the same mesh as the Bend with that strength stored (test
  `animation3_parity`). **KNOWN_MISMATCH** `generator-param/mesh.obj`: its bbox (x -99.2..347.9,
  y -353.7..95.5) fits none of our bends of an 8x8x8 cube (cube size_y 200 or 400, angle 0 or 45,
  limited or unlimited, strength 45 to 180 degrees tried); the case does not record the cube size or
  the Bend's size, mode and angle. The form needs those settings.
- **Solver residual** (KNOWN_MISMATCH `euler-cross`, `quaternion` rot_h and the strength curve of
  `generator-param`: two-key chords over a 1 s span): every sample is within
  6e-4 of the curve, but the 1e-4 tolerance fails (euler 5.9e-4 over a 340 degree range, strength
  1.5e-4 over 90). In units of 2^-19 of the span the reference's time offset is in (-0.53, 0.53)
  and not on the (k + 1/2) grid of a 19-step bisection. Hypotheses tried: (1) exact-third chord
  handles with 19 steps; (2) grid-length chord handles with the slope-preserving value offset;
  (3) grid-length handles with a value offset of chord/3; (4) early stop when |x(mid) - t| < 1e-6 s
  (it makes `tie-span-*` worse: 1.7e-4). Neither explains the offsets. The same code matches
  `color-track` (range 1) and `tie-span-24/30` (range 100, 7.3e-5 and 1.5e-5).
- **File format**: no version bump. The optional track fields (`tangents`, `before`, `after`,
  `points`, `quaternion`) are serde defaults written only when they differ from the default, so
  version 11 files load unchanged and files without the new data are byte-identical. (The brief
  asked for a bump with a migration; a bump would invalidate the byte-stable version-11 fixtures
  of `native_format` for no data change, so it is left for a change that needs one.) Track keys of
  the new kinds are validated against the node type like the others (`parse_track_target`).

### 1.9 Before / after behaviours (O38, brief 093)
`Track::before` and `Track::after` are `Constant` (default), `Continue`, `Repeat`, `OffsetRepeat`
or `Oscillate`. They are applied by remapping the (already resolved) time into the key range, `span
= last.time - first.time`, `delta = last.value - first.value`. Source: `keyframes/after-*`,
`before-repeat`, `after-continue` (the behaviour is read from the case description,
"after-behaviour X" / "before-behaviour X", because `case.toml` has no field).
- After: with `x = (t - first) / span`, `k = ceil(x) - 1`, `frac = x - k` in (0, 1]: **Repeat**
  evaluates at `first + frac * span` (value at 45 and 75 = 50.0001; frame 90 = 100: a cycle
  boundary reads the end of the cycle). **OffsetRepeat** adds `k * delta` (150.0001 @ 45, 200
  @ 60). **Oscillate** plays odd cycles backwards, `first + (1 - frac) * span` (0 @ 60, 100 @ 90).
- Before: with `x = (first - t) / span`: **Repeat** reads `last - frac * span` where `frac` is the
  fractional part of `x`, and at an exact boundary the value of `last` (`before-repeat` frame 0 =
  100, frame 5 = 7.4074 = frame 35). **OffsetRepeat** before the first key reads the same point as Repeat, shifted by `-(floor(x) + 1) * delta`
  (`before-offset`, keys 0 -> 0, 30 -> 100: -50 at frame -15, -100 at -30, -200 at -60; a cycle boundary
  reads the end of the cycle, shifted one delta further: the curve is continuous there). **Oscillate** before
  the first key mirrors the after rule (`before-oscillate`: 50 at -15, 100 at -30, 0 at -60). Both match
  the observation (`before_behavior` is a `case.toml` field since the follow-up; the description is the
  fallback for older cases).
- **Continue**: carries on with the slope of the end key's inner handle (the last key's left
  handle after; the first key's right handle before). The end handles of automatic keys are flat,
  so `after-continue` equals Constant (matches).
- Point tracks ignore the behaviours (constant).

### 1.10 Key reduction and bake (brief 093)
- `reduce_keys(track, tolerance)`: repeatedly removes the interior key whose removal changes the
  curve least, measured as the largest difference at the *original* key times, and stops when the
  cheapest removal would exceed `tolerance`. End keys are kept. A track with point keys is not
  reduced. Own design (no observation).
- `bake_param(scene, node, param, component, times)`: evaluates `scene_at` at each time and
  writes a linear key per time with the parameter's value (Float, Int, or a Vec3/Colour component).

### 1.11 Rotation: Euler filter and the quaternion option (O27, O53, brief 096)
- **Euler tracks** interpolate the stored component values with no unwrapping (`euler-cross`:
  170 -> -170 degrees goes the long way, -340 degrees of travel; rot_h 90.666889 @ 7, -0.000486
  @ 15). The shape matches (see the residual in §1.8).
- `euler_filter(track)` shifts every key by whole turns so it lies within half a turn of the
  previous key; times, interpolation and tangents are kept (own design).
- **Quaternion option** (`Track::quaternion` on any of `transform.rot.x|y|z`): O53 `quaternion`
  forced the key's quaternion item and observed the **same** Euler samples, so the golden data
  shows no separate path and that case is evaluated as Euler. The option is our own: for a node
  with the flag, `scene_at` evaluates the rotation as one orientation (`quaternion_rotation_at`):
  at the two neighbouring key times (union over the three tracks) the Euler values give
  orientations `Ry(-h) Rx(-p) Rz(-b)`, blended by shortest-arc slerp with a fraction that follows
  the segment's own curve, then decomposed back to `(h, p, b)`. 170 -> -170 about one axis then
  takes the short way. Parity-pending.

## 1b. Key editing commands (briefs 090 and 091, own design, parity-pending G5)
Every edit of the dope sheet and the value graph is a registry command, one undo step, callable
through RPC, MCP and the `forge` CLI. Keys are addressed as `{node, track, frame}` (fractional
frames allowed; matched to a nanosecond); values are in display units (degrees, percent).
`forge_anim::keyedit` holds the pure track functions.
- `key.move {keys, delta_frames, delta_value?, ripple?}`: moves keys in time and value; a key
  landing on another key replaces it; `ripple` moves every key at or after the earliest selected one
  of each track in time (later keys keep their value). Tangents and point arrays stay attached.
- `key.scale {keys, pivot_frame, factor}`: scales time about the pivot; user handles scale with the
  spacing; a negative factor mirrors (the handle sides swap).
- `key.delete {keys}`: removes keys; a track that loses its last key is removed.
- `key.set_interp {keys, interp}`; `key.paste {keys: [{node, track, frame, value, interp}]}`
  (replaces keys at the same frame, creates the track).
- `key.set_tangent {node, track, frame, preset, left?, right?, broken?, weighted?}`: presets `auto`,
  `linear` (chord), `ease`, `ease_in`, `ease_out`, `zero_angle`, `zero_length` (section 1.5b), or
  `user` with handle ends `{dt_frames, dv}` relative to the key.
- `track.set_behavior {node, track, before?, after?}` (section 1.9); `key.euler_filter {node}`
  (section 1.11, all `transform.rot.*` tracks of the node).
A command that changes nothing or names a missing key, track or preset is an error and adds no undo
step.

### Dope sheet (`crates/ui/src/dopesheet.rs`, `fcurve_logic.rs`)
A track tree (an object row with a summary of all its keys, then one row per track; click an object
row to collapse), a frame ruler (click or drag to scrub through `timeline.set_frame`), keys as
diamonds. Click selects (Shift toggles), a drag on empty space box-selects, a drag on a key moves the
selection by whole frames (snapping on; free when off) and sends one `key.move` on release, a summary
key stands for all keys of its object at that frame. Buttons: Delete (also the Delete key), Spline /
Linear / Step, Copy / Paste (at the current frame), Scale 1/2 and Scale 2 about the current frame,
Ripple, Snap, "Selected objects only". Pan with the scroll wheel, zoom with pinch / ctrl-scroll about
the cursor (`TimeView`). Shared pure logic (`TimeView`, `ValueView`, `KeySelection`, `hit_test`,
`box_select`, `scale_about`, `rows`) is covered without egui.

### Value graph (`crates/ui/src/fcurve.rs`)
One curve per numeric track of the selected objects, drawn by sampling `value_at_exact` (the
playback evaluator), coloured by axis (X red, Y green, Z blue); a checkbox per curve hides it. Keys
are dots; the selected spline keys show their two evaluated handles (`forge_anim::handles`: automatic,
chord or user, as the curve uses them). Dragging a handle sends `key.set_tangent` `user`; dragging a key
sends `key.move` with the frame and value change; box select as in the dope sheet. Toolbar: Frame all,
Frame selected, Snap, Normalised (each curve's key range maps to 0..100; value drags and handle drags
are off there), tangent presets, Break / Unify, Weighted / Unweighted, and numeric frame and value
fields for a single selected key. Which modifier keys do what in the reference is the open question
G5 (parity-pending).

## 2. Timeline, record and autokey (`crates/ui/src/{animate,timeline}.rs`)

Sources: docs/parity/keyframes.md §3 (one record makes 9 tracks, a recorded key is a
Spline key; autokey, shortcuts and the timeline's look were **not observed**). Everything
below that is not from that file is our own design and **parity-pending** until an
observation (O10 in the queue) shows the reference.

### 2.1 Scene settings (`forge-core::SceneSettings`, file version 5)
`Scene::settings() -> SceneSettings { fps: u32, start: i64, end: i64 }`, default
`30, 0, 90` (parity-pending defaults; the keyframe cases used 30 fps and 0-90).
- Valid when `1 <= fps <= 240` and `start <= end`; `Tx::set_settings` rejects anything
  else with `EditError::Settings` and is one undoable edit (it raises the scene revision,
  so the document is dirty; it stamps no node).
- Chosen over a root-less settings node because the scene graph has no node without a
  parent/tag slot and a fake node would show up in the tree, in exports and in `len()`.
- `file::save` always writes `settings: (fps: .., start: .., end: ..)` and version 5.
  Versions 1-4 load with the defaults. Invalid saved settings load as the defaults with a
  warning. The current frame and the playing flag are editor state and are not saved.
- Frame to time: `time = frame / fps` seconds; time to frame: `time * fps`.

### 2.2 Current time and the displayed scene
`EditorState.timeline.current` (frame, `i64`) is the only current time; it is clamped to
`[start, end]` by every command that sets it. The viewport, the Attributes panel, the
Coordinates panel, picking and the gizmo all read `scene_at(scene, current / fps)`. The
evaluated scene is cached per (scene revision, node count, frame, fps) and is the stored
scene itself in effect when nothing is animated (`scene_at` copies it unchanged).
Editing always targets the stored scene (`Document`); gizmo and Coordinates drags start
from the evaluated transform, so an animated object does not jump.

### 2.3 Record (`Command::RecordKeys`, K)
For every selected node (tags skipped, nodes of a type without the `transform.*`
params skipped) the current evaluated value of the 9 tracks is written as a key at the
current frame, in this order (keyframes.md §3):
`transform.pos.x|y|z`, `transform.scale.x|y|z`, `transform.rot.x|y|z` (rotation H, P, B).
- A new key is `Interp::Spline`. A key that already sits on that frame keeps its
  interpolation and gets the new value.
- One undo step, label "Record". With no selected recordable node: an error in the status
  bar, no edit.

### 2.4 Autokey and editing animated parameters
`EditorState.autokey` (Shift+K, Animate menu, timeline toggle). The animatable parameters
are Float and Vec3 parameters with `animatable` set; a Vec3 component is one track.
A parameter edit (Attributes panel, Coordinates panel, gizmo) with value `v` at frame `f`:
1. **Autokey on:** a Float writes a key `(f/fps, v)` on its track; a Vec3 writes keys on
   **all three** components (`v[0..3]`), as record does. Existing keys at `f` keep their
   interpolation; new keys are Spline. The stored value is not changed for components that
   have (or get) a track. One undo step per edit; the steps of one drag merge into one.
2. **Autokey off, parameter animated (rule 5, own design, parity-pending until O10):**
   a track of the edited parameter (or component) exists, so the edit **updates or
   creates the key at the current frame** on that track. Components of a Vec3 whose value
   did not change (within `1e-9` relative to the shown value) are left alone; components
   without a track change the stored value.
3. **Autokey off, parameter not animated:** the stored value changes, as before.
A gizmo or Coordinates edit writes `transform.pos|rot|scale` and skips a Vec3 whose value
did not change, so a move with autokey on keys only Position.
Values go through the parameter's range clamp first (`ParamDesc::validate`), like
`set_param`.

### 2.5 Keys, navigation, delete
- Key frames of the selected nodes: the key times of all their tracks times `fps`, sorted,
  de-duplicated within `1e-6`.
- Previous key: the largest key frame `< current` (rounded down); next key: the smallest
  `> current` (rounded up); none: no move. They are limited to the project range.
- **Delete Keys** removes, for each selected node, the key at the current frame from every
  track (a track loses itself with its last key). One undo step, label "Delete Keys";
  nothing to delete: no step.
- Marker state of a parameter row: *keyed* when any of its tracks has a key on the
  current frame, *animated* when it has a track but no key here, otherwise none.

### 2.6 Playback (`timeline::Timeline`)
`Timeline { current, playing }` plus an anchor `(clock time, frame)` set when playing
starts or the frame is set by hand. `update(now, settings)`: the frame is
`start + (anchor_frame - start + floor((now - anchor_time) * fps)) mod (end - start + 1)`.
- It follows the clock, so a slow frame drops frames instead of slowing down; at the end
  it loops to the start. 1 s at 30 fps advances 30 frames.
- `now` is `ctx.input(|i| i.time)`; the stepper itself takes it as an argument.
- Stopping keeps the current frame. Changing fps or range while playing re-anchors.

### 2.6a Playback modes and the playback range (brief 094, own design, **parity-pending G5**)
The stepper is `Timeline::update_with(now, &Playback)`; `update(now, &settings)` is the
Loop, real-time, project-range case of it (§2.6). `Playback { lo, hi, fps, mode,
drop_frames }`, built by `Playback::of(settings, time)`:
- **Playback range** `lo..=hi` = the preview range (§2.9) when one is set, else the project
  range. A current frame outside it is pulled to the nearest end at the next update.
- **Position function** (pure, `Playback::frame_after(from, steps) -> (frame, finished)`):
  with `len = hi - lo + 1` and `u = (from - lo) + steps`:
  - `Once`: `lo + min(u, len - 1)`; `finished` when `u >= len - 1` and `steps > 0`. The
    frame stays on `hi` and `playing` becomes false.
  - `Loop`: `lo + u mod len` (the §2.6 rule).
  - `PingPong`: `period = 2 (len - 1)`, `p = u mod period`, frame `lo + p` for `p < len`,
    else `lo + period - p`. Each end is shown once per turn (0 1 2 3 2 1 0 1 ...); a range of
    one frame stays put. Playback always starts forward from the current frame.
- **Steps** come from the injected clock `now` (seconds):
  - *Real-time* (`drop_frames = true`, the default): `steps = floor((now - t0) * fps)` since
    the anchor `(t0, from)`; a slow tick skips frames, the clock rate never changes.
  - *Every frame* (`drop_frames = false`): one step at most per update, taken when `1/fps`
    has passed since the previous step (`due = last + 1/fps`; `last = due` when the update
    is less than one frame late, else `last = now`). A slow tick plays slower than real time
    and never skips; there is no catch-up burst.
- The first update after `play`, a manual frame change or an fps / range / mode change only
  anchors (the frame does not change), as in §2.6.
- `timeline.play` in `Once` mode with the current frame on `hi` first jumps to `lo`.

### 2.7 Shortcuts (own design, parity-pending; none clashes with the existing ones)
Existing: Ctrl/Cmd+O, S, Shift+S, A, Shift+A, Z, Shift+Z, Y, Delete, 1-5 (shading),
W/E/R/Q (gizmo), F2 (rename). New, all inactive while a text field has the keyboard:

| Key | Command |
|---|---|
| K | Record |
| Shift+K | Autokey toggle |
| Space | Play / Stop, when the pointer is over the viewport |
| `,` / `.` | previous / next frame |
| Shift+`,` / Shift+`.` | previous / next key |
| M | add a marker at the current frame (`marker.add`) |
| Alt+`,` / Alt+`.` | previous / next marker (`marker.jump`) |

### 2.8 Make Editable and Current State to Object
Both copy the source's `transform.*` **tracks** along with its `transform.*` params
(EDITABLE.md §1.2), so an animated object stays animated after the conversion. Tracks of
other (generator) parameters have no counterpart on a polygon object and are dropped. The
baked mesh uses the stored parameter values, not the values at the current frame
(**parity-pending**).

### 2.9 Time: settings, preview range, markers, fps conversion, display (brief 094)
Sources: brief 087 (the rounding rules of §1.4), the area design G-animation.md. G5 §4
(where preview range, markers and playback modes are set) is **unanswered**: the whole
section is our own design and **parity-pending G5**; it is documented here so the
observation can replace it rule by rule.

**Data** (`forge-core::time`, saved in the scene, CORE.md §6.6): next to `SceneSettings`
(fps, project range) the scene holds `TimeConfig`:
| Field | Type, default | Meaning |
|---|---|---|
| `preview` | `Option<(i64, i64)>`, none | preview range, `start <= end`, inside the project range |
| `mode` | `Once \| Loop \| PingPong`, `Loop` | what happens at the end of the playback range |
| `drop_frames` | bool, true | true: real-time with frame dropping; false: every frame |
| `display` | `Frames \| Smpte \| Seconds`, `Frames` | how times are written in the UI and in `text` fields |
| `markers` | `Vec<Marker>`, empty | sorted by `(frame, id)` |
`Marker { id: u64, frame: i64, name: String, color: [u8; 3] }`: `id` is unique in the scene,
never reused while the marker exists (the next id is the highest + 1) and is the handle every
command uses. Colours are `#rrggbb` in commands (default `#e0a030`); names may be empty.
`Tx::set_time(TimeConfig)` validates (preview order and range, unique ids) and is one undoable edit that raises the scene revision.
`Tx::set_settings` also clips the preview to the new project range (dropped when nothing is
left) in the same step. `marker.add` and `marker.move` reject a frame outside the project range. Markers outside a shrunken range are kept, ignored by the jump, and
shown again when the range grows.

**fps conversion** (`timeline.set_fps`; the pure rule is `time::convert_frame`). Keys are
stored in seconds (§1.1), so **keys never move**: after `fps` 30 to 24 the key at 0.5 s is
at frame 12 and its value at every time is unchanged (the evaluation rounding of §1.4 is by
time, not by frame, and is unchanged). A key may therefore sit between frames (10 at 30 fps
is 0.3333 s = 8 at 24 fps; 1 at 30 fps is 0.8 at 24 fps). Frame-based data converts with the
O16 rule (087): the time of frame `f` at the old rate is rounded to a whole millisecond,
`ms = round_half_away(f * 1000 / old)`, and the new frame is `floor(ms * new / 1000 + 1/2)`
(half rounds up), all in integer arithmetic. Converted: project start and end, preview start
and end, every marker frame, the current frame (then clamped). Ranges keep `start <= end`
(rounding is monotone). Converting back does not always restore the old frames (30 to 24 to 30 maps 3 to 2 to 2
and 7 to 6 to 8); the command is one undo step, so undo is the way back. `timeline.settings` stays the raw edit:
the numbers are frames as given and nothing is converted.

**Display formats** (`time::format_time(frame, fps, display)`, non-drop-frame, always the
time of the frame):
- `Frames`: the frame number, `42`, `-3`.
- `Smpte`: `HH:MM:SS:FF`, `FF = |frame| mod fps` zero padded to the digits of `fps - 1`
  (at least 2), hours at least 2 digits, a leading `-` for negative frames:
  frame 100 at 30 fps is `00:00:03:10`; frame 3600 at 24 fps is `00:02:30:00`.
- `Seconds`: the millisecond-rounded time with three decimals and a unit, `1.400 s`.

**Commands** (COMMANDS.md): `timeline.set_fps`, `timeline.set_preview`,
`timeline.clear_preview`, `timeline.set_mode`, `timeline.set_frame_dropping`,
`timeline.set_display`, `timeline.info` and `marker.add|remove|rename|move|set_color|list|jump`.
Everything the panel does goes through them.

**Marker jump** (`marker.jump {forward}`): the next marker is the one with the smallest frame
`> current`, the previous the one with the largest frame `< current`, among markers inside
the project range; none: the frame stays. Several markers on one frame count once.

## 2b. Motion paths and onion skin (brief 095, own design, **parity-pending G5**)

Sources: the area design G-animation.md ("Display: motion paths, onion skin"). No
observation exists: every rule here is our own design. A reference-app display rule (frame
ticks, colours, per-object flags, how the ghost count is set) is unanswered; **GUI session
G5** (docs/parity/GUI-SESSIONS.md, timeline and animation display) is the one that could
settle it, with a new item "motion trajectory and ghosting display". Nothing here is a parity
claim.

### 2b.1 Sampling (`forge-anim::motion`, pure)
- `PathRange { start, end, step }` (frames, `start <= end`, `step >= 1`). `PathRange::new`
  raises the step until at most `MAX_PATH_SAMPLES = 2000` frames result. `frames()` is
  `start, start + step, ...` up to `end`, and `end` itself is appended when the walk does not
  land on it (so a path always ends at the range end).
- `sample_paths(scene, nodes, range, eval) -> Vec<MotionPath>`: `eval(time_seconds)` returns
  the scene animated to that time (`scene_at`, or the scheduler's `animate` with expression
  tags). It is called **once per frame** for all nodes. A sample is
  `world_matrix(node) * (0, 0, 0)` in the evaluated scene, frame time `frame / fps`.
  At most `MAX_PATH_NODES = 16` nodes (the first ones in the given order); tags and unknown
  nodes are skipped.
- `MotionPath { node, samples: Vec<(frame, pos)>, keys: Vec<(frame_f64, pos)> }`. Keys are
  the key times (times `fps`) of every `transform.*` track of the node and of its ancestors
  that lie in `[start, end]` (tolerance `1e-6`), sorted, de-duplicated within `1e-6`; the
  position of a key is evaluated at the key's exact time (it may lie between frames).
- A path whose samples all coincide (largest distance from the first sample below `1e-9`) is
  dropped: an object that does not move has no path, whatever drives it.
- Deterministic: the result depends only on the scene, nodes and range.

### 2b.2 Onion frames (`forge-anim::motion::onion_frames`)
`onion_frames(current, before, after, step, lo, hi) -> Vec<OnionFrame { frame, rank, forward }>`:
`before` frames at `current - k * step` and `after` at `current + k * step` for
`k = 1..=count`, keeping those inside `[lo, hi]` (the project range). `rank` is `k`. The list
is in ascending frame order. `step >= 1`; `before`/`after` at most `MAX_ONION = 10`.

### 2b.3 Evaluation, cache, background job (`forge-app::motion`)
- `compute_paths(scene, nodes, range)` samples with the scheduler's `animate` (tracks, then
  expression tags), so a path follows what the viewport shows. `compute_ghosts(scene, nodes,
  frames)` animates the scene to each ghost frame, evaluates each node's mesh in world space
  (`schedule::node_output`) and stores its wire (`GhostDraw`, VIEWPORT.md section 12).
- `AsyncCache<K, T>` holds the latest finished result and its key. `get(key, job)` returns
  `(Option<Arc<T>>, pending)`: the result for `key` when ready; otherwise the previous result
  (stale, so the display does not flicker) and `pending = true` while the job for `key` runs on
  one worker thread. A newer request replaces a queued one (at most one running, one queued).
  `synchronous = true` runs the job inline (tests, screenshots, headless).
- The key is **pure data**: `(scene revision, node count, nodes, range/frames, fps)`; paths do
  not depend on the current frame, onion frames do (current frame is part of their key). The
  revision rises on every edit, undo and redo, so an edited track, an undo or a scrub never
  shows a result computed for other content once the new one is ready; no state outside the
  key influences the result.
- Nodes: the selected objects, in id order (at most 16 get a path).

### 2b.4 Display settings and commands
`MotionDisplay` (view state, like `view.clone_display`: **not saved, not part of the
document, not undoable**; the documented toggles never edit the scene):
| Field | Default | Meaning |
|---|---|---|
| `paths` | false | draw motion paths of the selected objects |
| `range` | none (= project range) | custom `(start, end)` frames |
| `step` | 1 | path sample step, 1 to 1000 |
| `onion` | false | draw ghosts of the selected objects |
| `before`, `after` | 2, 2 | ghost counts, 0 to 10 |
| `onion_step` | 1 | frames between ghosts, 1 to 100 |
Commands (COMMANDS.md): `view.motion_paths {enabled?, start?, end?, reset_range?, step?}` and
`view.onion_skin {enabled?, before?, after?, step?}`. Absent `enabled` with no other field
toggles. `start` and `end` come together and need `start <= end`; `reset_range` returns to the
project range. Out-of-range numbers are an `InvalidArgs` error and change nothing. The View menu
has "Motion Paths" and "Onion Skin" toggles that run the commands.

### 2b.5 Tests
crates/anim/tests/motion.rs (range, frames, sampling against `scene_at`, keys, static object has no path, ancestors,
onion frames), crates/app/tests/motion.rs (cache semantics, undo/scrub invalidation, job result equals
synchronous result, ghosts), crates/viewport/tests/motion.rs (draw lists) and display.rs (GPU baseline),
crates/ui/tests/motion_view.rs (commands, screenshot).

## 3. Parity status

39 of 40 cases (41 sample files) match within 1·10⁻⁴ (§1.5a adds the O38 cases and their redos);
`UNSUPPORTED`: 1 (`linear-to-spline`, superseded by `linear-to-spline-2`), `KNOWN_MISMATCH`: 0.

**Parity-pending / not observed**: stored tangents and tangent modes (the reference
reads them back, `tangents.json`, but their labels and the clamp option were not read; the
read-back key at 20 in `four-keys` shows a non-zero slope on a local maximum, i.e. the
stored tangent is not what is evaluated), the monotone-key rule for uneven spacing and
the overshoot clamp, tie breaking, behaviour of two keys closer than 1 ms, autokey (§2.4 is our own design), sub-
frame fps other than 30 (the millisecond rounding is assumed independent of fps), Bool/Int/
Enum/Colour tracks, time remapping.
