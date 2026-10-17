# Smoothing / normals — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Unblocks: smoothing angle, normal weighting (GEOMETRY.md §3 parity-pending), viewport
smooth shading. **Normals must be exported** (OBJ `vn` lines; `golden info` shows
`normals: N`). Folder: `tests/golden/smoothing/<case>/`.

## How smoothing is controlled

| Question | Answer |
|---|---|
| What controls smooth vs faceted shading (object setting, tag, …)? | TODO |
| Is it present on new primitives by default? | TODO |
| Smoothing-angle parameter label, default, range, unit | TODO |
| Can individual edges be marked hard/sharp? How? | TODO |

## Cases

| Case | Object & settings | `golden info` (incl. normals count) | Notes |
|---|---|---|---|
| cube-default | default cube, default smoothing | | |
| cylinder-default | default cylinder, default smoothing | | caps vs sides |
| cube-angle-89 | default cube, smoothing angle 89° | | |
| cube-angle-91 | default cube, smoothing angle 91° | | |
| uneven | a mesh where faces of very different size/angle meet at a vertex (e.g. a cone with few segments), default smoothing | | reveals weighting |

## Behaviour notes
TODO
