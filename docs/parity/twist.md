# Twist deformer — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-20
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/twist/<case>/`.

## Setup (every case unless the row says otherwise)
- New scene. A Cube with **size (200, 400, 200) and segments (4, 8, 4)** at the origin
  (record it as `[params.input]`: `type = "cube"`, `size_y = 400.0`,
  `segments_x = 4`, `segments_y = 8`, `segments_z = 4`).
- The Twist deformer added as a **child of the cube** (that is how the app applies it),
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
Where it lives: Create > Deformer > Twist; created as a sibling above the cube and dragged
onto it (Cube > Twist). Attributes tabs: Basic, Coord., Object, Falloff. One scene for all
cases; each value restored after its export.

**What the export contains:** the deformed cube directly (`o Cube`); no Current State to
Object needed. `input.obj` in every folder is the undeformed input cube (byte-identical to
this form's `defaults` export and to `bend/defaults`).

Object tab:

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Size (X, Y, Z) | `size` | vec3 | (250, 250, 250) | not tested (Bend's clamps at 0) | not tested | cm | |
| Mode | `mode` | enum | Limited | – | – | – | Limited, Within Box, Unlimited |
| Angle | `angle` | float | 0 | none seen (−99999 accepted) | none seen (99999 accepted) | ° | total twist over the box height |
| Fit to Parent | – | button | – | – | – | – | not used |

There is no strength parameter: Angle is the only amount.

Falloff tab (listed only, not used): the same **Field** list as Bend (columns Name,
Blending, Opacity; "Double click to create a New Field") with Linear Field, Solid, Clamp
and one more icon button.

Enum options (label → our key): `Limited` → `limited` (default), `Within Box` →
`within_box`, `Unlimited` → `unlimited`.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 6, uvs 45 | angle 0: the undeformed cube |
| angle | angle = 90° | 162 / 160 (4-gons 160) / size (282.84271240234375, 400, 282.84271240234375), centre (0, 0, 0); bbox min (-141.42135620117188, -200, -141.42135620117188), max (141.42135620117188, 200, 141.42135620117188) | twist about the deformer's Y axis, linear in Y over the box (y −125 … 125): at y = 0 the corner (−100, 0, −100) moves to (0, 0, −141.42); below the box unchanged, above it the full 90° (corner (−100, 200, −100) → (100, 200, −100)) |
| angle-big | angle = 360° | 162 / 160 (4-gons 160) / size (279.3604431152344, 400, 279.3604431152344), centre (0, 0, 0); bbox min (-139.6802215576172, -200, -139.6802215576172), max (139.6802215576172, 200, 139.6802215576172) | at y = 0 the corner (−100, 0, −100) → (100, 0, 100) (half of 360°); above the box the full turn brings the points back to their rest place |
| mode-unlimited | angle = 90°, mode = Unlimited | 162 / 160 (4-gons 160) / size (282.84271240234375, 400, 282.84271240234375), centre (0, 0, 0); bbox min (-141.42135620117188, -200, -141.42135620117188), max (141.42135620117188, 200, 141.42135620117188) | the twist rate continues outside the box: corner (−100, −200, −100) → (−134.50, −200, −43.70) |
| mode-box | angle = 90°, mode = Within Box | 162 / 160 (4-gons 160) / size (282.84271240234375, 400, 282.84271240234375), centre (0, 0, 0); bbox min (-141.42135620117188, -200, -141.42135620117188), max (141.42135620117188, 200, 141.42135620117188) | inside the box as `angle`; the points above the box are **not** rotated (stay at rest) |
| size | angle = 90°, size = (100, 200, 100) | 162 / 160 (4-gons 160) / size (282.84271240234375, 400, 282.84271240234375), centre (0, 0, 0); bbox min (-141.42135620117188, -200, -141.42135620117188), max (141.42135620117188, 200, 141.42135620117188) | box y −100 … 100: same 90° over a shorter height; above it the full 90° as in Limited |
| moved | angle = 90°, deformer position (0, 100, 0) | 162 / 160 (4-gons 160) / size (282.84271240234375, 400, 282.84271240234375), centre (0, 0, 0); bbox min (-141.42135620117188, -200, -141.42135620117188), max (141.42135620117188, 200, 141.42135620117188) | box y −25 … 225: points up to y −50 unchanged, y = 100 (box centre) turned 45° |

## Behaviour notes
- What the deformer's default Size is, and whether "Fit to Parent" (if offered) was
  needed: default Size (250, 250, 250); Fit to Parent offered, not used.
- Which parameters are greyed out by others: none.
- Anything surprising (points outside the deformer box, mode behaviour): the twist
  starts at the bottom of the box (0° at y = −size_y/2) and grows linearly to Angle at
  the top. Limited: below the box 0°, above it the full Angle. Within Box: only points
  inside the box move. Unlimited: the same rate continues beyond the box both ways. The
  bounding boxes of angle, mode-*, size and moved are identical (the widest section is the
  45° one), but the files differ.
