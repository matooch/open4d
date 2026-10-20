# Bend deformer — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-20
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/bend/<case>/`.

## Setup (every case unless the row says otherwise)
- New scene. A Cube with **size (200, 400, 200) and segments (4, 8, 4)** at the origin
  (record it as `[params.input]`: `type = "cube"`, `size_y = 400.0`,
  `segments_x = 4`, `segments_y = 8`, `segments_z = 4`).
- The Bend deformer added as a **child of the cube** (that is how the app applies it),
  at its default position/rotation unless the row says otherwise.
- Export the **cube** (selected; the deformer comes with it). If the export writes the
  undeformed cube, note it and export a Current State to Object copy instead, saying so.
- Record the deformer's own params in `[params]` and its transform as `pos` / `rot`
  (degrees, H P B as shown) in `[params]` too.

## Parameters (as shown in the app)

Fill every parameter of the deformer's Object tab (label, default, min/max by typing a
huge and a negative value, unit, enum options), plus anything on a Falloff/Fields tab
(just list it; don't use it). Lowercase snake_case keys; use them in `case.toml`.

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
Where it lives: Create > Deformer > Bend. It is created as a sibling above the cube and was
dragged onto the cube in the Objects list (Cube > Bend). Attributes tabs: Basic, Coord.,
Object, Falloff. One scene for all cases; each value was restored after its export.

**What the export contains:** the deformed cube directly (`o Cube`); no Current State to
Object needed. `input.obj` (in every folder) is the `defaults` export, which is the
undeformed input cube because the default strength is 0.

Object tab:

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Size (X, Y, Z) | `size` | vec3 | (250, 250, 250) | 0 (−100 typed → 0) | none seen (9999999 accepted) | cm | the deformer box, centred on the deformer origin |
| Mode | `mode` | enum | Limited | – | – | – | Limited, Within Box, Unlimited |
| Strength | `strength` | float | 0 | none seen (−99999 accepted) | none seen (99999 accepted) | ° | bend angle |
| Angle | `angle` | float | 0 | none seen (−99999 accepted) | none seen (99999 accepted) | ° | direction of the bend around the deformer's Y axis |
| Keep Y-Axis Length | `keep_y_axis_length` | bool | **off** | – | – | – | |
| Fit to Parent | – | button | – | – | – | – | greyed while the deformer has no parent; not used |

Falloff tab (listed only, not used): a **Field** list (columns Name, Blending, Opacity;
empty, "Double click to create a New Field") with quick-add buttons Linear Field, Solid,
Clamp and one more icon button.

Coord. tab: P / S / R (H, P, B) like any object; `pos` / `rot` in `case.toml` are P and R.

Enum options (label → our key): `Limited` → `limited` (default), `Within Box` →
`within_box`, `Unlimited` → `unlimited`.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – (note the default strength: if it is 0 the export equals the cube; keep the case anyway) | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 6, uvs 45 | default strength is 0, so this is the undeformed cube (used as `input.obj`) |
| strength | strength = 90° | 162 / 160 (4-gons 160) / size (459.1549377441406, 459.1549377441406, 200), centre (129.5774688720703, 29.577468872070313, 0); bbox min (-100, -200, -100), max (359.1549377441406, 259.1549377441406, 100) | the part below the box (y < −125) is untouched; the box section bends toward +X; the part above the box continues straight (Limited) |
| strength-180 | strength = 180° | 162 / 160 (4-gons 160) / size (609.1549377441406, 379.5774688720703, 200), centre (204.5774688720703, -10.211265563964844, 0); bbox min (-100, -200, -100), max (509.1549377441406, 179.5774688720703, 100) | the top of the cube ends up pointing down (−Y) |
| angle | strength = 90°, angle (direction) = 45° | 162 / 160 (4-gons 160) / size (453.9609069824219, 500.5762939453125, 453.9609069824219), centre (126.98045349121094, 50.28814697265625, 126.98045349121094); bbox min (-100, -200, -100), max (353.9609069824219, 300.5762939453125, 353.9609069824219) | bend direction turned 45° about Y: the X and Z sizes are equal |
| mode-unlimited | strength = 90°, mode = Unlimited | 162 / 160 (4-gons 160) / size (553.8280639648438, 553.8280334472656, 200), centre (181.64361572265625, -22.488662719726563, 0); bbox min (-95.27041625976563, -299.4026794433594, -100), max (458.5576477050781, 254.42535400390625, 100) | the whole cube follows the arc, including the part below the box (min Y −299.4) |
| mode-box | strength = 90°, mode = Within Box | 162 / 160 (4-gons 160) / size (355.3467559814453, 454.42535400390625, 200), centre (77.67337799072266, 27.212677001953125, 0); bbox min (-100, -200, -100), max (255.3467559814453, 254.42535400390625, 100) | only points inside the box deform; the part above is not carried along |
| size | strength = 90°, size = (100, 200, 100) | 162 / 160 (4-gons 160) / size (377.3239440917969, 377.32395935058594, 200), centre (88.66197204589844, -11.338020324707031, 0); bbox min (-100, -200, -100), max (277.3239440917969, 177.32395935058594, 100) | – |
| keep-length | strength = 90°, Keep Y-Axis Length **on** (it is off by default, so every other case has it off) | 162 / 160 (4-gons 160) / size (334.1549377441406, 334.1549377441406, 200), centre (67.07746887207031, -32.92253112792969, 0); bbox min (-100, -200, -100), max (234.15493774414063, 134.15493774414063, 100) | smaller result than `strength` |
| moved | strength = 90°, deformer position (0, 100, 0) | 162 / 160 (4-gons 160) / size (355.3467559814453, 554.4253540039063, 200), centre (77.67337799072266, 77.21267700195313, 0); bbox min (-100, -200, -100), max (255.3467559814453, 354.42535400390625, 100) | same X extent as mode-box |
| rotated | strength = 90°, deformer rotation H 0, P 0, B 90 | 162 / 160 (4-gons 160) / size (465.0294494628906, 465.0294494628906, 200), centre (120.67945861816406, -38.47547912597656, 0); bbox min (-111.83526611328125, -270.9902038574219, -100), max (353.1941833496094, 194.03924560546875, 100) | the bend follows the deformer's rotation |

## Behaviour notes
- What the deformer's default Size is, and whether "Fit to Parent" (if offered) was
  needed: default Size (250, 250, 250); Fit to Parent is offered (greyed until the
  deformer is a child) and was **not** used.
- Which parameters are greyed out by others: none on the Object tab.
- Anything surprising (points outside the deformer box, mode behaviour): the bend is
  anchored at the **bottom** of the box (y = −size_y/2 in deformer space); points below
  the box stay put in Limited and Within Box. Limited carries the part above the box
  along rigidly; Within Box deforms only the points inside the box; Unlimited bends every
  point along the arc. The deformation is in the OBJ export itself. Strength and Angle
  had no clamp at ±99999; Size clamps at 0.
