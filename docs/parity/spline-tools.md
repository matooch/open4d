# Spline tools and editable splines — observation (O46)

Status: done · App version: R25 (25010) · OS: macOS (Darwin 25.6.0) · Observer: Claude (unattended O46 session) · Date: 2026-11-04
Method: scripted. One modeling command per probe, then a `ping` (QUEUE.md rule 11). No
dialog commands.

Measure every result through an Extrude, exactly as [splines-2.md](splines-2.md): Offset
20, caps off.

Output: `tests/golden/spline-tools/<case>/mesh.obj`, with `feature = "spline-tools"`.
`case.toml` holds the input splines' parameters under `[params.input]` and
`[params.input2]`, the command, and its settings.

Unblocks: B/028, 029, 030; C/041.

## Editable spline facts (record in Notes)

For a default Circle and a default Rectangle, each Made Editable, record:
- point count, interpolation type, closed flag;
- every point's position;
- every point's left and right tangent.

Also export each as a case (`editable-circle`, `editable-rectangle`).

## Cases

| Case | Input | Command and settings |
|---|---|---|
| editable-circle | default Circle | Make Editable (only) |
| editable-rectangle | default Rectangle | Make Editable |
| interp-linear | editable circle | interpolation set to Linear |
| interp-akima | editable circle | Akima |
| interp-bspline | editable circle | B-Spline |
| interp-bezier | editable circle | Bezier |
| chamfer | editable rectangle, all points selected | Chamfer, radius 40 |
| outline | editable circle | Create Outline, distance 20 |
| outline-neg | editable circle | Create Outline, distance −20 |
| union | two editable circles, the second at x 150 | Spline Union |
| subtract | the same | Spline Subtract (A minus B) |
| intersect | the same | Spline Intersect |
| and | the same | Spline And |
| or | the same | Spline Or |
| reverse | editable rectangle | Reverse Sequence |
| subdivide | editable rectangle | Subdivide, 2 subdivisions |
| cross-section | three editable circles at y 0, 100, 200, selected in order | Cross Section (if scriptable without a dialog) |
| interp-f/none | follow-up from brief 038: a default Circle, Intermediate Points None, extruded (Offset 100) | – |
| interp-f/natural | the same, Natural with Number 8; then `natural-3` with Number 3 | – |
| interp-f/subdivided | the same, Subdivided with Angle 5° and Maximum Length 5; then `-a20` (Angle 20°) and `-l50` (Maximum Length 50) | – |
| interp-f/uniform-small | the same, Uniform with Number 3; then `uniform-0` with Number 0 | – |
| interp-f/adaptive-angle | the same, Adaptive with Angle 10°; then `-a20` with 20° | – |
| interp-f/rect-uniform | a default Rectangle, Uniform 8, extruded | – |
| interp-f/star-uniform | a default Star, Uniform 4, extruded (unequal segments) | – |
| interp-f/helix-adaptive | a default Helix, Adaptive, swept with a small Circle (radius 5); then `-sub50` (Subdivision 50), `-end360` and `-end1080` (End Angle), `-bias` (Radial Bias 50 %, Height Bias 50 %) | – |
| interp-f/helix-handles | a default Helix made editable: record the tangent (handle) vectors of points 0, 1 and 100 in Notes | form only |
| prim-f/cog-tooth | follow-up from brief 037: a default Cogwheel made editable; record every point and tangent of the first tooth in Notes (or export the editable spline's points), then `cog-ratio` with the tooth ratios changed (record which) | form + mesh |
| prim-f/flower | Flower with inner 50, outer 150, 5 petals, extruded | – |
| prim-f/cissoid-t | Cissoid with tension 1 and then 4, extruded | – |
| prim-f/cycloid | Cycloid with end angle 720°, and then radius 50, r 25, extruded | – |
| prim-f/arc-types | Arc in each of its other types (record the type labels), extruded | – |
| prim-f/four-side | 4-Side as parallelogram and as trapezium | – |
| prim-f/profile | Profile as L, T, U and Z | – |
| prim-f/cycloid-types | Cycloid as hypocycloid; and the Cissoid types Lemniscate and Strophoid | – |
| prim-f/reverse | a default Star with Reverse on, extruded | – |
| prim-f/direction-auto | a clockwise closed spline (Star with Reverse) extruded with Direction Auto; and a Circle in the XZ plane, Direction Auto | – |

## Notes

### Editable spline facts (default Circle and Rectangle, each Made Editable)

Read through the app's public point/tangent access after Make Editable; raw values of the
editable spline's Type / Intermediate Points enums are given as index numbers.

- **Circle** (editable): 4 points, closed, one segment, Type raw 4 (Bezier, the editable
  default), Intermediate Points raw 2 (Uniform). Positions in order:
  (200, 0, 0), (0, 200, 0), (−200, 0, 0), (0, −200, 0). Left/right handle vectors
  (same length ±83):
  | point | left handle | right handle |
  |---|---|---|
  | 0 (200, 0, 0) | (0, −83, 0) | (0, 83, 0) |
  | 1 (0, 200, 0) | (83, 0, 0) | (−83, 0, 0) |
  | 2 (−200, 0, 0) | (0, 83, 0) | (0, −83, 0) |
  | 3 (0, −200, 0) | (−83, 0, 0) | (83, 0, 0) |
- **Rectangle** (editable): 4 points, closed, one segment, Type raw 0 (Linear — a hard-edge
  editable spline), Intermediate Points raw 3 (Adaptive). No tangent handles at all (the
  tangent read returns zero handles). Positions in order: (200, 200, 0), (−200, 200, 0),
  (−200, −200, 0), (200, −200, 0).
  A default Circle made editable therefore arrives as a Bezier spline with handles, a
  default Rectangle as a Linear spline without them.

### interp-f/helix-handles (form only)

Default Helix made editable: 101 points (Subdivision 100), open, Type raw 4 (Bezier),
Intermediate Points raw 2 (Uniform). Positions and handle vectors of points 0, 1 and 100:

| point | position | left handle | right handle |
|---|---|---|---|
| 0 | (200, 0, 0) | (−0, −6.278974, 0.500982) | (0, 6.278974, −0.500982) |
| 1 | (198.422940, 25.066647, −2) | (0.786964, −6.229462, 0.500982) | (−0.786964, 6.229462, −0.500982) |
| 100 | (200, 0, −200) | (−0, −6.278974, 0.500982) | (0, 6.278974, −0.500982) |

(The sweep cases under `interp-f-helix-adaptive*` pin the swept geometry.)

### prim-f/cog-tooth — editable Cogwheel structure

Default Cogwheel (20 teeth, see `prim-f-cog-tooth/case.toml` for all input parameters) made
editable: 244 points, closed, **2 segments**: the outer gear outline (240 points, 12 per
tooth) followed by the optional centre hole at the end of the list (4 points, radius 15:
(15, 0, 0), (0, −15, 0), (−15, 0, 0), (0, 15, 0)). Type raw 4 (Bezier), Intermediate Points
raw 3 (Adaptive). First tooth = points 0–11. Positions and handle vectors:

| pt | position | left handle | right handle |
|---|---|---|---|
| 0 | (179.728974, −9.873994, 0) | (−0.058397, −1.062963, 0) | (0.168696, 3.037410, 0) |
| 1 | (187.479175, −2.794469, 0) | (−3.040238, 0.106165, 0) | (0.773157, 0.063786, 0) |
| 2 | (190.571804, −2.539326, 0) | (−0.773157, −0.063786, 0) | (1.695476, 0.253842, 0) |
| 3 | (201.623325, 0.614709, 0) | (−3.739623, −1.465900, 0) | (3.739623, 1.465900, 0) |
| 4 | (219.763495, 10.329676, 0) | (−5.239800, −3.534309, 0) | (−0.081528, 1.734514, 0) |
| 5 | (219.321813, 17.261001, 0) | (0.136239, −1.731076, 0) | (−0.136239, 1.731076, 0) |
| 6 | (218.673759, 24.176085, 0) | (0.190813, −1.725913, 0) | (−5.728177, 2.671110, 0) |
| 7 | (199.237169, 30.933697, 0) | (3.922899, −0.862846, 0) | (−3.922899, 0.862846, 0) |
| 8 | (187.828310, 32.320061, 0) | (1.714312, 0.014514, 0) | (−0.773617, −0.057948, 0) |
| 9 | (184.733844, 32.088268, 0) | (0.773617, 0.057948, 0) | (−2.986199, −0.580456, 0) |
| 10 | (175.971580, 37.868235, 0) | (0.641775, −2.973624, 0) | (−0.223962, 1.040741, 0) |
| 11 | (175.026586, 42.020165, 0) | (0.248518, −1.035152, 0) | (−0.248518, 1.035152, 0) |

All z = 0. `prim-f-cog-ratio` changes the tooth Radial Ratio to 3 (default 1.5).

### prim-f/arc-types — Type labels

The Arc Type enum shows, in order: **Arc, Sector, Segment, Ring** (raw 0–3). The other
three are exported as cases (`prim-f-arc-sector/-segment/-ring`; Ring produces two
segments); the default Arc itself is already pinned by [splines-2.md](splines-2.md).
Readback labels used by other enums observed here: Cycloid = Cycloid / Epicycloid /
Hypocycloid; Cissoid = Cissoid / Lemniscate / Strophoid.

### Other observations

- **Cissoid Tension clamps upward**: 4 was accepted, but a set of 1 reads back as **1.5**
  (see `prim-f-cissoid-t1`), so the effective minimum is 1.5.
- **Uniform with Number 0** collapses the spline to its control points (the extruded
  `interp-f-uniform-0` mesh has only 8 vertices).
- **Boolean commands order**: for Subtract we got A-at-origin minus overlap with B (at
  x 150) selected first and A added second; the recipe is pinned in the case commands.
- **cross-section**: skipped. The Cross Section tool is not reachable from the scripting
  surface: it has no modeling-command entry and is not registered as a menu command, so
  driving it without a dialog would need a raw tool id. Needs the GUI (O10 territory).
