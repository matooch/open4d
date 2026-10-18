# Array generator — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/array/<case>/`.

## Where it lives
Create > Modeling > Array (new scene each case: File > New; in the last two cases the
same scene was reused with only the Array values changed). Attributes tabs: Basic,
Coord., Object.

Input: one default Cube with Size 50 / 50 / 50 as the child of the generator, at the
origin.

**What the export contains:** the result is written directly (no Current State to
Object needed), as one `o` group per copy, named `Cube.N` … `Cube.1` (highest number first;
e.g. `Cube.8` … `Cube.1` for 8 cubes). `input.obj` = the child cube alone, exported with the generator disabled
(same for all cases).

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Radius | `radius` | float | 250 | not tested | not tested | cm | |
| Copies | `copies` | int | 7 | not tested | not tested | – | **the result has Copies + 1 objects** (7 → 8 cubes) |
| Amplitude | `amplitude` | float | 0 | not tested | not tested | cm | vertical wave |
| Frequency | `frequency` | float | 0 | not tested | not tested | – | wave speed (animation) |
| Array Frequency | `array_frequency` | float | 10 | not tested | not tested | – | number of wave periods around the ring |
| Render Instance | `render_instance` | bool | off | – | – | – | not used |

No partial / end-angle option exists, so the **partial** row was skipped.

## Cases

| Case | Inputs and parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | cube child size (50, 50, 50) | 64 / 48 (4-gons 48) / size (550, 50, 550), centre (0, 0, 0); normals 18, uvs 4 | 8 cubes at 45° steps; the first written cube is centred at (0, 0, 250) (first vertex (-25, -25, 225)), the next at 45° towards +X; copies are rotated to face the centre |
| copies | copies = 5, radius = 300, cube size (50, 50, 50) | 48 / 36 (4-gons 36) / size (587.91650390625, 50, 650), centre (0, 0, 0); normals 14, uvs 4 | 6 cubes at 60° steps; objects `Cube.6` … `Cube.1` |
| wave | amplitude = 50, frequency = 2, cube size (50, 50, 50) | 64 / 48 (4-gons 48) / size (550, 150, 550), centre (0, 0, 0); normals 18, uvs 4 | at frame 0 the cubes are lifted by 0, +50, 0, -50, 0, +50, 0, -50 cm in output order |
| partial | – | skipped | no such option in this version |

## Behaviour notes
- Which parameters are greyed out or hidden by others: none seen.
- Anything surprising: Copies counts the extra copies, so N copies give N + 1 objects,
  evenly spaced over 360°. Each copy is a separate `o` group in the OBJ. The wave height
  at a given copy depends on Array Frequency (10 by default) and the current frame.
