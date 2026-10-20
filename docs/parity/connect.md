# Connect generator — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/connect/<case>/`.

## Where it lives
Create > Modeling > Connect. Each scene: File > New, add cube A ("Cube") and cube B
("Cube.1"), move B, add Connect, drag B then A onto it (Objects list: Connect > Cube,
Cube.1). The weld-off / order / apart cases reused the defaults scene with only the listed
change. Attributes tabs: Basic, Coord., Object.

Inputs: two default Cubes as children of the generator (first listed first).

**What the export contains:** the connected result directly, as **one object named after
the first child** (`o Cube`, or `o Cube.1` when Cube.1 is first); no Current State to
Object needed. Input meshes were exported with Connect (and the other cube) disabled:
`input.obj` = first child, `input2.obj` = second child, each in world position. With only
Connect disabled, the export holds both cubes as two `o` groups sharing a vertex list
(not used as a golden).

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Objects | `objects` | link list | empty | – | – | – | optional explicit list; empty = use the children |
| Weld | `weld` | bool | on | – | – | – | |
| Tolerance | `tolerance` | float | 0.1 | not tested | not tested | cm | |
| Phong Mode | `phong_mode` | enum | As Breaks | – | – | – | options: Manual, Average, Lowest, Highest, As Breaks |
| Textures | `textures` | bool | on | – | – | – | |
| Center Axis | `center_axis` | bool | off | – | – | – | |

Enum options (label → our key): `Manual` → `manual`, `Average` → `average`,
`Lowest` → `lowest`, `Highest` → `highest`, `As Breaks` → `as_breaks` (default).

## Cases

| Case | Inputs and parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | cubes at (0, 0, 0) and (200, 0, 0) (sharing a face), weld on | 12 / 11 (4-gons 11) / size (400, 200, 200), centre (100, 0, 0); normals 6, uvs 4 | the 4 shared points are merged and **one** of the two coincident faces at x = 100 is dropped (A keeps 5 faces, B all 6) |
| weld-off | as defaults, weld off | 12 / 12 (4-gons 12) / size (400, 200, 200), centre (100, 0, 0) | the shared points are still merged (12 vertices) but both faces at x = 100 remain |
| apart | cubes at (0, 0, 0) and (300, 0, 0), weld on | 16 / 12 (4-gons 12) / size (500, 200, 200), centre (150, 0, 0) | nothing to weld |
| order | as defaults, but the child order swapped in the object tree | 12 / 11 (4-gons 11) / size (400, 200, 200), centre (100, 0, 0) | object named `Cube.1`; its vertices come first (first vertex (100, -100, -100)) |

## Behaviour notes
- Which parameters are greyed out or hidden by others: none seen.
- Anything surprising: coincident points are shared between the children even with Weld
  off (same as the cube inputs exported together); Weld additionally removes the
  duplicate face. The output object takes the first child's name, and the first child's
  points and faces come first.
