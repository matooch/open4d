# Fields III — observation (O23)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: exactly as [fields.md](fields.md) (O7) and [fields-2.md](fields-2.md) (O13): field
objects made through their menu commands, sampled on the same 729-point grid, value channel.
Save `tests/golden/fields/<case>/samples.json`, with
`observation = "docs/parity/fields-3.md"`.

Unblocks: P2 layers and blend modes.

**Overlapping layers:** use two Spherical fields of size **200** (not 100), at (−75, 0, 0)
and (75, 0, 0), so they overlap on the grid. O13's cases didn't overlap, which is why Max
and Add came out the same.

## Cases

| Case | Layers, top to bottom | Changed | Notes |
|---|---|---|---|
| blend-normal | B (75, 0, 0) over A (−75, 0, 0) | – | Size 200 each. 7 ones, 74 between, 648 zeros. B alone: Normal on the top layer replaces what is below |
| blend-max | the same | top blend Max | 13 ones, 110 between, 606 zeros |
| blend-min | the same | top blend Min | 1 one, 38 between, 690 zeros |
| blend-add | the same | top blend Add | 29 ones, 94 between, 606 zeros (clamped to 1) |
| blend-subtract | the same | top blend Subtract | 1 one, 50 between, 678 zeros |
| blend-multiply | the same | top blend Multiply | 1 one, 38 between, 690 zeros |
| blend-overlay | the same | top blend Overlay | 8 ones, 41 between, 681 zeros |
| blend-screen | the same | top blend Screen | 13 ones, 110 between, 606 zeros; same counts as Max |
| blend-clip | the same | top blend Clip | 1 one, 38 between, 690 zeros; same counts as Min and Multiply |
| blend-normal-50 | the same | top layer strength 50 % | 1 one, 122 between, 606 zeros; max 1.0 because layer A shows through |
| three-layers | C (0, 75, 0) over B over A, all Normal | – | 7 ones, 74 between, 648 zeros; same counts as blend-normal (C replaces what is below) |
| formula-defaults | a Formula field | – | Record the formula text. Formula text as shown: `0.5 + sin( ( subfields + ( id / count ) + d ) * f * 360 ) * 0.5`; also parameters t (time delta, 100 %) and f (frequency, 1). Clamp Min and Clamp Max are **off** on this field. All 729 values between 0 and 1 (min 1.16e-6, max 0.99999884) |
| formula-custom | a Formula field | formula `x/100` (or the closest the field accepts) | **Text `x/100` gave 729 zeros** (the variable `x` is not a position here), so the case was made with `px/100` instead (the closest the field accepts): 243 ones, 81 between, 405 zeros. The `x/100` result was not kept as a case |
| radial-half | Radial field | end angle 180° | 369 ones, 360 zeros, nothing between |
| radial-transition | Radial field | end angle 180°, end transition 45° | 315 ones, 54 between, 360 zeros |
| radial-iterations | Radial field | iterations 3 | All 729 samples are 1.0, same as the defaults |
| linear-clip | Linear field | Clip to Shape on | 243 ones, 243 between, 243 zeros |
| spherical-clip | Spherical field | Clip to Shape on | 1 one, 6 between, 722 zeros; same counts as spherical-defaults |
| random-random-mode | Random field | Random Mode Random | All 729 between 0 and 1 (min 0.002839, max 0.999520) |
| random-global | Random field | Space Global | min 0.10056, max 0.95476: **the same as random-defaults** (Space Field); the field sits at the origin unrotated, so Global and Field coincide |
| group-field | a Group field holding two Sphericals (A and B, size 200) | – | Record the group's own blend settings. Inner list B over A, both Normal, strength 100 %; same counts as blend-normal (7 ones, 74 between, 648 zeros). The group's own settings: Enable Remapping **off**, Mirror Graph off, Strength 100 %, Inner Offset 0 %, Min 0 %, Max 100 %, Clamp Min and Max on, Multiplier 100 %, Contour Mode None, Color Mode Color; all in `case.toml` |
| shader-noise | a Shader field with a default Noise shader | – | Record the noise seed and type. Noise shader: seed 665, type Noise, octaves 5, space Texture, global scale 1, colours black and white, low clip 0, high clip 1; all in `case.toml` (`noise_*`). All 729 between 0 and 1 (min 0.1725, max 0.8682) |

## Notes
- All 22 cases wrote 729 samples (`tests/golden/fields/<case>/samples.json`) plus `case.toml`. Layer data are extra keys in `[params]`: `layers_top_to_bottom`, `layer_blend_modes_top_to_bottom`, `layer_strength_percent_top_to_bottom`, the object positions and `size`.
- Layer order: a layer inserted later goes on top, so the list reads C, B, A (as in O13). Blend mode and strength were set on the top layer; lower layers stay Normal at 100 %.
- A first run kept only one layer in the list (the same probe error as in O13); it was rerun with the layers created separately and the numbers above are from the rerun.
- Counts coincide for some modes on this grid (Max = Screen; Min = Multiply = Clip). Only counts were compared; the samples hold the values.
- The Formula field uses its own variable names (`subfields`, `id`, `count`, `d`, `f`, `px`); `x` alone gave zeros.
- shader-noise `case.toml`: the probe wrote an unset Material Tag link and the shader link as raw text, which is not valid TOML; both were hand-edited (`material_tag = ""`, `shader = "Noise"`), and the Shader field's own Invert (off) was renamed `shader_invert` because the Remapping Invert has the same key.
- Not checked: Clip to Shape with a rotated or scaled field; group fields with their own blend settings changed; shaders other than Noise.

