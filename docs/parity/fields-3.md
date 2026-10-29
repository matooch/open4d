# Fields III — observation (O23)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
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
| blend-normal | B (75, 0, 0) over A (−75, 0, 0) | – | Size 200 each |
| blend-max | the same | top blend Max | |
| blend-min | the same | top blend Min | |
| blend-add | the same | top blend Add | |
| blend-subtract | the same | top blend Subtract | |
| blend-multiply | the same | top blend Multiply | |
| blend-overlay | the same | top blend Overlay | |
| blend-screen | the same | top blend Screen | |
| blend-clip | the same | top blend Clip | |
| blend-normal-50 | the same | top layer strength 50 % | |
| three-layers | C (0, 75, 0) over B over A, all Normal | – | |
| formula-defaults | a Formula field | – | Record the formula text |
| formula-custom | a Formula field | formula `x/100` (or the closest the field accepts) | |
| radial-half | Radial field | end angle 180° | |
| radial-transition | Radial field | end angle 180°, end transition 45° | |
| radial-iterations | Radial field | iterations 3 | |
| linear-clip | Linear field | Clip to Shape on | |
| spherical-clip | Spherical field | Clip to Shape on | |
| random-random-mode | Random field | Random Mode Random | |
| random-global | Random field | Space Global | |
| group-field | a Group field holding two Sphericals (A and B, size 200) | – | Record the group's own blend settings |
| shader-noise | a Shader field with a default Noise shader | – | Record the noise seed and type |

## Notes
TODO
