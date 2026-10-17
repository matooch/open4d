# MoGraph spec checklist (verify each against manual + observation; add anything missing in the installed version's menus)
## Generators
- [ ] Cloner: modes Linear, Radial, Grid Array, Honeycomb, Object (vertex/edge/polygon/surface/volume/axis), Blend; Iterate/Random/Blend/Sort clone order; fix clone, fix texture, instance modes (Instance/Render Instance/Multi-Instance), effectors list, per-clone transforms, scale/rotation/color/weight, MoData output
- [ ] Matrix, Fracture (modes + connectors), Voronoi Fracture (sources, distribution, shape, details), MoSpline (Spline/Turtle/Simple modes), MoText, MoInstance, Tracer (trace links/vertices, interval, spline types)
## Effectors
- [ ] Plain, Delay, Formula, Inheritance, Python, Random, Shader, Sound, Spline, Step, Target, Time, Volume, Push Apart, Reeffector, Group
- [ ] Common: strength, selection, transform (P/S/R), color, weight, UV, visibility, falloff (shapes/directions/field list), min/max, mode (Add/Subtract/Multiply/etc.)
## Fields
- [ ] Solid, Spherical, Box, Cylinder, Capsule, Cone, Torus, Formula, Linear, Radial, Random, Shader, Sound, Step, Delay, Object, Group, Python
- [ ] Field layers: blend modes, remapping (curve, clamping, inverse), mask/folder layers, direction/color output
## Related
- [ ] MoGraph selection tag, Multi Shader, Color Shader, MoGraph Weight/Cache tags, Dynamics Body/Collider tags interplay, Cloner + deformer ordering rules
Each item: spec file docs/parity/<item>.md, golden export, parity test.
