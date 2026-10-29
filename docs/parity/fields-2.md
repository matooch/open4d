# Fields II — observation (O13)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: exactly as [fields.md](fields.md) (O7): menu command, one field layer,
value channel, the same 729-point grid, `tests/golden/fields/<case>/samples.json`,
`observation = "docs/parity/fields-2.md"`. Unblocks: P2 beyond Linear, Spherical and Box.

All 11 cases wrote 729 samples. `case.toml` holds every scalar parameter of the field (own
tab plus Remapping, as in O7), position and rotation. The case format has no section for
layers, so the layer data of the layer cases are extra keys in `[params]` (see Cases).
Lengths are in cm. Min and Max follow fields.md: the declared limit, and in brackets what a
±999999999 assignment kept where that differs.

## Parameters

The Remapping tab is the one in fields.md. Its defaults per type, as read: Inner Offset 50 %
and Mirror Graph on and Color Mode Color on Cylinder, Cone, Torus and Capsule; Radial has
Inner Offset 0 %, Mirror Graph on, Color Mode Color; Random has Inner Offset 0 %, Mirror Graph
**off**, Color Mode **No Remap**. All other Remapping defaults are the same as in fields.md.
Not recorded: the Remapping spline and gradient, Radial's Offset field list, Random's Loop
Period (a time, not a scalar), icon settings.

### Cylinder field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Height | `height` | 100 | 0 | 1e20 (kept 999999999) |
| Radius | `radius` | 75 | 0 | 1e20 (kept 999999999) |
| Direction | `direction` | Y+ | – | options: X+, X−, Y+, Y−, Z+, Z− |
| Clip to Shape | `clip_to_shape` | off | – | – |

### Cone field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Height | `height` | 100 | 0 | 1e20 (kept 999999999) |
| Radius | `radius` | 100 | 0 | 1e20 (kept 999999999) |
| Direction | `direction` | Y+ | – | options: X+, X−, Y+, Y−, Z+, Z− |
| Clip to Shape | `clip_to_shape` | off | – | – |

### Torus field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Radius | `radius` | 100 | 0 | 1e20 (kept 999999999) |
| Thickness | `thickness` | 35 | 0 | 1e20 (kept 999999999) |
| Direction | `direction` | Y+ | – | options: X+, Y+, Z+ (only three) |
| Clip to Shape | `clip_to_shape` | off | – | – |

### Capsule field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Top Radius | `top_radius` | 75 | 0 | 1e20 (kept 999999999) |
| Bottom Radius | `bottom_radius` | 75 | 0 | 1e20 (kept 999999999) |
| Height | `height` | 150 | 0 | 1e20 (kept 999999999) |
| Direction | `direction` | Y+ | – | options: X+, X−, Y+, Y−, Z+, Z− |
| Clip to Shape | `clip_to_shape` | off | – | – |

### Radial field
Angles are shown in degrees (stored as radians; the ±999999999 assignment was on the stored radians).

| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Start Angle | `start_angle_deg` | 0° | −1e20° (kept −999999999 rad) | 1e20° (kept 999999999 rad) |
| Start Transition | `start_transition_deg` | 0° | 0° | 360° |
| End Angle | `end_angle_deg` | 360° | −1e20° (kept −999999999 rad) | 1e20° (kept 999999999 rad) |
| End Transition | `end_transition_deg` | 0° | 0° | 360° |
| Iterations | `iterations` | 1 | 1 | 1e20 (kept 999999999) |
| Offset | `offset_deg` | 0° | −1e20° (kept −999999999 rad) | 1e20° (kept 999999999 rad) |
| Axis | `axis` | Y | – | options: X, Y, Z |
| Clip to Shape | `clip_to_shape` | off | – | – |

### Random field
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Random Mode | `random_mode` | Noise | – | options: Random, Sorted, Noise |
| Seed | `seed` | 123456 | 0 | 1e20 (kept 999999999) |
| Noise Type | `noise_type` | Perlin | – | 31 options (Box, Blistered Turbulence, Buya, Cell, Cranal, Dents, Displaced Turbulence, Electric, FBM, Gaseous, Hama, Luka, Mod. Noise, Naki, Perlin, Nutous, Ober, Pezo, Poxo, Sema, Stupl, Turbulence, VL Noise, Wavy Turbulence, Cell Voronoi, Displaced Voronoi, Voronoi 1, Voronoi 2, Voronoi 3, Zada, Ridged Multifractal) |
| Space | `space` | Field | – | options: Global, Field, None |
| Scale | `scale_percent` | 100 % | 0.01 % | 1e20 % |
| Relative Scale | `relative_scale_percent` | (100, 100, 100) % | (0.01, 0.01, 0.01) % | 1e20 % each |
| Animation Speed | `animation_speed_percent` | 0 % | −1e20 % | 1e20 % |
| Randomized Sample Positions | `randomized_sample_positions` | off | – | – |
| Sample Position Seed | `sample_position_seed` | 123456 | 1 | 2147483647 (kept 999999999) |
| Color Shift | `color_shift_percent` | 10 % | 0 % | 1e20 % |
| Color Alpha | `color_alpha` | None | – | options: None, Strength |
| Viewplane Preview | `viewplane_preview` | off | – | – |
| Show X / Show Y / Show Z | `show_x`, `show_y`, `show_z` | on | – | – |
| Viewplane Resolution | `viewplane_resolution` | 100 | 0 | 500 |
| Display Box | `display_box` | (100, 100, 100) | −1e20 each | 1e20 each |

The Noise Type list has one more entry than the 31 named above: an empty separator row.

## Cases

| Case | Field | Changed from default | Notes |
|---|---|---|---|
| cylinder-defaults | Cylinder | – | 1 sample is 1, 14 are between 0 and 1, 714 are 0 |
| cylinder-size | Cylinder | Radius 200, Height 300 | Fields are labelled Radius and Height; 25 ones, 122 between, 582 zeros |
| cone-defaults | Cone | – | 1 one, 6 between, 722 zeros |
| torus-defaults | Torus | – | 4 ones, 4 between, 721 zeros |
| capsule-defaults | Capsule | – | 3 ones, 14 between, 712 zeros |
| radial-defaults | Radial | – | **All 729 samples are 1.0**: the default angles are 0° to 360°, a full circle, so the field is 1 everywhere on the grid |
| random-defaults | Random | – | Mode Noise, Noise Type Perlin, Seed 123456, Space Field (all as the app shows). All values between 0 and 1: min 0.10056, max 0.95476 |
| random-seed | Random | Seed 12345 | min 0.10959, max 0.88282 |
| two-layers-max | Spherical ×2 | top layer's blend mode Max | Spheres (default size 100) at A (−100, 0, 0) and B (100, 0, 0), each in its own layer. **A layer inserted later goes on top**: the list reads, top to bottom, B then A. Blend mode was set on the top layer (B). New layers start at Normal blending, strength 100 %. Layer data in `[params]`: `layers_top_to_bottom`, `top_layer_field`, `top_layer_blend_mode`, `A_position`, `B_position`. 4 ones, 16 between, 709 zeros |
| two-layers-add | Spherical ×2 | top layer's blend mode Add | Same layers, top layer (B) set to Add. **The samples are the same as in two-layers-max**: with the grid used the two spheres never overlap with a non-zero value (the grid has no point inside both), so Max and Add cannot differ here. A second pair of cases with overlapping spheres would be needed |
| layer-opacity-50 | Spherical | layer strength 50 % | One Spherical layer (Normal blending) at strength 50 %; keys `layer_strength_percent`, `layer_blend_mode` in `[params]`. Values run 0 to 0.5: the layer strength scales the field value |

Blend mode options as the layer shows them, in order: Normal, Min, Subtract, Multiply, Overlay,
Max, Add, Screen, Clip (separators omitted). Default Normal.

## Notes
- A first run of the layer cases listed only one layer because of an error in the probe (the layers were created in a loop and one was lost); it was rerun with the layers created separately and the numbers above are from the rerun.
- Not checked: the samples of the colour and direction channels; Radial with a non-full angle or iterations > 1; Random with Space Global or None, other noise types, Animation Speed ≠ 0; layers with overlapping fields; Min/Subtract/Multiply/Overlay/Screen/Clip blending; field layer order beyond the two-layer case.
