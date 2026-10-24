# Bulge deformer — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-20
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/bulge/<case>/`.

## Setup (every case unless the row says otherwise)
- New scene. A Cube with **size (200, 400, 200) and segments (4, 8, 4)** at the origin
  (record it as `[params.input]`: `type = "cube"`, `size_y = 400.0`,
  `segments_x = 4`, `segments_y = 8`, `segments_z = 4`).
- The Bulge deformer added as a **child of the cube** (that is how the app applies it),
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
Where it lives: Create > Deformer > Bulge; created as a sibling and dragged onto the cube
(Cube > Bulge). Attributes tabs: Basic, Coord., Object, Falloff. One scene for all cases;
each value restored after its export. The Object tab has the same fields as Taper.

**What the export contains:** the deformed cube directly (`o Cube`); no Current State to
Object needed. `input.obj` in every folder is the undeformed input cube (same file as
`bend/defaults`, byte-identical to this form's `defaults`).

Object tab:

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Size (X, Y, Z) | `size` | vec3 | (250, 250, 250) | not tested (Bend's clamps at 0) | not tested | cm | |
| Mode | `mode` | enum | Limited | – | – | – | Limited, Within Box, Unlimited |
| Strength | `strength` | float | 0 | none seen (−99999 accepted) | none seen (99999 accepted) | % | |
| Curvature | `curvature` | float | **100** | 0 (−99999 typed → 0) | none seen (99999 accepted) | % | |
| Fillet | `fillet` | bool | off | – | – | – | added case `fillet` |
| Fit to Parent | – | button | – | – | – | – | not used |

Falloff tab (listed only, not used): the same **Field** list as Bend (Name, Blending,
Opacity; Linear Field, Solid, Clamp and one more button).

Enum options (label → our key): `Limited` → `limited` (default), `Within Box` →
`within_box`, `Unlimited` → `unlimited`.

In the notes below "rows" are the X of the column x = −100, z = −100 at y = −200, −150,
…, 200 (the scale is the same in X and Z, Y is unchanged).

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 6, uvs 45 | strength 0: the undeformed cube |
| strength | strength = 50 % | 162 / 160 (4-gons 160) / size (300, 400, 300), centre (0, 0, 0); normals 30, uvs 45 | rows −100, −100, −118, −142, −150, −142, −118, −100, −100: scale 1 + 0.5·(1 − u²) with u = y / 125; 1 at the box ends and outside |
| negative | strength = −50 % | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 30, uvs 45 | pinches: rows −100, −100, −82, −58, −50, −58, −82, −100, −100 |
| curvature | strength = 50 %, curvature = 50 % | 162 / 160 (4-gons 160) / size (300, 400, 300), centre (0, 0, 0); normals 38, uvs 45 | rows −100, −100, −114, −136, −150, −136, −114, −100, −100: fits 1 + 0.5·(0.5·(1 − u²) + 0.5·(1 − abs(u))) |
| mode-unlimited | strength = 50 %, mode = Unlimited | 162 / 160 (4-gons 160) / size (300, 400, 300), centre (0, 0, 0); normals 38, uvs 45 | the parabola continues outside the box: rows −22, −78, −118, …, −78, −22 (y ±200 → scale 0.22) |
| mode-box | strength = 50 %, mode = Within Box | 162 / 160 (4-gons 160) / size (300, 400, 300), centre (0, 0, 0); normals 30, uvs 45 | **byte-identical to `strength`**: the bulge is already 0 at the box ends, so Limited and Within Box give the same result here |
| fillet | strength = 50 %, Fillet **on** (added: option not listed) | 162 / 160 (4-gons 160) / size (300, 400, 300), centre (0, 0, 0); normals 62, uvs 45 | rows −100, −100, −105.2, −132.4, −150, −132.4, −105.2, −100, −100: fits 1 + 0.5·smoothstep(1 − abs(u)) |

## Behaviour notes
- What the deformer's default Size is, and whether "Fit to Parent" (if offered) was
  needed: default Size (250, 250, 250); Fit to Parent offered, not used.
- Which parameters are greyed out by others: none.
- Anything surprising (points outside the deformer box, mode behaviour): the bulge is
  centred on the box (largest at y = 0, zero at y = ±size_y/2), scaling X and Z only.
  Limited and Within Box are identical for this input; Unlimited extrapolates and shrinks
  the points beyond the box. Curvature defaults to 100 %. The formulas are fits to the
  exported numbers.
