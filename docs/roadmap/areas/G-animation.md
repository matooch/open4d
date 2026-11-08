# G · Animation: keys, dope sheet, value graph, speed graph

## Scope (v1)

- [x] Tracks on Float and Vec3 components, Spline/Linear/Step interpolation, `scene_at`
  (brief 09); timeline, record, autokey, playback (brief 10).
- [ ] **Keyframe rework:** the O16 rules (millisecond rounding at 24 fps and quarter
  frames; the tangent at a monotone key between uneven spans).
- [ ] **Tracks for every animatable type:**
  - bool, int and enum: step;
  - colour: per-channel tracks;
  - link: step;
  - point-level animation (per-point positions, morph-like);
  - the parameters of generators, deformers, effectors, fields and materials.
- [ ] **Tangent model:**
  - auto (clamped, as observed), user tangents, broken (left ≠ right), weighted
    (length-free);
  - zero-angle and zero-length options;
  - ease-in / ease-out presets;
  - overshoot prevention option.
- [ ] **Track behaviour before/after:** constant, repeat, offset repeat, oscillate,
  continue.
- [ ] **Dope sheet:**
  - track tree (objects → tracks), summary rows, key display;
  - box select, move, scale (time), ripple;
  - copy/paste keys; snapping to frames; markers.
- [ ] **Value graph (F-curves):**
  - curves per track, multiple selection, frame all/selected, normalised view;
  - tangent handles (edit, break, unify, weight);
  - key value/time numeric entry; curve colours per axis (X red, Y green, Z blue, as the
    reference does).
- [ ] **Speed graph:**
  - the first derivative (velocity) of the selected curves, drawn and editable;
  - dragging the speed curve's handle shapes ease in/out by adjusting the neighbouring
    tangents;
  - "constant speed" and "ease" presets.
- [ ] **Key reduction** (bake, then reduce within a tolerance); bake to keys (from
  expressions, dynamics and tags).
- [x] **Time (brief 094, parity-pending G5):** preview range, playback modes (realtime with frame drops, every frame),
  fps conversion, time display (frames, SMPTE, seconds), markers.
- [ ] **Rotation:** Euler filter (unwrap), and a quaternion interpolation option per track.
- [x] **Display (brief 095, parity-pending G5):** motion paths (trajectories), onion skin (ghosting) in the viewport.
- [ ] **v2:** motion layers, motion clips (non-linear animation), sound tracks.

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O2, O12, O16 | keyframes I–III | done (26 track files) |
| O27 | [animation-2.md](../../parity/animation-2.md): all rotation and scale axes, parameter tracks, tags, loops | todo |
| O38 | [fcurves.md](../../parity/fcurves.md): user tangents set by script, broken, weighted, zero angle/length flags, ease presets, overshoot option off, before/after behaviours | todo |
| O53 | [animation-3.md](../../parity/animation-3.md): bool/enum/colour/link tracks, point-level animation, Euler crossing ±180°, quaternion option | todo |
| G5 | [GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md) §G5: dope sheet and F-curve editor behaviour (handles, speed curve view, presets) | gui-only |

## Implementation design

- **Data:** `forge-core::track` (brief 09) grows:
  - `Key { time, value: KeyValue, interp, tangent: Tangent { left, right, mode, weighted, broken }, flags }`;
  - `KeyValue` = `Float(f64)` | `Step(Value)`;
  - file format bump.
- **Maths:** `forge-anim::interp`.
  - The Bézier-in-parameter evaluation and the solver stay as brief 09 derived them. Auto
    tangents follow the rules from O2/O12/O16, plus the rework.
  - Weighted tangents free the time component.
  - The speed curve is the analytic derivative dv/dt of the segment.
  - Editing in the speed graph maps back onto tangents, own design, documented in
    ANIM.md: speed handles scale the outgoing and incoming tangent lengths.
- **UI** (`forge-ui`, one module per manager):
  - `dopesheet.rs` and `fcurve.rs` share a `TimeView` (pan/zoom, frame ruler, snapping)
    and a `KeySelection`;
  - pure functions (`fcurve_logic.rs`) for hit testing, box select, scaling and tangent
    editing, so egui_kittest and unit tests cover them without a GPU.
- **Commands:** every edit is a registry command (`key.move`, `key.set_tangent`,
  `track.set_behavior`, …), so agents can animate through MCP.
- **Performance:** `scene_at` is cached per frame (brief 10). Playback evaluates
  incrementally: only animated nodes and their dependants are re-evaluated (N/180).

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 087 | Keyframe rework: O16 rules (ms rounding at other fps; uneven monotone tangent) | 009 | O16 |
| 088 | Tracks for all value types + point-level animation | 087 | O53 |
| 089 | Tangent model: user, broken, weighted, zero-angle/length, ease presets, overshoot option | 087 | O38 |
| 090 | Dope sheet manager | 089, 011 | G5 |
| 091 | Value graph (F-curve) manager | 089, 090 | G5, O38 |
| 092 | Speed graph mode and speed editing | 091 | G5 |
| 093 | Track before/after behaviours, key reduction, bake | 089 | O27, O38 |
| 094 | Time: markers, preview range, playback modes, fps conversion, time display | 010 | G5 |
| 095 | Motion paths and onion skin | 010, 143 | – |
| 096 | Rotation: Euler filter, quaternion interpolation option | 089 | O27, O53 |
| 097 | Keyframe indicators on every parameter; key context menu | 088, 150 | G4 |
| 098 | Animated evaluation performance (dependency-aware re-evaluation) | 180 | – |
| 099 | S7 acceptance test and animation docs | 090–096 | – |

## Done when
S7 passes. Every observed keyframe case matches at level B. The dope sheet, value graph
and speed graph are fully usable with mouse and keyboard, and fully scriptable.
