# Primitives and generators

A **generator** is an object whose geometry is computed from its parameters. Changing a
parameter rebuilds the mesh; nothing is baked until you make the object editable.

## Parametric primitives

| Type id | Object |
|---|---|
| `forge.plane` | Plane |
| `forge.cube` | Cube |
| `forge.sphere` | Sphere |
| `forge.cylinder` | Cylinder |
| `forge.cone` | Cone |
| `forge.disc` | Disc |
| `forge.torus` | Torus |
| `forge.tube` | Tube |
| `forge.capsule` | Capsule |
| `forge.pyramid` | Pyramid |
| `forge.platonic` | Platonic solids |
| `forge.oil_tank` | Oil tank |
| `forge.polygon_prim` | Regular polygon |
| `forge.figure` | Simple figure |
| `forge.landscape` | Landscape |

Most primitives share an **orientation** option that chooses which axis they stand along.
Round primitives (cylinder, cone, disc, torus, tube, capsule) are built by revolving a
profile, so they also share segment and cap options.

Create one with `object.create` and a type id, then edit parameters in the Attribute
manager or with `param.set`.

## Generators with inputs

Some generators take their children as input: they consume the children and show the
combined result. Modelling generators, spline generators, subdivision and boolean
operations work this way. Children that are consumed are hidden from the viewport while
the generator is active.

## Making geometry editable

`mesh.make_editable` converts the evaluated result into a plain polygon object that you can
edit by point, edge or polygon. *Current state to object* does the same while keeping the
original.
