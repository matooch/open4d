# Deformers II — observation (O9)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Level A (exact meshes).
Unblocks: P1 (the remaining deformers).

## Setup for every case

Use the same input as bend.md: a cube made with its menu command, size_y 400, segments
4/8/4. Make the deformer through its menu command as the cube's **child**, at the
origin. Set only what the row gives. Export the cube with the usual settings to
`tests/golden/<deformer>/<case>/mesh.obj`.

`case.toml`:
- `feature = "<deformer>"`, as in the folder name: `squash-stretch`, `wrap`, `ffd`,
  `formula`, `melt`.
- `observation = "docs/parity/deformers-2.md"`.
- `[params]` holds **every** deformer parameter under our keys.
- `[params.input]` holds the cube's params, as the bend cases do.

How it was run: the deformer was made through its menu command (it lands at the top level)
and then moved under the cube. All five deformers are made with the cube already at
200 × 400 × 200 and 4/8/4 segments, so their size fields are the app's own defaults, not fitted to the cube.
Percent fields are written in percent (key ends `_percent`), angles in degrees
(`_deg`), lengths in cm. The two formula fields that hold text are written as text under
`formula_d`, `formula_x`, `formula_y`, `formula_z`.

## Parameters (fill once per deformer)

For each deformer, list the Object tab as shown: label, our key, default, min and max.

Min and Max are the limits the field declares; in brackets, what a ±999999999 assignment
kept where it differs. Vector fields were not assignment-tested.

### Squash & Stretch
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Top | `top` | 100 | 0 | 1e20 (kept 999999999) |
| Center | `center_percent` | 0 % | −100 % | 100 % |
| Bottom | `bottom` | −100 | −1e20 (kept −999999999) | 0 |
| Aspect | `aspect_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Factor | `factor_percent` | 100 % | 0 % | 1e20 % (kept 999999999 as a fraction) |
| Expand | `expand_percent` | 100 % | 0 % | 1e20 % (kept 999999999 as a fraction) |
| Smooth Start | `smooth_start_percent` | 0 % | 0 % | 100 % |
| Smooth End | `smooth_end_percent` | 0 % | 0 % | 100 % |
| Curvature | `curvature_percent` | 100 % | 0 % | 100 % |
| Type | `curvature_type` | Square | – | options: Square, Cubic, Quartic, Custom, Spline |
| Power | `power` | 2 | 0 | 1e20 (kept 999999999) |

The Curve (a spline) is not recorded. A Field list is also on the object; it is empty by default and not recorded.

### Wrap
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Width | `width` | 400 | 0 | 1e20 (kept 999999999) |
| Height | `height` | 400 | 0 | 1e20 (kept 999999999) |
| Radius | `radius` | 200 | 0 | 1e20 (kept 999999999) |
| Wrap | `wrap` | Cylindrical | – | options: Spherical, Cylindrical |
| Longitude Start | `longitude_start_deg` | 180° | −1e20° | 1e20° |
| Longitude End | `longitude_end_deg` | 360° | −1e20° | 1e20° |
| Latitude Start | `latitude_start_deg` | −45° | −90° | 90° |
| Latitude End | `latitude_end_deg` | 45° | −90° | 90° |
| Movement | `movement` | 0 | −1e20 (kept −999999999) | 1e20 (kept 999999999) |
| Scale Z | `scale_z_percent` | 100 % | 0 % | 10000 % |
| Tension | `tension_percent` | 100 % | 0 % | 100 % |

### FFD
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Grid Size | `grid_size` | (300, 300, 300) | −1e20 each | 1e20 each |
| Grid Points X | `grid_points_x` | 3 | 2 | 200 |
| Grid Points Y | `grid_points_y` | 3 | 2 | 200 |
| Grid Points Z | `grid_points_z` | 3 | 2 | 200 |

The grid points themselves are editable points of the object: 27 points at x, y, z ∈ {−150, 0, 150}, x varying fastest, then y, then z (point 0 is (−150, −150, −150), point 8 is (150, 150, −150), point 26 is (150, 150, 150)).

### Formula
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Size | `size` | (400, 400, 400) | (0, 0, 0) | 1e20 each |
| Effect | `effect` | Y Radial | – | options: Manual, Spherical, Cylindrical, X Radial, Y Radial, Z Radial |
| d(u,v,x,y,z,t) | `formula_d` | `Sin((u+t)*2.0*PI)*0.2` | – | text |
| X(x,y,z,t) | `formula_x` | `x` | – | text |
| Y(x,y,z,t) | `formula_y` | `y` | – | text |
| Z(x,y,z,t) | `formula_z` | `z` | – | text |

### Melt
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Strength | `strength_percent` | 25 % | 0 % | 100 % |
| Radius | `radius` | 100 | 0 | 1e20 (kept 999999999) |
| Vertical Randomness | `vertical_randomness_percent` | 100 % | 0 % | 100 % |
| Radial Randomness | `radial_randomness_percent` | 100 % | 0 % | 100 % |
| Melted Size | `melted_size_percent` | 400 % | 100 % | 1000 % |
| Noise Scale | `noise_scale_percent` | 10 % | 0.1 % | 10000 % |

## Cases

| Case | Deformer | Changed from default | Notes |
|---|---|---|---|
| squash-stretch/defaults | Squash & Stretch | – | 162 vertices, 160 faces; bbox ±100, ±200, ±100: the cube is unchanged (Factor 100 %) |
| squash-stretch/factor | Squash & Stretch | factor 50 % | bbox x ±200, y ±149.9975, z ±200 (size 400 × 299.995 × 400) |
| squash-stretch/factor-big | Squash & Stretch | factor 150 % | bbox x ±100, y ±250.0025, z ±100 (size 200 × 500.005 × 200) |
| squash-stretch/bulge | Squash & Stretch | factor 50 %, bulge 100 % (or the closest setting) | **No setting is called bulge.** Expand was used as the closest; its 100 % is also its default, so this is the same as squash-stretch/factor (same bbox). The Curvature field is also at its default 100 % |
| wrap/defaults | Wrap | – | The default Wrap is Cylindrical. bbox (−212.132, −200, −100)–(212.132, 200, 129.289) |
| wrap/cylindrical | Wrap | type Cylindrical (or the second option) | Cylindrical is the second option **and the default**: the same mesh as wrap/defaults |
| wrap/angles | Wrap | width angle 180°, height 300 | The Wrap has no width angle field; the Longitude span (180° to 360°) is already 180° by default. Only Height 300 was set. **Height has no effect in Cylindrical mode**: the file is byte-identical to wrap/defaults. Changing Width, Radius or the Wrap type did change the mesh, checked separately and not kept |
| ffd/defaults | FFD | – | Undeformed: every vertex is where a plain cube of the same size has it (largest difference 0). 162 vertices, 160 faces |
| ffd/point-moved | FFD | default grid; move the top-front-right grid point by (0, 100, 0), done through the script | Grid point 8, at (150, 150, −150) before the move. 96 of the 162 vertices move, by up to 48.2 cm; the bounding box stays ±100, ±200, ±100 because the cube is taller than the 300 grid |
| formula/defaults | Formula | – | at frame 0. Formula text in Parameters above. bbox (−100, −238.556, −100)–(100, 240, 100) |
| formula/frame-10 | Formula | – | Document time frame 10 at 30 fps (t = 1/3 s). bbox (−100, −236.896, −100)–(100, 234.641, 100). `case.toml` carries `frame` and `fps` |
| melt/defaults | Melt | – | bbox (−225.984, 0, −242.676)–(248.252, 121.306, 250.795): y now runs only from 0 to 121 |
| melt/strength | Melt | strength 50 % | bbox (−312.972, 0, −341.190)–(350.616, 27.067, 354.915) |

## Notes

- Three rows repeat another case because the row's value is already a default (wrap/cylindrical, squash-stretch/bulge, and the width angle of wrap/angles); the planner may want other values (Spherical for Wrap, Expand away from 100 %).
- All 13 files have 162 vertices and 160 faces; a deformer never changes the topology.
- The Squash & Stretch Top and Bottom (±100) stay at the app's defaults and are not fitted to the 400 tall cube.
- The Wrap Width/Height/Radius defaults (400, 400, 200) also did not follow the cube.
- Not checked: Wrap Spherical, Formula with other formulas or the Manual effect, Squash & Stretch with the Spline/Custom type, FFD with other grid sizes, any Field in the falloff list.
