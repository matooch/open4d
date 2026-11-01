# Soft bodies and connectors — observation (O41)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: as [dynamics.md](dynamics.md) (O39): frame-stepped, global transforms per frame as
track samples. For soft bodies, export `mesh.obj` at frames 15, 30 and 60 as `mesh.obj`,
`mesh-f30.obj` and `mesh-f60.obj`; `case.toml` names the extra meshes in
`[params] extra_meshes`.

Use `feature = "soft-bodies"` and `tests/golden/soft-bodies/<case>/`. Level C.
Unblocks: I/117, 118.

Record the soft body settings table and the connector settings table once.

## Cases

| Case | Scene | Notes |
|---|---|---|
| soft-sphere-drop | a sphere (r 50, segments 16) at y 200, rigid body tag set to soft body, defaults; a collider plane at y 0 | Meshes at 15/30/60 |
| soft-sphere-stiff | as above, with the soft body stiffness at max | |
| soft-sphere-pressure | as above, with pressure at a high value | |
| hinge | two cubes (size 50) side by side at y 300; the left one static, the right one dynamic; a Hinge connector between them | Track the right cube |
| spring | a static cube at y 300 and a dynamic cube at y 150 linked by a Spring object, default stiffness | Track the dynamic cube |
| ball-socket | as hinge, with a Ball and Socket connector | |

## Notes
TODO
