# Spline tools and editable splines — observation (O46)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
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

## Notes
TODO
