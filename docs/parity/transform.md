# Transforms — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Unblocks: object transforms, rotate gizmo, object-local gizmo axes, coordinate panel.

All cases use a **default cube** (its vertex order is fixed, so per-vertex comparison
reveals the exact rotation math). Folder: `tests/golden/transform/<case>/`.
`[params]` keys: `pos_1..3`, `rot_1..3`, `scale_1..3` = the 1st/2nd/3rd field of each
group **in the order the app shows them** (we don't assume which axis they mean).

## The coordinate fields (as shown)

| Group | Field 1 label | Field 2 label | Field 3 label | Unit shown | Default |
|---|---|---|---|---|---|
| Position | | | | | |
| Rotation | | | | | |
| Scale | | | | | |
| Size (if separate) | | | | | |

- Rotation field 1 turns the cube around which **screen/world** axis? field 2? field 3? TODO
- Is there a rotation-order setting? Options and default: TODO
- Object vs world coordinates switch? Options: TODO

## Cases

| Case | Set (field order) | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| identity | nothing | | |
| rot-1-30 | rot_1 = 30 | | |
| rot-2-30 | rot_2 = 30 | | |
| rot-3-30 | rot_3 = 30 | | |
| rot-combined | rot = (30, 45, 60) | | the one that reveals rotation order |
| scale | scale = (2, 3, 4) | | does export bake scale? |
| full | pos = (10, 20, 30), rot = (30, 45, 60), scale = (2, 3, 4) | | |
| child | cube as child of a null at pos (0, 50, 0), rot_1 = 90; cube pos (100, 0, 0) | | does export bake the parent? |

## Behaviour notes
- Does the exported OBJ contain world-space points (transforms baked)? TODO
- Anything that changes when you switch object/world coordinates: TODO
