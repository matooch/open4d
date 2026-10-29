# Cloner and basic effectors — observation (O8)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Level B (numeric samples).
Unblocks: P3 (cloner, clone data, effectors).

Every case starts from a new document with a cloner made through its menu command and,
as its only child, a default cube scaled down to size 20 × 20 × 20. Read the cloner's
per-clone data **after evaluation**, through the app's documented clone-data call. For
each clone, record:
- its matrix: position, then the X, Y and Z axis vectors, in cloner space;
- its colour (RGB 0…1);
- its weight;
- whether it is visible.

Save `tests/golden/cloner/<case>/samples.json`:

```json
{ "kind": "clones", "tolerance": 1e-4,
  "clones": [ { "pos": [x,y,z], "x": [..], "y": [..], "z": [..],
                "color": [r,g,b], "weight": w, "visible": true }, ... ] }
```

Also export the cloner as OBJ to `mesh.obj` in the same folder, with the usual settings,
and add `mesh = "mesh.obj"` to `case.toml`.

`case.toml`: `feature = "cloner"`. `[params]` holds `mode` and every cloner parameter
shown for that mode, plus each effector's parameters under `[params.effector]`, using
our key names.

How it was run: the cube is set to 20 × 20 × 20 and placed under the cloner. Effectors were
made through their own menu commands while the cloner was selected, which adds them to the
cloner's effector list. The scene is evaluated, then the cloner's clone data is read (matrix,
colour, weight, visible flag per clone). Visible is the clone-on flag; all clones in all cases are visible.
The mesh is the OBJ export of the whole scene with the usual settings; the cloner is in
Instance mode (its default) and the export holds the cloned cubes (3 clones: 24 vertices).

**The new cloner's mode is Grid Array, not Linear.** A cloner made through its menu command
starts in Grid Array mode (Count 3 × 1 × 3). "Defaults" below means: set the mode, leave the rest.

## Parameters (fill once per mode)

For each mode, list the Object tab and the Transform tab as shown: label, our key,
default, min and max.

Min and Max are the limits the field declares; where a ±999999999 assignment was tried, the
value it kept is in brackets after. Percent fields are shown in percent (key ends `_percent`)
and angles in degrees (key ends `_deg`); lengths in cm. Vector fields were not assignment-tested.

### Common to all modes (Object tab and Transform tab)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Mode | `mode` | Grid Array | – | options: Object, Linear, Radial, Grid Array, Honeycomb |
| Clones | `clones_mode` | Iterate | – | options: Iterate, Random, Blend, Sort |
| Reset Coordinates | `reset_coordinates` | on | – | – |
| Fix Texture | `fix_texture` | Off | – | options: Off, Straight, Alternate X, Alternate Y |
| Instance Mode | `instance_mode` | Instance | – | options: Instance, Render Instance, Multi-Instance |
| Viewport Mode | `viewport_mode` | Object | – | options: Off, Points, Matrix, Bounding Box, Object |
| Display (Transform tab) | `display` | None | – | options: None, Weight, UV, Color, Index |
| P (Transform tab) | `position` | (0, 0, 0) | −1e20 | 1e20 |
| R (Transform tab) | `rotation_deg` | (0°, 0°, 0°) | −1e20° | 1e20° |
| S (Transform tab) | `scale` | (1, 1, 1) | −1e20 | 1e20 |
| Color (Transform tab) | `color` | (1, 1, 1) | 0 | 1 |
| W(UV)-Orientation | `uv_orientation` | Z+ | – | options: X+, X−, Y+, Y−, Z+, Z− |
| Weight | `weight_percent` | 0 % | 0 % | 1e20 % (kept 999999999 as a fraction) |
| Animation Mode | `animation_mode` | Play | – | options: Play, Loop, Fixed, Fixed Loop |

The Time field is not recorded (not a number). The Effectors list is a list of objects: `effector_count` in `case.toml` says how many it held.

### Linear mode
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Count | `count` | 3 | 0 | 2147483647 (kept 999999999) |
| Offset | `offset` | 0 | 0 | 2147483647 (kept 999999999) |
| Mode | `linear_mode` | Per Step | – | options: End Point, Per Step |
| Amount | `amount_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| P (per step) | `transform_position` | (0, 50, 0) | −1e20 | 1e20 |
| S (per step) | `transform_scale_percent` | (100 %, 100 %, 100 %) | −1e20 % | 1e20 % |
| R (per step) | `transform_rotation_deg` | (0°, 0°, 0°) | −1e20° | 1e20° |
| Step Mode | `step_mode` | Single Value | – | options: Single Value, Cumulative |
| Step Size | `step_size_percent` | 100 % | 0 % | 1e20 % (kept 999999999 as a fraction) |
| Step Rotation | `step_rotation_deg` | (0°, 0°, 0°) | −1e20° | 1e20° |

### Radial mode
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Count | `count` | 5 | 0 | 2147483647 (kept 999999999) |
| Radius | `radius` | 50 | −1e20 (kept −999999999) | 1e20 (kept 999999999) |
| Plane | `plane` | XZ | – | options: XY, ZY, XZ |
| Align | `align` | on | – | – |
| Start Angle | `start_angle_deg` | 0° | −1e20° | 1e20° (kept ±999999999 rad-equivalent) |
| End Angle | `end_angle_deg` | 360° | −1e20° | 1e20° |
| Offset | `offset_deg` | 0° | −1e20° | 1e20° |
| Offset Variation | `offset_variation_percent` | 0 % | 0 % | 1e20 % (kept 999999999 as a fraction) |
| Offset Seed | `offset_seed` | 1234567 | 0 | 2147483647 |

### Grid Array mode
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Count | `count` | (3, 1, 3) | (1, 1, 1) | (2000000, 2000000, 2000000) |
| Mode | `grid_mode` | Per-Step | – | options: Per-Step, Endpoint |
| Size | `size` | (200, 200, 200) | −1e20 each | 1e20 each |
| Form | `form` | Cubic | – | options: Cubic, Sphere, Cylinder, Object |
| Fill | `fill_percent` | 100 % | 0 % | 100 % |

### Honeycomb mode
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Orientation | `orientation` | Z (XY) | – | options: Z (XY), X (ZY), Y (XZ) |
| Offset Direction | `offset_direction` | Height | – | options: Width, Height |
| Offset | `offset_percent` | 50 % | 0 % | 100 % |
| Offset Variation | `offset_variation_percent` | 0 % | 0 % | 100 % |
| Perpendicular Variation | `perpendicular_variation_percent` | 0 % | 0 % | 100 % |
| Seed | `seed` | 123456 | 0 | 2147483647 |
| Count Width | `count_width` | 10 | 1 | 2147483647 (kept 999999999) |
| Count Height | `count_height` | 10 | 1 | 2147483647 (kept 999999999) |
| Mode | `honeycomb_mode` | Per Step | – | options: Per Step, End Point |
| Size Width | `size_width` | 300 | 0 | 1e20 (kept 999999999) |
| Size Height | `size_height` | 346.4101616 | 0 | 1e20 (kept 999999999) |
| Form | `form` | Square | – | options: Circle, Square, Spline |

### Object mode
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Object | `object` | empty (link) | – | – |
| Align Clone | `align_clone` | on | – | – |
| Up Vector | `up_vector` | None | – | options: None, X+, X−, Y+, Y−, Z+, Z− |
| Distribution | `distribution` | Surface (after linking an object) | – | options: Vertex, Edge, Polygon Center, Surface, Volume, Axis |
| Selection | `selection` | empty | – | – |
| Enable (selection) | `selection_enable` | on | – | – |
| Enable Scaling | `enable_scaling` | off | – | – |
| Scale | `selection_scale_percent` | 100 % | −1e20 % | 1e20 % |
| Seed (Surface and Volume) | `seed` | 1234567 | −2147483648 | 2147483647 |
| Count (Surface and Volume) | `count` | 20 | 0 | 2147483647 |

Up Vector, Distribution and the rest of the Object-mode fields appear only once an object is linked. The Spline of the Scale and the link itself are not numbers and are not recorded. The Object, Up Vector and Distribution defaults are as read after linking a sphere: Distribution Surface; the case sets Vertex. Which of Seed and Count show for which Distribution was not checked.

## Cases

| Case | Mode | Changed from default | Effector | Notes |
|---|---|---|---|---|
| linear-defaults | Linear | – | – | 3 clones at (0, 0, 0), (0, 50, 0), (0, 100, 0), unrotated, white, weight 0, all visible. `mesh.obj`: 24 vertices, 18 faces, bbox (−10, −10, −10)–(10, 110, 10) |
| linear-count-step | Linear | count 5, position offset (0, 50, 0), rotation step H 10° | – | "Position offset" is read as the Transform tab's clone Position set to (0, 50, 0); the per-step position is already (0, 50, 0) by default. "Rotation step" is the Step Rotation field |
| radial-defaults | Radial | – | – | 5 clones |
| radial-plane-start-end | Radial | radius 150, plane XZ (or the second option), start 0°, end 180° | – | XZ is already the default, so the second listed option **ZY** was used. Start 0° is the default |
| grid-defaults | Grid Array | – | – | 9 clones (3 × 1 × 3) |
| grid-3x2x4 | Grid Array | count (3, 2, 4), size (300, 200, 400) | – | 24 clones |
| honeycomb-defaults | Honeycomb | – | – | 50 clones (a circle form of 10 × 10) |
| object-vertex | Object, on a default sphere with 8 segments | distribution Vertex | – | 26 clones, one per sphere vertex. The sphere is in the scene as well, so `mesh.obj` holds the sphere (26 vertices) plus the clones: 234 vertices, 188 faces |
| linear-plain | Linear | – | Plain, position (0, 100, 0), defaults otherwise | (0, 100, 0) is the Plain effector's default position. Clones move up by 100: y 100, 150, 200 |
| linear-plain-field | Linear, count 10 | – | Plain, position (0, 100, 0), a Linear field in its falloff list | Making the Linear field with the effector selected put it into the effector's field list by itself (1 layer, nothing was added by the probe). Every clone is at x 0, where the field reads 0.5, so each clone moved up by 50: y 50 … 500 |
| grid-random | Grid Array | – | Random, defaults (record its seed) | **Seed 12345, Position (50, 50, 50) are the Random effector's defaults** |
| grid-random-seed | Grid Array | – | Random, seed 12345, position (50, 50, 50) | Same as grid-random: both values are the defaults, and the samples are the same (first four positions checked equal) |

## Notes

- Cloner made through the menu starts in Grid Array mode, not Linear. Every mode case sets the mode explicitly.
- A Cloner with an effector gets one more field, named after the effector (Random: 100 %), the effector's strength on this cloner; it shows in `case.toml` as `random = 100.0`.
- Effector parameters in `[params.effector]` use the label as shown; when two fields have the same label the second gets the field number appended (`scale_1061`, `use_alpha_strength_1071`). Effector fields that are not numbers (the field list, the time offset) are not recorded.
- Plain effector defaults seen: Strength 100 %, Transform Mode Relative, Position on with P (0, 100, 0), Rotation off, Scale off, Color Mode shown as Fields Color, Deformation Off. Random effector defaults: Strength 100 %, Max 100 %, Min −100 %, Random Mode Random, Seed 12345, P (50, 50, 50), Space Global, Animation Speed 100 %, Scale 100 %.
- Not covered by the form and not observed: Clones modes other than Iterate, Fill below 100 %, other Grid forms, other Object distributions, effector Color Mode, Fix Texture, Multi-Instance.
