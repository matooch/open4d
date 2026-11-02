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

## Notes
TODO
