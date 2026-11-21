# Animation III: more track types — observation (O53)

Status: partial · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-20
Method: as [animation-2.md](animation-2.md): 30 fps, sampled every frame 0–60. Output in
`tests/golden/animation-3/<case>/` as track samples. Use a `mesh.obj` where the row says.

Unblocks: G/088, 096.

## Cases

| Case | Setup | Sample |
|---|---|---|
| bool-track | the cube's Phong tag "angle limit" checkbox keyed off at 0 and on at 30 (or any keyable checkbox) | the checkbox as 0/1, param `flag` |
| enum-track | a sphere's Type keyed Standard at 0 and Icosahedron at 30 | the enum index, param `type_index`; and `mesh.obj` at frame 29 and `mesh-f30.obj` at frame 30 (`extra_meshes`) |
| color-track | a material's colour keyed (1, 0, 0) at 0 and (0, 0, 1) at 30 | params `color_r`, `color_g`, `color_b` |
| int-track | a cube's segments X keyed 1 at 0 and 10 at 30 | `segments_x` (integer stepping) |
| link-track | a Bend deformer's… (any link field that can be keyed; skip if none) | – |
| pla | a cube made editable, point 0 moved by (0, 100, 0) and keyed with point-level animation at frame 30 (keyed at frame 0 at rest) | `mesh.obj` at frame 15 |
| euler-cross | a cube's rotation H keyed 170° at 0 and −170° at 30 | `rot_h`: does it go the long way (−340°) or the short way? |
| quaternion | the same, with the track's interpolation set to quaternion (if the option exists) | rot_h, rot_p, rot_b |
| tie-span-24 | follow-up from brief 087: document fps 24; a cube's position X keyed 0 at frame 0 and 100 at frame 10 (auto tangents), sampled at every frame 0–10 | `pos_x`; settles whether the handle-time grid is in seconds or frames |
| tie-span-30 | the same at fps 30, keys at frames 0 and 7 | `pos_x` |
| generator-param | a Bend's strength keyed 0 at 0 and 90 at 30, on the O9 cube | `mesh.obj` at frame 15 |

## Follow-up (planner, after brief 088)

| Case | Redo as | What was missing |
|---|---|---|
| generator-param | generator-param-2 | `case.toml` must hold the cube (size, segments) and every Bend setting (size, mode, strength/angle, limits), not only the animated parameter |

## Notes

Seven cases observed, four not observed; golden check: ok (945 cases). Every observed case
has a `case.toml` (`feature = "animation-3"`, `observation = "docs/parity/animation-3.md"`,
`[params]` with the keys) and one `samples-<param>.json` per sampled value (`kind = "track"`,
`fps`, `tolerance 0.0001`, `[[frame, value], ...]` in frame order).

- **int-track** (`segments_x`, keys 1 @ 0 → 10 @ 30): the parameter holds and then steps —
  1 at frames 0–3, 2 at 4–6, 3 at 7–9, 4 at 10–13, 5 at 14–16, 6 at 17–19, 7 at 20–23,
  8 at 24–26, 9 at 27–29, 10 from 30 on to 60. The track is an ordinary track on the
  parameter itself (the cube's Subdivision X parameter), keys written with
  keys inserted into that track; the app evaluates it and the parameter follows.
- **color-track** (`color_r/g/b`, material colour (1, 0, 0) @ 0 → (0, 0, 1) @ 30): linear.
  r 1.0 @ 0, 0.5 @ 15, 0.0 @ 30, stays 0 to 60; g 0.0 at every frame; b 0.0 @ 0,
  0.500001 @ 15, 1.0 @ 30, stays 1 to 60. Read from the material's colour parameter.
- **euler-cross** (`rot_h`, 170° @ 0 → −170° @ 30): the app takes the **long** way, through
  0. 170 @ 0, 90.666889 @ 7, −0.000486 @ 15, −90.666889 @ 23, −170 @ 30, −170 @ 60. That is
  −340° of travel and no unwrapping of the Euler angle.
- **quaternion** (same rotation, key interpolation forced to the key's quaternion item):
  rot_h is identical to euler-cross at every sampled frame (170 / 90.666889 / −0.000486 /
  −90.666889 / −170 / −170) and rot_p, rot_b stay 0. Setting the item does **not** make the
  interpolation take the short way here, so no separate quaternion path was observed.
  Note the choice: the form's "interpolation … (if the option exists)" was read as the key's
  quaternion item on the H track; the setter accepted value 2 and read back 2, but that is a
  parameter value, not proof that the app treats the track as quaternion.
- **tie-span-24** (fps 24, `pos_x` 0 @ 0 → 100 @ 10, auto tangents): 0, 9.999974, 20.000015,
  29.999988, 40, 50.000143, 60, 70, 80, 90, 100 at frames 0–10 — uniform 10 units per frame.
- **tie-span-30** (fps 30, keys 0 @ 0 → 100 @ 7): 0, 14.285731, 28.57141, 42.857078, 57.1428,
  71.42859, ... 100 @ 10 — uniform 100/7 per frame. Same shape at both fps, so the handle
  grid follows frames, not seconds: the same frame span gives the same curve regardless of
  the document fps. (Both cases sampled every frame 0–10.)
- **generator-param** (Bend strength 0 @ 0 → 90 @ 30 on the O9 cube): the strength reads
  0 @ 0, 45.000129 @ 15, 90 @ 30, 90 @ 60, and `mesh.obj` is the bent cube at frame 15
  (27943 bytes, 8 vertices). Also written: `samples-strength.json` with the strength curve
  itself, which the row did not ask for.
- **bool-track — not observed:** the Phong tag's "angle limit" track holds two keys
  (frames 0 and 30) and its curve evaluates 0 → 0.5 → 1, but the tag parameter itself reads 0
  at frames 0, 15 and 30. Checked again on an object-level checkbox (a sphere's "render
  perfect"): the track animates, the parameter keeps its base value. Boolean parameters are
  therefore not driven by script-created tracks in this app version.
- **enum-track — not observed:** the sphere's Type track holds keys 0 (Standard) and 30
  (index 4) and the curve evaluates, but the parameter reads 0 at frames 0, 15, 30 and 60.
  The sphere never switches type, so the frame-29 and frame-30 meshes would have been
  identical and none was written.
- **link-track — not observed:** the only link-typed rows exposed to scripting are the "layer"
  rows (the "layer" link field) on the Bend and on the cube, and no value can be written to
  a key on them (`SetValue` raises "must be real number, not an object"). No link field
  in this build takes a scripted key, so the row's "skip if none" applies.
- **pla — not observed:** point-level animation could not be created. Making the cube editable
  and moving point 0 to (−100, 100, −100) at frame 30 while the record command was on moved
  the point in the base mesh (it reads (−100, 0, −100) at every frame) instead of keying it;
  the only tracks the record command created were the object's position/scale/rotation tracks
  with zero keys. No PLA track can be reached from the bridge (a point-animation data type exists in the interface but no
  point-animation parameter is exposed), so no frame-15 mesh was written.
