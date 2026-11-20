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

## Follow-up cases (planner, after briefs 028 and 029)

Briefs 028 and 029 matched the cases above except Create Outline's offset (the export is the
circle scaled by 1.100013 for distance 20, not a plain offset), and left the rows below open.
Same export procedure as above.

| Case | Input | Command and settings |
|---|---|---|
| outline-r100-d10 | editable circle, radius 100 | Create Outline, distance 10 |
| outline-rect | editable rectangle | Create Outline, distance 20 |
| outline-arc-open | editable default Arc (open) | Create Outline, distance 20 |
| subdivide-3 | editable rectangle, all points selected | Subdivide, subdivisions 3 |
| chamfer-flat | editable rectangle, all points selected | Chamfer, radius 40, Flat on |
| chamfer-star | editable default Star, all points selected | Chamfer, radius 10 |
| bool-circle-rect | an editable circle and an editable rectangle 300 × 100, same centre | Spline Union |
| bool-nested | an editable circle (radius 200) and one of radius 100, same centre | Spline Subtract |
| bool-disjoint | two editable circles at x −300 and x 300 | Spline Union |
| interp-cubic-natural | editable rectangle with point 0 moved by (50, 50, 0) | Type Cubic, Intermediate Points Natural, Number 8 |
| interp-akima-natural | the same | Type Akima, Intermediate Points Natural, Number 8 |
| interp-bspline-natural | the same | Type B-Spline, Intermediate Points Natural, Number 8 |
| tangent-readback-moved | editable circle with point 0 moved by (50, 0, 0) | none; record every point's tangent read-back |
| editable-bezier-moved-extrude | editable circle, point 0 moved by (50, 0, 0) and its right handle scaled ×2 | Extrude, movement (0, 0, 20) |
| editable-ring | default Ring made editable | none; record segments and closed flags |

Cross Section is GUI-only: answer it in GUI session G3 if possible.

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

- Batch-03 `outline-r100-d10`: Observed; vertices: 144 (unused 0); faces: 72 (4-gons 72); bbox min: (-110.001303, -110.001303, -20); bbox max: (110.001303, 110.001303, 0); size: (220.002606, 220.002606, 20); centre: (0, 0, -10); normals: 68; uvs: 74; complete measured parameters in case.toml.

- Batch-03 `outline-rect`: Observed; vertices: 16 (unused 0); faces: 8 (4-gons 8); bbox min: (-220, -220, -20); bbox max: (220, 220, 0); size: (440, 440, 20); centre: (0, 0, -10); normals: 4; uvs: 10; complete measured parameters in case.toml.

- Batch-03 `outline-arc-open`: Observed; vertices: 180 (unused 0); faces: 90 (4-gons 90); bbox min: (0.01528, 0, -20); bbox max: (219.979126, 219.99987, 0); size: (219.96384600000002, 219.99987, 20); centre: (109.997203, 109.999935, -10); normals: 92; uvs: 182; complete measured parameters in case.toml.

- Batch-03 `subdivide-3`: Observed; vertices: 24 (unused 0); faces: 12 (4-gons 12); bbox min: (-200, -200, 0); bbox max: (200, 200, 20); size: (400, 400, 20); centre: (0, 0, 10); normals: 4; uvs: 26; complete measured parameters in case.toml.

- Batch-03 `chamfer-flat`: Observed; vertices: 16 (unused 0); faces: 8 (4-gons 8); bbox min: (-200, -200, 0); bbox max: (200, 200, 20); size: (400, 400, 20); centre: (0, 0, 10); normals: 8; uvs: 18; complete measured parameters in case.toml.

- Batch-03 `chamfer-star`: Observed; vertices: 640 (unused 0); faces: 320 (4-gons 320); bbox min: (-178.280665, -178.280665, 0); bbox max: (178.280665, 178.280665, 20); size: (356.56133, 356.56133, 20); centre: (0, 0, 10); normals: 336; uvs: 642; complete measured parameters in case.toml.

- Batch-03 `bool-circle-rect`: Observed; vertices: 136 (unused 0); faces: 68 (4-gons 68); bbox min: (-200, -200, 0); bbox max: (200, 200, 20); size: (400, 400, 20); centre: (0, 0, 10); normals: 68; uvs: 138; complete measured parameters in case.toml.

- Batch-03 `bool-nested`: Observed; vertices: 272 (unused 0); faces: 136 (4-gons 136); bbox min: (-200, -200, 0); bbox max: (200, 200, 20); size: (400, 400, 20); centre: (0, 0, 10); normals: 68; uvs: 138; complete measured parameters in case.toml.

- Batch-03 `bool-disjoint`: Observed; vertices: 272 (unused 0); faces: 136 (4-gons 136); bbox min: (-500, -200, 0); bbox max: (500, 200, 20); size: (1000, 400, 20); centre: (0, 0, 10); normals: 116; uvs: 138; complete measured parameters in case.toml.

- Batch-03 `interp-cubic-natural`: Observed; vertices: 72 (unused 0); faces: 36 (4-gons 36); bbox min: (-279.218107, -279.218107, 0); bbox max: (307.33882, 307.33882, 20); size: (586.556927, 586.556927, 20); centre: (14.060356500000012, 14.060356500000012, 10); normals: 36; uvs: 74; complete measured parameters in case.toml.

- Batch-03 `interp-akima-natural`: Observed; vertices: 72 (unused 0); faces: 36 (4-gons 36); bbox min: (-252.812071, -252.812071, 0); bbox max: (280.144033, 280.144033, 20); size: (532.956104, 532.956104, 20); centre: (13.665980999999988, 13.665980999999988, 10); normals: 36; uvs: 74; complete measured parameters in case.toml.

- Batch-03 `interp-bspline-natural`: Observed; vertices: 72 (unused 0); faces: 36 (4-gons 36); bbox min: (-199.382716, -199.382716, 0); bbox max: (227.006173, 227.006173, 20); size: (426.38888899999995, 426.38888899999995, 20); centre: (13.811728500000001, 13.811728500000001, 10); normals: 36; uvs: 74; complete measured parameters in case.toml.

- Batch-03 `tangent-readback-moved`: Observed; vertices: 72 (unused 0); faces: 36 (4-gons 36); bbox min: (-200, -199.268033, 0); bbox max: (250, 199.268033, 20); size: (450, 398.536066, 20); centre: (25, 0, 10); normals: 36; uvs: 74; complete measured parameters in case.toml.

- Batch-03 `editable-bezier-moved-extrude`: Observed; vertices: 72 (unused 0); faces: 36 (4-gons 36); bbox min: (-199.358565, -199.780612, 0); bbox max: (250, 200.478808, 20); size: (449.358565, 400.25942, 20); centre: (25.3207175, 0.3490979999999979, 10); normals: 36; uvs: 74; complete measured parameters in case.toml.

- Batch-03 `editable-ring`: Observed; vertices: 144 (unused 0); faces: 72 (4-gons 72); bbox min: (-200, -200, 0); bbox max: (200, 200, 20); size: (400, 400, 20); centre: (0, 0, 10); normals: 36; uvs: 74; complete measured parameters in case.toml.

## Batch-03 checkpoint (2026-11-18)

Hermes; public scripted bridge, version 25010. New mesh cases: 15; form-only observed rows: 0; not observed rows: 0. Full golden check blocked by unrelated restrict-bend duplicate params table; no commit or push. Queue remains partial, collection and publication tracked separately. Original observations and fixtures preserved.

### Batch-03 literal readings and probe notes

- `editable-ring`: Ring means the default Circle with its visible Ring switch on (outer radius 200, inner radius 100), then Make Editable. The segment and closed flags above are the actual readback.
- Interpolation rows saying “the same” use a fresh default editable Rectangle with point 0 moved by (50, 50, 0). All measurement wrappers use Offset 20 with Start Cap and End Cap off. Direction remains the wrapper's read-back default Auto except the requested moved-Bezier movement, whose measured mesh extends from z 0 to 20. Outline may reverse the resulting winding, and its Auto wrapper consequently extends to z −20; no coordinate flip was applied.
- Boolean order: the second input selected first, the first added second, matching the prior form recipe. Disjoint inputs retain the requested world centers −300 and 300; nested subtraction retains two contours.
- The first Outline preparation was discarded when a duplicate Start Cap/End Cap description label made the probe's uniqueness assertion too strict. Fresh preparation used the verified public cap control; no failed-preparation export was published. Linear splines have no tangent allocation; their tangent list is explicitly empty, not invented zero handles.

### Batch-03 complete spline measurements

Positions and handles are in centimeters. Empty segment lists denote the single implicit segment; `closed` is the spline closed flag. Linear/interpolating splines have no stored Bezier handle allocation, recorded as an empty tangent list. The manifest retains all primitive, editable and wrapper settings.

#### outline-r100-d10

```json
{
  "point_count": 8,
  "closed": true,
  "segments": [
    {
      "closed": false,
      "cnt": 4
    },
    {
      "closed": false,
      "cnt": 4
    }
  ],
  "points": [
    [
      100.0,
      0.0,
      0.0
    ],
    [
      6.123233995736766e-15,
      100.0,
      0.0
    ],
    [
      -100.0,
      1.2246467991473532e-14,
      0.0
    ],
    [
      -1.8369701987210297e-14,
      -100.0,
      0.0
    ],
    [
      -2.0174284744164585e-14,
      -110.00130344230024,
      0.0
    ],
    [
      -110.00130344230024,
      1.5283025515194686e-14,
      0.0
    ],
    [
      6.76524901503781e-15,
      110.00130344230024,
      0.0
    ],
    [
      110.00130344230024,
      1.717823970562254e-15,
      0.0
    ]
  ],
  "tangents": [
    {
      "left": [
        0.0,
        -41.5,
        0.0
      ],
      "right": [
        -0.0,
        41.5,
        -0.0
      ]
    },
    {
      "left": [
        41.5,
        -2.541142108230758e-15,
        0.0
      ],
      "right": [
        -41.5,
        2.541142108230758e-15,
        -0.0
      ]
    },
    {
      "left": [
        5.082284216461516e-15,
        41.5,
        0.0
      ],
      "right": [
        -5.082284216461516e-15,
        -41.5,
        -0.0
      ]
    },
    {
      "left": [
        -41.5,
        7.623426324692272e-15,
        0.0
      ],
      "right": [
        41.5,
        -7.623426324692272e-15,
        -0.0
      ]
    },
    {
      "left": [
        45.650540928554605,
        -8.385868324124944e-15,
        -0.0
      ],
      "right": [
        -45.6505409285546,
        8.385868324124943e-15,
        0.0
      ]
    },
    {
      "left": [
        -5.590578882749963e-15,
        -45.6505409285546,
        -0.0
      ],
      "right": [
        5.590578882749964e-15,
        45.650540928554605,
        0.0
      ]
    },
    {
      "left": [
        -45.650540928554605,
        2.795289441374982e-15,
        -0.0
      ],
      "right": [
        45.650540928554605,
        -2.795289441374982e-15,
        0.0
      ]
    },
    {
      "left": [
        -0.0,
        45.650540928554605,
        -0.0
      ],
      "right": [
        0.0,
        -45.650540928554605,
        0.0
      ]
    }
  ]
}
```

#### outline-rect

```json
{
  "point_count": 8,
  "closed": true,
  "segments": [
    {
      "closed": false,
      "cnt": 4
    },
    {
      "closed": false,
      "cnt": 4
    }
  ],
  "points": [
    [
      200.0,
      200.0,
      0.0
    ],
    [
      -200.0,
      200.0,
      0.0
    ],
    [
      -200.0,
      -200.0,
      0.0
    ],
    [
      200.0,
      -200.0,
      0.0
    ],
    [
      220.0,
      -220.0,
      0.0
    ],
    [
      -220.0,
      -220.0,
      0.0
    ],
    [
      -220.0,
      220.0,
      0.0
    ],
    [
      220.0,
      220.0,
      0.0
    ]
  ],
  "tangents": []
}
```

#### outline-arc-open

```json
{
  "point_count": 10,
  "closed": true,
  "segments": [],
  "points": [
    [
      200.0,
      0.0,
      0.0
    ],
    [
      184.77590650225736,
      76.53668647301795,
      0.0
    ],
    [
      141.4213562373095,
      141.42135623730948,
      0.0
    ],
    [
      76.53668647301797,
      184.77590650225736,
      0.0
    ],
    [
      1.2246467991473532e-14,
      200.0,
      0.0
    ],
    [
      0.06733915001931129,
      219.99988663565057,
      0.0
    ],
    [
      84.19039850322402,
      203.2536018880789,
      0.0
    ],
    [
      155.56357202219505,
      155.56357202219502,
      0.0
    ],
    [
      203.2536018880789,
      84.190398503224,
      0.0
    ],
    [
      219.99988663565057,
      0.06733915001929937,
      0.0
    ]
  ],
  "tangents": [
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        -0.0,
        20.750000000000004,
        0.0
      ]
    },
    {
      "left": [
        7.940681221575614,
        -19.170500299609202,
        0.0
      ],
      "right": [
        -7.940681221575614,
        19.170500299609202,
        0.0
      ]
    },
    {
      "left": [
        14.672465709620862,
        -14.672465709620864,
        0.0
      ],
      "right": [
        -14.672465709620862,
        14.672465709620864,
        0.0
      ]
    },
    {
      "left": [
        19.170500299609202,
        -7.940681221575615,
        0.0
      ],
      "right": [
        -19.170500299609202,
        7.940681221575615,
        0.0
      ]
    },
    {
      "left": [
        20.750000000000004,
        -1.2705710541153791e-15,
        0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        22.80743866371329,
        -1.3965528378133035e-15,
        0.0
      ]
    },
    {
      "left": [
        -21.07132577041128,
        8.72802891128606,
        0.0
      ],
      "right": [
        21.08756119588818,
        -8.734753844709491,
        0.0
      ]
    },
    {
      "left": [
        -16.139720597302734,
        16.139720597302734,
        0.0
      ],
      "right": [
        16.139720597302738,
        -16.139720597302738,
        0.0
      ]
    },
    {
      "left": [
        -8.734753844709491,
        21.08756119588819,
        0.0
      ],
      "right": [
        8.72802891128606,
        -21.07132577041128,
        0.0
      ]
    },
    {
      "left": [
        -0.0,
        22.80743866371329,
        0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    }
  ]
}
```

#### subdivide-3

```json
{
  "point_count": 12,
  "closed": true,
  "segments": [],
  "points": [
    [
      200.0,
      200.0,
      0.0
    ],
    [
      66.66666666666669,
      200.0,
      0.0
    ],
    [
      -66.66666666666663,
      200.0,
      0.0
    ],
    [
      -200.0,
      200.0,
      0.0
    ],
    [
      -200.0,
      66.66666666666669,
      0.0
    ],
    [
      -200.0,
      -66.66666666666663,
      0.0
    ],
    [
      -200.0,
      -200.0,
      0.0
    ],
    [
      -66.6666666666666,
      -200.0,
      0.0
    ],
    [
      66.66666666666663,
      -200.0,
      0.0
    ],
    [
      200.0,
      -200.0,
      0.0
    ],
    [
      200.0,
      -66.6666666666666,
      0.0
    ],
    [
      200.0,
      66.66666666666663,
      0.0
    ]
  ],
  "tangents": []
}
```

#### chamfer-flat

```json
{
  "point_count": 8,
  "closed": true,
  "segments": [],
  "points": [
    [
      200.0,
      160.0,
      0.0
    ],
    [
      160.0,
      200.0,
      0.0
    ],
    [
      -160.0,
      200.0,
      0.0
    ],
    [
      -200.0,
      160.0,
      0.0
    ],
    [
      -200.0,
      -160.0,
      0.0
    ],
    [
      -160.0,
      -200.0,
      0.0
    ],
    [
      160.0,
      -200.0,
      0.0
    ],
    [
      200.0,
      -160.0,
      0.0
    ]
  ],
  "tangents": [
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    }
  ]
}
```

#### chamfer-star

```json
{
  "point_count": 32,
  "closed": true,
  "segments": [],
  "points": [
    [
      173.50505032771906,
      -9.421973270883006,
      0.0
    ],
    [
      173.50505032771906,
      9.421973270883017,
      0.0
    ],
    [
      102.82420757422037,
      34.557065785652746,
      0.0
    ],
    [
      97.14323000080906,
      48.272158890919705,
      0.0
    ],
    [
      129.34893884884312,
      116.02425646484359,
      0.0
    ],
    [
      116.02425646484356,
      129.34893884884312,
      0.0
    ],
    [
      48.2721588909197,
      97.14323000080907,
      0.0
    ],
    [
      34.55706578565274,
      102.82420757422037,
      0.0
    ],
    [
      9.421973270883013,
      173.50505032771903,
      0.0
    ],
    [
      -9.421973270883013,
      173.50505032771903,
      0.0
    ],
    [
      -34.55706578565274,
      102.82420757422037,
      0.0
    ],
    [
      -48.2721588909197,
      97.14323000080907,
      0.0
    ],
    [
      -116.02425646484356,
      129.34893884884312,
      0.0
    ],
    [
      -129.34893884884312,
      116.02425646484359,
      0.0
    ],
    [
      -97.14323000080906,
      48.272158890919705,
      0.0
    ],
    [
      -102.82420757422037,
      34.557065785652746,
      0.0
    ],
    [
      -173.50505032771903,
      9.421973270883024,
      0.0
    ],
    [
      -173.50505032771903,
      -9.421973270883004,
      0.0
    ],
    [
      -102.82420757422037,
      -34.55706578565274,
      0.0
    ],
    [
      -97.14323000080907,
      -48.2721588909197,
      0.0
    ],
    [
      -129.34893884884312,
      -116.02425646484355,
      0.0
    ],
    [
      -116.02425646484357,
      -129.3489388488431,
      0.0
    ],
    [
      -48.27215889091971,
      -97.14323000080906,
      0.0
    ],
    [
      -34.55706578565275,
      -102.82420757422037,
      0.0
    ],
    [
      -9.421973270883035,
      -173.50505032771903,
      0.0
    ],
    [
      9.42197327088299,
      -173.50505032771903,
      0.0
    ],
    [
      34.55706578565273,
      -102.82420757422037,
      0.0
    ],
    [
      48.272158890919684,
      -97.14323000080907,
      0.0
    ],
    [
      116.02425646484353,
      -129.34893884884315,
      0.0
    ],
    [
      129.3489388488431,
      -116.02425646484357,
      0.0
    ],
    [
      97.14323000080903,
      -48.27215889091975,
      0.0
    ],
    [
      102.82420757422032,
      -34.557065785652796,
      0.0
    ]
  ],
  "tangents": [
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        4.783268391494297,
        1.7009968876924841,
        -0.0
      ]
    },
    {
      "left": [
        4.783268391494298,
        -1.7009968876924813,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        -4.04963299103416,
        1.4401059171789343,
        -0.0
      ]
    },
    {
      "left": [
        -1.8452142896129178,
        -3.881831608941113,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        2.17949508179632,
        4.585067949925454,
        -0.0
      ]
    },
    {
      "left": [
        4.585067949925453,
        2.179495081796322,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        -3.8818316089411127,
        -1.8452142896129184,
        -0.0
      ]
    },
    {
      "left": [
        1.4401059171789343,
        -4.0496329910341595,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        -1.7009968876924817,
        4.783268391494298,
        -0.0
      ]
    },
    {
      "left": [
        1.7009968876924817,
        4.783268391494298,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        -1.4401059171789343,
        -4.0496329910341595,
        -0.0
      ]
    },
    {
      "left": [
        3.8818316089411127,
        -1.8452142896129184,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        -4.585067949925453,
        2.179495081796322,
        -0.0
      ]
    },
    {
      "left": [
        -2.17949508179632,
        4.585067949925454,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        1.8452142896129178,
        -3.881831608941113,
        -0.0
      ]
    },
    {
      "left": [
        4.04963299103416,
        1.4401059171789343,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        -4.783268391494298,
        -1.7009968876924813,
        -0.0
      ]
    },
    {
      "left": [
        -4.783268391494298,
        1.7009968876924826,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        4.04963299103416,
        -1.4401059171789352,
        -0.0
      ]
    },
    {
      "left": [
        1.845214289612919,
        3.881831608941113,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        -2.179495081796322,
        -4.585067949925453,
        -0.0
      ]
    },
    {
      "left": [
        -4.585067949925454,
        -2.1794950817963206,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        3.881831608941114,
        1.8452142896129182,
        -0.0
      ]
    },
    {
      "left": [
        -1.4401059171789343,
        4.04963299103416,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        1.7009968876924817,
        -4.783268391494299,
        -0.0
      ]
    },
    {
      "left": [
        -1.7009968876924828,
        -4.783268391494299,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        1.440105917178935,
        4.0496329910341595,
        -0.0
      ]
    },
    {
      "left": [
        -3.8818316089411127,
        1.8452142896129196,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        4.585067949925455,
        -2.1794950817963237,
        -0.0
      ]
    },
    {
      "left": [
        2.1794950817963223,
        -4.585067949925455,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        0.0,
        0.0
      ],
      "right": [
        -1.845214289612919,
        3.8818316089411127,
        -0.0
      ]
    },
    {
      "left": [
        -4.0496329910341595,
        -1.4401059171789365,
        -0.0
      ],
      "right": [
        0.0,
        0.0,
        0.0
      ]
    }
  ]
}
```

#### bool-circle-rect

```json
{
  "point_count": 4,
  "closed": true,
  "segments": [],
  "points": [
    [
      200.0,
      0.0,
      0.0
    ],
    [
      1.2246467991473532e-14,
      200.0,
      0.0
    ],
    [
      -200.0,
      2.4492935982947064e-14,
      0.0
    ],
    [
      -3.6739403974420595e-14,
      -200.0,
      0.0
    ]
  ],
  "tangents": [
    {
      "left": [
        0.0,
        -83.0,
        0.0
      ],
      "right": [
        0.0,
        83.0,
        0.0
      ]
    },
    {
      "left": [
        83.0,
        -5.082284216461516e-15,
        0.0
      ],
      "right": [
        -83.0,
        5.082284216461516e-15,
        0.0
      ]
    },
    {
      "left": [
        1.0164568432923031e-14,
        83.0,
        0.0
      ],
      "right": [
        -1.0164568432923031e-14,
        -83.0,
        -0.0
      ]
    },
    {
      "left": [
        -83.0,
        1.5246852649384545e-14,
        0.0
      ],
      "right": [
        83.0,
        -1.5246852649384545e-14,
        0.0
      ]
    }
  ]
}
```

#### bool-nested

```json
{
  "point_count": 8,
  "closed": true,
  "segments": [
    {
      "closed": false,
      "cnt": 4
    },
    {
      "closed": false,
      "cnt": 4
    }
  ],
  "points": [
    [
      200.0,
      0.0,
      0.0
    ],
    [
      1.2246467991473532e-14,
      200.0,
      0.0
    ],
    [
      -200.0,
      2.4492935982947064e-14,
      0.0
    ],
    [
      -3.6739403974420595e-14,
      -200.0,
      0.0
    ],
    [
      100.0,
      0.0,
      0.0
    ],
    [
      6.123233995736766e-15,
      100.0,
      0.0
    ],
    [
      -100.0,
      1.2246467991473532e-14,
      0.0
    ],
    [
      -1.8369701987210297e-14,
      -100.0,
      0.0
    ]
  ],
  "tangents": [
    {
      "left": [
        0.0,
        -83.0,
        0.0
      ],
      "right": [
        0.0,
        83.0,
        0.0
      ]
    },
    {
      "left": [
        83.0,
        -5.082284216461516e-15,
        0.0
      ],
      "right": [
        -83.0,
        5.082284216461516e-15,
        0.0
      ]
    },
    {
      "left": [
        1.0164568432923031e-14,
        83.0,
        0.0
      ],
      "right": [
        -1.0164568432923031e-14,
        -83.0,
        -0.0
      ]
    },
    {
      "left": [
        -83.0,
        1.5246852649384545e-14,
        0.0
      ],
      "right": [
        83.0,
        -1.5246852649384545e-14,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        -41.5,
        0.0
      ],
      "right": [
        0.0,
        41.5,
        0.0
      ]
    },
    {
      "left": [
        41.5,
        -2.541142108230758e-15,
        0.0
      ],
      "right": [
        -41.5,
        2.541142108230758e-15,
        0.0
      ]
    },
    {
      "left": [
        5.082284216461516e-15,
        41.5,
        0.0
      ],
      "right": [
        -5.082284216461516e-15,
        -41.5,
        -0.0
      ]
    },
    {
      "left": [
        -41.5,
        7.623426324692272e-15,
        0.0
      ],
      "right": [
        41.5,
        -7.623426324692272e-15,
        0.0
      ]
    }
  ]
}
```

#### bool-disjoint

```json
{
  "point_count": 8,
  "closed": true,
  "segments": [
    {
      "closed": false,
      "cnt": 4
    },
    {
      "closed": false,
      "cnt": 4
    }
  ],
  "points": [
    [
      200.0,
      0.0,
      0.0
    ],
    [
      0.0,
      200.0,
      0.0
    ],
    [
      -200.0,
      2.4492935982947064e-14,
      0.0
    ],
    [
      -5.684341886080802e-14,
      -200.0,
      0.0
    ],
    [
      800.0,
      0.0,
      0.0
    ],
    [
      600.0,
      200.0,
      0.0
    ],
    [
      400.0,
      2.4492935982947064e-14,
      0.0
    ],
    [
      600.0,
      -200.0,
      0.0
    ]
  ],
  "tangents": [
    {
      "left": [
        0.0,
        -83.0,
        0.0
      ],
      "right": [
        0.0,
        83.0,
        0.0
      ]
    },
    {
      "left": [
        83.0,
        -5.082284216461516e-15,
        0.0
      ],
      "right": [
        -83.0,
        5.082284216461516e-15,
        0.0
      ]
    },
    {
      "left": [
        1.0164568432923031e-14,
        83.0,
        0.0
      ],
      "right": [
        -1.0164568432923031e-14,
        -83.0,
        -0.0
      ]
    },
    {
      "left": [
        -83.0,
        1.5246852649384545e-14,
        0.0
      ],
      "right": [
        83.0,
        -1.5246852649384545e-14,
        0.0
      ]
    },
    {
      "left": [
        0.0,
        -83.0,
        0.0
      ],
      "right": [
        0.0,
        83.0,
        0.0
      ]
    },
    {
      "left": [
        83.0,
        -5.082284216461516e-15,
        0.0
      ],
      "right": [
        -83.0,
        5.082284216461516e-15,
        0.0
      ]
    },
    {
      "left": [
        1.0164568432923031e-14,
        83.0,
        0.0
      ],
      "right": [
        -1.0164568432923031e-14,
        -83.0,
        -0.0
      ]
    },
    {
      "left": [
        -83.0,
        1.5246852649384545e-14,
        0.0
      ],
      "right": [
        83.0,
        -1.5246852649384545e-14,
        0.0
      ]
    }
  ]
}
```

#### interp-cubic-natural

```json
{
  "point_count": 4,
  "closed": true,
  "segments": [],
  "points": [
    [
      250.0,
      250.0,
      0.0
    ],
    [
      -200.0,
      200.0,
      0.0
    ],
    [
      -200.0,
      -200.0,
      0.0
    ],
    [
      200.0,
      -200.0,
      0.0
    ]
  ],
  "tangents": []
}
```

#### interp-akima-natural

```json
{
  "point_count": 4,
  "closed": true,
  "segments": [],
  "points": [
    [
      250.0,
      250.0,
      0.0
    ],
    [
      -200.0,
      200.0,
      0.0
    ],
    [
      -200.0,
      -200.0,
      0.0
    ],
    [
      200.0,
      -200.0,
      0.0
    ]
  ],
  "tangents": []
}
```

#### interp-bspline-natural

```json
{
  "point_count": 4,
  "closed": true,
  "segments": [],
  "points": [
    [
      250.0,
      250.0,
      0.0
    ],
    [
      -200.0,
      200.0,
      0.0
    ],
    [
      -200.0,
      -200.0,
      0.0
    ],
    [
      200.0,
      -200.0,
      0.0
    ]
  ],
  "tangents": []
}
```

#### tangent-readback-moved

```json
{
  "point_count": 4,
  "closed": true,
  "segments": [],
  "points": [
    [
      250.0,
      0.0,
      0.0
    ],
    [
      1.2246467991473532e-14,
      200.0,
      0.0
    ],
    [
      -200.0,
      2.4492935982947064e-14,
      0.0
    ],
    [
      -3.6739403974420595e-14,
      -200.0,
      0.0
    ]
  ],
  "tangents": [
    {
      "left": [
        0.0,
        -83.0,
        0.0
      ],
      "right": [
        -0.0,
        83.0,
        -0.0
      ]
    },
    {
      "left": [
        83.0,
        -5.082284216461516e-15,
        0.0
      ],
      "right": [
        -83.0,
        5.082284216461516e-15,
        -0.0
      ]
    },
    {
      "left": [
        1.0164568432923031e-14,
        83.0,
        0.0
      ],
      "right": [
        -1.0164568432923031e-14,
        -83.0,
        -0.0
      ]
    },
    {
      "left": [
        -83.0,
        1.5246852649384545e-14,
        0.0
      ],
      "right": [
        83.0,
        -1.5246852649384545e-14,
        -0.0
      ]
    }
  ]
}
```

#### editable-bezier-moved-extrude

```json
{
  "point_count": 4,
  "closed": true,
  "segments": [],
  "points": [
    [
      250.0,
      0.0,
      0.0
    ],
    [
      1.2246467991473532e-14,
      200.0,
      0.0
    ],
    [
      -200.0,
      2.4492935982947064e-14,
      0.0
    ],
    [
      -3.6739403974420595e-14,
      -200.0,
      0.0
    ]
  ],
  "tangents": [
    {
      "left": [
        0.0,
        -83.0,
        0.0
      ],
      "right": [
        -0.0,
        166.0,
        -0.0
      ]
    },
    {
      "left": [
        83.0,
        -5.082284216461516e-15,
        0.0
      ],
      "right": [
        -83.0,
        5.082284216461516e-15,
        -0.0
      ]
    },
    {
      "left": [
        1.0164568432923031e-14,
        83.0,
        0.0
      ],
      "right": [
        -1.0164568432923031e-14,
        -83.0,
        -0.0
      ]
    },
    {
      "left": [
        -83.0,
        1.5246852649384545e-14,
        0.0
      ],
      "right": [
        83.0,
        -1.5246852649384545e-14,
        -0.0
      ]
    }
  ]
}
```

#### editable-ring

```json
{
  "point_count": 8,
  "closed": true,
  "segments": [
    {
      "closed": true,
      "cnt": 4
    },
    {
      "closed": true,
      "cnt": 4
    }
  ],
  "points": [
    [
      200.0,
      0.0,
      0.0
    ],
    [
      1.2246467991473532e-14,
      200.0,
      0.0
    ],
    [
      -200.0,
      2.4492935982947064e-14,
      0.0
    ],
    [
      -3.6739403974420595e-14,
      -200.0,
      0.0
    ],
    [
      100.0,
      0.0,
      0.0
    ],
    [
      6.123233995736766e-15,
      100.0,
      0.0
    ],
    [
      -100.0,
      1.2246467991473532e-14,
      0.0
    ],
    [
      -1.8369701987210297e-14,
      -100.0,
      0.0
    ]
  ],
  "tangents": [
    {
      "left": [
        0.0,
        -83.0,
        0.0
      ],
      "right": [
        -0.0,
        83.0,
        -0.0
      ]
    },
    {
      "left": [
        83.0,
        -5.082284216461516e-15,
        0.0
      ],
      "right": [
        -83.0,
        5.082284216461516e-15,
        -0.0
      ]
    },
    {
      "left": [
        1.0164568432923031e-14,
        83.0,
        0.0
      ],
      "right": [
        -1.0164568432923031e-14,
        -83.0,
        -0.0
      ]
    },
    {
      "left": [
        -83.0,
        1.5246852649384545e-14,
        0.0
      ],
      "right": [
        83.0,
        -1.5246852649384545e-14,
        -0.0
      ]
    },
    {
      "left": [
        0.0,
        -41.5,
        0.0
      ],
      "right": [
        -0.0,
        41.5,
        -0.0
      ]
    },
    {
      "left": [
        41.5,
        -2.541142108230758e-15,
        0.0
      ],
      "right": [
        -41.5,
        2.541142108230758e-15,
        -0.0
      ]
    },
    {
      "left": [
        5.082284216461516e-15,
        41.5,
        0.0
      ],
      "right": [
        -5.082284216461516e-15,
        -41.5,
        -0.0
      ]
    },
    {
      "left": [
        -41.5,
        7.623426324692272e-15,
        0.0
      ],
      "right": [
        41.5,
        -7.623426324692272e-15,
        -0.0
      ]
    }
  ]
}
```
