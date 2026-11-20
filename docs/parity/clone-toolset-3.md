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
| `cloner/selection-tag` | not observed: the scripted clone selection never reaches the effector. `c4d.modules.mograph.GeSetMoDataSelection` on the MoGraph Selection tag (1021338) reads back correctly (clones 2 and 4 selected, and `GeGetMoDataSelection` on the cloner reports the same array), and the Plain effector's Selection field (param 1021) takes the tag as a link (`GetParameter` returns `MoGraph Selection`), yet the effector still offsets all five clones by (0, 100, 0) (3 attempts, see Notes) |
| `cloner/weight-tag` | not observed: `c4d.modules.mograph.GeSetMoDataWeights` on the MoGraph Weightmap tag (440000231) reads back 0, 0.25, 0.5, 0.75, 1, but the Plain effector with Weight Transform on offsets every clone by the full (0, 100, 0) instead of scaling by the weight, so the scripted weights do not reach the stored tag data (see Notes) |
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
| `cloner/on-spline` | not observed: the cloner never produces MoData in Object mode from a script. `cl[2010] = 0` reads back 0 and the Object-mode description exposes `1200` Count (DTYPE_VECTOR), `1201` Size, `1202` Fill, `1203` Form, `1204` Mode, `1205` Object (DTYPE_BASEOBJECT); a Helix (object ID 5185, `GetTypeName()` = “Helix”) links into `1205` and reads back as that object, the spline converts to 901 points and the cloner sits in the document, yet `c4d.modules.mograph.GeGetMoData(cloner)` returns None for Count written as 10 and as Vector(10, 1, 1) / (1, 1, 1) / (10, 10, 10) |
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
| `effector-f/remap-minmax` | not observed: a field cannot be linked from a script — `c4d.FieldList` has no `InsertObject` (marker `remap` in /Users/bigman/forge-observer/probes/o50_b83.json) — and the app stopped answering before a fallback was tried. See /Users/bigman/forge-observer/log.md |
| `effector-f/visibility` | not observed: the reference app stopped answering the MCP bridge (execute_python timed out after 120 s, then three health checks one minute apart failed to get an answer; see /Users/bigman/forge-observer/log.md) |
| `effector-f/order-swapped` | not observed: same app stop; only the A-then-B order was exported as `effector-f/order` |
| `effector-f/per-effector-50` | not observed: same app stop, hit while the per-effector strength container was being probed |
| `effector-f/linear-plain-field` | not observed: same app stop; the existing `cloner/linear-plain-field` case was left untouched (rule: never edit existing golden cases) |
| `random-f/count50-s12345` | not observed: the app stopped answering the bridge before the `random-f` family was started |
| `random-f/indexed-50` | not observed: the app stopped answering the bridge before the `random-f` family was started |
| `random-f/gaussian-50` | not observed: the app stopped answering the bridge before the `random-f` family was started |
| `random-f/noise-steps` | not observed: the app stopped answering the bridge before the `random-f` family was started |
| `random-f/turbulence` | not observed: the app stopped answering the bridge before the `random-f` family was started |
| `random-f/synchronized` | not observed: the app stopped answering the bridge before the `random-f` family was started |
| `random-f/sorted` | not observed: the app stopped answering the bridge before the `random-f` family was started |
| `random-f/rot-h` | not observed: the app stopped answering the bridge before the `random-f` family was started |

## Notes
- Blend order: 5 clones of a cube (size 20) and a sphere (r 10) in Linear mode at the default (0, 50, 0) step bake to a 358-point / 396-polygon mesh; clone weights read 0 because no weight tag is present, as in the O8 cloner cases.
- Clone selection and clone weights are not scriptable through the Python API in this build. `GeSetMoDataSelection` and `GeSetMoDataWeights` update only the live MoGraph data: the corresponding getters read the values back, the cloner's own MoData reports the same selection array, and the Plain effector accepts the selection tag in its Selection field (1021), but the effector evaluation still treats every clone as selected and weight 1, so the clones are all offset. Neither tag exposes a writable per-clone array through its description (the Selection tag offers only Clear, 1000, and the Fields list, 1005; the Weightmap tag is the same shape), and painting the tag or filling its Fields list is a GUI action. Both cases are recorded as not observed rather than filing an unrestricted effector as the restricted case.
- Linear mode parameter map read from the description (depth 1): `1270` Count, `1279` Offset (checkbox), `1271` Mode (1 = Per Step, 0 = End Point), `1272` Amount, `1273` P, `1275` S, `1274` R, `1276` Step Mode, `1278` Step Size, `1277` Step Rotation. The cloner Transform tab is `5501` P, `5502` S, `5503` R, `5504` Color, `5505` Weight, and it is applied to every clone on top of the per-step transform (see `cloner/transform-rs`).
- Offset in this build is a checkbox only: toggling `1279` leaves the parameter list unchanged and adds one clone beyond Count instead of shifting the pattern.
- Object mode: the Object link param is `1205` and it accepts a Helix spline, but `GeGetMoData` stays None, so the on-spline cases are recorded as not observed.
- Effector parameter map (depth 1): `1000` Strength, `1010` P (offset), `1060` Position toggle, `1064` Weight Transform; the clone “Selection” link is `1021`.
- MoSpline (plugin 440000054) mode parameter is `1145`: the cycle reads Simple (0) / Spline (1) / Turtle (3); the source spline link is `1123` and Simple/Turtle generate their spline with no source at all.
- Tracer sampling: the trace is built from the frames the document is actually stepped through. Stepping frames 0–30 with `SetTime` + `ExecutePasses` gives Sample Step 1 → 30 samples per traced vertex and Sample Step 5 → 7 samples (frames 1, 5, …, 30). Jumping straight to frame 30 leaves a degenerate 2-point spline with both points at the frame-30 position, so the frame-by-frame step is part of the case.
- Tracer tracing-mode parameter `1110` does not read back through the Python API (`GetParameter` returns None for it), so the mode is left at its default and the case records `trace_mode = "default"` rather than naming a mode.
- MoInstance (plugin 1018957) reports `GetName()` = "MoInstance": in Object mode with a Cube linked it produces exactly one clone, and that clone's matrix stays identity (no time offset) no matter where the referenced object is on its keyframe track.
- Extrude results (5116 + `MCOMMAND_CURRENTSTATETOOBJECT`) can carry child objects of their own; exporting the result without stripping them writes extra objects into `mesh.obj` (it produced 144 position lines / 3 objects instead of 72 / 1 for `clone-spline/spline-mode`). All meshes recorded here were exported after removing the children.

### Effector field (O50 follow-up, plain effector 1021337)

- Description read at depth 1: Strength `1000`, Selection link `1021`, P `1010`, S `1011`, R `1012` (each vector with its own unit id — 1718445428, 1718773089, 1717856114 — so R is an angle unit, not a distance), Position toggle `1060`, Rotation toggle `1062`, Weight Transform `1064`, Minimum `1001`, Maximum `1002`.
- Cycles as stored (id = index): Transform Mode Relative 0 / Absolute 1 / Remap 2; Transform Space Node 0 / Effector 1 / Object 2; Color Mode Off 0 / Custom Color 1 / Effector Color 3 / Fields Color 5; Blending Mode Default 0 / Add 1 / Subtract 2 / Multiply 3 / Divide 4.
- The R vector is stored in radians, and its three components act as Heading / Pitch / Bank, not as X / Y / Z. Writing `Vector(0, 90, 0)` into `1012` turns the clones by 90 *radians*: after the 2 pi wrap the axes read y (0, -0.4481, -0.8940), z (0, 0.8940, -0.4481), i.e. 2.0354 rad. Writing `Vector(0, 1.5708, 0)` gives a clean 90 deg (y (0, 1, 0), z (-1, 0, 0)). A value in the *first* component turns the clones about Y: `effector-f/absolute-rs` writes 45 deg there and the clone axes come out x (1.4142, 0, 1.4142), y (0, 2, 0), z (-1.4142, 0, 1.4142).
- Absolute Scale has no observable effect here: `effector-f/abs-scale` (toggle on) and `abs-scale-off` (toggle off) both scale every clone by 3, i.e. 1 + Uniform Scale 2, not to 2.
- Colour reaches the clones through the Color Mode cycle plus the blending cycle: Custom Color (1, 0, 0) at Strength 50 % with Use Alpha/Strength on gives (0.25, 0.375, 0) against a cloner colour of (0, 0.5, 0), while the same effector with the toggle off gives exactly (1, 0, 0). Add sums effector and clone colour (0.5, 0, 0) + (0, 0.5, 0) = (0.5, 0.5, 0); Multiply gives (0, 0, 0).
- With two effectors the creation order is what applies: A (Rotation) before B (P (100, 0, 0)) turns every clone and sends B's translation to z, giving (0, 0, 100) (0, 50, 100) (0, 100, 100).
- A directory of cases under `tests/golden/cloner/effector-f/` makes `golden -- check` fail with `cloner/effector-f: missing case.toml` — the harness reads that path as a case, not as a parent. The effector cases therefore live at `tests/golden/effector-f/<case>/` with `feature = "effector-f"`.
