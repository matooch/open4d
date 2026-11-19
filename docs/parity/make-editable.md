# Make Editable and Current State to Object — observation (O3)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-28
Method: scripted or GUI ([agents/OBSERVER.md](../agents/OBSERVER.md)); level A for the
meshes, level C for the tree. Unblocks: the parity-pending rules of brief 03.

For each case, record the **object tree after the command** (names, types as shown,
order, tags, children) and export the result as OBJ with the usual settings to
`tests/golden/make-editable/<case>/mesh.obj` (for a tree with several polygon objects,
export the whole scene and note which object is which group).

## Cases

| Case | Scene before | Command | Tree after (names / types / tags / children) | Notes |
|---|---|---|---|---|
| cube | default cube with its smoothing tag | Make Editable | `Cube` (type Polygon, 8 points, 6 polygons); tags in order: Phong, UVW, then two unnamed tags (the point and polygon data the editable object stores; not shown in the Objects list). No children. | name kept; Phong kept; a UVW tag is added; **no new selection tags**. |
| cube-deformed | cube (segments 4/8/4, size_y 400) > Bend strength 90 | Make Editable on the cube | `Cube` (Polygon, 162 points, 160 polygons; tags Phong, UVW, two unnamed data tags) > child `Bend` (Bend, enabled, strength still 90°). | the bend is **not applied**: the points are the unbent box (bbox ±100 × ±200 × ±100); the Bend stays as a child and still deforms live, so the exported mesh is bent (bbox -100…359.15, -200…259.15). The Bend was inserted under the cube by the probe: the Create menu with the cube selected makes it a separate top-level object. |
| cube-deformed-csto | same | Current State to Object on the cube | Top level, in order: `Cube` (Cube, unchanged, tag Phong) > child `Bend`; then **`Cube`** (Polygon, 162 points, 160 polygons, tags Phong, UVW, two data tags), no children. | the new object is the **next sibling after the original**, with the **same name** (no suffix; the OBJ exporter names them Cube and Cube_1). Its points are the bent result (bbox -100…359.15, -200…259.15). The original keeps its Bend child and stays selected. The export has both, 320 faces. |
| subdivision | Subdivision Surface > default cube | Make Editable on the generator | One top-level object `Subdivision Surface` (Polygon, 98 points, 96 polygons; tags Phong, UVW, two data tags). No children. | **one object**, named after the generator; the input cube is gone (consumed); the Phong tag came from the cube. Geometry = the subdivided cube (level 2). The cube was placed under the generator by the probe. |
| subdivision-csto | same | Current State to Object | | |
| extrude | Extrude > Rectangle spline (defaults) | Make Editable | `Extrude` (Polygon, 8 points, 4 polygons: the side walls; tags Phong, UVW, two data tags) with two children in order: `Cap 1` (Polygon, 4 points, 1 polygon; tags UVW, Polygon Selection named C1, Phong, two data tags) and `Cap 2` (same, selection tag C2). | **caps are separate polygon objects, as children** of the extruded object, named Cap 1 (the start cap) and Cap 2. The Rectangle input is gone. In the OBJ: groups Extrude (4 faces), Cap_1 (1), Cap_2 (1); bbox z 0…100. |
| array | Array > default cube | Make Editable | | |
| sphere-moved | sphere at position (10, 20, 30), rotation H 45° | Make Editable | `Sphere` (Polygon, 114 points, 128 polygons; tags Phong, UVW, two data tags), position (10, 20, 30), rotation H 45° unchanged. | position and rotation stay **on the object**. Name kept; the exported bbox is centred at (10, 20, 30). |
| spline | Circle spline | Make Editable | `Circle` (type Spline, 4 points, closed; two unnamed data tags: segment data and point data). No children. | name kept; becomes an editable spline with 4 points. Tree only, no export. |

## Follow-up cases (planner, after brief 030)

Brief 030 matched all 8 meshes and trees above except the `array` tree. Record these the same way
(mesh plus the tree after: names, types, tags in order, children, transforms).

| Case | Scene before | Command | Tree after (names / types / tags / children) | Notes |
|---|---|---|---|---|
| array-tree | as `array` | Make Editable | `Array` becomes a selected Null with eight parametric Cube children, ordered `Cube.8` through `Cube.1`; each retains its Phong tag. Full local transforms below. | Observed by MCP on 2026-11-17. Mesh: 64 vertices, 48 quad faces, size (700, 200, 700), 10 normals, 4 UVs. Isolated golden check: `ok: 1 cases`. Full check blocked by pre-existing `fields/restrict-bend` duplicate TOML key; no commit. |
| extrude-cap-off | Extrude of a Rectangle, end cap off | Make Editable | See extrude-cap-off readback below | Observed 2026-11-17; mesh and manifest exported; isolated validation passed. |
| extrude-triangles | Extrude of a Rectangle, Cap Type Triangles | Make Editable | See extrude-triangles readback below | Observed 2026-11-17; mesh and manifest exported; isolated validation passed. |
| lathe-caps | Lathe of an Arc, caps on | Make Editable | See lathe-caps readback below | Observed 2026-11-17; mesh and manifest exported; isolated validation passed. |
| extrude-csto | as `extrude` | Current State to Object | See extrude-csto readback below | Observed 2026-11-17; mesh and manifest exported; isolated validation passed. |
| cloner | Cloner (Grid, 3 × 1 × 1) of a default cube | Make Editable | See cloner readback below | Observed 2026-11-17; mesh and manifest exported; isolated validation passed. |
| two-objects | a default cube and a default sphere, both selected | Make Editable | See two-objects readback below | Observed 2026-11-17; mesh and manifest exported; isolated validation passed. |
| two-objects-csto | the same | Current State to Object | See two-objects-csto readback below | Observed 2026-11-17; mesh and manifest exported; isolated validation passed. |

## Behaviour notes
- **Undo:** each command (Make Editable, Current State to Object) is **one undo step**: a single Undo restored the exact tree before the command in all 9 cases (checked with the scripting interface's undo, not the menu).
- **Selection after:** the object the command was run on stays the only selected object, in every case. For Make Editable it is the converted object (now polygon, Null or spline); for Current State to Object it is the **original**, not the new copy. The caps of the extrude are not selected.
- Both commands were run through their command (by command id) with the object selected; menu paths and shortcut keys were not read.
- Creating an object from the Create menu while another is selected makes a **separate top-level object**, never a child; the child relations (cube under Subdivision Surface, Array, Extrude; Bend under cube) were set by the probe, as dragging in the Objects list would.
- Every Make Editable result carries two unnamed data tags after Phong and UVW (point and polygon data; segment and point data for the spline). They are not selection tags: **no new selection tags** appear except Cap 1 / Cap 2's `C1`/`C2` polygon selections after the Extrude conversion.
- Exports: whole scene, the usual settings (scale 1 cm, normals Vertices, UVs Original, no material, Flip Z off); R25 writes six decimals and an `o` line per object. Which object is which: cube-deformed-csto `Cube` = original (bent live), `Cube_1` = new copy; subdivision-csto `Cube` = the input under the generator (subdivided live), `Subdivision_Surface` = new copy; extrude `Extrude`, `Cap_1`, `Cap_2`; array `Cube.8` … `Cube.1`. For the CSTO cases the file holds both objects, so counts double (324 points / 320 faces; 196 / 192).
- Not read: the menu texts' exact labels, keyboard shortcuts, any dialog the commands may show, behaviour with several objects selected.

## array-tree retry readback

The input was a menu-created default Array (Radius 250, Copies 7, Amplitude 0, Frequency 0, Array Frequency 10, Render Instance off) with a menu-created default cube (Size 200 × 200 × 200, Segments X/Y/Z 1, Separate Surfaces off, Fillet off, Fillet Radius 3, Fillet Subdivision 3). All scalar and vector input parameters read back are recorded in the new case manifest. No existing case was modified.

After Make Editable, local transforms were read back as follows. Position is in centimeters; Rotation is the raw H/P/B tuple in radians. Values below are copied from readback without rounding. Names and tag/type labels are as reported by the application.

| Object | Type | Local position | Local rotation (radians) | Local scale | Tags in order | Selected |
|---|---|---|---|---|---|---|
| Array | Null | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] | none | true |
| Cube.8 | Cube | [0.0, 0.0, 250.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] | Phong (Phong) | false |
| Cube.7 | Cube | [176.77669529663686, 0.0, 176.7766952966369] | [-0.7853981633974483, 0.0, 0.0] | [1.0, 1.0, 1.0] | Phong (Phong) | false |
| Cube.6 | Cube | [250.0, 0.0, 1.5308084989341916e-14] | [-1.5707963267948966, 0.0, 0.0] | [1.0, 1.0, 1.0] | Phong (Phong) | false |
| Cube.5 | Cube | [176.7766952966369, -0.0, -176.77669529663686] | [-2.356194490192345, 0.0, 0.0] | [1.0, 1.0, 1.0] | Phong (Phong) | false |
| Cube.4 | Cube | [3.061616997868383e-14, -0.0, -250.0] | [-3.141592653589793, 0.0, 0.0] | [1.0, 1.0, 1.0] | Phong (Phong) | false |
| Cube.3 | Cube | [-176.77669529663686, 0.0, -176.77669529663692] | [-3.9269908169872414, 0.0, 0.0] | [1.0, 1.0, 1.0] | Phong (Phong) | false |
| Cube.2 | Cube | [-250.0, 0.0, -4.5924254968025744e-14] | [-4.71238898038469, 0.0, 0.0] | [1.0, 1.0, 1.0] | Phong (Phong) | false |
| Cube.1 | Cube | [-176.77669529663692, -0.0, 176.77669529663683] | [-5.497787143782138, 0.0, 0.0] | [1.0, 1.0, 1.0] | Phong (Phong) | false |

All eight Cube children have no children of their own. The parent Null has no tags. The result is not eight polygon objects: the cube children remain parametric. The whole scene was exported with scale 1 cm, Normals Vertices, UVs Original, Material None, no axis flips/swaps, no face flip, Resolve N-gons off, Objects as Groups off, and No Animation. The scratch document was closed after export.

`golden info`:

```text
vertices: 64 (unused 0)
faces: 48 (4-gons 48)
bbox min: (-350, -100, -350)
bbox max: (350, 100, 350)
size: (700, 200, 700)
centre: (0, 0, 0)
normals: 10
uvs: 4
```

The first attempt failed because the probe asserted the creation command return value; that command returned no value despite creating the Array. On this retry, actual created objects were checked instead. An extra raw tree JSON was initially rejected by the typed samples validator and removed from this new case; the tree is documented here instead. The new mesh-only case passes the isolated validator. Full-working-tree validation remains blocked by the unrelated pre-existing duplicate key.

## extrude-cap-off readback (2026-11-17)

Input settings are read back in the new manifest. Literal interpretation of the planner row; unchanged settings are menu defaults. Position in centimeters; rotations are raw radians. Full ordered tree, tag names/types and selection:

```json
[
  {
    "name": "Extrude",
    "type": "Polygon",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "UVW",
        "type": "UVW"
      },
      {
        "name": "Phong",
        "type": "Phong"
      },
      {
        "name": "S",
        "type": "Polygon Selection"
      },
      {
        "name": "C1",
        "type": "Polygon Selection"
      },
      {
        "name": "C2",
        "type": "Polygon Selection"
      },
      {
        "name": "R1",
        "type": "Polygon Selection"
      },
      {
        "name": "R2",
        "type": "Polygon Selection"
      },
      {
        "name": "ES1",
        "type": "Edge Selection"
      },
      {
        "name": "ES2",
        "type": "Edge Selection"
      },
      {
        "name": "EC1",
        "type": "Edge Selection"
      },
      {
        "name": "EC2",
        "type": "Edge Selection"
      },
      {
        "name": "ER1",
        "type": "Edge Selection"
      },
      {
        "name": "ER2",
        "type": "Edge Selection"
      },
      {
        "name": "",
        "type": ""
      },
      {
        "name": "",
        "type": ""
      }
    ],
    "selected": true,
    "children": [],
    "points": 8,
    "polygons": 5
  }
]
```

`golden info`:

```text
vertices: 8 (unused 0)
faces: 5 (4-gons 5)
bbox min: (-200, -200, 0)
bbox max: (200, 200, 100)
size: (400, 400, 100)
centre: (0, 0, 50)
normals: 5
uvs: 10
```

## extrude-triangles readback (2026-11-17)

Input settings are read back in the new manifest. Literal interpretation of the planner row; unchanged settings are menu defaults. Position in centimeters; rotations are raw radians. Full ordered tree, tag names/types and selection:

```json
[
  {
    "name": "Extrude",
    "type": "Polygon",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "UVW",
        "type": "UVW"
      },
      {
        "name": "Phong",
        "type": "Phong"
      },
      {
        "name": "S",
        "type": "Polygon Selection"
      },
      {
        "name": "C1",
        "type": "Polygon Selection"
      },
      {
        "name": "C2",
        "type": "Polygon Selection"
      },
      {
        "name": "R1",
        "type": "Polygon Selection"
      },
      {
        "name": "R2",
        "type": "Polygon Selection"
      },
      {
        "name": "ES1",
        "type": "Edge Selection"
      },
      {
        "name": "ES2",
        "type": "Edge Selection"
      },
      {
        "name": "EC1",
        "type": "Edge Selection"
      },
      {
        "name": "EC2",
        "type": "Edge Selection"
      },
      {
        "name": "ER1",
        "type": "Edge Selection"
      },
      {
        "name": "ER2",
        "type": "Edge Selection"
      },
      {
        "name": "",
        "type": ""
      },
      {
        "name": "",
        "type": ""
      }
    ],
    "selected": true,
    "children": [],
    "points": 8,
    "polygons": 8
  }
]
```

`golden info`:

```text
vertices: 8 (unused 0)
faces: 8 (3-gons 4, 4-gons 4)
bbox min: (-200, -200, 0)
bbox max: (200, 200, 100)
size: (400, 400, 100)
centre: (0, 0, 50)
normals: 8
uvs: 10
```

## lathe-caps readback (2026-11-17)

Input settings are read back in the new manifest. Literal interpretation of the planner row; unchanged settings are menu defaults. Position in centimeters; rotations are raw radians. Full ordered tree, tag names/types and selection:

```json
[
  {
    "name": "Lathe",
    "type": "Polygon",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "UVW",
        "type": "UVW"
      },
      {
        "name": "Phong",
        "type": "Phong"
      },
      {
        "name": "S",
        "type": "Polygon Selection"
      },
      {
        "name": "C1",
        "type": "Polygon Selection"
      },
      {
        "name": "C2",
        "type": "Polygon Selection"
      },
      {
        "name": "R1",
        "type": "Polygon Selection"
      },
      {
        "name": "R2",
        "type": "Polygon Selection"
      },
      {
        "name": "ES1",
        "type": "Edge Selection"
      },
      {
        "name": "ES2",
        "type": "Edge Selection"
      },
      {
        "name": "EC1",
        "type": "Edge Selection"
      },
      {
        "name": "EC2",
        "type": "Edge Selection"
      },
      {
        "name": "ER1",
        "type": "Edge Selection"
      },
      {
        "name": "ER2",
        "type": "Edge Selection"
      },
      {
        "name": "",
        "type": ""
      },
      {
        "name": "",
        "type": ""
      }
    ],
    "selected": true,
    "children": [],
    "points": 1184,
    "polygons": 1152
  }
]
```

`golden info`:

```text
vertices: 1184 (unused 0)
faces: 1152 (4-gons 1152)
bbox min: (-200, 0, -200)
bbox max: (200, 199.999998, 200)
size: (400, 199.999998, 400)
centre: (0, 99.999999, 0)
normals: 1184
uvs: 1221
```

## extrude-csto readback (2026-11-17)

Input settings are read back in the new manifest. Literal interpretation of the planner row; unchanged settings are menu defaults. Position in centimeters; rotations are raw radians. Full ordered tree, tag names/types and selection:

```json
[
  {
    "name": "Extrude",
    "type": "Extrude",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "Phong",
        "type": "Phong"
      },
      {
        "name": "S",
        "type": "Cache Proxy Tag"
      },
      {
        "name": "C1",
        "type": "Cache Proxy Tag"
      },
      {
        "name": "C2",
        "type": "Cache Proxy Tag"
      },
      {
        "name": "R1",
        "type": "Cache Proxy Tag"
      },
      {
        "name": "R2",
        "type": "Cache Proxy Tag"
      },
      {
        "name": "ES1",
        "type": "Cache Proxy Tag"
      },
      {
        "name": "ES2",
        "type": "Cache Proxy Tag"
      },
      {
        "name": "EC1",
        "type": "Cache Proxy Tag"
      },
      {
        "name": "EC2",
        "type": "Cache Proxy Tag"
      },
      {
        "name": "ER1",
        "type": "Cache Proxy Tag"
      },
      {
        "name": "ER2",
        "type": "Cache Proxy Tag"
      }
    ],
    "selected": true,
    "children": [
      {
        "name": "Rectangle",
        "type": "Rectangle",
        "position": [
          0.0,
          0.0,
          0.0
        ],
        "rotation_radians": [
          0.0,
          0.0,
          0.0
        ],
        "scale": [
          1.0,
          1.0,
          1.0
        ],
        "tags": [],
        "selected": false,
        "children": []
      }
    ]
  },
  {
    "name": "Extrude",
    "type": "Polygon",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "UVW",
        "type": "UVW"
      },
      {
        "name": "Phong",
        "type": "Phong"
      },
      {
        "name": "S",
        "type": "Polygon Selection"
      },
      {
        "name": "C1",
        "type": "Polygon Selection"
      },
      {
        "name": "C2",
        "type": "Polygon Selection"
      },
      {
        "name": "R1",
        "type": "Polygon Selection"
      },
      {
        "name": "R2",
        "type": "Polygon Selection"
      },
      {
        "name": "ES1",
        "type": "Edge Selection"
      },
      {
        "name": "ES2",
        "type": "Edge Selection"
      },
      {
        "name": "EC1",
        "type": "Edge Selection"
      },
      {
        "name": "EC2",
        "type": "Edge Selection"
      },
      {
        "name": "ER1",
        "type": "Edge Selection"
      },
      {
        "name": "ER2",
        "type": "Edge Selection"
      },
      {
        "name": "",
        "type": ""
      },
      {
        "name": "",
        "type": ""
      }
    ],
    "selected": false,
    "children": [],
    "points": 8,
    "polygons": 6
  }
]
```

`golden info`:

```text
vertices: 16 (unused 0)
faces: 12 (4-gons 12)
bbox min: (-200, -200, 0)
bbox max: (200, 200, 100)
size: (400, 400, 100)
centre: (0, 0, 50)
normals: 6
uvs: 10
```

## cloner readback (2026-11-17)

Input settings are read back in the new manifest. Literal interpretation of the planner row; unchanged settings are menu defaults. Position in centimeters; rotations are raw radians. Full ordered tree, tag names/types and selection:

```json
[
  {
    "name": "Cloner",
    "type": "Null",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [],
    "selected": true,
    "children": [
      {
        "name": "Cube 0",
        "type": "Cube",
        "position": [
          -200.0,
          0.0,
          0.0
        ],
        "rotation_radians": [
          0.0,
          0.0,
          0.0
        ],
        "scale": [
          1.0,
          1.0,
          1.0
        ],
        "tags": [
          {
            "name": "Phong",
            "type": "Phong"
          }
        ],
        "selected": false,
        "children": []
      },
      {
        "name": "Cube 1",
        "type": "Cube",
        "position": [
          0.0,
          0.0,
          0.0
        ],
        "rotation_radians": [
          0.0,
          0.0,
          0.0
        ],
        "scale": [
          1.0,
          1.0,
          1.0
        ],
        "tags": [
          {
            "name": "Phong",
            "type": "Phong"
          }
        ],
        "selected": false,
        "children": []
      },
      {
        "name": "Cube 2",
        "type": "Cube",
        "position": [
          200.0,
          0.0,
          0.0
        ],
        "rotation_radians": [
          0.0,
          0.0,
          0.0
        ],
        "scale": [
          1.0,
          1.0,
          1.0
        ],
        "tags": [
          {
            "name": "Phong",
            "type": "Phong"
          }
        ],
        "selected": false,
        "children": []
      }
    ]
  }
]
```

`golden info`:

```text
vertices: 24 (unused 0)
faces: 18 (4-gons 18)
bbox min: (-300, -100, -100)
bbox max: (300, 100, 100)
size: (600, 200, 200)
centre: (0, 0, 0)
normals: 6
uvs: 4
```

## two-objects readback (2026-11-17)

Input settings are read back in the new manifest. Literal interpretation of the planner row; unchanged settings are menu defaults. Position in centimeters; rotations are raw radians. Full ordered tree, tag names/types and selection:

```json
[
  {
    "name": "Sphere",
    "type": "Polygon",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "Phong",
        "type": "Phong"
      },
      {
        "name": "UVW",
        "type": "UVW"
      },
      {
        "name": "",
        "type": ""
      },
      {
        "name": "",
        "type": ""
      }
    ],
    "selected": true,
    "children": [],
    "points": 114,
    "polygons": 128
  },
  {
    "name": "Cube",
    "type": "Polygon",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "Phong",
        "type": "Phong"
      },
      {
        "name": "UVW",
        "type": "UVW"
      },
      {
        "name": "",
        "type": ""
      },
      {
        "name": "",
        "type": ""
      }
    ],
    "selected": true,
    "children": [],
    "points": 8,
    "polygons": 6
  }
]
```

`golden info`:

```text
vertices: 122 (unused 0)
faces: 134 (3-gons 32, 4-gons 102)
bbox min: (-100, -100, -100)
bbox max: (100, 100, 100)
size: (200, 200, 200)
centre: (0, 0, 0)
normals: 120
uvs: 153
```

## two-objects-csto readback (2026-11-17)

Input settings are read back in the new manifest. Literal interpretation of the planner row; unchanged settings are menu defaults. Position in centimeters; rotations are raw radians. Full ordered tree, tag names/types and selection:

```json
[
  {
    "name": "Sphere",
    "type": "Sphere",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "Phong",
        "type": "Phong"
      }
    ],
    "selected": true,
    "children": []
  },
  {
    "name": "Sphere",
    "type": "Polygon",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "Phong",
        "type": "Phong"
      },
      {
        "name": "UVW",
        "type": "UVW"
      },
      {
        "name": "",
        "type": ""
      },
      {
        "name": "",
        "type": ""
      }
    ],
    "selected": false,
    "children": [],
    "points": 114,
    "polygons": 128
  },
  {
    "name": "Cube",
    "type": "Cube",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "Phong",
        "type": "Phong"
      }
    ],
    "selected": true,
    "children": []
  },
  {
    "name": "Cube",
    "type": "Polygon",
    "position": [
      0.0,
      0.0,
      0.0
    ],
    "rotation_radians": [
      0.0,
      0.0,
      0.0
    ],
    "scale": [
      1.0,
      1.0,
      1.0
    ],
    "tags": [
      {
        "name": "Phong",
        "type": "Phong"
      },
      {
        "name": "UVW",
        "type": "UVW"
      },
      {
        "name": "",
        "type": ""
      },
      {
        "name": "",
        "type": ""
      }
    ],
    "selected": false,
    "children": [],
    "points": 8,
    "polygons": 6
  }
]
```

`golden info`:

```text
vertices: 244 (unused 0)
faces: 268 (3-gons 64, 4-gons 204)
bbox min: (-100, -100, -100)
bbox max: (100, 100, 100)
size: (200, 200, 200)
centre: (0, 0, 0)
normals: 120
uvs: 153
```

## Follow-up batch outcome (2026-11-17)

All eight planner follow-ups are observed (seven new in this batch; array-tree preserved from the previous batch). New cap results differ from the older baseline notes above: the current menu-created Extrude converts to one polygon object with caps integrated, not child cap objects. The Current State copy likewise has no cap children. No inference is made about why the older baseline differs. Lathe of the default Arc was interpreted literally: default 360-degree lathe angle and default Arc, Start Cap and End Cap enabled; it produces no separate cap children. Cloner conversion was verified to produce three cube children and a 600-wide exported mesh, rather than the 200-wide input cube. Both multi-object inputs remain selected after Make Editable; Current State leaves originals selected and places each unselected copy immediately after its source in scene order (Sphere, Sphere copy, Cube, Cube copy).

The first preparation invocation failed because function definitions were not retained between bridge calls; the saved scratch module resolved this. The deprecated cap setting did not change the visible End Cap; no golden was written from that preparation. A fresh scratch scene using the visible parameter label read back End Cap off and exported the observed case. All successful scratch documents were closed.

Validation: isolated copied-root check passed for seven new cases. Full golden check exit 1 remains blocked solely by the pre-existing fields/restrict-bend duplicate table at line 7; cleanroom exit 0. The five-case checkpoint and end-job commits were blocked by the required full validator; no commit or push.
