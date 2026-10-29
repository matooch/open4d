# Fields: values in space — observation (O7)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Level B (numeric samples).
Unblocks: P2 (fields), and through them deformer falloff and effectors.

Every case: a new document with one field object, made through its menu command, at the
origin with no rotation unless the row says otherwise. Sample the field's **value**
(0…1, after its own remapping) at every point of a grid:
x, y, z ∈ {−300, −225, …, 300}, a step of 75, which gives 9 × 9 × 9 = 729 points. Use the
app's documented field sampling call. Save
`tests/golden/fields/<case>/samples.json`:

```json
{ "kind": "field", "tolerance": 1e-5,
  "samples": [[[x, y, z], value], ...] }
```

Points go in x-fastest order: x varies first, then y, then z. Use full float precision.
`case.toml`: `feature = "fields"`. `[params]` holds `type` plus **every** parameter of
the field under our key names, including position and rotation.

How it was sampled: the field object was put in a field list as its only layer (layer
strength 100 %, normal blending) and the list was sampled for its value channel at the 729
world positions. The values include the field's own Remapping tab. All 16 cases wrote 729 samples.

## Parameters (fill once per field type)

For each field type, list the Field tab and the Remapping tab as shown: label, our key,
default, min and max. Do this the way r25-defaults.md does.

Min and Max: the field's declared limit, and in brackets what a ±999999999 assignment kept
where that differs. Lengths are in cm. Fields the app shows in percent (a stored fraction ×
100) are written in percent here and in `case.toml` (key ends `_percent`); the ±999999999
assignment was made on the stored fraction, so in percent it is ±99 999 999 900 %.

### Linear field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Length | `length` | 100 | 0 | 1e20 (kept 999999999) |
| Direction | `direction` | X+ | – | options: X+, X−, Y+, Y−, Z+, Z− |
| Clip to Shape | `clip_to_shape` | off | – | – |

### Spherical field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Size | `radius` | 100 | 0 | 1e20 (kept 999999999) |
| Clip to Shape | `clip_to_shape` | off | – | – |

`radius` is the field's Size: at the default the value is 1 up to half of it and reaches 0 at 100 from the centre.

### Box field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Size | `size` | (100, 100, 100) | (0, 0, 0) | 1e20 each |
| Clip to Shape | `clip_to_shape` | off | – | – |

The Box Size limits are the declared ones; the ±999999999 assignment was not tried on a vector.

### Remapping tab (shared)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Enable Remapping | `enable_remapping` | on | – | – |
| Strength | `strength_percent` | 100 % | −1e20 % (kept −999999999 as a fraction) | 1e20 % (kept 999999999 as a fraction) |
| Invert | `invert` | off | – | – |
| Inner Offset | `inner_offset_percent` | Linear field 0 %; Spherical and Box 50 % | 0 % | 100 % |
| Min | `min_percent` | 0 % | −1e20 % (kept −999999999) | 1e20 % (kept 999999999) |
| Clamp Min | `clamp_min` | on | – | – |
| Max | `max_percent` | 100 % | −1e20 % (kept −999999999) | 1e20 % (kept 999999999) |
| Clamp Max | `clamp_max` | on | – | – |
| Multiplier | `multiplier_percent` | 100 % | −1e20 % (kept −999999999) | 1e20 % (kept 999999999) |
| Contour Mode | `contour_mode` | None | – | options: None, Quadratic, Step, Quantize, Curve |
| Curve (hidden by default) | `curve_percent` | 0 % | −100 % | 100 % |
| Steps (hidden by default) | `steps` | 1 | 1 | 2147483647 (kept 999999999) |
| Spline Offset (hidden) | `spline_offset_percent` | 0 % | −1e20 % | 1e20 % |
| Spline Range (hidden) | `spline_range_percent` | 100 % | −1e20 % | 1e20 % |
| Mirror Graph (hidden) | `mirror_graph` | Linear off; Spherical and Box on | – | – |
| Color Mode | `color_mode` | Color | – | options: No Remap, Color, Gradient |
| Length (colour/direction channels) | `length_mode` | Use Value | – | options: No Remap, Normalize, Use Value, Inverse Square |
| Scale (under Length) | `length_scale_percent` | 100 % | −1e20 % | 1e20 % |
| Invert Direction | `invert_direction` | on | – | – |
| Strength (rotation channel) | `strength_mode` | No Remap | – | options: No Remap, Normalize, Remap, Use Value, Inverse Square |
| Scale (under that Strength) | `strength_scale_percent` | 100 % | −1e20 % | 1e20 % |
| Invert Rotations | `invert_rotations` | off | – | – |

Not recorded as keys: the Remapping spline, its animation speed, the gradient and the Icon
settings (not scalars). The remap Color swatch default is (0.098071, 0.633227, 0.333986)
on the Linear field and (0.765662, 0.524827, 0.281835) on Spherical and Box; the object
Color is (1, 0.9, 0.4) on all three. They are not in `case.toml`.

## Cases

| Case | Field | Changed from default | Notes |
|---|---|---|---|
| linear-defaults | Linear | – | 729 samples. Along x: 0 up to x −150, then 0.125 at −75, 0.5 at 0, 0.875 at 75, 1 from 150; the value does not depend on y or z. So the ramp is 0.5 + x / 200: the full ramp is **2 × Length** wide, centred on the field |
| linear-direction-x | Linear | Direction X− | X+ is the default, so the closest different option was used. Same ramp mirrored |
| linear-length | Linear | Length 400 | Ramp 0.5 + x / 800, so values run 0.125 (x −300) to 0.875 (x 300); no sample is 0 or 1 |
| linear-rotated | Linear | rotation H 45° | Rotation (45°, 0°, 0°) on the field |
| spherical-defaults | Spherical | – | 1 at the centre, 0.5 at 75 from it (the six points on the axes), 0 beyond 100. Falloff is linear between half the size and the size |
| spherical-radius | Spherical | Size 250 | Size is read as the radius, key `radius` |
| spherical-inner | Spherical | Inner Offset 50 % | **50 % is already the default**, so this equals spherical-defaults |
| spherical-moved | Spherical | position (100, 0, 0) | Samples are taken in world positions, so the sphere is centred on x 100 |
| box-defaults | Box | – | |
| box-size | Box | Size (400, 200, 100) | |
| box-inner | Box | Inner Offset 50 % | **50 % is already the default**, so this equals box-defaults |
| spherical-interp-linear | Spherical | Interpolation Linear | **No Interpolation option exists** on the field. Contour Mode None (the default) was used as the nearest option, so this equals spherical-defaults |
| spherical-interp-step | Spherical | Interpolation Step | No Interpolation option; Contour Mode **Step** used. The samples are only 0 or 1: 7 ones (centre and the six axis points at 75) |
| spherical-remap-invert | Spherical | Invert on | Remapping > Invert |
| spherical-remap-minmax | Spherical | Min 20 %, Max 80 % | Nearest to "inner 0.2, outer 0.8": the Remapping tab has Min and Max, not inner and outer. Values run 0.2 to 0.8 |
| spherical-remap-contour | Spherical | Contour Mode Quadratic | First non-None option |

If a named option doesn't exist, use the closest one and write down what you used.

## Notes

- Five cases repeat another one because the row's value is already a default or the option does not exist: spherical-inner, box-inner and spherical-interp-linear are the same data as their defaults case; linear-direction-x was changed to X− for that reason. The planner may want other values.
- The `case.toml` files hold every scalar parameter, with position and rotation (degrees, H P B).
- Sampling used the field object directly, not a Mograph object, so no clone or deformer falloff is involved.
- Not checked: Clip to Shape on; other field types; fields with a parent or scale; the sampling of the colour and direction channels.
