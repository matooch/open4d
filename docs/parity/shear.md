# Shear deformer — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-20
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/shear/<case>/`.

## Setup (every case unless the row says otherwise)
- New scene. A Cube with **size (200, 400, 200) and segments (4, 8, 4)** at the origin
  (record it as `[params.input]`: `type = "cube"`, `size_y = 400.0`,
  `segments_x = 4`, `segments_y = 8`, `segments_z = 4`).
- The Shear deformer added as a **child of the cube** (that is how the app applies it),
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
Where it lives: Create > Deformer > Shear; created as a sibling and dragged onto the cube
(Cube > Shear). Attributes tabs: Basic, Coord., Object, Falloff. One scene for all cases;
each value restored after its export.

**What the export contains:** the deformed cube directly (`o Cube`); no Current State to
Object needed. `input.obj` in every folder is the undeformed input cube (same file as
`bend/defaults`, byte-identical to this form's `defaults`).

Object tab:

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Size (X, Y, Z) | `size` | vec3 | (250, 250, 250) | not tested (Bend's clamps at 0) | not tested | cm | |
| Mode | `mode` | enum | Limited | – | – | – | Limited, Within Box, Unlimited |
| Strength | `strength` | float | 0 | none seen (−99999 accepted) | none seen (99999 accepted) | % | |
| Angle | `angle` | float | 0 | none seen (−99999 accepted) | none seen (99999 accepted) | ° | shear direction around the deformer's Y axis |
| Curvature | `curvature` | float | **100** | 0 (−99999 typed → 0) | none seen (99999 accepted) | % | |
| Fillet | `fillet` | bool | off | – | – | – | added case `fillet` |
| Fit to Parent | – | button | – | – | – | – | not used |

Falloff tab (listed only, not used): the same **Field** list as Bend (Name, Blending,
Opacity; Linear Field, Solid, Clamp and one more button).

Enum options (label → our key): `Limited` → `limited` (default), `Within Box` →
`within_box`, `Unlimited` → `unlimited`.

In the notes "rows" are the X of the column x = −100, z = −100 at y = −200, −150, …, 200.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 6, uvs 45 | strength 0: the undeformed cube |
| strength | strength = 50 % | 162 / 160 (4-gons 160) / size (262.5, 400, 200), centre (31.25, 0, 0); normals 24, uvs 45 | rows −100, −100, −99.375, −94.375, −84.375, −69.375, −49.375, −37.5, −37.5: +X offset 62.5·t² (t = (y + 125) / 250; 62.5 = 50 % × 125); Y and Z unchanged |
| angle | strength = 50 %, angle = 45° | 162 / 160 (4-gons 160) / size (244.1941680908203, 400, 244.1941680908203), centre (22.097084045410156, 0, 22.097084045410156); normals 44, uvs 45 | same offset turned 45° toward +Z: X and Z both shift by up to 44.19 (62.5·cos 45°) |
| curvature | strength = 50 %, curvature = 50 % | 162 / 160 (4-gons 160) / size (262.5, 400, 200), centre (31.25, 0, 0); normals 22, uvs 45 | rows −100, −100, −96.5625, −87.8125, −76.5625, −62.8125, −46.5625, −37.5, −37.5: fits 62.5·(0.5·t + 0.5·t²) |
| mode-unlimited | strength = 50 %, mode = Unlimited | 162 / 160 (4-gons 160) / size (305, 400, 200), centre (53.125, 0, 0); normals 26, uvs 45 | the parabola continues: y −200 → +5.625, y 200 → +105.625 |
| mode-box | strength = 50 %, mode = Within Box | 162 / 160 (4-gons 160) / size (250.625, 400, 200), centre (25.3125, 0, 0); normals 24, uvs 45 | inside the box as `strength`; the two rows above the box are not moved |
| fillet | strength = 50 %, Fillet **on** (added: option not listed) | 162 / 160 (4-gons 160) / size (262.5, 400, 200), centre (31.25, 0, 0); normals 20, uvs 45 | rows −100, −100, −98.25, −86.5, −68.75, −51, −39.25, −37.5, −37.5: fits 62.5·(3t² − 2t³) |

## Behaviour notes
- What the deformer's default Size is, and whether "Fit to Parent" (if offered) was
  needed: default Size (250, 250, 250); Fit to Parent offered, not used.
- Which parameters are greyed out by others: none.
- Anything surprising (points outside the deformer box, mode behaviour): the shear
  offset grows from 0 at the bottom of the box to strength × size_y/2 at the top
  (direction +X turned by Angle about Y). Limited holds the full offset above the box;
  Within Box leaves points outside unmoved; Unlimited extrapolates. Curvature defaults to
  100 %. Formulas are fits to the exported numbers.
