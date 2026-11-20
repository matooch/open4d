# Clone-toolset generators II — observation (O31)

Status: partial · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-20
Method: scripted. Each generator is made through its menu command; children are
inputs, as named. Export the whole result as `tests/golden/<feature>/<case>/mesh.obj`, and
also save the clone data (`samples.json`, kind `clones`, as [cloner.md](cloner.md)) where
the generator has clones. Use `observation = "docs/parity/clone-generators-2.md"`.

Don't use the clone toolset's brand-style names in folder names or keys. Use the feature
names given below.

Unblocks: P3 (Matrix, Fracture, Voronoi Fracture, the text generator, Tracer).

## Parameters (fill once per generator, as in cloner.md)

### Matrix (`matrix`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Mode | `mode` | 3 (options: Object, Linear, Radial, Grid, Honeycomb) | – | – |
| Generate | `generate` | 0 (options: Matrices Only, Thinking Particles) | – | – |
| Count | `count` | (40, 1, 40) | (1, 1, 1) | (2000000, 2000000, 2000000) |
| Mode | `count_mode` | 0 (options: Per-Step, Endpoint) | – | – |
| Size | `size` | (10.5, 10.5, 10.5) | – | – |
| Form | `count_form` | 0 (options: Cubic, Sphere, Cylinder, Object) | – | – |
| Fill | `count_fill` | 1 | 0 | 1 |
| Display | `display` | 0 (options: None, Weight, UV, Color, Index) | – | – |
| P | `p` | (0, 0, 0) | – | – |
| R | `r_deg` | (0, 0, 0) | – | – |
| S | `s` | (1, 1, 1) | – | – |
| Color | `color` | (1, 1, 1) | (0, 0, 0) | (1, 1, 1) |
| W(UV)-Orientation | `w_uv_orientation` | 3 (options: X+, X-, Y+, Y-, Z+, Z-) | – | – |
| Weight | `weight` | 0 | 0 | – |
| Time | `time` | <BaseTime> | – | – |
| Effectors | `effectors` | <InExcludeData> | – | – |

Matrix per mode (defaults only; the app shows no usable Min/Max for these):

#### Mode Object

| Our key | Default |
|---|---|
| `mode` | Object |
| `generate` | Matrices Only |
| `count` | 5 |
| `radius` | 50 |
| `plane` | XZ |
| `align` | on |
| `start_angle_deg` | 0 |
| `end_angle_deg` | 360 |
| `offset_deg` | 0 |
| `offset_variation_percent` | 0 |
| `offset_seed` | 1234567 |
| `display` | None |
| `p` | (0, 0, 0) |
| `r_deg` | (0, 0, 0) |
| `s` | (1, 1, 1) |
| `color` | (1, 1, 1) |
| `weight_percent` | 0 |

#### Mode Linear

| Our key | Default |
|---|---|
| `mode` | Linear |
| `generate` | Matrices Only |
| `count` | (40, 1, 40) |
| `count_mode` | Per-Step |
| `size` | (10.5, 10.5, 10.5) |
| `form` | Cubic |
| `fill_percent` | 100 |
| `display` | None |
| `p` | (0, 0, 0) |
| `r_deg` | (0, 0, 0) |
| `s` | (1, 1, 1) |
| `color` | (1, 1, 1) |
| `w_uv_orientation` | Y- |
| `weight_percent` | 0 |

#### Mode Radial

| Our key | Default |
|---|---|
| `mode` | Radial |
| `generate` | Matrices Only |
| `count` | 3 |
| `offset` | 0 |
| `per_step_mode` | Per Step |
| `amount_percent` | 100 |
| `p` | (0, 50, 0) |
| `s_percent` | (100, 100, 100) |
| `r_deg` | (0, 0, 0) |
| `step_mode` | Single Value |
| `step_size_percent` | 100 |
| `step_rotation_deg` | (0, 0, 0) |
| `display` | None |
| `effects_p` | (0, 0, 0) |
| `effects_r_deg` | (0, 0, 0) |
| `s` | (1, 1, 1) |
| `color` | (1, 1, 1) |
| `weight_percent` | 0 |

#### Mode Grid

| Our key | Default |
|---|---|
| `mode` | Grid |
| `generate` | Matrices Only |
| `object` |  |
| `align_clone` | on |
| `display` | None |
| `p` | (0, 0, 0) |
| `r_deg` | (0, 0, 0) |
| `s` | (1, 1, 1) |
| `color` | (1, 1, 1) |
| `weight_percent` | 0 |

#### Mode Honeycomb

| Our key | Default |
|---|---|
| `mode` | Honeycomb |
| `generate` | Matrices Only |
| `count` | (40, 1, 40) |
| `count_mode` | Per-Step |
| `size` | (10.5, 10.5, 10.5) |
| `form` | Cubic |
| `fill_percent` | 100 |
| `display` | None |
| `p` | (0, 0, 0) |
| `r_deg` | (0, 0, 0) |
| `s` | (1, 1, 1) |
| `color` | (1, 1, 1) |
| `w_uv_orientation` | Y- |
| `weight_percent` | 0 |

### Fracture (`fracture`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Mode | `mode` | 0 (options: Straight, Explode Segments, Explode Segments & Connect) | – | – |
| Display | `display` | 0 (options: None, Weight, UV, Color, Index) | – | – |
| P | `p` | (0, 0, 0) | – | – |
| R | `r_deg` | (0, 0, 0) | – | – |
| S | `s` | (1, 1, 1) | – | – |
| Color | `color` | (1, 1, 1) | (0, 0, 0) | (1, 1, 1) |
| Weight | `weight` | 0 | 0 | – |
| Effectors | `effectors` | <InExcludeData> | – | – |

### Voronoi Fracture (`voronoi-fracture`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| MoGraph Selection | `mograph_selection` | – | – | – |
| MoGraph Weightmap | `mograph_weightmap` | – | – | – |
| Colorize Fragments | `colorize_fragments` | 1 | 0 | 1 |
| Create N-Gon Surfaces | `create_n_gon_surfaces` | 1 | 0 | 1 |
| Offset Fragments | `offset_fragments` | 0 | 0 | – |
| Invert | `invert` | 0 | 0 | 1 |
| Hull Only | `hull_only` | 0 | 0 | 1 |
| Thickness | `thickness` | 0 | – | – |
| Hollow Object | `hollow_object` | 0 | 0 | 1 |
| Optimize and Close Holes | `optimize_and_close_holes` | 0 | 0 | 1 |
| Scale | `scale` | (1, 1, 1) | (0.01, 0.01, 0.01) | (100, 100, 100) |
| Save Result to File | `save_result_to_file` | 0 | 0 | 1 |
| Autoupdate Settings | `autoupdate_settings` | 1 | 0 | 1 |
| Autoupdate Animation | `autoupdate_animation` | 0 | 0 | 1 |
| Show All Used Points | `show_all_used_points` | 0 | 0 | 1 |
| Viewport Amount | `viewport_amount` | 1 | 0.1 | 1 |
| Sources | `sources` | <InExcludeData> | – | – |
| Sort Result | `sort_result` | 0 | 0 | 1 |
| Invert Sort | `invert_sort` | 0 | 0 | 1 |
| Sort Result by | `sort_by` | 1036 (options: X, Y, Z, User) | – | – |
| Distance to Object | `distance_to_object` | – | – | – |
| Enable Detailing | `enable_detailing` | 0 | 0 | 1 |
| Activate in Viewport | `activate_in_viewport` | 1 | 0 | 1 |
| Maximum Edge Length | `maximum_edge_length` | 10 | 0.1 | – |
| Noise Surface | `noise_surface` | 0 | 0 | 1 |
| Artefact Prevention Strength | `artefact_prevention_strength` | 20 | 1 | – |
| Smooth Normals | `smooth_normals` | 1 | 0 | 1 |
| Use Original Edges | `use_original_edges` | 1 | 0 | 1 |
| Phong Angle | `phong_angle` | 0.698132 | 0 | 3.141593 |
| Relax Inside Edges | `relax_inside_edges` | 0 | 0 | – |
| Keep Original Surface | `keep_original_surface` | 1 | 0 | 1 |
| Noise Type | `noise_type` | 21 (options: Box, Blistered Turbulence, Buya, Cell Noise, Cranal, Dents, Displaced Turbulence, Electric, FBM, Gaseous, Hama, Luka, Mod Noise, Naki, Nutous, Ober, Pezo, Poxo, Sema, Stupl, Turbulence, VL Noise, Wavy Turbulence, Cell Voronoi, Displaced Voronoi, Voronoi 1, Voronoi 2, Voronoi 3, Zada, Perlin, Ridged Multifractal) | – | – |
| Noise Strength | `noise_strength` | 20 | – | – |
| Noise Seed | `noise_seed` | 12345 | 0 | – |
| Octaves | `octaves` | 6 | 0 | 16 |
| Global Scale | `global_scale` | 10 | 0 | – |
| Relative Scale | `relative_scale` | (1, 1, 1) | (0, 0, 0) | – |
| Animation Speed | `animation_speed` | 0 | 0 | – |
| Low Clip | `low_clip` | 0 | 0 | 1 |
| High Clip | `high_clip` | 1 | 0 | 1 |
| Strength at Depth | `strength_at_depth` | <SplineData> | – | – |
| Use Connector | `use_connector` | – | – | – |
| Enable Geometry Glue | `enable_geometry_glue` | 0 | 0 | 1 |
| Glue Type | `glue_type` | 1 (options: Falloff, Cluster, Point Distance) | – | – |
| Glue Rest | `glue_rest` | 0 | 0 | 1 |
| Falloffs | `falloffs` | <InExcludeData> | – | – |
| Display | `display` | 0 (options: None, Weight, UV, Color, Index) | – | – |
| P | `p` | (0, 0, 0) | – | – |
| R | `r_deg` | (0, 0, 0) | – | – |
| S | `s` | (1, 1, 1) | – | – |
| Color | `color` | (1, 1, 1) | (0, 0, 0) | (1, 1, 1) |
| Weight | `weight` | 0 | 0 | – |
| Effectors | `effectors` | <InExcludeData> | – | – |
| Inside Faces | `inside_faces` | 0 | 0 | 1 |
| Inside Vertices | `inside_vertices` | 0 | 0 | 1 |
| Inside Vertex Map | `inside_vertex_map` | 0 | 0 | 1 |
| Outside Faces | `outside_faces` | 0 | 0 | 1 |
| Outside Vertices | `outside_vertices` | 0 | 0 | 1 |
| Outside Vertex Map | `outside_vertex_map` | 0 | 0 | 1 |
| Surface Break Edges | `surface_break_edges` | 0 | 0 | 1 |
| Edge Vertices | `edge_vertices` | 0 | 0 | 1 |
| Edge Vertex Map | `edge_vertex_map` | 0 | 0 | 1 |

### Text generator of the clone toolset (`clone-text`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Depth | `depth` | 20 | – | – |
| Subdivision | `subdivision` | 1 | 1 | – |
| Text Spline | `text_spline` | Text | – | – |
| Font | `font` | <FontData> | – | – |
| Align | `align` | 0 (options: Left, Middle, Right) | – | – |
| Height | `height` | 200 | – | – |
| Horizontal Spacing | `horizontal_spacing` | 0 | – | – |
| Vertical Spacing | `vertical_spacing` | 0 | – | – |
| Intermediate Points | `intermediate_points` | 3 (options: None, Natural, Uniform, Adaptive, Subdivided) | – | – |
| Number | `number` | 8 | 0 | 5000 |
| Angle | `angle` | 0.087266 | 0 | 1.570796 |
| Maximum Length | `maximum_length` | 5 | 0.01 | – |
| Shader Indexing | `shader_indexing` | 0 (options: Word Letter Index, Line Letter Index, All Letter Index) | – | – |
| Start Cap | `start_cap` | 1 | 0 | 1 |
| End Cap | `end_cap` | 1 | 0 | 1 |
| Separate Bevel Controls | `separate_bevel_controls` | 0 | 0 | 1 |
| Bevel Shape | `bevel_shape` | 0 (options: Round, Curve, Solid, Step) | – | – |
| Size | `size` | 0 | – | – |
| Extended Shape | `extended_shape` | 0 | 0 | 1 |
| Height | `bevel_height` | 0 | – | – |
| Shape Depth | `shape_depth` | 1 | – | – |
| Segments | `segments` | 3 | 1 | 100 |
| Bevel Shape | `end_bevel_shape` | 0 (options: Round, Curve, Solid, Step) | – | – |
| Size | `end_bevel_size` | 0 | – | – |
| Extended Shape | `end_bevel_extended_shape` | 0 | 0 | 1 |
| Height | `end_bevel_height` | 0 | – | – |
| Curve | `curve` | <SplineData> | – | – |
| Shape Depth | `end_bevel_shape_depth_percent` | 1 | – | – |
| Extended Shape | `end_bevel_extended_shape_absolute` | 0 | 0 | 1 |
| Use Absolute Value | `use_absolute_value` | 0 | 0 | 1 |
| Max Shape Height | `max_shape_height` | 5 | – | – |
| Segments | `end_bevel_segments` | 3 | 1 | 100 |
| Bevel Outside | `bevel_outside` | 0 | 0 | 1 |
| Avoid Self Intersections | `avoid_self_intersections` | 1 | 0 | 1 |
| Caps Type | `caps_type` | 2 (options: Triangles, Quadrangles, N-gon, Delaunay, Regular Grid) | – | – |
| Break Phong Rounding | `break_phong_rounding` | 1 | 0 | 1 |
| Shell | `shell` | 0 | 0 | 1 |
| Start Cap | `polygon_selection_start_cap_enabled` | 0 | 0 | 1 |
| End Cap | `polygon_selection_end_cap_enabled` | 0 | 0 | 1 |
| Start Bevel | `start_bevel` | 0 | 0 | 1 |
| End Bevel | `end_bevel` | 0 | 0 | 1 |
| Start Shell | `start_shell` | 0 | 0 | 1 |
| End Shell | `end_shell` | 0 | 0 | 1 |
| Start Cap | `edge_selection_start_cap_enabled` | 0 | 0 | 1 |
| End Cap | `edge_selection_end_cap_enabled` | 0 | 0 | 1 |
| Start Step | `start_step` | 0 | 0 | 1 |
| End Step | `end_step` | 0 | 0 | 1 |
| Effects | `effects` | <InExcludeData> | – | – |
| Axis | `axis` | (0.5, 0.5, 0.5) | (0, 0, 0) | (1, 1, 1) |
| P | `p` | (0, 0, 0) | – | – |
| R | `r_deg` | (0, 0, 0) | – | – |
| S | `s` | (1, 1, 1) | – | – |
| Color | `color` | (1, 1, 1) | (0, 0, 0) | (1, 1, 1) |
| Effects | `group2_effects` | <InExcludeData> | – | – |
| Axis | `group2_axis_percent` | (0.5, 0.5, 0.5) | (0, 0, 0) | (1, 1, 1) |
| Align Height | `align_height` | 1 | 0 | 1 |
| P | `group2_p` | (0, 0, 0) | – | – |
| R | `group2_r_deg` | (0, 0, 0) | – | – |
| S | `group2_s` | (1, 1, 1) | – | – |
| Color | `group2_color` | (1, 1, 1) | (0, 0, 0) | (1, 1, 1) |
| Effects | `group3_effects` | <InExcludeData> | – | – |
| Axis | `group3_axis_percent` | (0.5, 0.5, 0.5) | (0, 0, 0) | (1, 1, 1) |
| Align Height | `group3_align_height` | 1 | 0 | 1 |
| P | `group3_p` | (0, 0, 0) | – | – |
| R | `group3_r_deg` | (0, 0, 0) | – | – |
| S | `group3_s` | (1, 1, 1) | – | – |
| Color | `group3_color` | (1, 1, 1) | (0, 0, 0) | (1, 1, 1) |
| Effects | `group4_effects` | <InExcludeData> | – | – |
| Axis | `group4_axis_percent` | (0.5, 0.5, 0.5) | (0, 0, 0) | (1, 1, 1) |
| Align Height | `group4_align_height` | 1 | 0 | 1 |
| P | `group4_p` | (0, 0, 0) | – | – |
| R | `group4_r_deg` | (0, 0, 0) | – | – |
| S | `group4_s` | (1, 1, 1) | – | – |
| Color | `group4_color` | (1, 1, 1) | (0, 0, 0) | (1, 1, 1) |
| Weight | `weight` | 0 | 0 | – |
| Weight | `group2_weight_percent` | 0 | 0 | – |
| Weight | `group3_weight_percent` | 0 | 0 | – |
| Weight | `group4_weight_percent` | 0 | 0 | – |

### Tracer (`tracer`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Tracing Mode | `tracing_mode` | Trace Paths (options: Trace Paths, Trace Single Object, Connect All Objects) | – | – |
| Sample Step | `sample_step` | 1 | – | – |
| Trace Active | `trace_active` | on | – | – |
| Trace Vertices | `trace_vertices` | on | – | – |
| Use Tp Subgroups | `use_tp_subgroups` | on | – | – |
| Handle Cloners | `handle_cloners` | Nodes Only (options: Nodes Only, Nodes and Edges) | – | – |
| Include Cloner | `include_cloner` | off | – | – |
| Space | `space` | Global (options: Global, Local) | – | – |
| Limit | `limit` | None (options: None, From, To) | – | – |
| Type | `type` | Linear (options: Linear, Cubic, Bezier, B-Spline, Akima) | – | – |
| Close Spline | `close_spline` | off | – | – |
| Intermediate Points | `intermediate_points` | None (options: None, Natural, Uniform, Adaptive, Subdivided) | – | – |
| Number | `number` | 8 | – | – |
| Angle Deg | `angle_deg` | 5 | – | – |
| Maximum Length | `maximum_length` | 5 | – | – |
| Reverse Sequence | `reverse_sequence` | off | – | – |

## Cases

| Case (folder) | Generator | Inputs | Changed | Output |
|---|---|---|---|---|
| matrix/defaults | Matrix | – | – | samples only (no geometry); also export a 20 cm cube at the origin so the case has a mesh |
| matrix/linear | Matrix | – | mode Linear, count 5 | the same |
| fracture/explode-segments | Fracture | a default cube, then Make Editable and Disconnect every polygon | mode Explode Segments | mesh and samples |
| fracture/plain | Fracture with a Plain effector P (0, 100, 0) | the same | mode Explode Segments | mesh and samples |
| voronoi-fracture/defaults | Voronoi Fracture | a default cube | – (record the point source and its seed) | mesh |
| voronoi-fracture/count-10 | Voronoi Fracture | a default cube | point count 10, seed 1 | mesh |
| voronoi-fracture/sphere | Voronoi Fracture | a default sphere | point count 10, seed 1 | mesh |
| clone-text/defaults | the text generator | – | – | mesh. Record the font name shown. If the font is not a system default, use a plain system sans font and record which |
| clone-text/depth | the text generator | – | depth 50, text `AB` | mesh |
| tracer/two-nulls | Tracer | two Nulls keyed from (0, 0, 0) at 0 to (100, 100, 0) at 30 and from (100, 0, 0) to (200, 100, 0) | trace mode Connect All Objects | export at frame 30 through an Extrude, as in [splines-2.md](splines-2.md) |

## Case results

| Case | State |
|---|---|
| `matrix/defaults` | 8 points / 6 polygons; samples kind `clones`, 1600 clones; clone positions (-204.75, 0, -204.75) (-194.25, 0, -204.75) (-183.75, 0, -204.75) (-173.25, 0, -204.75) (-162.75, 0, -204.75) (-152.25, 0, -204.75) |
| `matrix/linear` | 8 points / 6 polygons; samples kind `clones`, 5 clones; clone positions (0, 0, 0) (0, 50, 0) (0, 100, 0) (0, 150, 0) (0, 200, 0) |
| `fracture/explode-segments` | 24 points / 6 polygons; no samples.json |
| `fracture/plain` | 24 points / 6 polygons; no samples.json |
| `voronoi-fracture/defaults` | not observed: the Voronoi Fracture cache stayed empty and Current State to Object returned no geometry (3 attempts, see Notes) |
| `voronoi-fracture/count-10` | not observed: same as voronoi-fracture/defaults (cache empty, no baked geometry) |
| `voronoi-fracture/sphere` | not observed: same as voronoi-fracture/defaults (cache empty, no baked geometry) |
| `clone-text/defaults` | 844 points / 367 polygons; samples kind `clones`, 4 clones; clone positions (59.814453, 69.873047, 0) (173.4375, 69.873047, 0) (281.884766, 69.873047, 0) (358.837891, 69.873047, 0) |
| `clone-text/depth` | 300 points / 328 polygons; samples kind `clones`, 2 clones; clone positions (65.478516, 71.728516, 0) (196.09375, 71.728516, 0) |
| `tracer/two-nulls` | not observed: the Tracer emitted no spline with either the Nulls as children or in its Trace Link list, so the Extrude above it baked empty (3 attempts, see Notes) |

## Notes
- Probes are in /Users/bigman/forge-observer/probes (o31_b33.py, o31_b34.py, o31_b38.py,
  o31_b40.py, o31_desc.json, o31_tables.md). Clones are counted with
  reading the clone count from the generator's clone data after the document has been evaluated, and geometry is baked with
  running the Current State to Object command on the generator.
- Matrix parameter list is mode dependent. The table under "Parameters" is the description dump
  taken with the generator's default Mode (Grid), so it lists the Grid/Honeycomb parameters plus
  the shared object tab. Per-mode defaults for all five modes are under "Matrix per mode". Count
  is a three-component vector in Grid and Honeycomb (default (40, 1, 40) = 1600 matrices) and a
  plain integer in Linear and Radial.
- Matrix Linear uses its own step parameters, not Size: with Count 5 the clones sit 50 cm apart
  along +Y (P = (0, 50, 0) on that tab), while Size 10.5 stays at its default and does not affect
  the step. In Grid mode Size is the step along each axis.
- Matrix/defaults therefore emits 1600 clone matrices and no geometry of its own; the case's
  mesh.obj is the standalone 20 cm child cube, as the form asks.
- Fracture: a default cube made editable and fully disconnected is 24 points / 6 quads. In
  Explode Segments mode the result is 6 fragments (24 points / 6 quads in total, one polygon per
  point quadruple), i.e. the pieces are separate objects and every piece keeps the cube's own
  points - no new points are created.
- fracture/plain put a Plain effector at P (0, 100, 0) as a second child of the Fracture and
  changed nothing: the baked point and polygon counts and the point coordinates are identical to
  fracture/explode-segments. So a Plain effector parented under the Fracture does not displace the
  fragments; the Effectors field of the Fracture is what feeds it. Recorded as observed, not as a
  bug: the case documents that children alone are inert.
- Voronoi Fracture produced no geometry through the bridge: after setting the point source to
  Point Count and the count/seed the generator's cache stayed empty and Current State to Object
  returned no object (the bake of the Voronoi object gave 0 points, 0 polygons). 3 attempts, then
  dropped per the debugging budget. Source parameters seen on the object (Object tab):
  point source count 10, seed 1, "Create N-gon Surfaces" and "Colorize Fragments" off by default.
  The three voronoi-fracture cases stay unobserved for that reason.
- The Text generator's Font field is a font-data field that exposes only get and set of the font description, so the font *name* shown in the UI could not be read through the bridge.
  clone-text/defaults and clone-text/depth were made with the default font the generator is
  created with (Cinema 4D's standard font), and no font was substituted.
- Tracer: not observed. With two keyed Nulls, Trace Active on, both Nulls in the Trace Link list
  (an include/exclude list field) and the document set to frame 30, Current State to Object on
  the Extrude above the Tracer produced 0 points and 0 polygons; switching on the Tracer's second option and
  leaving the two Nulls as ordinary children gave the same empty result. The scripted keyframe
  path works (keys are created and read back), so the missing piece is the Tracer's input binding,
  not the animation. 3 attempts, then dropped per the debugging budget.
- Our keys: snake_case of the visible label; where a generator has two labels with the same name
  the second one gets a descriptive suffix (`count_mode` is Matrix's Count Mode, `per_step_mode` its per-step mode);
  R is exported in degrees as `r_deg`; percentages keep the app's 0-100 range. The per-mode table
  uses the key names the probe read back, which are the same scheme.
- App version number read back through the scripting interface: 25010 (interface version 25004).
