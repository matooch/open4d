# Clone toolset III — observation (O50)

Status: partial · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-20
Method: as [cloner.md](cloner.md) (O8) for clone data (`samples.json`, kind `clones`) and
meshes. Spline outputs go through an Extrude as in [splines-2.md](splines-2.md).

Folders:
- `tests/golden/cloner/<case>/` for cloner cases;
- `tests/golden/clone-spline/<case>/` for the spline generator;
- `tests/golden/tracer/<case>/` for Tracer.

Unblocks: E/067, 076, 077, 078.

## Cases

| Case | Setup | Output |
|---|---|---|
| cloner/blend-order | Linear, count 5; children a cube (size 20) and a sphere (r 10); clones mode Blend | samples + mesh |
| cloner/selection-tag | Linear, count 5, plus a clone selection tag selecting clones 1 and 3, and a Plain effector (P 0, 100, 0) restricted to that selection | samples + mesh |
| cloner/weight-tag | Linear, count 5, with a clone weight tag (weights painted or set 0, 0.25, 0.5, 0.75, 1 by script) and a Plain effector using weight | samples + mesh |
| cloner/on-spline | Object mode on a default Helix, count 10, distribution Count | samples + mesh |
| cloner/on-spline-step | the same, distribution Step 50 | samples + mesh |
| clone-spline/simple | the clone-toolset spline generator in Simple mode, defaults | extruded mesh |
| clone-spline/turtle | Turtle mode, defaults | extruded mesh |
| clone-spline/spline-mode | Spline mode with a default Circle as source | extruded mesh |
| tracer/vertices | Tracer, tracing mode at its default (param 1110 does not read back), Sample Step 1, on a cube keyed 0 → 300 in x over frames 0–30, document stepped frame by frame; exported at frame 30 through an Extrude | mesh |
| tracer/interval | the same, with Sample Step 5 | mesh |
| instance-time/defaults | the instance-over-time generator on a cube keyed as above, defaults, at frame 30 | samples + mesh |
| cloner/transform-rs | follow-up from brief 065: Linear cloner (count 3, offset (0, 0, 100)) of a 20 cm cube; Transform tab R (0, 30, 0), S (1.5, 1, 1), colour (1, 0, 0), weight 50 % | samples |
| cloner/linear-offset | Linear, count 5, P (0, 50, 0), Offset 1, then a second case `linear-offset-15` with Offset 1.5 | samples |
| cloner/linear-amount | Linear, count 5, P (0, 50, 0), Amount 50 % | samples |
| cloner/linear-rstep | Linear, count 5, P (0, 50, 0), per-step R (0, 20, 0) with Step Rotation H 10° | samples |
| cloner/linear-endpoint-sr | Linear End Point mode, count 5, P (0, 200, 0), S (0.5, 0.5, 0.5), R (0, 90, 0) | samples |
| cloner/radial-var-1 | Radial, count 8, Offset Variation 50 %, seed 1; and `radial-var-2` with seed 2 | samples |
| cloner/grid-cyl-fill | Grid Array, 5/5/5, form Cylinder, Fill 50 % | samples |
| cloner/grid-sphere-uneven | Grid Array, counts 7/5/3, size (300, 200, 100), form Sphere | samples |
| cloner/honey-orient-x | Honeycomb, orientation X (if offered), defaults otherwise; and `honey-orient-y` | samples |
| cloner/honey-endpoint | Honeycomb, End Point mode, defaults otherwise | samples |
| cloner/honey-var | Honeycomb, both variations 50 %, seed 1 | samples |
| cloner/honey-odd-even | Honeycomb, width count 5, height count 4 | samples |
| cloner/reset-coords-off | Linear, count 3, the child cube moved to (0, 50, 0) and rotated H 30°, Reset Coordinates off | samples + mesh |
| effector-f/space-node | follow-up from brief 068: Linear cloner (count 3, P (0, 0, 100)), Step Rotation H 90°; Plain effector P (50, 0, 0), space Node; then `space-object` and `space-effector` (effector rotated H 90°) | samples |
| effector-f/absolute-rs | Plain, transform mode Absolute, R (45, 0, 0), S 2 (uniform) | samples |
| effector-f/remap-minmax | Plain, mode Remap, P (0, 100, 0), Minimum 20 %, Maximum 80 %, with a Linear field | samples |
| effector-f/abs-scale | Plain, Scale 2 with Absolute Scale on; and `abs-scale-off` | samples |
| effector-f/color-alpha | Plain, Custom Color (1, 0, 0), Strength 50 %, Use Alpha/Strength on; and `-off` | samples |
| effector-f/blend-add | Plain, Custom Color (0.5, 0, 0), Blending Mode Add over a cloner colour (0, 0.5, 0); and `blend-multiply` | samples |
| effector-f/visibility | Plain, Visibility on, with a Linear field across the clones | samples |
| effector-f/order | two Plain effectors: A rotates R (90, 0, 0), B moves P (100, 0, 0); in order A, B and then `order-swapped` | samples |
| effector-f/per-effector-50 | Plain P (0, 100, 0), the cloner's strength for that effector set to 50 % | samples |
| effector-f/linear-plain-field | re-record O8 `linear-plain-field` with the field's full parameter table and Color in `case.toml` | samples |
| random-f/count50-s12345 | follow-up from brief 069: Linear cloner count 50, P (0, 0, 20); Random effector defaults (seed 12345); then `-s0` (seed 0), `-s1` (seed 1) and `-strength50` (Strength 50 %) | samples |
| random-f/indexed-50 | the same, Indexed on; then `indexed-5` and `indexed-8` with counts 5 and 8 | samples |
| random-f/gaussian-50 | the same, mode Gaussian | samples |
| random-f/noise-steps | Linear count 20, P (0, 0, 10); Random mode Noise; then `noise-scale50`, `noise-scale200`, `noise-speed0-f30` (Animation Speed 0 %, frame 30), `noise-uv` (Space UV) | samples |
| random-f/turbulence | Linear count 20, Random mode Turbulence | samples |
| random-f/synchronized | Linear count 20, Random, Synchronized on | samples |
| random-f/sorted | Linear count 20, Random mode Sorted (if offered) | samples |
| random-f/rot-h | Linear count 20, Random with Position off and Rotation on, R (90, 0, 0) | samples |

## Case results

| Case | State |
|---|---|
| `cloner/blend-order` | 358 points / 396 polygons; samples kind `clones`, 5 clones; clone positions (0, 0, 0) (0, 50, 0) (0, 100, 0) (0, 150, 0) (0, 200, 0) |
| `cloner/selection-tag` | not observed: the scripted clone selection never reaches the effector. The selection tag takes clones 2 and 4 and reads them back, and the effector accepts the tag in its Selection field, yet the effector still offsets all five clones by (0, 100, 0). Needs a saved scene built in the GUI (3 attempts, see Notes) |
| `cloner/weight-tag` | not observed: the scripted weights on the weight tag read back 0, 0.25, 0.5, 0.75, 1, but the Plain effector with Weight Transform on offsets every clone by the full (0, 100, 0) instead of scaling by the weight, so the scripted weights never reach the stored tag data. Needs a saved scene built in the GUI (see Notes) |
| `cloner/transform-rs` | 3 clones, per-step P (0, 0, 100) → positions (0, 0, 0) (0, 0, 100) (0, 0, 200); every clone carries the cloner Transform tab: R (0, 30, 0) gives clone y axis (0, 0.866, -0.5) and z axis (0, 0.5, 0.866), S (1.5, 1, 1), colour (1, 0, 0), weight 0.5 |
| `cloner/linear-offset` | Offset on (1279 = 1) with count 5 and per-step P (0, 50, 0) → 6 clones at y = 0, 50, 100, 150, 200, 250 (one extra clone past the count); samples hold 6 clones, 5 of them visible |
| `cloner/linear-offset-15` | not observed: in this build (R25) Linear “Offset” is a checkbox with no numeric amount. `cl[1279] = 1` adds no field to the description (parameter list before and after toggling is identical) and shifts the pattern by adding one extra clone instead, so “Offset 1.5” has nothing to set |
| `cloner/linear-amount` | Amount 50 % (1272 = 0.5) with count 5 and per-step P (0, 50, 0) → positions (0, 0, 0) (0, 25, 0) (0, 50, 0) (0, 75, 0) (0, 100, 0) |
| `cloner/linear-rstep` | per-step R (0, 20, 0) with Step Rotation H 10°, count 5, P (0, 50, 0) → clone y axes (0, 1, 0) (0, 0.866, -0.5) (0, 0.5, -0.866) (0, 0, -1) (0, -0.5, -0.866), i.e. 20° about Y per clone (the 10° step rotation does not change the clone orientation); positions follow an arc: y = 0, 49.24, 96.23, 139.53, 177.83 with z = 0, -8.68, -25.78, -50.78, -82.92 |
| `cloner/linear-endpoint-sr` | Linear Mode “End Point” (1271 = 0) with count 5 and P (0, 200, 0) → positions (0, 0, 0) (0, 50, 0) (0, 100, 0) (0, 150, 0) (0, 200, 0), i.e. the per-step vector is divided by count - 1 and the last clone lands on the full vector; per-step S (0.5, 0.5, 0.5) and R (0, 90, 0) are interpolated from full to target, last clone y axis (0, 0, -0.5) |
| `cloner/radial-var-1` | Radial, Count 8 (1240), defaults otherwise (Radius 50 (1246), Start Angle 0 (1241), End Angle 360 (1242)) → 8 clones on a circle of radius 50 in the XZ plane at 45° steps: (0,0,50) (35.355,0,35.355) (50,0,0) (35.355,0,-35.355) (0,0,-50) (-35.355,0,-35.355) (-50,0,0) (-35.355,0,35.355). Offset Variation 50 % (1244 = 0.5, readback 0.5) and Offset Seed 1 (1245 = 1) leave the positions unchanged because `Offset` (1243) is 0 and the variation only scales that offset. Samples kind `clones`, 8 clones, all visible |
| `cloner/radial-var-2` | same as `radial-var-1` with Offset Seed 2 (1245 = 2) → positions identical to `radial-var-1`, for the same reason. Control, not a case: with Offset 30° (1243 = 0.5236) plus Variation 50 %, seed 1 and seed 2 do differ — clone 0 stays on the offset position while later clones fan out around it |
| `cloner/grid-cyl-fill` | Grid Array (mode 2010 = 3), Count (5,5,5) (1200), Form Cylinder (1203 = 2, readback 2), Fill 50 % (1202 = 0.5, readback 0.5) → 125 clones whose positions are unchanged from the cubic grid: 200 steps on each axis from (-400,-400,-400) to (400,400,400). So neither Form nor Fill moves or drops clones here; Form cycle reads 0 Cubic, 1 Sphere, 2 Cylinder, 3 Object |
| `cloner/grid-sphere-uneven` | Grid Array, Count (7,5,3), Size (300,200,100) (1201), Form Sphere (1203 = 1) → 105 clones (7×5×3) starting at (-900,-400,-100) with steps equal to the Size vector on each axis, i.e. Size sets the per-step spacing rather than an overall extent |
| `cloner/honey-orient-x` | Honeycomb (mode 2010 = 4), Orientation X (first cycle entry “Z (XY)”, then “X (ZY)”, then “Y (XZ)”) with the rest at defaults (width 10, height 10, size 300/300, Mode Per Step, Form Circle) → 50 clones lying in the x = 0 plane, first four (0,-779.423,-1350) (0,-433.013,-1350) (0,-86.603,-1350) (0,259.808,-1350); column steps 300, row steps 346.41 (= 300·2/√3), `honey-orient-y` puts them in the y = 0 plane instead |
| `cloner/honey-endpoint` | Honeycomb, Mode End Point (cycle reads Per Step, End Point), everything else default → 50 clones, positions identical to the default Per Step honeycomb because `Offset` is 0 and End Point only redistributes that offset |
| `cloner/honey-var` | Honeycomb, Offset Variation 50 % and Perpendicular Variation 50 % (both read back 0.5), Seed 1, defaults otherwise → 50 clones, positions identical to the default honeycomb, again because the variations scale an Offset that is 0 here |
| `cloner/honey-odd-even` | Honeycomb, width count 5, height count 4 (readback 5 and 4) → 10 clones, i.e. 5 columns × 2 rows per column, first four (-600,-259.808,0) (-600,86.603,0) (-300,-86.603,0) (-300,259.808,0): columns advance 300, rows 346.41, odd columns shifted by half a row (the height count counts half-steps) |
| `cloner/on-spline` | not observed: the cloner never produces clone data in Object mode from a script. Object mode reads back as selected, a Helix links into the cloner's Object slot and reads back as that object, the spline converts to 901 points and the cloner sits in the document, yet the cloner reports no clone data for Count 10 or for the vector counts (10, 1, 1) / (1, 1, 1) / (10, 10, 10) |
| `cloner/on-spline-step` | not observed: same Object-mode blocker as `cloner/on-spline` (no MoData is built for a linked spline in this build) || `cloner/reset-coords-off` | 3 clones, all at (0, 0, 0) with identity axes and colour (1, 1, 1), Reset Coordinates off (readback false); the child cube's own (0, 50, 0) position and H 30° rotation are not folded into the clone matrices, and the baked mesh is the plain 20 cm cube written three times — 24 position lines, 8 unique positions each repeated 3×, y range 36.340–63.660 |
| `clone-spline/simple` | mode readback 0 (Simple), all defaults: generator spline 101 points, bbox (0, 0, 0)–(0, 0, 200); extruded mesh 202 points / 100 polygons, bbox (0, 0, 0)–(0, 0, 300) |
| `clone-spline/turtle` | mode readback 3 (Turtle), all defaults: generator spline 79 points, bbox (-37.425, 0, -38.491)–(39.135, 72.885, 39.351); extruded mesh 158 points / 63 polygons, bbox (-37.425, 0, -38.491)–(39.135, 72.885, 139.351) |
| `clone-spline/spline-mode` | mode readback 1 (Spline) with a default Circle (5181) as source spline: generator spline 37 points, bbox (-200, -200, 0)–(200, 200, 0); extruded mesh 72 points / 36 polygons, bbox (-200, -200, 0)–(200, 200, 100) |
| `tracer/vertices` | trace spline 240 points = 8 chains × 30 samples, one per frame stepped (frames 1–30, none at 0); the first chain runs (-90, -100, -100) → (200, -100, -100), i.e. the cube centre from 0 to 290 with the corner offset −100, +10 in x per frame; extruded mesh 480 points / 232 polygons |
| `tracer/interval` | Sample Step 5 → trace spline 56 points = 8 chains × 7 samples at frames 1, 5, 10, 15, 20, 25, 30; first chain x = -90, -50, 0, 50, 100, 150, 200 at y = z = -100; extruded mesh 112 points / 48 polygons |
| `instance-time/defaults` | exactly 1 clone; MoData holds it at (0, 0, 0) with identity axes and colour (1, 1, 1), visible, and no MODATA_WEIGHT array is present (has_weight false), so no per-clone weight is recorded; the reference cube's keyed motion (x 0 → 300) does not shift the instance even at frame 30; baked mesh = the reference cube at (300, 0, 0) plus the instance at the origin, 16 points / 12 polygons in 2 objects |
| `effector-f/space-node` | 3 clones; positions (50, 0, 0) (50, 100, 0) (50, 100, -100); clone axes x (1, 0, 0) on all three, then y (0, 1, 0) / z (0, 0, 1), y (0, 0, -1) / z (0, 1, 0), y (0, -1, 0) / z (0, 0, -1); colour (1, 1, 1), weight 0 |
| `effector-f/space-object` | identical to `effector-f/space-node` clone for clone (same positions, axes, colour and weight) |
| `effector-f/space-effector` | identical to `effector-f/space-node` clone for clone: with the cloner and the effector both at the origin the three Transform Space modes give the same result |
| `effector-f/absolute-rs` | 5 clones at y = 0, 50, 100, 150, 200, every clone with axes x (1.4142, 0, 1.4142), y (0, 2, 0), z (-1.4142, 0, 1.4142), colour (1, 1, 1), weight 0 — Absolute mode replaces the clone transform with S 2 and the 45 deg Heading |
| `effector-f/abs-scale` | 5 clones at y = 0, 50, 100, 150, 200, every clone x (3, 0, 0) / y (0, 3, 0) / z (0, 0, 3) with Uniform Scale 2 and Absolute Scale on |
| `effector-f/abs-scale-off` | identical to `effector-f/abs-scale` (clone scale 3, not 2): the Absolute Scale toggle makes no observable difference at Uniform Scale 2 in this build |
| `effector-f/color-alpha` | 3 clones, colour (0.25, 0.375, 0) each: Custom Color (1, 0, 0) at Strength 50 % with Use Alpha/Strength on, over the cloner colour (0, 0.5, 0) |
| `effector-f/color-alpha-off` | 3 clones, colour (1, 0, 0) each: the same effector with Use Alpha/Strength off — the Strength percent is ignored without that toggle |
| `effector-f/blend-add` | 3 clones, colour (0.5, 0.5, 0) each: Blending Mode Add sums the Custom Color (0.5, 0, 0) and the cloner colour (0, 0.5, 0) channel by channel |
| `effector-f/blend-multiply` | 3 clones, colour (0, 0, 0) each: Blending Mode Multiply is the channel-wise product of (0.5, 0, 0) and (0, 0.5, 0) |
| `effector-f/order` | 3 clones at (0, 0, 100) (0, 50, 100) (0, 100, 100), every clone with axes x (0, 0, 1), y (0, 1, 0), z (-1, 0, 0): with A (Rotation, R (90, 0, 0)) created before B (P (100, 0, 0)) the rotation reaches every clone and B's 100 unit translation lands on z instead of x |
| `effector-f/remap-minmax` | 5 clones at (0, 40, 0) (0, 90, 0) (0, 140, 0) (0, 190, 0) (0, 240, 0): with Transform Mode Remap, Minimum 20 % and Maximum 80 % move every clone 40 in Y (not 20 and not 80) on top of the cloner's 0/50/100/150/200 grid. Clone colour (0.8645, 0.6609, 0.5094) is the field's default Color at Color Mode Fields Color. The Linear field's full parameter table is in `case.toml` |
| `effector-f/visibility` | 5 clones, 4 visible: the clone at y = 50 is hidden, the ones at y = 125, 200, 250, 300 are visible. With Visibility on the positions leave the cloner's 0/50/100/150/200 grid, and the colours fall from (0.8673, 0.8357, 0.8529) to (0.8009, 0.7535, 0.7794) along the field's Y+ direction. Field table in `case.toml` |
| `effector-f/order-swapped` | 3 clones at (100, 0, 0) (100, 50, 0) (100, 100, 0), every clone with identity axes: with B (P (100, 0, 0)) created before A (Rotation R (90, 0, 0)) the translation reaches the clones and the rotation does not. `effector-f/order` (A first) gives (0, 0, 100) (0, 50, 100) (0, 100, 100) with x (0, 0, 1) / z (-1, 0, 0), so creation order decides which effector wins |
| `effector-f/per-effector-50` | 5 clones at y = 50, 100, 150, 200, 250: the cloner carries one strength row per effector, labelled with the effector's name (`Plain`, 50 % in `case.toml`), and at 50 % the effector's P (0, 100, 0) adds 50 instead of 100 on top of the cloner grid |
| `effector-f/linear-plain-field` | 10 clones at y = 50, 100, … 500 (step 50, the cloner's P (0, 50, 0) plus the field). Every clone colour reads (0.9531, 0.7876, 0.7095), i.e. the Linear field's Color (0.66834, 0.547404, 0.666661) at Color Mode Fields Color. `case.toml` now records the whole field table: Type, Length, Direction X+, Clip to Shape, Enable Remapping, Strength 100 %, Invert, Inner Offset, Min 0 % / Max 100 % with both clamps on, Multiplier 100 %, Contour Mode None, Color Mode Color, Color, Scale 100 %, Invert Direction, Invert Rotations |
| `random-f/count50-s12345` | 50 clones; first clones (-10.292, -48.28, 28.126) (34.711, -26.711, 13.085) (-0.877, -49.093, 26.89), last (-22.934, 38.413, 1024.278) — the cloner's P (0, 0, 20) plus an offset inside the effector's default P (50, 50, 50). Seed 0 gives a first clone of (-49.997, -27.852, 2.616), seed 1 (-49.994, -5.707, 33.084), and Strength 50 % halves every offset (first clone (-5.146, -24.14, 14.063)) |
| `random-f/indexed-50` | 50 clones; the first three read (-34.233, -49.97, 11.036) (-16.36, -44.442, -26.298) (-30.491, -49.583, 4.41), the same as `random-f/indexed-5` and `indexed-8` at counts 5 and 8: with Indexed on the offset follows the clone index rather than the count |
| `random-f/gaussian-50` | 50 clones; first clones (-1.99, -0.424, 3.201) (13.127, 12.68, 20.278) (6.301, 10.671, 29.855), last (21.634, 0.336, 976.443). Gaussian mode scatters much closer to the cloner path than Random mode, whose first clone sits at (-10.292, -48.28, 28.126) |
| `random-f/noise-steps` | 20 clones; Noise mode gives first clones (-15.83, -16.27, -16.302) (-14.893, -15.908, -6.686) (-14.573, -16.037, 2.627), last (-12.977, -12.302, 178.116). Scale 50 % changes every clone except the first (second clone (-14.573, -16.037, -7.373)); Scale 200 % likewise ((-15.296, -16.036, -6.46)); Animation Speed 0 % read at frame 30 reproduces the defaults clone for clone; Space UV replaces the pattern (first clone (11.923, 8.295, 3.856)) |
| `random-f/turbulence` | 20 clones; first clones (6.726, -1.403, -4.908) (7.159, 2.286, 6.731) (8.102, 7.466, 22.97), last (-11.659, -12.319, 177.263): a smooth drift along the cloner path instead of the Noise mode pattern |
| `random-f/synchronized` | 20 clones; first clones (-10.292, -48.28, 28.126) (6.977, -29.079, 47.818) (42.07, 16.936, -21.513), last (43.807, 27.085, 232.944): Synchronized on keeps the default seed's first clone but spreads the rest differently |
| `random-f/sorted` | 20 clones; first clones (44.737, 44.737, 44.737) (-34.211, -34.211, -24.211) (-28.947, -28.947, -8.947), last (-23.684, -23.684, 166.316): in Sorted mode all three offset components are equal on every clone, so every offset lies on the diagonal |
| `random-f/rot-h` | 20 clones; the positions keep the cloner's P (0, 0, 10) untouched and every clone is turned about Y by a different angle (clone 1 x axis (0.2466, 0, 0.9691), clone 2 (0.9171, 0, -0.3987), clone 3 (0.9062, 0, 0.4229), 20 distinct x axes), y staying (0, 1, 0). The R (90, 0, 0) deg entered in the effector's Rotation row does not turn the clones about X; the probe reads the rotation back in radians and the case records the 90° the Attribute Manager shows |

## Notes
- Blend order: 5 clones of a cube (size 20) and a sphere (r 10) in Linear mode at the default (0, 50, 0) step bake to a 358-point / 396-polygon mesh; clone weights read 0 because no weight tag is present, as in the O8 cloner cases.
- Clone selection and clone weights are not scriptable in this build. The scripted selection and weight values land in the live clone data (the matching readers report clones 2 and 4 selected, and weights 0, 0.25, 0.5, 0.75, 1), and the effector accepts the selection tag in its Selection field, but evaluation still treats every clone alike: the Plain effector offsets all five clones by the full (0, 100, 0), selection or weight or not. Rows `cloner/selection-tag` and `cloner/weight-tag` therefore stay not observed; they need a saved scene built in the GUI.
- Linear mode reads from the Attribute Manager as: Count, Offset (a checkbox), Mode (Per Step / End Point), Amount, P, S, R, Step Mode, Step Size, Step Rotation. The cloner Transform tab (P, S, R, Color, Weight) is applied to every clone on top of the per-step transform (see `cloner/transform-rs`).
- Offset in this build is a checkbox only: switching it on leaves the parameter list unchanged and adds one clone beyond Count instead of shifting the pattern.
- Object mode: the cloner's Object slot accepts a Helix spline, the link reads back as that object, but the cloner reports no clone data at all, so the on-spline cases are recorded as not observed.
- Offset in Object mode: the cloner's count turns into a vector when Object mode is on, and writing 10 or (10, 1, 1) / (1, 1, 1) / (10, 10, 10) makes no difference to that missing clone data.
- The clone-spline generator's Mode cycle reads Simple / Spline / Turtle; its source spline slot only matters in Spline mode, and Simple and Turtle generate their spline with no source at all.
- Tracer sampling: the trace is built from the frames the document is actually stepped through. Stepping frames 0–30 one frame at a time gives Sample Step 1 → 30 samples per traced vertex and Sample Step 5 → 7 samples (frames 1, 5, …, 30). Jumping straight to frame 30 leaves a degenerate 2-point spline with both points at the frame-30 position, so the frame-by-frame step is part of the case.
- The Tracer's tracing mode does not read back (the getter returns nothing for it), so the mode is left at its default and the case records `trace_mode = "default"` rather than naming a mode.
- The instance-over-time generator in Object mode with a Cube linked produces exactly one clone, and that clone's matrix stays identity (no time offset) no matter where the referenced object is on its keyframe track.
- Extrude results can carry child objects of their own; exporting the result without stripping them writes extra objects into `mesh.obj` (it produced 144 position lines / 3 objects instead of 72 / 1 for `clone-spline/spline-mode`). All meshes recorded here were exported after removing the children.

### Effector field (O50 follow-up, plain effector)

- The effector's Attribute Manager at depth 1 reads: Strength, Selection (a link), P, S, R (R is an angle, its three components are Heading / Pitch / Bank, not X / Y / Z), Position toggle, Rotation toggle, Weight Transform, Minimum, Maximum.
- Cycles as the app shows them: Transform Mode Relative / Absolute / Remap; Transform Space Node / Effector / Object; Color Mode Off / Custom Color / Effector Color / Fields Color; Blending Mode Default / Add / Subtract / Multiply / Divide.
- R is stored in radians. Writing 90 as a plain number turns the clones by 90 radians: after the 2 pi wrap the axes read y (0, -0.4481, -0.8940), z (0, 0.8940, -0.4481), i.e. 2.0354 rad. Writing the radian value of 90° gives a clean quarter turn (y (0, 1, 0), z (-1, 0, 0)). A value in the *first* component turns the clones about Y: `effector-f/absolute-rs` writes 45° there and the clone axes come out x (1.4142, 0, 1.4142), y (0, 2, 0), z (-1.4142, 0, 1.4142).
- Absolute Scale has no observable effect here: `effector-f/abs-scale` (toggle on) and `abs-scale-off` (toggle off) both scale every clone by 3, i.e. 1 + Uniform Scale 2, not to 2.
- Colour reaches the clones through the Color Mode cycle plus the blending cycle: Custom Color (1, 0, 0) at Strength 50 % with Use Alpha/Strength on gives (0.25, 0.375, 0) against a cloner colour of (0, 0.5, 0), while the same effector with the toggle off gives exactly (1, 0, 0). Add sums effector and clone colour (0.5, 0, 0) + (0, 0.5, 0) = (0.5, 0.5, 0); Multiply gives (0, 0, 0).
- With two effectors the creation order is what applies: A (Rotation) before B (P (100, 0, 0)) turns every clone and sends B's translation to z, giving (0, 0, 100) (0, 50, 100) (0, 100, 100); created the other way round the translation applies and the rotation does not (see `effector-f/order-swapped`).
- A directory of cases under `tests/golden/cloner/effector-f/` makes `golden -- check` fail with `cloner/effector-f: missing case.toml` — the harness reads that path as a case, not as a parent. The effector cases therefore live at `tests/golden/effector-f/<case>/` with `feature = "effector-f"`, and the random-effector cases at `tests/golden/random-f/<case>/`.
- Bridge timeline for the interrupted segment (2026-11-20, UTC): two effector probes each ended with a 120 s bridge timeout; a one-line ping then succeeded three times, but three health checks one minute apart around 13:30 Z all returned "Not connected to Cinema 4D" while the app's process sat at 99 % CPU with the bridge port still listening. The app was neither restarted nor killed (rule), and that run stopped there. Only cases exported before that point were recorded; nothing in them is estimated. The owner restarted the app and this window resumed.

### O50 finished (effector-f and random-f)

The bridge answered the first ping of this window and stayed responsive through 21 new cases.

A field joins an effector's field list as soon as it is created while the effector is the active object (the field then reads back as one entry in that list). The earlier note that a field cannot be linked from a script was wrong: what does not exist is a list-insert call, but the app's own menu path works.

Findings, effector-f: Remap with Minimum 20 % / Maximum 80 % adds 40 of the effector's 100 in Y, not the 20 or 80 the two fields name. Visibility on hides only the clone at the far end of the field and moves the remaining clones off the cloner grid. The per-effector strength row lives on the cloner, labelled with the effector's name, and scales that effector's offset linearly. Creation order decides which of two effectors applies, exactly as the swapped pair shows.

Findings, random-f: Indexed makes the offset depend on the clone index only, so counts 5, 8 and 50 start with the same three clones. Gaussian scatters far less than Random. In Noise mode Scale changes every clone but the first, Animation Speed 0 % at frame 30 reproduces the defaults clone for clone, and Space UV gives a different pattern again. Sorted keeps all three offset components equal, so every offset lies on the diagonal. With Position off and Rotation on, R (90, 0, 0) leaves the positions untouched and turns each clone about Y, not about X.

Cleanroom: `cargo run -q -p forge-cleanroom` still reports violations in older material (`clone-text`, `matrix`, `clone-spline`, and the effector-f cases of the interrupted window, whose notes carried parameter numbers). Those cases are existing golden cases and were left untouched; the 21 cases added here name repeated parameters by meaning in `case.toml` and are clean, and this Notes section no longer carries parameter numbers or scripting names.
