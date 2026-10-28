# Modeling tools — observation (O6)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)): the tools run through the
app's documented modeling-command scripting call, or `run_modeling_command`. Level A
(exact meshes). Unblocks: batch 11 (U4 modeling tools).

## Setup for every case

- New document. Make a **default cube** with its menu command, set the segments the row
  gives, then Make Editable.
- Select the components the row names. Our cube has the reference's vertex and polygon
  order, so find them by position:
  - "top polygon" is the polygon whose points all have y = +100;
  - "top edges" are the 4 edges of that polygon;
  - "top-front edge" is the edge with y = +100 and z = −100.
  Record the indices you selected in the row.
- Run the tool with the settings the row gives. Every other setting stays at the tool's
  default.
- Export the cube with the usual OBJ settings to `tests/golden/modeling-tools/<case>/mesh.obj`.
- `case.toml`: `feature = "modeling-tools"`. `[params]` holds `tool`, the selection
  (`select_polys = [...]`, `select_edges = [[a, b], ...]` as point pairs,
  `select_points = [...]`), the cube's segment counts, and every tool setting you could
  read, under our key names (lowercase words with `_`).

## Tool settings (fill once per tool)

For each tool, list every setting as it is shown: label, default, and the min and max
that a ±999999999 assignment keeps, as in r25-defaults.md. Read them from the tool's
settings container (or from the GUI, if the script can't reach them).

### Extrude
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Extrude Inner (Inset)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Bevel
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Cube segments | Selection | Tool + settings | `golden info` | Notes |
|---|---|---|---|---|---|
| extrude-top | 1/1/1 | top polygon | Extrude, offset 50 | | |
| extrude-top-negative | 1/1/1 | top polygon | Extrude, offset −50 | | |
| extrude-two | 2/1/1 | the 2 top polygons | Extrude, offset 50, Preserve Groups **on** | | |
| extrude-two-separate | 2/1/1 | the 2 top polygons | Extrude, offset 50, Preserve Groups **off** | | |
| inset-top | 1/1/1 | top polygon | Extrude Inner, offset 20 | | |
| inset-two | 2/1/1 | the 2 top polygons | Extrude Inner, offset 20, Preserve Groups on | | |
| bevel-poly | 1/1/1 | top polygon | Bevel (polygon mode), offset 20, subdivision 0 | | |
| bevel-edge | 1/1/1 | top-front edge | Bevel (edge mode), offset 20, subdivision 0 | | |
| bevel-edge-sub | 1/1/1 | top-front edge | Bevel (edge mode), offset 20, subdivision 2 | | |
| bevel-edges-top | 1/1/1 | top edges | Bevel (edge mode), offset 20, subdivision 1 | | |
| subdivide-1 | 1/1/1 | all polygons | the Subdivide command, 1 subdivision, smoothing off | | |
| triangulate | 2/2/2 | all polygons | the Triangulate command | | |
| reverse-normals | 1/1/1 | top polygon | the Reverse Normals command | | |
| optimize | 1/1/1 after Connect of two cubes at the same place | all | the Optimize command, defaults | | |

## Notes
TODO
