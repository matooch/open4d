# Spline primitives II — observation (O17)

Status: observed (22 of 22 cases; Min/Max of the coupled Cogwheel rows are TODO, see Notes) · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Level A. Unblocks: the
spline primitives we don't have yet (PLAN phase 5).

Splines don't export to OBJ (see [splines.md](splines.md)), so every spline is measured
through an **Extrude**:
- The spline is made through its menu command and set as the only child of an Extrude.
- The Extrude is set to Offset 20, with **Start Cap and End Cap off**.
- Export the Extrude as `tests/golden/splines-2/<case>/mesh.obj`. The side walls then hold
  every spline point twice: once at z 0 and once at z 20.

`case.toml`:
- `feature = "splines-2"`, `observation = "docs/parity/splines-2.md"`.
- `[params]`: `extrude_offset = 20.0`, `caps = false`.
- `[params.input]`: `type = "<name>_spline"` plus **every** parameter of the spline under
  our keys.

Our keys: the spline's own Type option is `<name>_type` (`arc_type`, `fourside_type`,
`cycloid_type`, `profile_type`, `cissoid_type`), because `type` already names the spline.
Angles are in degrees and percents in percent, as the Attribute Manager shows them. Every
case writes the full parameter list including Plane, Reverse and the four Intermediate
Points values (`intermediate`, `intermediate_number`, `intermediate_angle`,
`intermediate_max_length`). The Cogwheel also writes its guide colour as `[r, g, b]`.

## Parameters (fill once per spline, as in r25-defaults.md)

Min and Max come from assigning -999999999 and 999999999 and reading back what the
Attribute Manager holds. Angle rows that show ±57295779455 took the assignment as a plain
number of radians and did not clamp: they are effectively unbounded. A "–" in Min/Max means
the row is a toggle, a menu or text; menu options are listed in the Max column.

**Default Intermediate Points differ per spline**: Arc, Circle and Helix default to
**Uniform** (Number 8); every other spline in this form defaults to **Adaptive** (Angle 5°).
Number 8, Angle 5° and Maximum Length 5 are the same on all of them.

### Arc
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Type | `arc_type` | Arc | – | options: Arc, Sector, Segment, Ring |
| Radius | `radius` | 200 | 0 | 999999999 |
| Inner Radius | `inner_radius` | 100 | 0 | 999999999 |
| Start Angle | `start_angle` | 0 | -360 | 360 |
| End Angle | `end_angle` | 90 | -360 | 360 |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Uniform | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 999999999 |

### n-Side
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Radius | `radius` | 200 | 0 | 999999999 |
| Sides | `sides` | 6 | 2 | 999999999 |
| Rounding | `rounding` | off | – | – |
| Radius (rounding) | `rounding_radius` | 50 | 0 | 999999999 |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Adaptive | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 999999999 |

### Star
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Inner Radius | `inner_radius` | 100 | 0 | 999999999 |
| Outer Radius | `outer_radius` | 200 | 0 | 999999999 |
| Twist | `twist` | 0 % | -100 | 100 |
| Points | `points` | 8 | 3 | 1000000 |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Adaptive | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 999999999 |

### Flower
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Inner Radius | `inner_radius` | 100 | 0 | 999999999 |
| Outer Radius | `outer_radius` | 200 | 0 | 999999999 |
| Petals | `petals` | 8 | 3 | 999999999 |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Adaptive | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 999999999 |

### 4-Side
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Type | `fourside_type` | Diamond | – | options: Diamond, Kite, Parallelogram, Trapezium |
| a | `a` | 200 | 0 | 999999999 |
| b | `b` | 100 | 0 | 999999999 |
| Angle | `angle` | 30 | -90 | 90 |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Adaptive | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 999999999 |

### Cogwheel
The Object tab has two groups, **Teeth** (Type … Undercut Ratio) and an inner-shape group
(Type … Cutout Orientation). Defaults are as read from a fresh object. The numeric limits
of the Teeth rows move with each other (see Notes), so only the rows whose limit does not
depend on another value are given; the rest are TODO.

| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Legacy Mode | `legacy_mode` | off | – | – |
| Show Guides | `show_guides` | off | – | – |
| Guide Color | `guide_color` | (1, 0.913725, 0.498039) | 0 per channel | 1 per channel |
| Type (Teeth) | `teeth_type` | Involute | – | options: None, Involute, Ratchet, Flat |
| Teeth | `teeth` | 20 | 5 | 2000 |
| Lock Radius | `lock_radius` | off | – | – |
| Orientation (Teeth) | `teeth_orientation` | 0 | unbounded | unbounded |
| Undercut | `undercut` | off | – | – |
| Root Radius | `root_radius` | 180 | TODO (coupled) | TODO (coupled) |
| Addendum Radius | `addendum_radius` | 220 | TODO (coupled) | TODO (coupled) |
| Pitch Radius | `pitch_radius` | 200 | TODO (coupled) | TODO (coupled) |
| Module | `module` | 20 | TODO (coupled) | TODO (coupled) |
| Diametral Pitch | `diametral_pitch` | 0.05 | TODO (coupled) | TODO (coupled) |
| Dedendum | `dedendum` | 20 | TODO (coupled) | TODO (coupled) |
| Pressure Angle | `pressure_angle` | 20 | 0 | TODO (coupled; the control itself allows 30) |
| Radial Ratio | `radial_ratio` | 1.5 | 0 | 20 |
| Witness Ratio | `witness_ratio` | 10 % | 0 | 100 |
| Undercut Ratio | `undercut_ratio` | 10 % | 0 | 100 |
| Type (inner) | `inner_type` | None | – | options: None, Spokes, Holes, Arches, Waves |
| Orientation (inner) | `inner_orientation` | 0 | unbounded | unbounded |
| Invert | `invert` | off | – | – |
| Center Hole | `center_hole` | on | – | – |
| Radius (hole) | `hole_radius` | 15 | 0 | 999999999 |
| Cutout | `cutout` | off | – | – |
| Depth | `cutout_depth` | 10 | -999999999 | 999999999 |
| Width | `cutout_width` | 10 | 0 | TODO (coupled) |
| Cutout Orientation | `cutout_orientation` | 0 | unbounded | unbounded |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Adaptive | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 999999999 |

### Cycloid
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Type | `cycloid_type` | Cycloid | – | options: Cycloid, Epicycloid, Hypocycloid |
| Radius | `radius` | 25 | 0 | 999999999 |
| r | `r` | 50 | 0 | 999999999 |
| a | `a` | 75 | 0 | 999999999 |
| Start Angle | `start_angle` | 0 | unbounded | unbounded |
| End Angle | `end_angle` | 360 | unbounded | unbounded |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Adaptive | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 999999999 |

### Profile
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Type | `profile_type` | H Shape | – | options: H Shape, L Shape, T Shape, U Shape, Z Shape |
| Height | `height` | 200 | 0 | 999999999 |
| b | `b` | 100 | 0 | 999999999 |
| s | `s` | 20 | 0 | 999999999 |
| t | `t` | 20 | 0 | 999999999 |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Adaptive | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 999999999 |

### Cissoid
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Type | `cissoid_type` | Cissoid | – | options: Cissoid, Lemniscate, Strophoid |
| Width | `width` | 400 | 0 | 999999999 |
| Tension | `tension` | 2 | 1.5 | 100 |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Adaptive | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 999999999 |

### Formula
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| X(t) | `x_t` | `100.0*t` | – | text |
| Y(t) | `y_t` | `100.0*Sin(t*PI)` | – | text |
| Z(t) | `z_t` | `0.0` | – | text |
| Tmin | `t_min` | -1 | -999999999 | 999999999 |
| Tmax | `t_max` | 1 | -999999999 | 999999999 |
| Samples | `samples` | 20 | 2 | 999999999 |
| Cubic Interpolation | `cubic_interpolation` | off | – | – |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Adaptive | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 999999999 |

### Rectangle, Circle, Helix
These keep the tables in [splines.md](splines.md) (R20). In R25 the defaults read the
same (Rectangle 400 / 400, rounding radius 50; Circle radius 200, radius Y 200, inner
radius 100; Helix 200 / 0° / 200 / 720° / 50 % / 200 / 50 % / 100), apart from the
Intermediate Points default of Circle and Helix, which is **Uniform** in R25. Their size
limits read 0 to 999999999, Helix Subdivision 3 to 999999999, Radial and Height Bias 0 to
100 %.

## Cases

Counts are vertices / faces of the Extrude export (offset 20, caps off). Size and centre
are computed from the exported OBJ's bounding box. "Segments" is what the spline reports
for its own segment count (0 for a single closed or open path, 2 for the two loops of a
Ring and of the Cogwheel); "points" is the spline's own control point count.

| Case | Spline | Changed from default | Read back | Result |
|---|---|---|---|---|
| arc-defaults | Arc | – | type Arc, radius 200, inner 100, 0° to 90°, Uniform 8 | 74 / 36 (4-gons 36); size (199.96847, 200, 20), centre (100.015765, 100, 10); 5 spline points |
| arc-angles | Arc | start 30°, end 300° | start 30, end 300 (not clamped) | 74 / 36; size (373.616612, 400.987831, 20), centre (-13.603225, -0.362025, 10) |
| arc-sector | Arc | type Sector | type Sector | 90 / 45; size (197.108832, 199.824958, 20), centre (101.445584, 99.912479, 10) |
| nside-defaults | n-Side | – | radius 200, sides 6, Adaptive | 12 / 6; size (400, 346.410162, 20), centre (0, 0, 10); 6 points, so the Adaptive intermediate adds nothing on straight edges |
| nside-rounding | n-Side | Rounding on, radius 20 | rounding on, 20 | 132 / 66; size (393.986328, 346.410162, 20), centre (0, 0, 10); 12 spline points |
| nside-8 | n-Side | sides 8 | sides 8 | 16 / 8; size (400, 400, 20), centre (0, 0, 10) |
| star-defaults | Star | – | 100 / 200, twist 0 %, 8 points | 32 / 16; size (400, 400, 20), centre (0, 0, 10); 16 spline points |
| star-twist | Star | twist 30 %, points 5 | twist 30 %, points 5 | 20 / 10; size (361.803399, 380.422606, 20), centre (19.0983, 0, 10) |
| flower-defaults | Flower | – | 100 / 200, 8 petals | 944 / 472; size (393.380354, 393.380354, 20), centre (0, 0, 10); 24 spline points, curved edges so Adaptive adds many points |
| fourside-defaults | 4-Side | – | Diamond, a 200, b 100, 30° | 8 / 4; size (200, 100, 20), centre (0, 0, 10) |
| fourside-types | 4-Side | second Type option (Kite) | Kite | 8 / 4; size (178.885438, 223.606798, 20), centre (0, -67.082039, 10) |
| cog-defaults | Cogwheel | – | all values in the table above | 2216 / 1108; size (439.52699, 439.52699, 20), centre (0, 0, 10); 2 segments (outer outline and the centre hole), 244 spline points |
| cycloid-defaults | Cycloid | – | Cycloid, 25 / 50 / 75, 0° to 360° | 188 / 93; size (236.937433, 150, 20), centre (78.539817, 25, **-10**); 63 spline points. **The extrusion went to negative z (-20 to 0)**, see Notes |
| cycloid-epi | Cycloid | type Epicycloid | Epicycloid | 266 / 132; size (272.507401, 234.089577, 20), centre (13.7463, -25.892406, 10) |
| profile-defaults | Profile | – | H Shape, 200 / 100 / 20 / 20 | 24 / 12; size (100, 200, 20), centre (50, 100, 10); the shape sits in the +x, +y quadrant (origin at a corner), 12 points |
| cissoid-defaults | Cissoid | – | Cissoid, width 400, tension 2 | 202 / 100; size (400, 200, 20), centre (0, 100, 10); an open curve of 101 points (a strip of 100 faces) |
| formula-defaults | Formula | – (three formula texts recorded) | X(t) `100.0*t`, Y(t) `100.0*Sin(t*PI)`, Z(t) `0.0`, t -1 to 1, 20 samples | 42 / 20; size (200, 200, 20), centre (0, 0, 10); open curve of 21 points |
| rectangle-rounding | Rectangle | Rounding on, radius 50 | rounding on, 50 | 144 / 72; size (400, 400, 20), centre (0, 0, 10); 8 spline points |
| circle-ellipse | Circle | Ellipse on, radii 200 / 100 | ellipse on, radius 200, radius Y 100 | 72 / 36; size (400, 200, 20), centre (0, 0, 10); 4 spline points |
| circle-ring | Circle | Ring on, inner radius 100 | ring on, inner 100 | 144 / 72; size (400, 400, 20), centre (0, 0, 10); **2 segments**, 8 spline points |
| helix-uniform | Helix | the R25 default (Uniform 8) | Uniform, 8; 200 / 0° / 200 / 720° / 50 % / 200 / 50 % / 100 | 1802 / 900; size (400.001976, 403.133125, 219.731333), centre (-0.000988, -1.57182, -90.114274); 101 spline points. The extrusion is not along +z only, see Notes |
| circle-uniform | Circle | the R25 default (Uniform 8) | Uniform, 8 | 72 / 36; size (400, 400, 20), centre (0, 0, 10); 4 spline points |

`helix-uniform` and `circle-uniform` are the Helix and Circle at their R25 defaults: that
default is already Uniform with Number 8, so nothing was changed to make them.

## Notes
- All 22 cases ran without an error and every export succeeded. No dialog, no crash.
- **Intermediate Points default**: Uniform for Arc, Circle and Helix; Adaptive for all the
  others. The golden meshes follow that default (arc-* and circle-uniform/helix-uniform are
  Uniform).
- **Extrude direction is not always +z.** With Direction on Auto the Extrude pushed the
  default Cycloid to z -20…0 (the other flat splines to z 0…20). The Cycloid is the only
  flat case that goes negative. The Helix, a 3D curve, exported z from -199.98 to 19.75.
  So the sign of the extrusion depends on the spline; I did not find which rule it
  follows. TODO: needs a follow-up with Direction set to Z.
- **Open splines** (Arc, Cissoid, Formula, Arc Sector, Helix) give a strip: faces = points
  minus one (cissoid 100 faces for 101 points). Closed splines give a loop of faces = points.
- **Cogwheel limits are coupled.** Assigning ±999999999 to one of Root Radius, Addendum
  Radius, Pitch Radius, Module, Diametral Pitch, Dedendum or Pressure Angle changes the
  others, so the Min and Max read back in sequence depend on the order and left the object
  in a changed state (Dedendum 199.9999, Pressure Angle 2.56° and so on). That is why the
  defaults were re-read from a fresh object, and why those limits are TODO. The values
  that did not depend on other rows are in the table.
- The Cogwheel's Witness Ratio and Undercut Ratio are percents (10 %); the golden
  `case.toml` holds 10.0.
- The Type row of each spline is `<name>_type` in `case.toml` (see the top).
- Not covered by a case, per the form: Arc types Segment and Ring, 4-Side Parallelogram and
  Trapezium, Profile types other than H, Cissoid types Lemniscate and Strophoid, Cycloid
  Hypocycloid, Cubic Interpolation on the Formula, Plane XZ/ZY and Reverse on any of them.
