# Soft bodies and connectors — observation (O41)

Status: observed, 4 of 6 cases · App version: R25 25.010 · OS: macOS · Observer: Hermes observer, scripted unattended run · Date: 2026-11-20
Method: as [dynamics.md](dynamics.md) (O39): frame-stepped, global transforms per frame as
track samples. For soft bodies, export `mesh.obj` at frames 15, 30 and 60 as `mesh.obj`,
`mesh-f30.obj` and `mesh-f60.obj`; `case.toml` names the extra meshes in
`[params] extra_meshes`.

Use `feature = "soft-bodies"` and `tests/golden/soft-bodies/<case>/`. Level C.
Unblocks: I/117, 118.

### Soft body settings, at their defaults

Read back from the dynamic body tag after switching Soft Body from off to Made of Polygons/Lines.

| Setting | Default | Min / Max / Options |
|---|---|---|
| Soft Body | Off, set to Made of Polygons/Lines for these cases | Off / Made of Polygons/Lines / Made of Clones |
| Keep Soft Body Shape | on | on / off |
| Structural | 100 % | 0 - 1e+18 |
| Damping (structural) | 20 % | 0 - 1e+18 |
| Elastic Limit (structural) | 1000 | 0 - 1e+18 |
| Shear | 50 % | 0 - 1e+18 |
| Damping (shear) | 20 % | 0 - 1e+18 |
| Elastic Limit (shear) | 1000 | 0 - 1e+18 |
| Flexion | 50 % | 0 - 1e+18 |
| Damping (flexion) | 20 % | 0 - 1e+18 |
| Elastic Limit (flexion) | 180 deg | 0 - 1e+18 |
| Pressure | 0 | 0 - 1e+18 |

### Connector settings, at their defaults

Read back from the connector object the app's own command creates.

| Setting | Default | Min / Max / Options |
|---|---|---|
| Type | Hinge | 10 options, labels not read |
| Object A | empty | — |
| Reference Axis A | 0 | 4 options, labels not read |
| Attachment A | 0 | 3 options, labels not read |
| Offset | 0, 0, 0 | — |
| Region Of Influence | 0.2 | — |
| Shape Conservation | 1.0 | — |
| Damping | 0.2 | — |
| Object B, Reference Axis B, Attachment B, Offset, Region Of Influence, Shape Conservation, Damping | as for A | — |
| Ignore Collisions | on | on / off |
| Cone Radius | 60 deg | — |
| Ellipse | off | on / off |
| Cone Radius Y | 60 deg | — |
| Steering Angle | 0 | — |
| Suspension Rest Position | 0 | — |
| Suspension Stiffness | 1.0 | — |
| Suspension Damping | 0.2 | — |
| Bounce | 0 | — |

Record the soft body settings table and the connector settings table once.

## Cases

| Case | Scene | Notes |
|---|---|---|
| soft-sphere-drop | a sphere (r 50, segments 16) at y 200, rigid body tag set to soft body, defaults; a collider plane at y 0 | Samples: the sphere falls and deforms. Local y extent -50..50 at frame 0, -53.290..46.710 at frame 15, -50.659..48.686 at frame 30 and -55.202..49.167 at frame 60, while its origin goes from y 200 to y 34.085. Meshes at frames 15, 30 and 60 (mesh.obj, mesh-f30.obj, mesh-f60.obj); the frame-60 mesh has the plane plus the squashed sphere, bbox 400.0 x 104.373 x 400.0 cm. Stepping the frames one at a time matters: jumping straight to a frame leaves the soft body rigid. |
| soft-sphere-stiff | as above, with the soft body stiffness at max | Structural (the soft body stiffness row) assigned 1000, read back 1000. The sphere still falls (origin y 35.160 at frame 60) but deforms less than the default case: local y extent -51.144..49.672 at frame 60 instead of -55.202..49.167. Meshes at 15, 30 and 60. |
| soft-sphere-pressure | as above, with pressure at a high value | Pressure assigned 100, read back 100. The frames are identical to the default case down to the last digit (-53.290..46.710 at 15, -50.659..48.686 at 30, -55.202..49.167 at 60): the value is stored but changes nothing visible in this build. Recorded as observed. |
| hinge | two cubes (size 50) side by side at y 300; the left one static, the right one dynamic; a Hinge connector between them | Connector Type reads Hinge at its default; Object A = Left, Object B = Right, both filled from the script. The right cube swings about the hinge and ends at (-299.510, -35.899, 0.000) with H 0.0 / P 0.0 / B 258.208 deg at frame 90, that is, it swings under the plane. |
| spring | a static cube at y 300 and a dynamic cube at y 150 linked by a Spring object, default stiffness | Track the dynamic cube |
| ball-socket | as hinge, with a Ball and Socket connector | |

## Notes
TODO

## Notes

Window of 2026-11-20, scripted and unattended. Four of six cases are observed:
`soft-sphere-drop`, `soft-sphere-stiff`, `soft-sphere-pressure` and `hinge`.

- Everything here needs frames stepped one at a time. Setting the document to frame 15, 30 or 60
  directly leaves the soft body undeformed and motionless; stepping 0 to 60 one frame at a time does
  simulate it. The soft body also needs Soft Body set on the second of the two rows carrying that
  label (the first is the block header and accepts a write without changing anything).
- Pressure at 100 makes no visible difference to the deformation in this build. The reading is
  recorded, not adjusted.
- The connector's Object A and Object B slots can be filled from a script; earlier notes that said
  links cannot be set applied to keying a link, not to setting it.
- Two cases remain: `spring` (the app has a separate Spring object, next to the Connector) and
  `ball-socket` (the Connector's Type set to the ball and socket option). Both use the hinge recipe
  above with a different object or type; this window ended before them.
