# Fields: values in space — observation (O7)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
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

## Parameters (fill once per field type)

For each field type, list the Field tab and the Remapping tab as shown: label, our key,
default, min and max. Do this the way r25-defaults.md does.

### Linear field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Spherical field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Box field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Remapping tab (shared)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Field | Changed from default | Notes |
|---|---|---|---|
| linear-defaults | Linear | – | |
| linear-direction-x | Linear | direction X+ (or the closest option) | |
| linear-length | Linear | length 400 | |
| linear-rotated | Linear | rotation H 45° | |
| spherical-defaults | Spherical | – | |
| spherical-radius | Spherical | radius 250 | |
| spherical-inner | Spherical | inner offset 50 % | |
| spherical-moved | Spherical | position (100, 0, 0) | |
| box-defaults | Box | – | |
| box-size | Box | size (400, 200, 100) | |
| box-inner | Box | inner offset 50 % | |
| spherical-interp-linear | Spherical | interpolation Linear (or the closest option) | |
| spherical-interp-step | Spherical | interpolation Step | |
| spherical-remap-invert | Spherical | remapping: invert on | |
| spherical-remap-minmax | Spherical | remapping: inner 0.2, outer 0.8 (or the closest options shown) | |
| spherical-remap-contour | Spherical | remapping: contour mode Quadratic (or the first non-None option) | |

If a named option doesn't exist, use the closest one and write down what you used.

## Notes
TODO
