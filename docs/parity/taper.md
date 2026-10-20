# Taper deformer — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-20
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/taper/<case>/`.

## Setup (every case unless the row says otherwise)
- New scene. A Cube with **size (200, 400, 200) and segments (4, 8, 4)** at the origin
  (record it as `[params.input]`: `type = "cube"`, `size_y = 400.0`,
  `segments_x = 4`, `segments_y = 8`, `segments_z = 4`).
- The Taper deformer added as a **child of the cube** (that is how the app applies it),
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
Where it lives: Create > Deformer > Taper; created as a sibling and dragged onto the cube
(Cube > Taper). Attributes tabs: Basic, Coord., Object, Falloff. One scene for all cases;
each value restored after its export.

**What the export contains:** the deformed cube directly (`o Cube`); no Current State to
Object needed. `input.obj` in every folder is the undeformed input cube (same file as
`bend/defaults`, and byte-identical to this form's `defaults`).

Object tab:

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Size (X, Y, Z) | `size` | vec3 | (250, 250, 250) | not tested (Bend's clamps at 0) | not tested | cm | |
| Mode | `mode` | enum | Limited | – | – | – | Limited, Within Box, Unlimited |
| Strength | `strength` | float | 0 | none seen (−99999 accepted) | none seen (99999 accepted) | % | positive shrinks toward the top |
| Curvature | `curvature` | float | **100** | 0 (−99999 typed → 0) | none seen (99999 accepted) | % | |
| Fillet | `fillet` | bool | off | – | – | – | added case `fillet` |
| Fit to Parent | – | button | – | – | – | – | not used |

Falloff tab (listed only, not used): the same **Field** list as Bend (Name, Blending,
Opacity; Linear Field, Solid, Clamp and one more button).

Enum options (label → our key): `Limited` → `limited` (default), `Within Box` →
`within_box`, `Unlimited` → `unlimited`.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 6, uvs 45 | strength 0: the undeformed cube |
| strength | strength = 50 % | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 46, uvs 45 | bbox unchanged (the bottom ring keeps full size); XZ scale per row from y −200 up: 1, 1, 0.995, 0.955, 0.875, 0.755, 0.595, 0.5, 0.5 — fits 1 − 0.5·t² with t = (y + 125) / 250 inside the box, 0.5 above it |
| negative | strength = −50 % | 162 / 160 (4-gons 160) / size (300, 400, 300), centre (0, 0, 0); normals 42, uvs 45 | widens toward the top: scale 1 … 1.5 (rows 1, 1, 1.005, 1.045, 1.125, 1.245, 1.405, 1.5, 1.5) |
| curvature | strength = 50 %, curvature = 50 % | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 38, uvs 45 | scale rows 1, 1, 0.9725, 0.9025, 0.8125, 0.7025, 0.5725, 0.5, 0.5 — fits 1 − 0.5·(0.5·t + 0.5·t²): Curvature blends linear (0 %) and quadratic (100 %) |
| mode-unlimited | strength = 50 %, mode = Unlimited | 162 / 160 (4-gons 160) / size (199, 400, 199), centre (0, 0, 0); normals 50, uvs 45 | the quadratic continues outside the box: y −200 → 0.955, y 200 → 0.155 |
| mode-box | strength = 50 %, mode = Within Box | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 50, uvs 45 | inside the box as `strength`; the two rows above the box are not scaled (back to 1) |
| size | strength = 50 %, size = (100, 200, 100) | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 30, uvs 45 | box y −100 … 100: rows 1, 1, 1, 0.96875, 0.875, 0.71875, 0.5, 0.5, 0.5 |
| fillet | strength = 50 %, Fillet **on** (added: option not listed) | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 38, uvs 45 | scale rows 1, 1, 0.986, 0.892, 0.75, 0.608, 0.514, 0.5, 0.5 — fits 1 − 0.5·(3t² − 2t³) (smooth start and end) |

## Behaviour notes
- What the deformer's default Size is, and whether "Fit to Parent" (if offered) was
  needed: default Size (250, 250, 250); Fit to Parent offered, not used.
- Which parameters are greyed out by others: none.
- Anything surprising (points outside the deformer box, mode behaviour): the scale
  is applied in X and Z only (Y unchanged), anchored at the bottom of the box (scale 1 at
  y = −size_y/2, 1 − strength at the top). Curvature defaults to 100 %. Limited holds the
  top value above the box; Within Box leaves points outside unchanged; Unlimited
  extrapolates the curve. The formulas above are fits to the exported numbers, not app
  documentation.
