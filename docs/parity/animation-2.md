# Animation II: more tracks and tags — observation (O27)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
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
TODO
