# Editing meshes

Once an object is a polygon object you can work on its components.

## Edit modes

`select.mode` switches between **Model**, **Points**, **Edges** and **Polys**. In a
component mode the selection helpers apply to components: `select.all`, `select.none`,
`select.invert`, `select.grow` and `select.shrink`. Moving, rotating and scaling
components uses the gizmo; soft selection (`edit.soft_selection`) and symmetry spread the
effect to neighbours.

## Modelling tools

| Tool | Command |
|---|---|
| Extrude | `mesh.extrude` |
| Extrude inner | `mesh.extrude_inner` |
| Bevel | `mesh.bevel` |
| Subdivide | `mesh.subdivide` |
| Triangulate | `mesh.triangulate` |
| Optimize | `mesh.optimize` |
| Reverse normals | `mesh.reverse_normals` |
| Loop cut, knife, slide | `mesh.loop_cut`, `mesh.knife`, `mesh.slide` |
| Bridge, close hole, stitch, pen | `mesh.bridge`, `mesh.close_hole`, `mesh.stitch`, `mesh.pen` |
| Array | `mesh.array_command` |

The palette buttons run these with the stored selection and default settings. Tools that
need the pointer (cuts, bridge, pen) live in the Mesh menu. Every tool is also a registry
command, so it can be scripted: see the [command reference](reference/commands.md) for
the arguments of each.

## Snapping and axes

Snapping (`snap.*`) and axis mode (`edit.axis_mode`) are available as commands and in the
Edit menu. `object.set_axis`, `object.center_axis` and `object.align_axis` move an object's
axis without moving its geometry.
