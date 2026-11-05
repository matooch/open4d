# Deformers III — observation (O19)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-04
Method: exactly as [deformers-2.md](deformers-2.md) (O9). The input is a cube made with
its menu command, size_y 400, segments 4/8/4. The deformer is made through its menu
command and moved under the cube as its child.
- Export the cube as `tests/golden/<deformer>/<case>/mesh.obj`.
- `case.toml` lists every deformer parameter under our keys, and the cube (or sphere) under
  `[params.input]`.
- `observation = "docs/parity/deformers-3.md"`.

Unblocks: P1, the remaining deformers.

## How the deformers were made

Each deformer was created through its menu command and then moved under the cube as a
child. The menu command ids found this run (Explosion 5145, Shatter 5148, Wind 5149,
Smoothing 1024529, Displacer 1018685, Spline Wrap 1019221, Polygon Reduction 1001253,
Bevel deformer icon command 431000028) stay in the observer probes and are not committed.
The **Polygon Reduction** menu command creates an object whose type name reads
"Legacy Polygon Reduction" — it is the only Polygon Reduction object the menu creates, so
it is the one observed. The Bevel deformer is not in the normal command registry; its
menu item is an icon command (found by name search over the command registry), and the
object is named "Bevel" in the scene.

Percent fields are written in percent (key ends `_percent`), angles in degrees (`_deg`).
Fields whose unit class is a length are written as plain numbers (the app's own display
value without a unit suffix), as with the other deformers in deformers-2.

## Parameters (fill once per deformer)

Min and Max are the limits the field declares; in brackets, what a ±999999999 assignment
kept where it differs. Vector fields were not assignment-tested. Every deformer here has
an empty Field list on its Falloff tab by default (group id 5100); it is not recorded.
The two exceptions found: Explosion, Shatter, Spline Wrap, Polygon Reduction and
Bevel deformer showed **no** Falloff group in their description at all; Smoothing,
Displacer and Wind have the Fields group (empty by default).

### Explosion (`explosion`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Strength | `strength_percent` | 0 % | 0 % | 100 % |
| Speed | `speed` | 100 | −1e20 | 1e20 (kept ±999999999) |
| Angle Speed | `angle_speed_deg` | 100° | −1e20° | 1e20° (kept ±999999999) |
| End Size | `end_size` | 0 | 0 | 1000 |
| Randomness | `randomness_percent` | 100 % | 0 % | 100 % |

### Shatter (`shatter`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Strength | `strength_percent` | 0 % | 0 % | 100 % |
| Angle Speed | `angle_speed_deg` | 100° | −1e20° | 1e20° (kept ±999999999) |
| End Size | `end_size` | 0 | 0 | 1000 |
| Randomness | `randomness_percent` | 100 % | 0 % | 100 % |

### Smoothing deformer (`smoothing-deformer`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Strength | `strength_percent` | 100 % | 0 % | 100 % |
| Type | `type` | Smooth | – | options: Relax, Smooth, Stretch |
| Iterations | `iterations` | 10 | 1 | 2147483647 (kept 999999999) |
| Stiffness | `stiffness_percent` | 30 % | 0 % | 100 % |
| Keep Displacements | `keep_displacements` | off | 0 | 1 |
| Stiffness Map | `stiffness_map` | empty | – | text (a link) |
| Memory (read-only) | – | `0 B` | – | not settable |

Has an empty Field list (Falloff tab); not recorded.

### Displacer (`displacer`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Emulation | `emulation` | off | 0 | 1 |
| Strength | `strength_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Height | `height` | 10 | −1e20 | 1e20 (kept ±999999999) |
| Type | `type` | Intensity (Centered) | – | options: Intensity, Intensity (Centered), Red / Green, RGB (XYZ Tangent), RGB (XYZ Object), RGB (XYZ World) |
| Direction | `direction` | Vertex normal | – | options: Vertex normal, Spherical, Planar |
| Process Normal Tag | `process_normal_tag` | off | 0 | 1 |
| Channel | `channel` | Custom Shader | – | options: Color, Luminance, Transparency, Environment, Bump, Alpha, Specular, Displacement, Diffusion, Normal, Custom Shader |
| Shader | `shader` | empty | – | a shader link |
| Offset U | `offsetu` | 0 | −1e20 | 1e20 (kept ±999999999) |
| Offset V | `offsetv` | 0 | −1e20 | 1e20 (kept ±999999999) |
| Length U | `lengthu_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Length V | `lengthv_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Tile | `tile` | on | 0 | 1 |
| Use | `use` | Gray | – | options: Alpha, Gray, Red, Green, Blue |
| Invert | `invert` | off | 0 | 1 |
| Object | `object` | on | 0 | 1 |
| Camera | `camera` | off | 0 | 1 |
| Backface Cull | `backface_cull` | off | 0 | 1 |

Has an empty Field list (Falloff tab); not recorded.

### Polygon Reduction (`polygon-reduction`) — menu creates "Legacy Polygon Reduction"
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Reduction Strength | `reduction_strength_percent` | 90 % | 0 % | 100 % |
| Mesh Quality Factor | `mesh_quality_factor` | 1000 | 0 | 20000 |
| Co-Planar Optimization | `co_planar_optimization` | on | 0 | 1 |
| Bounday Curve Preservation (the app's spelling) | `bounday_curve_preservation` | on | 0 | 1 |
| Polygon Quality Preservation | `polygon_quality_preservation` | off | 0 | 1 |

No Falloff group in its description.

### Bevel deformer (`bevel-deformer`) — the scene object is named "Bevel"
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Component Mode | `component_mode` | Edges | – | options: Points, Edges, Polygons |
| Selection | `selection` | empty | – | text (a selection name) |
| Use Angle | `use_angle` | on | 0 | 1 |
| Angle Threshold | `angle_threshold_deg` | 40° | 0° | 1e20° (kept 999999999) |
| Bevel Mode | `bevel_mode` | Chamfer | – | options: Chamfer, Solid |
| Offset Mode | `offset_mode` | Fixed Distance | – | options: Fixed Distance, Radial, Proportional |
| Offset | `offset` | 1 | 0 | 1e20 (kept ±999999999) |
| Subdivision | `subdivision` | 0 | 0 | 1e20 (kept 999999999) |
| Depth | `depth_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Limit | `limit` | on | 0 | 1 |
| Shape | `shape` | Round | – | options: Round, User, Profile |
| Tension | `tension_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Mitering | `mitering` | Default | – | options: Default, Uniform, Radial, Patch |
| Ending | `ending` | Default | – | options: Default, Extend, Inset |
| Partial Rounding | `partial_rounding` | None | – | options: None, Full, Convex |
| Corner N-gons | `corner_n_gons` | off | 0 | 1 |
| Rounding N-gons | `rounding_n_gons` | off | 0 | 1 |
| Phong Break Rounding | `phong_break_rounding` | off | 0 | 1 |
| Phong Break Miters | `phong_break_miters` | off | 0 | 1 |

No Falloff group in its description.

### Spline Wrap (`spline-wrap`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Spline | `spline` | empty | – | a spline link |
| Rail | `rail` | empty | – | a spline link |
| Axis | `axis` | +X | – | options: +X, −X, +Y, −Y, +Z, −Z |
| Strength | `strength_percent` | 100 % | 0 % | 1e20 % (kept 999999999 as a fraction) |
| Offset | `offset_percent` | 0 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| From | `from_percent` | 0 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| To | `to_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Mode | `mode` | Fit Spline | – | options: Fit Spline, Keep Length |
| End Mode | `end_mode` | Extend | – | options: Clamp, Extend |
| Scale from Rail | `scale_from_rail` | off | 0 | 1 |
| Size Strength | `size_strength_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Spline Size Strength | `spline_size_strength_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Rotation from Rail | `rotation_from_rail` | on | 0 | 1 |
| Up Vector | `up_vector` | (0, 0, 0) | −1e20 each | 1e20 each |
| Banking | `banking_deg` | 0° | −1e20° | 1e20° (kept ±999999999) |
| Rotation Strength | `rotation_strength_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Spline Rotation Strength | `spline_rotation_strength_percent` | 100 % | −1e20 % | 1e20 % (kept ±999999999 as a fraction) |
| Fix Bounding Box | `fix_bounding_box` | off | 0 | 1 |
| Bounding Box Center | `bounding_box_center` | (0, 0, 0) | −1e20 each | 1e20 each |
| Bounding Box Size | `bounding_box_size` | (200, 200, 200) | −1e20 each | 1e20 each |

No Falloff group found in its description, in contrast to Smoothing/Displacer/Wind.

### Wind (`wind`)
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Amplitude | `amplitude` | 50 | 0 | 1e20 |
| Size | `size` | 100 | 0 | 1e20 |
| Frequency | `frequency` | 1 | 0 | 1e20 |
| Turbulence | `turbulence_percent` | 50 % | 0 % | 100 % |
| fx | `fx` | 1 | 0 | 1e20 |
| fy | `fy` | 1 | 0 | 1e20 |
| Flag | `flag` | on | 0 | 1 |

Has an empty Field list (Falloff tab); not recorded. Min/Max taken from the field
declarations; the ±999999999 assignment check was not run on Wind (time ran short).

## Cases

| Case | Deformer | Changed from default | Notes |
|---|---|---|---|
| explosion/defaults | Explosion | – | 162 vertices, 160 faces; bbox ±100, ±200, ±100: the cube is unchanged (Strength 0 %, at frame 0) |
| explosion/strength-50 | Explosion | strength 50 % | 640 vertices, 160 faces; bbox (−701.699, −865.045, −612.044)–(805.058, 701.724, 702.380). The polygon islands fly apart and the export splits them: the face count stays 160 but every face exports its own vertices |
| shatter/defaults | Shatter | – | 162 vertices, 160 faces, bbox ±100, ±200, ±100: the cube is unchanged (Strength 0 %, at frame 0) |
| shatter/strength-50 | Shatter | strength 50 % | 640 vertices, 160 faces; bbox (−145.331, −17.428, −141.107)–(144.018, 17.428, 138.279). The polygons rotate around their centers without flying outwards |
| smoothing-deformer/defaults | Smoothing deformer | – | input: cube made editable (4/4/4 segments, Fillet off), point 0 moved from (−100, −200, −100) by (50, 50, 50). 98 vertices, 96 faces; bbox (−77.047, −154.821, −77.047)–(77.786, 155.560, 77.786): the moved point was pulled back by smoothing |
| smoothing-deformer/iterations-10 | Smoothing deformer | iterations 10 | **Iterations 10 is already the default**, so the mesh is the same as defaults (identical bbox). Exported as its own case |
| displacer/defaults | Displacer | – | 162 vertices, 160 faces, bbox ±100, ±200, ±100: with no shader the Displacer moves nothing despite Height 10 |
| displacer/height-noise | Displacer | Shading: a Noise shader with default settings, Height 20 | 162 vertices, 160 faces; bbox (−107.091, −210.865, −106.800)–(113.327, 204.663, 110.652). The noise shader's default seed reads 665 (the app assigns it on creation); recorded under `[params.shader]` |
| polygon-reduction/defaults | Polygon Reduction | – | input: a default sphere (100 radius, 16 segments) instead of the cube. 13 vertices, 22 faces from the original 257/512: Reduction Strength 90 % did most of the reduction already at the defaults |
| polygon-reduction/50 | Polygon Reduction | strength 50 % | same input. 58 vertices, 112 faces; the mesh is still near the full sphere |
| bevel-deformer/defaults | Bevel deformer | – | input: cube with segments 1/1/1. 24 vertices, 26 faces at offset 1: all 12 edges chamfered (Use Angle on, 40°) |
| bevel-deformer/offset-20 | Bevel deformer | offset 20, subdivision 2 | 96 vertices, 98 faces; bbox stays ±100, ±200, ±100 |
| spline-wrap/defaults | Spline Wrap | the spline link set to a default Helix at the origin | 162 vertices, 160 faces; bbox (98.575, −251307.554, −20228.516)–(2320.472, 248791.547, 20234.225): with Mode Fit Spline the cube stretches along the whole helix (720°, height 200). The Helix is at the origin and its parameters are under `[params.spline]`; the spline itself is not exported into the OBJ |
| spline-wrap/arc | Spline Wrap | the link set to a default Arc spline | 162 vertices, 160 faces; bbox (−31109.951, −31424.238, −200)–(877.734, 874.024, 200). Arc (200 radius, 0°–90°) recorded under `[params.spline]` |
| wind/frame-10 | Wind | – | document at frame 10, 30 fps (`frame` and `fps` in the toml). 162 vertices, 160 faces; bbox (−100, −200, −172.439)–(100, 200, 161.438): the amplitude pushes x-slices in z; y stays exact |

## Follow-up cases (planner, after brief 053)

Brief 053 matched the strength-0 cases exactly and the strength-50 and wind cases only
statistically (the random stream and wind noise are unknown). These pin each law with tiny
inputs. Use a plane made editable as the child; frame 0 unless stated; **write `frame` and
`fps` into every case.toml** (`wind/frame-10` lacked them).

| Case | Deformer | Child | Changed from default |
|---|---|---|---|
| explosion/one-poly-25 | Explosion | a 1 × 1-segment plane | strength 25 % |
| explosion/one-poly-100 | Explosion | the same | strength 100 % |
| explosion/two-poly-50 | Explosion | a 2 × 1-segment plane | strength 50 % |
| explosion/rand-0 | Explosion | a 2 × 1-segment plane | strength 50 %, Randomness 0 % |
| explosion/end-size-100 | Explosion | the same | strength 50 %, End Size 100 % |
| explosion/speed-0 | Explosion | the same | strength 50 %, Speed 0 |
| explosion/angle-speed-200 | Explosion | the same | strength 50 %, Angle Speed 200 |
| shatter/one-poly-25 | Shatter | a 1 × 1-segment plane | strength 25 % |
| shatter/two-poly-50 | Shatter | a 2 × 1-segment plane | strength 50 % |
| shatter/rand-0 | Shatter | the same | strength 50 %, Randomness 0 % |
| wind/frame-0 | Wind | default cube 4 × 8 × 4 segments (as wind/frame-10) | frame 0 |
| wind/frame-20 | Wind | the same | frame 20 |
| wind/flag-off | Wind | the same | frame 10, Flag off |
| wind/size-50 | Wind | the same | frame 10, Size 50 |
| wind/size-200 | Wind | the same | frame 10, Size 200 |
| wind/row-x | Wind | a 10 × 1-segment plane along x | frame 10 |

## Notes

- All 15 cases of the form were run; none were skipped. 15 `mesh.obj` + `case.toml` pairs.
- A deformer never changes the face count of the cube (160 in every case); Explosion and
  Shatter split the vertices apart (640 vertices) because each island exports separately.
- The Polygon Reduction menu command only creates the "Legacy Polygon Reduction" object; if
  a non-legacy Polygon Reduction exists in R25 it is not reachable from a menu command. The
  two polygon-reduction cases use it and say so in their descriptions.
- The Bevel deformer's scene object is named "Bevel" (type name too); its menu command is
  an icon command, not a normal command id — worth knowing for the implementer's testing.
- smoothing-deformer/iterations-10 duplicates defaults because 10 is the default Iterations;
  the form's row said "iterations 10 (or the closest field)".
- Not checked: Shatter/Explosion at nonzero frames, Displacer with other Channel values,
  Bevel deformer in Solid mode or with a selection, Spline Wrap with a Rail or Keep Length,
  Wind limits assignment check, any Field in the Falloff list of the three deformers that
  have one.