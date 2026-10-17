# Subdivision surface — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Unblocks: the subdivision generator. Folder: `tests/golden/subdivision/<case>/`.
Export the **generator's result** (the subdivided mesh), not the input.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Notes |
|---|---|---|---|---|---|---|
| TODO (viewport/editor level) | `level_view` | int | | | | |
| TODO (render level) | `level_render` | int | | | | which one does export use? |
| TODO (type / scheme) | `scheme` | enum | | | | list every option |
| TODO (boundary / UV options) | | | | | | |

## Cases

Input meshes are default primitives placed under the generator.

| Case | Input & settings | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| cube-1 | default cube, level 1 | | |
| cube-2 | default cube, level 2 | | |
| cube-3 | default cube, level 3 | | |
| plane-1 | default plane, level 1 | | open boundary rule |
| tris-1 | a primitive with triangles (e.g. default pyramid/cone), level 1 | | triangle rule |
| ngon-1 | a primitive with an n-gon face (e.g. cylinder with n-gon caps if available), level 1 | | n-gon rule |
| scheme-<key> | default cube, level 2, each non-default scheme option | | |

Also export the **input** of `tris-1` and `ngon-1` once (as `input.obj` in the same
folder) so we know the exact starting mesh.

## Behaviour notes
TODO
