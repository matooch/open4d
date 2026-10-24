# Spherify deformer — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-20
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/spherify/<case>/`.

## Setup (every case unless the row says otherwise)
- New scene. A Cube with **size (200, 400, 200) and segments (4, 8, 4)** at the origin
  (record it as `[params.input]`: `type = "cube"`, `size_y = 400.0`,
  `segments_x = 4`, `segments_y = 8`, `segments_z = 4`).
- The Spherify deformer added as a **child of the cube** (that is how the app applies it),
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
Where it lives: Create > Deformer > Spherify; created as a sibling and dragged onto the
cube (Cube > Spherify). Attributes tabs: Basic, Coord., Object, Falloff. One scene for all
cases; each value restored after its export.

**What the export contains:** the deformed cube directly (`o Cube`); no Current State to
Object needed. Because the default strength is 50 % (not 0), the `defaults` export is
already deformed; `input.obj` in every folder is the undeformed input cube taken from
`bend/defaults` (the same cube, exported with a strength-0 Bend).

Object tab:

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Radius | `radius` | float | 200 | 0 (−100 typed → 0) | none seen (9999999 accepted) | cm | |
| Strength | `strength` | float | **50** | 0 (−99999 typed → 0) | **100** (99999 typed → 100) | % | |
| Fit to Parent | – | button | – | – | – | – | not used |

There is **no Size and no Mode** on Spherify (the form's generic "default Size" does not
apply; the Coord. bar shows a 400 × 400 × 400 extent for the sphere gizmo).

Falloff tab (listed only, not used): the same **Field** list as Bend (Name, Blending,
Opacity; Linear Field, Solid, Clamp and one more button).

Enum options (label → our key): none.

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | 162 / 160 (4-gons 160) / size (300, 400, 300), centre (0, 0, 0); normals 230, uvs 45 | default strength is 50 %, so this is deformed: every point lies halfway between its rest place and its projection on the radius-200 sphere (corner (−100, −200, −100) → (−90.82, −181.65, −90.82)) |
| strength | strength = 50 % (or the app's strength param) | 162 / 160 (4-gons 160) / size (300, 400, 300), centre (0, 0, 0); normals 230, uvs 45 | Strength typed as 50 explicitly: **byte-identical to `defaults`** |
| strength-full | strength = 100 % | 162 / 160 (4-gons 160) / size (400, 400, 400), centre (0, 0, 0); normals 162, uvs 45 | every point exactly 200 from the deformer origin (min = max distance 200): a radial projection onto the sphere; corner → (−81.65, −163.30, −81.65) |
| radius | strength = 100 %, radius = 100 | 162 / 160 (4-gons 160) / size (200, 200, 200), centre (0, 0, 0); normals 162, uvs 45 | every point exactly 100 from the origin |
| moved | strength = 100 %, deformer position (50, 100, 0) | 162 / 160 (4-gons 160) / size (400, 400, 400), centre (50, 100, 0); normals 162, uvs 45 | every point exactly 200 from (50, 100, 0): the sphere is centred on the deformer; no distance falloff, points far from the deformer are projected too |

## Behaviour notes
- What the deformer's default Size is, and whether "Fit to Parent" (if offered) was
  needed: Spherify has no Size (Radius 200 instead); Fit to Parent offered, not used.
- Which parameters are greyed out by others: none.
- Anything surprising (points outside the deformer box, mode behaviour): there is no
  box or mode. Strength is clamped to 0 … 100 % and defaults to 50 %. The result is a
  linear blend between each point and its radial projection onto the sphere (centre =
  deformer origin), for every point regardless of distance.
