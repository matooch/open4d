# R25 defaults and ranges — observation (O4)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-28
Bridge: MCP plugin 0.2.0 · Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)); objects created **through
their menu command** (O0 finding). Unblocks: brief 07 (switch our defaults to R25).

O1 showed R25 builds the same geometry as R20 but several defaults moved (sphere
segments, cylinder/torus segments and radii, extrude offset, boole type and hide new
edges). This form records every parameter of every feature we already implement, so our
defaults can follow R25 and our tests can keep the R20 defaults for R20 cases.

## How to fill it

For each feature: create the object with its menu command in a new document, read every
parameter we have a key for (the keys are in the feature's existing observation file
`docs/parity/<feature>.md`, "Our key" column), then type a huge and a negative value into
each numeric one and read back what it clamps to. Copy the R20 default from the existing
observation file. **Mark rows where R20 ≠ R25 with `!`** in the first column. Labels: only
where the label changed since R20.

Table format, one per feature (`<feature>` as in `tests/golden/<feature>/`):

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|

## How the values were read

One probe (outside the repo) made each object with its menu command in an empty document and
read every visible parameter of its Attribute Manager page: label, default and, for numbers,
what the app keeps after a value of 999999999 and of -999999999 is assigned through the
scripting interface (it clamps to the field's limits, like typing into the field). Angles are
shown in degrees and percentages in %, as the app shows them. The R20 column is copied from
the feature's observation file. **`!` marks R20 ≠ R25.** "none (±999999999 accepted)" = no
limit seen. Nothing was typed into the GUI, so a limit the app enforces only in the GUI would
not show here.

Rows that exist only in R25 (or were not in the R20 file) are under "New-object parameters"
below, not in the tables. Checkbox defaults are on/off.

## Features

### plane

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `width` | 400 | 400 | 0 | none (±999999999 accepted) | – |
|  | `height` | 400 | 400 | 0 | none (±999999999 accepted) | – |
| ! | `segments_w` | 20 | 10 | 1 | 1000 | – |
| ! | `segments_h` | 20 | 10 | 1 | 1000 | – |
|  | `orientation` | +Y | +Y | – | – | – |

### cube

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `size_x` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `size_y` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `size_z` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `segments_x` | 1 | 1 | 1 | 1000 | – |
|  | `_y` | 1 | 1 | 1 | 1000 | – |
|  | `_z` | 1 | 1 | 1 | 1000 | – |
|  | `separate_surfaces` | off | off | – | – | – |
|  | `fillet` | off | off | – | – | – |
| ! | `fillet_radius` | 40 | 3 | 0 | none (±999999999 accepted) | – |
| ! | `fillet_segments` | 5 | 3 | 1 | 1000 | – |

### sphere

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `radius` | 100 | 100 | 0 | none (±999999999 accepted) | – |
| ! | `segments` | 24 | 16 | 3 | 1000 | – |
|  | `type` | Standard | Standard | – | – | – |
|  | `render_perfect` | on | on | – | – | – |

### cylinder

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `radius` | 50 | 50 | 0 | none (±999999999 accepted) | – |
|  | `height` | 200 | 200 | 0 | none (±999999999 accepted) | – |
| ! | `height_segments` | 1 | 4 | 1 | 1000 | – |
| ! | `rotation_segments` | 36 | 16 | 3 | 1000 | – |
|  | `orientation` | +Y | +Y | – | – | – |
|  | `caps` | on | on | – | – | – |
|  | `cap_segments` | 1 | 1 | 1 | 1000 | Segments |
|  | `fillet` | off | off | – | – | – |
| ! | `fillet_segments` | 5 | 3 | 1 | 1000 | Segments (under Fillet) |
| ! | `fillet_radius` | 20 | 3 | 0 | none (±999999999 accepted) | Radius (under Fillet) |
|  | `slice` | off | off | – | – | – |
|  | `slice_from` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
| ! | `slice_to` | 360 | 180 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `regular_grid` | off | off | – | – | – |
|  | `regular_grid_width` | 10 | 10 | 0.01 | none (±999999999 accepted) | – |

### cone

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `top_radius` | 0 | 0 | 0 | none (±999999999 accepted) | – |
|  | `bottom_radius` | 100 | 100 | 0 | none (±999999999 accepted) | – |
|  | `height` | 200 | 200 | 0 | none (±999999999 accepted) | – |
| ! | `height_segments` | 8 | 4 | 1 | 1000 | – |
| ! | `rotation_segments` | 36 | 16 | 3 | 1000 | – |
|  | `orientation` | +Y | +Y | – | – | – |
|  | `caps` | on | on | – | – | – |
| ! | `cap_segments` | 3 | 1 | 1 | 1000 | – |
| ! | `fillet_segments` | 5 (reading uncertain) | 3 | 1 | 1000 | – |
|  | `slice` | off | off | – | – | – |
|  | `slice_from` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
| ! | `slice_to` | 360 | 180 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `regular_grid` | off | off | – | – | – |
|  | `regular_grid_width` | 10 | 10 | 0.01 | none (±999999999 accepted) | – |

### disc

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `inner_radius` | 0 | 0 | 0 | none (±999999999 accepted) | – |
|  | `outer_radius` | 100 | 100 | 0 | none (±999999999 accepted) | – |
| ! | `disc_segments` | 4 | 1 | 1 | 1000 | – |
| ! | `rotation_segments` | 36 | 16 | 3 | 1000 | – |
|  | `orientation` | +Y | +Y | – | – | – |
|  | `slice` | off | off | – | – | – |
|  | `slice_from` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
| ! | `slice_to` | 360 | 180 | none (±999999999 accepted) | none (±999999999 accepted) | – |

### torus

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
| ! | `ring_radius` | 200 | 100 | 0 | none (±999999999 accepted) | – |
| ! | `ring_segments` | 36 | 32 | 3 | 1000 | – |
|  | `pipe_radius` | 50 | 50 | 0 | none (±999999999 accepted) | – |
| ! | `pipe_segments` | 18 | 16 | 3 | 1000 | – |
|  | `orientation` | +Y | +Y | – | – | – |
|  | `slice` | off | off | – | – | – |
|  | `slice_from` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
| ! | `slice_to` | 360 | 180 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `regular_grid` | off | off | – | – | – |
|  | `regular_grid_width` | 10 | 10 | 0.01 | none (±999999999 accepted) | – |

### tube

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
| ! | `outer_radius` | 200 | 100 | 0 | none (±999999999 accepted) | – |
|  | `inner_radius` | 50 | 50 | 0 | none (±999999999 accepted) | – |
| ! | `rotation_segments` | 36 | 16 | 3 | 1000 | – |
|  | `cap_segments` | 1 | 1 | 1 | 1000 | – |
|  | `height` | 100 | 100 | 0 | none (±999999999 accepted) | – |
| ! | `height_segments` | 1 | 4 | 1 | 1000 | – |
|  | `orientation` | +Y | +Y | – | – | – |
|  | `fillet` | off | off | – | – | – |
|  | `slice` | off | off | – | – | – |
|  | `slice_from` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
| ! | `slice_to` | 360 | 180 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `regular_grid` | off | off | – | – | – |
|  | `regular_grid_width` | 10 | 10 | 0.01 | none (±999999999 accepted) | – |

### capsule

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `radius` | 50 | 50 | 0 | none (±999999999 accepted) | – |
|  | `height` | 200 | 200 | 0 | none (±999999999 accepted) | – |
| ! | `height_segments` | 8 (from the mesh: cylinder part split at Y -50 … 50 in steps of 12.5) | 4 | 1 | 1000 | – |
| ! | `cap_segments` | 8 (from the mesh: 8 latitude steps per hemisphere) | 4 | 1 | 1000 | – |
| ! | `rotation_segments` | 36 | 16 | 3 | 1000 | – |
|  | `orientation` | +Y | +Y | – | – | – |
|  | `slice` | off | off | – | – | – |
|  | `slice_from` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
| ! | `slice_to` | 360 | 180 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `regular_grid` | off | off | – | – | – |
|  | `regular_grid_width` | 10 | 10 | 0.01 | none (±999999999 accepted) | – |

### pyramid

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `size_x` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `size_y` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `size_z` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `segments` | 1 | 1 | 1 | 1000 | – |
|  | `orientation` | +Y | +Y | – | – | – |

### circle

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `ellipse` | off | off | – | – | – |
|  | `ring` | off | off | – | – | – |
|  | `radius` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `radius_y` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `inner_radius` | 100 | 100 | 0 | none (±999999999 accepted) | – |
|  | `plane` | XY | XY | – | – | – |
|  | `reverse` | off | off | – | – | – |
| ! | `intermediate` | Adaptive | Uniform | – | – | – |
|  | `intermediate_number` | 8 | 8 | 0 | 5000 | – |
|  | `intermediate_angle` | 5 | 5 | 0 | 90 | – |
|  | `intermediate_max_length` | 5 | 5 | 0.01 | none (±999999999 accepted) | – |

### rectangle

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `width` | 400 | 400 | 0 | none (±999999999 accepted) | – |
|  | `height` | 400 | 400 | 0 | none (±999999999 accepted) | – |
|  | `rounding` | off | off | – | – | – |
|  | `rounding_radius` | 50 | 50 | 0 | none (±999999999 accepted) | – |
|  | `plane` | XY | XY | – | – | – |
|  | `reverse` | off | off | – | – | – |
|  | `intermediate` | Adaptive | Adaptive | – | – | – |
|  | `intermediate_number` | 8 | 8 | 0 | 5000 | – |
|  | `intermediate_angle` | 5 | 5 | 0 | 90 | – |
|  | `intermediate_max_length` | 5 | 5 | 0.01 | none (±999999999 accepted) | – |

### helix

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `start_radius` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `start_angle` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `end_radius` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `end_angle` | 720 | 720 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `radial_bias` | 50 | 50 | 0 | 100 | – |
|  | `height` | 200 | 200 | 0 | none (±999999999 accepted) | – |
|  | `height_bias` | 50 | 50 | 0 | 100 | – |
|  | `subdivision` | 100 | 100 | 3 | none (±999999999 accepted) | – |
|  | `plane` | XY | XY | – | – | – |
|  | `reverse` | off | off | – | – | – |
| ! | `intermediate` | Adaptive | Uniform | – | – | – |
|  | `intermediate_number` | 8 | 8 | 0 | 5000 | – |
|  | `intermediate_angle` | 5 | 5 | 0 | 90 | – |
|  | `intermediate_max_length` | 5 | 5 | 0.01 | none (±999999999 accepted) | – |

### subdivision

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `scheme` | Catmull-Clark (N-Gons) | Catmull-Clark (N-Gons) | – | – | – |
|  | `level_view` | 2 | 2 | 0 | 6 | – |
| ! | `level_render` | 3 | 2 | 0 | 6 | – |
| ! | `subdivide_uvs` | Standard | Edge | – | – | – |

### boole

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `mode` | A subtract B | A subtract B | – | – | – |
|  | `high_quality` | on | on | – | – | – |
|  | `single_object` | off | off | – | – | – |
| ! | `hide_new_edges` | off | on | – | – | – |
|  | `phong_breaks` | off | off | – | – | – |
|  | `select_intersections` | off | off | – | – | – |
|  | `optimize_points` | 0.01 cm | 0.01 | 0.0001 | none (±999999999 accepted) | – |

### symmetry

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `mirror_plane` | ZY | ZY | – | – | – |
|  | `weld_points` | on | on | – | – | – |
|  | `tolerance` | 0.01 | 0.01 | 0 | none (±999999999 accepted) | Tolerance |
|  | `symmetrical` | on | on | – | – | – |
|  | `clamp_points_on_axis` | off | off | – | – | – |
|  | `delete_polygons_on_axis` | off (greyed) | off | – | – | – |
|  | `auto_flip` | off | off | – | – | – |

### array

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `radius` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `copies` | 7 | 7 | 1 | 1000 | – |
|  | `amplitude` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `frequency` | 0 | 0 | 0 | none (±999999999 accepted) | – |
|  | `array_frequency` | 10 | 10 | 0 | none (±999999999 accepted) | – |
|  | `render_instance` | off | off | – | – | – |

### connect

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `weld` | on | on | – | – | – |
|  | `tolerance` | 0.1 | 0.1 | 0 | none (±999999999 accepted) | – |
|  | `phong_mode` | As Breaks | As Breaks | – | – | – |
|  | `textures` | on | on | – | – | – |
|  | `center_axis` | off | off | – | – | – |

### instance

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `instance_mode` | Instance | Instance | – | – | – |

### extrude

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
| ! | `movement_z` | 20 | 100 | none (±999999999 accepted) | none (±999999999 accepted) | Movement (X, Y, Z) → Offset (one number; new Direction list) |
|  | `subdivision` | 1 | 1 | 1 | 4000 | – |
|  | `iso_subdivision` | 10 | 10 | 3 | 4000 | Iso Subdivision |
|  | `flip_normals` | off | off | – | – | – |
|  | `hierarchical` | off | off | – | – | – |
|  | `cap_start` | Cap (list) | on | – | – | Start (list) → checkbox |
|  | `cap_end` | Cap (list) | on | – | – | End (list) → checkbox |
|  | `cap_type` | N-gons | N-gon | – | – | Type |

### lathe

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `angle` | 360 | 360 | none (±999999999 accepted) | none (±999999999 accepted) | – |
| ! | `subdivision` | 24 | 32 | 1 | 4000 | – |
| ! | `isoparm_subdivision` | 4 | 10 | 3 | 4000 | – |
|  | `movement` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `scaling` | 100 | 100 | 0 | none (±999999999 accepted) | – |
|  | `flip_normals` | off | off | – | – | – |
|  | `cap_start` | Cap (list) | on | – | – | Start (list) → checkbox |
|  | `cap_end` | Cap (list) | on | – | – | End (list) → checkbox |
|  | `cap_type` | N-gons (same as Extrude) | N-gon | – | – | Type |

### loft

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
| ! | `mesh_subdivision_u` | 30 | 16 | 2 | 4000 | – |
| ! | `mesh_subdivision_v` | 10 | 4 | 2 | 4000 | – |
|  | `isoparm_subdivision_u` | 10 | 10 | 3 | 4000 | – |
|  | `organic_form` | off | off | – | – | – |
|  | `subdivision_per_segment` | on | on | – | – | – |
|  | `loop` | off | off | – | – | – |
|  | `linear_interpolation` | off | off | – | – | – |
|  | `adapt_uv` | on | on | – | – | – |
|  | `flip_normals` | off | off | – | – | – |
|  | `cap_start` | Cap (list) | on | – | – | Start (list) → checkbox |
|  | `cap_end` | Cap (list) | on | – | – | End (list) → checkbox |
| ! | `cap_type` | Triangles | N-gon | – | – | Type |

### sweep

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
| ! | `isoparm_subdivision` | 5 | 10 | 3 | 10000 | – |
|  | `end_scale` | 100 | 100 | 0 | none (±999999999 accepted) | – |
|  | `end_rotation` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `start_growth` | 0 | 0 | 0 | 100 | – |
|  | `end_growth` | 100 | 100 | 0 | 100 | – |
|  | `parallel_movement` | off | off | – | – | – |
|  | `constant_cross_section` | on | on | – | – | – |
|  | `banking` | on | on | – | – | – |
|  | `keep_segments` | off | off | – | – | – |
|  | `use_rail_direction` | on | on | – | – | – |
|  | `two_rail` | on | on | – | – | – |
|  | `use_rail_scale` | on | on | – | – | – |
|  | `flip_normals` | off | off | – | – | – |
|  | `stick_uvs` | off | off | – | – | – |
|  | `cap_start` | – (not recorded) | on | – | – | Start (list) → checkbox |
|  | `cap_end` | – (not recorded) | on | – | – | End (list) → checkbox |
|  | `cap_type` | – (not recorded) | N-gon | – | – | Type |

### bend

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `size_x` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `size_y` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `size_z` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `mode` | Limited | Limited | – | – | – |
|  | `strength` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `angle` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `keep_y_axis_length` | off | off | – | – | Keep Y-Axis Length |

### twist

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `size_x` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `size_y` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `size_z` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `mode` | Limited | Limited | – | – | – |
|  | `angle` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |

### taper

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `size_x` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `size_y` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `size_z` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `mode` | Limited | Limited | – | – | – |
|  | `strength` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `curvature` | 100 | 100 | 0 | none (±999999999 accepted) | – |
|  | `fillet` | off | off | – | – | – |

### bulge

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `size_x` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `size_y` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `size_z` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `mode` | Limited | Limited | – | – | – |
|  | `strength` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `curvature` | 100 | 100 | 0 | none (±999999999 accepted) | – |
|  | `fillet` | off | off | – | – | – |

### shear

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `size_x` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `size_y` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `size_z` | 250 | 250 | 0 | none (±999999999 accepted) | – |
|  | `mode` | Limited | Limited | – | – | – |
|  | `strength` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `angle` | 0 | 0 | none (±999999999 accepted) | none (±999999999 accepted) | – |
|  | `curvature` | 100 | 100 | 0 | none (±999999999 accepted) | – |
|  | `fillet` | off | off | – | – | – |

### spherify

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `strength` | 50 | 50 | 0 | 100 | – |
|  | `radius` | 200 | 200 | 0 | none (±999999999 accepted) | – |

### smoothing

Read from the tag that a menu-made cube already carries (R25 puts it on new primitives too).

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|
|  | `angle_limit` | on | on | – | – | – |
|  | `angle` | 40 | 40 | 0 | 180 | Phong Angle |
|  | `use_edge_breaks` | on | on | – | – | – |

## New-object parameters not in R20

Label as the Attribute Manager shows it (tab > field), R25 default, limits. We don't implement these yet.

**cone**

| Label | R25 default | Limits / options |
|---|---|---|
| Caps > Top Fillet | off | – |
| Caps > Top Fillet Radius | 3 | 0 … none (±999999999 accepted) |
| Caps > Top Fillet Height | 3 | 0 … none (±999999999 accepted) |
| Caps > Bottom Fillet | off | – |
| Caps > Bottom Fillet Radius | 3 | 0 … none (±999999999 accepted) |
| Caps > Bottom Fillet Height | 3 | 0 … none (±999999999 accepted) |

**tube**

| Label | R25 default | Limits / options |
|---|---|---|
| Object Properties > Fillet Segments | 3 | 1 … 1000 |
| Object Properties > Fillet Radius | 3 | 0 … none (±999999999 accepted) |

**extrude**

| Label | R25 default | Limits / options |
|---|---|---|
| Object Properties > Direction | Auto | – |
| Caps > Separate Bevel Controls | off | – |
| Both Bevels > Bevel Shape | Round | – |
| Both Bevels > Size | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Extended Shape | off | – |
| Both Bevels > Height | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Shape Depth | 100 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Segments | 3 | 1 … 100 |
| End Bevel > Bevel Shape | Round | – |
| End Bevel > Size | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Extended Shape | off | – |
| End Bevel > Height | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Shape Depth | 100 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Use Absolute Value | off | – |
| End Bevel > Max Shape Height | 5 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Segments | 3 | 1 … 100 |
| Caps > Bevel Outside | off | – |
| Caps > Avoid Self Intersections | on | – |
| Caps > Break Phong Rounding | on | – |
| Shell | off | – |
| Start Cap | off | – |
| End Cap | off | – |
| Start Bevel | off | – |
| End Bevel | off | – |
| Start Shell | off | – |
| End Shell | off | – |
| Start Step | off | – |
| End Step | off | – |

**lathe**

| Label | R25 default | Limits / options |
|---|---|---|
| Caps > Separate Bevel Controls | off | – |
| Both Bevels > Bevel Shape | Round | – |
| Both Bevels > Size | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Extended Shape | off | – |
| Both Bevels > Height | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Shape Depth | 100 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Segments | 3 | 1 … 100 |
| End Bevel > Bevel Shape | Round | – |
| End Bevel > Size | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Extended Shape | off | – |
| End Bevel > Height | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Shape Depth | 100 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Use Absolute Value | off | – |
| End Bevel > Max Shape Height | 5 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Segments | 3 | 1 … 100 |
| Caps > Bevel Outside | off | – |
| Caps > Avoid Self Intersections | on | – |
| Caps > Break Phong Rounding | on | – |
| Shell | off | – |
| Start Cap | off | – |
| End Cap | off | – |
| Start Bevel | off | – |
| End Bevel | off | – |
| Start Shell | off | – |
| End Shell | off | – |
| Start Step | off | – |
| End Step | off | – |

**loft**

| Label | R25 default | Limits / options |
|---|---|---|
| Caps > Separate Bevel Controls | off | – |
| Both Bevels > Bevel Shape | Round | – |
| Both Bevels > Size | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Extended Shape | off | – |
| Both Bevels > Height | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Shape Depth | 100 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Segments | 3 | 1 … 100 |
| End Bevel > Bevel Shape | Round | – |
| End Bevel > Size | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Extended Shape | off | – |
| End Bevel > Height | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Shape Depth | 100 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Use Absolute Value | off | – |
| End Bevel > Max Shape Height | 5 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Segments | 3 | 1 … 100 |
| Caps > Bevel Outside | off | – |
| Caps > Avoid Self Intersections | on | – |
| Caps > Break Phong Rounding | on | – |
| Shell | off | – |
| Start Cap | off | – |
| End Cap | off | – |
| Start Bevel | off | – |
| End Bevel | off | – |
| Start Shell | off | – |
| End Shell | off | – |
| Start Step | off | – |
| End Step | off | – |

**sweep**

| Label | R25 default | Limits / options |
|---|---|---|
| Details > Rotation from | -180 | none (±999999999 accepted) … none (±999999999 accepted) |
| Details > Rotation to | 180 | none (±999999999 accepted) … none (±999999999 accepted) |
| Caps > Separate Bevel Controls | off | – |
| Both Bevels > Bevel Shape | Round | – |
| Both Bevels > Size | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Extended Shape | off | – |
| Both Bevels > Height | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Shape Depth | 100 | none (±999999999 accepted) … none (±999999999 accepted) |
| Both Bevels > Segments | 3 | 1 … 100 |
| End Bevel > Bevel Shape | Round | – |
| End Bevel > Size | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Extended Shape | off | – |
| End Bevel > Height | 0 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Shape Depth | 100 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Use Absolute Value | off | – |
| End Bevel > Max Shape Height | 5 | none (±999999999 accepted) … none (±999999999 accepted) |
| End Bevel > Segments | 3 | 1 … 100 |
| Caps > Bevel Outside | off | – |
| Caps > Avoid Self Intersections | on | – |
| Caps > Break Phong Rounding | on | – |
| Shell | off | – |
| Start Cap | off | – |
| End Cap | off | – |
| Start Bevel | off | – |
| End Bevel | off | – |
| Start Shell | off | – |
| End Shell | off | – |
| Start Step | off | – |
| End Step | off | – |

**bend**

| Label | R25 default | Limits / options |
|---|---|---|
| Object Properties > Alignment | Y+ | – |

**twist**

| Label | R25 default | Limits / options |
|---|---|---|
| Object Properties > Alignment | Y+ | – |

**taper**

| Label | R25 default | Limits / options |
|---|---|---|
| Object Properties > Alignment | Y+ | – |

**bulge**

| Label | R25 default | Limits / options |
|---|---|---|
| Object Properties > Alignment | Y+ | – |

**shear**

| Label | R25 default | Limits / options |
|---|---|---|
| Object Properties > Alignment | Y+ | – |

## Notes

- Defaults that moved since R20 (all `!` rows) match what r25-drift.md found: Plane Width/Height Segments 10 (R20 20); Sphere Segments 16; Cube Fillet Radius 3 / Subdivision 3; Cylinder Height Segments 4, Rotation Segments 16, Fillet Segments 3, Fillet Radius 3; Cone, Disc, Tube, Capsule Rotation/Height/Cap Segments (see tables); Torus Ring Radius 100, Ring Segments 32, Pipe Segments 16; Boole Hide new edges on; Extrude Offset 100 (Movement Z 20); Lathe Subdivision 32, Isoparm 10; Loft Mesh Subdivision U 16, V 4; Sweep Isoparm 10; Subdivision Renderer 2, Subdivide UVs Edge; circle and helix Intermediate Points Uniform (rectangle stays Adaptive).
- **Slice To reads 180° on every object with a Slice tab (R20 file: 360, "reading uncertain").** The raw value read back was π, so the app reports 180; check it once in the GUI before the R25 target relies on it.
- **Boole Boolean Type reads A subtract B** (list: A union B, A subtract B, A intersect B, A without B) when the object is made by the menu with nothing selected. r25-drift.md says the R25 default is union; that case may have been built with the menu on two selected cubes. Not resolved here: re-check with the objects selected before trusting either.
- Hide new edges reads on, as in r25-drift.md; High Quality on.
- Subdivision: the OpenSubdiv-only rows (Boundary Interpolation etc.) are hidden for the default type and were not read; R20 values stand for them.
- Extrude, Lathe, Loft, Sweep share the same Caps and Bevel tabs in R25 (Start/End Cap are checkboxes now, R20: lists); the Caps Type default is N-gon for all four (R20 Loft: Triangles).
- Sweep: the Details tab (Rotation from -180, to 180) is under new parameters.
- Deformers: the box Size, Mode, Strength etc. are the same for the six; the new Alignment list (default Y+) sits on every deformer except Spherify. Spherify reads Strength 50 (0 … 100), Radius 200.
- Rows the app showed with no label (selection-tag checkboxes at the bottom of Extrude/Lathe/Loft/Sweep: Shell, Start Cap, …) are listed as read; defaults all off.
- Min/max are what the scripting interface kept, not typed GUI values (see "How the values were read").
- Not read: labels that changed without a matching R20 row (only a few are marked in the last column), Windows vs macOS.
