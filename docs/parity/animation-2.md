# Animation II: more tracks and tags — observation (O27)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-20
Method: as [keyframes.md](keyframes.md): 30 fps, keys made with the record command,
sampled at every frame 0–90. Each case writes one track file per sampled value, as
`samples-<param>.json` (kind `track`). `param` is `pos_x`, `pos_y`, `pos_z`, `rot_h`,
`rot_p`, `rot_b`, `scale_x`, `scale_y`, `scale_z`, or the parameter named. Rotation is
in degrees. Use the object's **evaluated** values: with a tag, read what the tag produces.

Folder `tests/golden/animation-2/<case>/`, with `feature = "animation-2"` and
`observation = "docs/parity/animation-2.md"`. `[params]` lists the keys and every tag
setting.

Unblocks: P5 (tags) and batch 09 follow-ups.

## Cases

| Case | Setup | Sample |
|---|---|---|
| rot-all-axes | a cube keyed with record at 0 (rotation 0, 0, 0) and 30 (rotation H 90, P 45, B 30) | rot_h, rot_p, rot_b |
| scale-keys | a cube keyed at 0 (scale 1) and 30 (scale 2, 1, 0.5) | scale_x, scale_y, scale_z |
| param-track | a cube with a key on its size Y at 0 (200) and 30 (400): use the parameter's own key, not record | the evaluated size Y as `size_y` |
| param-mesh | the same, but also export `mesh.obj` of the cube at frame 15 | the same, plus the mesh |
| target-tag | a cube at (0, 0, 0) with a Target tag aimed at a Null that is keyed from (300, 0, 0) at 0 to (0, 300, 0) at 30 | rot_h, rot_p, rot_b of the cube |
| align-to-spline | a cube with an Align to Spline tag on a default Circle spline, its position keyed 0 % at 0 and 100 % at 90 | pos_x, pos_y, pos_z |
| align-to-spline-tangential | the same, Tangential on | pos and rot, all six |
| vibrate-tag | a cube with a Vibrate tag: position on, amplitude (50, 50, 50), frequency 2, seed shown | pos_x, pos_y, pos_z. Record the seed |
| parent-child | a Null keyed in rotation H 0 → 90 over 30 frames, with a cube child at (100, 0, 0) | the cube's **global** position as pos_x, pos_y, pos_z |
| loop-after | a cube keyed 0 → 100 in pos_x over 30 frames, track after-behaviour set to Repeat (or the loop option) | pos_x |
| offset-repeat | the same, after-behaviour Offset Repeat (or the closest) | pos_x |

## Notes

All eleven cases were observed in one scripted session; every case has one
`samples-<param>.json` per sampled value (94 samples: frames 0-90 every frame plus the half
frames 0.5, 14.5, 29.5, in frame order) and a `case.toml`. Golden check: ok (926 cases).

- **The record command does not work on tags:** `c4d.CallCommand(12410)` returns False for a tag
  (no keys appear). Tag parameters here were keyed through the tag's own track
  (`CTrack(tag, descid)` + `AddKey`), which the app evaluates the same way. Object keys use the
  record command as the form asks.
- **Target tag** (plugin 5676, target link row `1001` "Target Object"): with the target Null at
  (300, 0, 0) the cube reports rot_h 270 deg, rot_p 0; at frame 15 (target at (150, 150, 0))
  rot_h 270, rot_p 45; from frame 30 (target at (0, 300, 0)) it holds rot_h 0, rot_p 90. rot_b is
  0 at every frame. The tag only aims: it writes rotation, never position. The generic row
  `110050` ("Basic Properties") also accepts an object link but has no effect - only row 1001
  aims the object.
- **Align to Spline** (tag 5699 on a default Circle, plugin 5181, radius 200): the tag's
  *Position* parameter is 0-1, not percent (0 % at 0, 100 % at 1.0). Over 90 frames the cube makes
  one full revolution: pos_x 200 -> 100.068 (f15) -> -100.069 (f30) -> -200 (f45) -> 200 (f90);
  pos_y 0 -> 173.304 (f15) -> 173.304 (f30) -> 0.002 (f45) -> -0.126 (f90); pos_z 0 at every frame.
  The y values are 200*sin(60 deg), so the spline starts at +x and runs counter-clockwise seen
  from +y. At f90 the endpoint is not bit-exact (-0.126 instead of 0).
- **Tangential** adds orientation on top of the same positions: rot_p 90 -> 30.006 (f15) ->
  -30.006 (f30) -> -90 (f45) -> 89.965 (f90); rot_h 0 -> 90 -> 90 -> 0 -> 270; rot_b 0 everywhere.
- **Vibrate tag**: seed 0, amplitude (50, 50, 50), frequency 2, position only (scale and rotation
  off), Regular Pulse off, Relative on. No keys are needed: the position is offset by a bounded
  wobble (|offset| stays under ~30 on every axis over 0-90; pos_x -10.480 at f0, -7.412 at f30,
  6.340 at f45). The values are only reproducible for this seed.
- **param-track / param-mesh**: keying the size Y parameter itself (200 at f0, 400 at f30) gives
  size_y 200 -> 300.000 (f15) -> 400, then constant; the record command is not involved. The
  `mesh.obj` exported at frame 15 has y = +-150.000143 and x/z = +-100, i.e. the evaluated
  (interpolated) size, not the 200 of the first key.
- **parent-child**: the cube's *global* position follows the parent's rotation: (100, 0, 0) at f0,
  (70.711, 0, 70.711) at f15, (0, 0, 100) at f30, then constant. pos_y is 0 at every frame.
- **Track after-behaviour**: with keys 0 -> 100 over frames 0-30, Repeat gives pos_x 50 at f45 and
  100 at f90 (frame 90, the document's last frame, is not wrapped back to 0); Offset Repeat gives
  150 at f45 and 300 at f90, i.e. 100 per 30-frame cycle added without bound.
- **rot-all-axes / scale-keys**: the record command writes all three channels at once; the sampled
  midpoints are exactly the linear midpoints (rotation 45 / 22.5 / 15 at f15; scale_x 1.5,
  scale_z 0.75) and the untouched scale_y stays 1.
- Deviations: none in the setups. Tag parameters the form does not pin (Vibrate's Regular Pulse /
  Relative, the Align tag's default Axis) were left at their defaults and are recorded in each
  `case.toml`.
