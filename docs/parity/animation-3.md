# Animation III: more track types — observation (O53)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
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
| generator-param | a Bend's strength keyed 0 at 0 and 90 at 30, on the O9 cube | `mesh.obj` at frame 15 |

## Notes
TODO
