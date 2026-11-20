# Selection commands — observation (O26)

Status: partial (6 of 11 baseline cases observed; 2 additional variants observed) · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: scripted. Unblocks: batch 11 (loop, ring, grow, shrink, convert).

Every case:
- Start from a **cube made editable** with the segments given.
- Select the start set through the script, by index.
- Run the selection command named in the row.
- Read back the resulting point, edge and polygon selections as index lists. Edges are
  written as point pairs, sorted with the lower point first.

Folder `tests/golden/selection-ops/<case>/` holds:
- `mesh.obj`: the cube, exported so the case has output. Its point and polygon order
  are the reference for the indices.
- `selection.json`:
  ```json
  { "points": [..], "edges": [[a, b], ..], "polys": [..] }
  ```
  This is the result after the command. Use empty lists for the unused sets.
- `case.toml`: `feature = "selection-ops"`, `observation = "docs/parity/selection-ops.md"`.
  `[params]` holds the segments, `start_points` / `start_edges` / `start_polys`, and
  `command`.

Previous attempt (2026-10-29): seven selection commands run in one probe, including
"Grow Selection…" (a dialog), crashed the app. Follow QUEUE.md rule 11: one command per probe,
a `ping` after each. Loop and Ring Selection are interactive tools that wait for mouse clicks:
mark `loop-edge`, `loop-poly` and `ring-edge` as "needs the GUI" and skip them. For grow,
shrink and convert, use the command **without** a dialog: the plain menu entry, not the one
ending in `…`. If only the dialog version exists, skip the case with that reason.

## Cases

| Case | Segments | Start | Command |
|---|---|---|---|
| loop-edge | 4/4/4 | one horizontal edge on the front face, at mid height | Loop Selection (edges) |
| loop-poly | 4/4/4 | one front polygon, on the second row | Loop Selection (polygons, horizontal) |
| ring-edge | 4/4/4 | one horizontal edge on the front face | Ring Selection |
| grow-poly | 4/4/4 | one front polygon in the middle | Grow Selection |
| shrink-poly | 4/4/4 | the whole front face (16 polygons) | Shrink Selection |
| grow-point | 4/4/4 | one point in the middle of the front face | Grow Selection |
| convert-poly-to-point | 2/2/2 | the top 4 polygons | Convert Selection, polygons → points |
| convert-point-to-poly | 2/2/2 | the top 9 points | Convert Selection, points → polygons (record whether "all" or "any" is the rule) |
| convert-edge-to-poly | 2/2/2 | the 4 outer edges of the top face | Convert Selection, edges → polygons |
| select-connected | 2/2/2, then Disconnect the top polygons | one top polygon | Select Connected |
| invert-poly | 2/2/2 | the top 4 polygons | Invert |

## Notes
Observed (6): shrink-poly, convert-poly-to-point, convert-point-to-poly, convert-edge-to-poly,
select-connected, invert-poly. Every case wrote `mesh.obj`, `selection.json`, `case.toml`.

Not observed (5), no output written:
- loop-edge, loop-poly, ring-edge: interactive tools, need the GUI (O10 territory).
- grow-poly, grow-point: the only menu entry is "Grow Selection…", which opens a dialog (it crashed the app in the previous attempt). TODO: needs a human at the GUI.

Results, as read back after the command:
- shrink-poly: the 16 front polygons shrink to the 4 middle ones (5, 6, 9, 10).
- invert-poly: the 4 top polygons (16-19) invert to the other 20; no points or edges selected.
- convert-poly-to-point: the 4 top polygons give the 9 top points (2, 5, 8, 11, 14, 17, 20, 23, 24). The source polygons stay selected.
- convert-point-to-poly: the 9 top points give the 4 top polygons only (tolerant off: a polygon needs all its points; side polygons with 2 selected points are not chosen). The source points stay selected. Historical missing variant resolved on 2026-11-17 by `convert-point-to-poly-tolerant-on` below; the existing baseline fixture is unchanged.
- convert-edge-to-poly: the 8 border segments of the top face give no polygon (a polygon needs all 4 edges selected). The form said "4 outer edges"; at 2/2/2 each side has 2 segments, so all 8 were selected. A separate edges-to-points run (not committed) gave the 8 border points, which confirmed the edge code.
- select-connected: after Disconnect (default settings, no Preserve Groups) every top polygon is its own piece, so selecting 16 and running Select Connected leaves only polygon 16. This case does not show spreading. Historical missing variant resolved on 2026-11-17 by `select-connected-preserve-groups` below; the existing baseline fixture is unchanged.

How commands were run: Shrink, Select Connected and Invert through their menu commands on the active object in polygon mode. Convert Selection and Disconnect through the modeling-command call (Convert has no dialog; its from/to/tolerant values were set explicitly: points 0, edges 1, polygons 2, confirmed by the results above). One command per probe, a ping after each.

## convert-point-to-poly-tolerant-on (2026-11-17)

New named variant permitted by the missing variant in Notes; existing cases are unchanged. Menu-created cube, Segments 2/2/2, made editable. Full input readback in manifest. One modeling/selection command per probe with ping between. Selection result:

```json
{
  "points": [
    2,
    5,
    8,
    11,
    14,
    17,
    20,
    23,
    24
  ],
  "edges": [],
  "polys": [
    2,
    3,
    6,
    7,
    10,
    11,
    14,
    15,
    16,
    17,
    18,
    19
  ]
}
```

`golden info`:

```text
vertices: 26 (unused 0)
faces: 24 (4-gons 24)
bbox min: (-100, -100, -100)
bbox max: (100, 100, 100)
size: (200, 200, 200)
centre: (0, 0, 0)
normals: 6
uvs: 9
```

## select-connected-preserve-groups (2026-11-17)

New named variant permitted by the missing variant in Notes; existing cases are unchanged. Menu-created cube, Segments 2/2/2, made editable. Full input readback in manifest. One modeling/selection command per probe with ping between. Selection result:

```json
{
  "points": [],
  "edges": [],
  "polys": [
    16,
    17,
    18,
    19
  ]
}
```

`golden info`:

```text
vertices: 34 (unused 0)
faces: 24 (4-gons 24)
bbox min: (-100, -100, -100)
bbox max: (100, 100, 100)
size: (200, 200, 200)
centre: (0, 0, 0)
normals: 6
uvs: 9
```

## Safe-scope outcome (2026-11-17)

Two new named variants resolve the explicit missing variants from Notes. Tolerant on selected 12 polygons, exactly the polygons with any top-face input point (checked against the exported geometry); all nine source points remain selected. Disconnect with Preserve Groups increased the mesh from 26 to 34 points; selecting polygon 16 and running Select Connected selected 16, 17, 18, 19, demonstrating spreading across the connected top patch. The six existing fixture folders were not modified. Both successful scratch documents were closed.

Five baseline cases remain explicitly outstanding, and were not retried:
- `loop-edge`: not observed: needs the GUI: interactive Loop/Ring Selection.
- `loop-poly`: not observed: needs the GUI: interactive Loop/Ring Selection.
- `ring-edge`: not observed: needs the GUI: interactive Loop/Ring Selection.
- `grow-poly`: not observed: needs the GUI: only Grow Selection dialog is available; previous run crashed.
- `grow-point`: not observed: needs the GUI: only Grow Selection dialog is available; previous run crashed.

O26 remains partial, not complete. All nine newly created cases across O3 and O26 pass isolated copied-root validation (exit 0, ok: 9 cases). Full golden check exit 1 remains blocked by the unrelated pre-existing fields/restrict-bend duplicate table at line 7; cleanroom exit 0. No commit or push, because the required full golden validator has not passed. The next ten jobs remain queued and were not started.
