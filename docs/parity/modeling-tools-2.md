# Modeling tools II — observation (O25)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [modeling-tools.md](modeling-tools.md) (O6):
- A default cube made editable, with the segments given.
- The tool's **Reset Values** before each case.
- Selection by position, recorded as indices or point pairs.
- Export as `tests/golden/modeling-tools/<case>/mesh.obj`, with
  `observation = "docs/parity/modeling-tools-2.md"`.

Fill a settings table for each new tool, as O6 does. Commands with no reachable settings
get a "TODO: not readable" line.

Unblocks: batch 11 (U4 modeling tools).

## Cases

| Case | Cube segments | Selection | Tool + settings | Notes |
|---|---|---|---|---|
| loopcut-mid | 1/1/1 | none | Loop/Path Cut (or the Knife in Loop mode) across the 4 side faces at half height | Record how the cut was placed |
| loopcut-quarter | 1/1/1 | none | the same, at a quarter of the height | |
| knife-line | 1/1/1 | top polygon | Knife in Line mode from the middle of the front edge to the middle of the back edge | If the script can't drive it, write the reason and skip |
| bridge-two | 1/1/1, then delete the top and bottom polygons | the two open border loops | Bridge | |
| weld-two-points | 1/1/1 | points 0 and 1 | Weld (to their centre) | |
| stitch | 2/1/1, then Disconnect polygon 6 | its 4 points and their old neighbours | Stitch and Sew, if it can be driven | Skip if not |
| melt-two | 2/1/1 | the 2 top polygons | Melt | |
| dissolve-edge | 2/1/1 | the middle top edge | Dissolve | |
| split-top | 1/1/1 | top polygon | Split | |
| disconnect-top | 1/1/1 | top polygon | Disconnect, Preserve Groups on | |
| mirror-x | 1/1/1 | all | Mirror across X | |
| close-hole | 1/1/1, then delete the top polygon | – | Close Polygon Hole on the top border | |
| connect-points | 1/1/1 | points 0 and 3 (a diagonal of one face) | Connect Points/Edges | |
| subdivide-smooth | 1/1/1 | all | Subdivide, 1 subdivision, Hyper (smoothing) **on** | |
| subdivide-2 | 1/1/1 | all | Subdivide, 2 subdivisions, smoothing off | |
| smooth | 4/4/4 | all | the Smooth tool or command, defaults | |
| untriangulate | 2/2/2, Triangulate first | all | Untriangulate | |
| center-axis | 1/1/1, moved to (50, 0, 0) by moving all points | – | Center Axis to the objects | Record the object position after |
| array-command | 1/1/1 | all | the Array command, 3 copies, offset (300, 0, 0) | |
| extrude-ngon | 2/1/1 | the 2 top polygons | Extrude, offset 50, Create N-gons **on** | |
| inset-ngon | 2/1/1 | the 2 top polygons | Extrude Inner, offset 20, Create N-gons on | |
| bevel-solid | 1/1/1 | the top-front edge | Bevel, Bevel Mode Solid, offset 20 | |
| bevel-proportional | 1/1/1 | the top-front edge | Bevel, Offset Mode Proportional, offset 30 % | |
| bevel-mitering-uniform | 1/1/1 | the top 4 edges | Bevel, offset 20, subdivision 2, Mitering Uniform | |

## Notes
TODO
