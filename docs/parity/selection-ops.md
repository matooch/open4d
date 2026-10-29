# Selection commands — observation (O26)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
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
TODO
