# 09 · Keyframe tracks and interpolation

Status: ready (after 08) · Branch: the planner's session branch · Planner: main session · Implementer: Claude Sonnet 5.5

## Goal
Any animatable float parameter, and each component of a vector parameter, can carry a
track of keys. The scene can be evaluated at any time. The default spline interpolation,
Linear, Step and the out-of-range behaviour reproduce the reference samples in
`tests/golden/keyframes/` (level B, tolerance from the files).

## Inputs
- `docs/parity/keyframes.md` (O2): the rules and the 9 cases. Read its §1 and §3 closely.
- `tests/golden/keyframes/*/samples*.json` and `case.toml`; brief 08's `parse_samples`
  and `compare_samples`.
- `docs/specs/CORE.md` (§1 values, §4 transactions, §6 file), `crates/core/src/{scene,document,file,param,transform}.rs`.
- `crates/anim` (empty stub crate `forge-anim`).

## Design decisions (planner)
1. **Data lives in core, maths in anim.**
   - `forge-core` gets `pub struct Track { pub keys: Vec<Key> }` and
     `pub struct Key { pub time: f64 /* seconds */, pub value: f64, pub interp: Interp }`.
   - `pub enum Interp { Spline, Linear, Step }`, default `Spline`.
   - Keys are sorted by time, with no two keys at the same time.
   - Tangents are automatic, and nothing else for now: no stored tangents yet.
2. **Addressing:** a track is keyed by a `TrackKey` string: `"<param id>"` for a float
   parameter, `"<param id>.<x|y|z>"` for a component of a Vec3. Examples:
   `"transform.pos.x"` and `"transform.rot.x"` (rotation H is component x).
   - A node gets `pub tracks: BTreeMap<String, Track>`.
   - Only parameters whose descriptor is `animatable` and Float or Vec3 can be tracked.
   - Track values are in **parameter units**: radians for angles, as the params store them.
3. **Edits and undo:** `Tx::set_track(id, key, Option<Track>)` (None removes it), plus
   helpers `Tx::set_key(id, track_key, Key)` and `Tx::remove_key(id, track_key, time)`.
   All are undoable, and all stamp the node's revision, so caches recompute.
4. **File format:** version 4 writes `tracks` per node and stays deterministic. Versions
   1–3 still load (no tracks). Update CORE.md §6.
5. **Evaluation** in `forge-anim`:
   - `pub fn value_at(track: &Track, time: f64) -> f64`.
   - `pub fn scene_at(scene: &Scene, time: f64) -> Scene` returns a copy whose animated
     params hold their values at `time`.
   - If core has no way to build that copy, add a narrow `pub fn with_params(&self,
     overrides: impl Iterator<Item = (NodeId, String, Value)>) -> Scene` (or similar) in
     core, validated like `set_param`. Core must not depend on anim.
6. **Rules** from O2, each one citing its case:
   - Before the first key, the value is the first key's value; after the last key, the
     last key's value (`before-after`).
   - **Step:** a key's value holds from its own time until the next key's time
     (`step`: 29.5 → 0, 30 → 100).
   - **Linear:** straight lines between keys (`linear`).
   - **Spline (default):** ease in and out with automatic tangents and no overshoot.
     - A key that is a local extreme gets a flat tangent (`three-keys` peaks at exactly
       100).
     - A key between a lower and a higher neighbour gets a non-zero tangent but never
       overshoots its neighbours (`three-keys-monotone`, `uneven`).
     - Derive the exact tangent rule and curve form from the samples. The curve is not
       exactly symmetric: 50.00014 at frame 15 in `two-keys`, which hints at a Bézier in
       a curve parameter rather than directly in time.
   - **Rotation:** a rotation track interpolates the raw value, with no angle wrapping
     (`rotation`: 0 → 270°).
7. **Parity test** `crates/anim/tests/keyframes_parity.rs`:
   - For every `tests/golden/keyframes/` case, build the track(s) from `case.toml`.
     Frames are frame / fps seconds; `rotation` values are in degrees, so convert to
     radians and compare in degrees.
   - Evaluate the track at every sample frame and compare with `compare_samples`.
   - `UNSUPPORTED` and `KNOWN_MISMATCH` are asserted exactly, as in the other parity tests.

## Hints (planner)
- Work out the spline rule on `two-keys` first: values at frames 0.5, 1 and 2 are
  0.0857, 0.3195 and 1.2866. A plain smoothstep, 3t² − 2t³, gives 0.0832, 0.3259 and
  1.2741 at those frames. So it is close, but not the rule.
- Try a cubic Bézier in a parameter s, with x(s) for time and y(s) for value, where the
  control points sit a third of the way along each segment and the tangents are flat at
  extremes. Solve x(s) = t for s with Newton or bisection.
- Then derive the tangent at a middle key from `three-keys-monotone` (102.47 at frame 31)
  and `uneven`.
- IMPLEMENTER.md §3.7: three genuinely different hypotheses per mismatch, then pin it.
- `vector` is three identical tracks. One test case per file is fine.

## Done when
- [ ] Spec `docs/specs/ANIM.md` §1 written: data model, addressing, rules with case
      citations, the spline formula, and what is parity-pending (stored tangents,
      tangent modes and the clamp option: O2 couldn't read their labels).
      CORE.md §1, §4 and §6 updated.
- [ ] Tests:
  - core: track validation, undo of `set_key`, and v4 save/load round-trip with v3
    still loading;
  - anim: unit tests for each rule and `scene_at` on a cube with an animated
    `transform.pos.x`;
  - the parity test above, with its result stated as N/9 cases (11 sample files).
- [ ] `sh scripts/gate.sh` prints `GATE: all passed`. `docs/parity/STATUS.md` has a row
      "Keyframes + interpolation".
- [ ] Committed with the trailer below; report written.

Commit trailer:
```
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01AF8235TxjbnG4zHfXN22Nb
```

## Out of scope
UI, recording and the timeline (brief 10), the F-curve editor, tangent editing,
non-float tracks (bool, enum, colour), and time remapping.

## Report (implementer fills in)
