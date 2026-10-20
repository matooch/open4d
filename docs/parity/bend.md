# Bend deformer — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Exporter settings: [export-settings.md](export-settings.md) (note any deviation here).
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
| TODO | TODO | | | | | | |

Enum options (label → our key): TODO

## Cases

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – (note the default strength: if it is 0 the export equals the cube; keep the case anyway) | | |
| strength | strength = 90° | | |
| strength-180 | strength = 180° | | |
| angle | strength = 90°, angle (direction) = 45° | | |
| mode-unlimited | strength = 90°, mode = Unlimited | | |
| mode-box | strength = 90°, mode = Within Box | | |
| size | strength = 90°, size = (100, 200, 100) | | |
| keep-length | strength = 90°, Keep Y-axis length off (or the app's equivalent, if offered) | | |
| moved | strength = 90°, deformer position (0, 100, 0) | | |
| rotated | strength = 90°, deformer rotation H 0, P 0, B 90 | | |

## Behaviour notes
- What the deformer's default Size is, and whether "Fit to Parent" (if offered) was
  needed: TODO
- Which parameters are greyed out by others: TODO
- Anything surprising (points outside the deformer box, mode behaviour): TODO
