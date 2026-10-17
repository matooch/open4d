# Boolean generator — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-17
Unblocks: booleans. Folder: `tests/golden/boole/<case>/`.

Setup for every case: object A = default cube (200 cm, at the origin); object B =
default cube at position (50, 50, 50) cm (typed in its Coord. tab; unit cm). Both under
the boolean generator, A first. Export the generator's result.

Generator: Create > Modeling > "Boole". Each case: File > New, add cube A ("Cube"),
add cube B ("Cube.1") and set its position, add Boole, drag B onto Boole, then drag A
onto Boole (a dropped object becomes the first child), so the Objects list shows
Boole > Cube, Cube.1. Options set in the Boole's Attributes "Object" tab (tabs: Basic,
Coord., Object). Export settings as in [export-settings.md](export-settings.md).

**What the export contains:** only the boolean result, no separate copies of the
inputs. With the default "Create single object" off, the result is written as
**two `o` groups named after the inputs**, `o Cube` and `o Cube.1`, sharing one vertex
list: the faces kept from A are in `Cube`, the new faces coming from B's surface are
in `Cube.1`. (mode-a_minus_b: `Cube` has 9 faces, `Cube.1` 3 faces; the file lists 13
vertices before `o Cube` and 1 more vertex (-50, -50, -50) before `o Cube.1`.)
A without B has only `o Cube`. No "Current State to Object" step was used.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Notes |
|---|---|---|---|---|
| Boolean Type | `mode` | enum | A subtract B | 4 options, listed below |
| High Quality | `high_quality` | bool | on | when off, the next four checkboxes are greyed out |
| Hide new edges | `hide_new_edges` | bool | off | |
| Create single object | `single_object` | bool | off | not used in any case |
| Create Phong breaks at intersections | `phong_breaks` | bool | off | not used |
| Select intersections | `select_intersections` | bool | off | not used |
| Optimize Points | `optimize_points` | float | 0.01 cm | greyed out with the defaults; not used |

Mode options (label → our key), dropdown order: `A union B` → `union`,
`A subtract B` → `a_minus_b` (default), `A intersect B` → `intersect`,
`A without B` → `a_without_b`.

## Cases

| Case | Settings | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| mode-union | A union B | 20 / 18 (4-gons 18) / size (250, 250, 250), centre (25, 25, 25) | objects `Cube`, `Cube.1`; normals 6, uvs 16 |
| mode-a_minus_b | A subtract B (default) | 14 / 12 (4-gons 12) / size (200, 200, 200), centre (0, 0, 0) | objects `Cube`, `Cube.1`; new corner vertex (-50, -50, -50); normals 6, uvs 16 |
| mode-intersect | A intersect B | 8 / 6 (4-gons 6) / size (150, 150, 150), centre (25, 25, 25) | objects `Cube`, `Cube.1`; bbox (-50, -50, -50)..(100, 100, 100) |
| mode-a_without_b | A without B | 13 / 9 (4-gons 9) / size (200, 200, 200), centre (0, 0, 0) | only object `Cube`; uvs 10. Same 13 vertices and the same 9 faces (vertex indices) as the `Cube` group of mode-a_minus_b; B's 3 faces are missing (left open). |
| quality-off | A subtract B, High Quality off | 35 / 35 (3-gons 27, 4-gons 8) / size (200.0044937133789, 200.00599670410156, 200), centre (0.002750396728515625, 0.0019989013671875, 0.0001983642578125) | objects `Cube`, `Cube.1`; triangulated, and coordinates off by up to about 0.005 (bbox (-99.99949645996094, -100.0009994506836, -99.99980163574219)..(100.00499725341797, 100.00499725341797, 100.00019836425781)); normals 10, uvs 26 |
| hide-edges | A subtract B, Hide new edges on | 14 / 9 (4-gons 6, 6-gons 3) / size (200, 200, 200), centre (0, 0, 0) | objects `Cube`, `Cube.1`; the three cut faces of A become 6-sided n-gons (e.g. `f 10/6/2 5/4/2 3/1/2 4/2/2 8/7/2 9/5/2`), so the exporter **does** write faces with more than 4 corners |

## Behaviour notes
- What happens when the objects don't overlap / touch exactly: observed in the
  viewport only (no export), A subtract B with Hide new edges on: with B moved to
  (500, 0, 0) (no overlap) and to (200, 0, 0) (B's face exactly on A's +X face), the
  viewport shows A as a plain whole cube and B only as its selection outline.
- With Create single object off, the result keeps one group per input in the OBJ.
- High Quality off triangulates the result and introduces small coordinate errors.
