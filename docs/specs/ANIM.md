# Animation — keyframe tracks and interpolation (`crates/anim`, data in `crates/core`)

Sources: docs/parity/keyframes.md (O2, 9 cases) and docs/parity/keyframes-2.md (O12, 8
cases), golden folder `tests/golden/keyframes/` (17 cases, 19 sample files, level B,
tolerance 1e-4). Every rule below cites the case that shows it. The exact curve was
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
parameter wins before the last `.x|y|z` is split off. Int, Bool, Enum and Colour tracks
are out of scope. Vec3 tracks need not cover all three components; a component without a
track keeps the stored value.

### 1.3 Edits
`Tx::set_track(id, key, Option<Track>)` (None removes), `Tx::set_key(id, key, Key)`
(creates the track, replaces a key at exactly the same time), `Tx::remove_key(id, key,
time)` (removes the track with its last key; a missing track or key is a no-op). All
validate first (`EditError::Track`), are undoable, and stamp the node's revision.

### 1.4 Evaluation time
`value_at(track, time_seconds)`:
1. **Time resolution** (cases `two-keys`, `long-span`, `scale-1000`, `negative`,
   `mixed-interp`): the reference evaluates at times rounded to whole milliseconds,
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
middle key has the central difference `m = (v_next − v_prev) / (t_next − t_prev)`
(`three-keys-monotone`: 2.5 per frame, exact), limited so that neither handle's value
offset exceeds its segment's rise: `|m|·h ≤ min(|v − v_prev|, |v_next − v|)` (no
overshoot). The clamp and the central-difference weighting for uneven spacing at a
monotone key are **parity-pending**: no case has a monotone middle key with uneven
spacing, and the clamp never triggers in the data.

Handle positions do not depend on the neighbours' interpolation (`mixed-interp`).

**Solving `x(s) = t`** is part of the observed behaviour, not just an implementation
detail (cases `two-keys`, `long-span`, `scale-1000`, `negative`, `four-keys`): the
reference finds `s` by bisection with a fixed number of halvings and returns the middle
of the final interval. With 19 halvings the value matches every sample to ≤ 3·10⁻¹⁰
(18 or 20 halvings leave errors of 1.4·10⁻⁴ and 7·10⁻⁵). The result is a sawtooth in the
time domain of 2⁻¹⁹ of the segment (5.7·10⁻⁵ frames for 30 frames), which is why
`two-keys` reads 50.000143 at frame 15 instead of 50. `x(mid) < t` goes up. An exact
tie (`|x(mid) − t| ≤ 10⁻¹² · span`) goes up when `mid ≥ 0.5` and down otherwise, which
reproduces `two-keys` frame 15 and `four-keys` frames 15 (up) and 45 (down); read-back
handle lengths a hair above a third explain this (**parity-pending**: three tie
samples).

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

### 2.8 Make Editable and Current State to Object
Both copy the source's `transform.*` **tracks** along with its `transform.*` params
(EDITABLE.md §1.2), so an animated object stays animated after the conversion. Tracks of
other (generator) parameters have no counterpart on a polygon object and are dropped. The
baked mesh uses the stored parameter values, not the values at the current frame
(**parity-pending**).

## 3. Parity status

17/17 cases (19 sample files) match within 1·10⁻⁴; the largest error is 2.7·10⁻¹⁰
(float noise of the recorded values). No `UNSUPPORTED`, no `KNOWN_MISMATCH`.

**Parity-pending / not observed**: stored tangents and tangent modes (the reference
reads them back, `tangents.json`, but their labels and the clamp option were not read; the
read-back key at 20 in `four-keys` shows a non-zero slope on a local maximum, i.e. the
stored tangent is not what is evaluated), the monotone-key rule for uneven spacing and
the overshoot clamp, tie breaking, behaviour of two keys closer than 1 ms, autokey (§2.4 is our own design), sub-
frame fps other than 30 (the millisecond rounding is assumed independent of fps), Bool/Int/
Enum/Colour tracks, time remapping.
