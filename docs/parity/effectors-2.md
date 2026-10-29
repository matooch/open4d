# Effectors II — observation (O14)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: exactly as [cloner.md](cloner.md) (O8):
- a cloner made through its menu command, set to Linear mode with Count 5 and P per step
  (0, 50, 0), with a 20 cm cube child;
- the effector made through its menu command while the cloner is selected;
- clone data and `mesh.obj` in `tests/golden/cloner/<case>/`, with
  `observation = "docs/parity/effectors-2.md"`.

Unblocks: P3 effectors.

Added to `case.toml` over cloner.md: `document_frame` in `[params]` (the frame the data was
read at) and `name` in each effector table. A second effector is `[params.effector_2]`;
effectors are numbered in the order the cloner's effector list gives them.
Percent values are written in percent. Min and Max as in fields.md (declared limit, and in
brackets what a ±999999999 assignment kept). All 11 cases wrote 5 clones and a `mesh.obj`.

## Parameters

Shared by all six: Strength (`strength`, 100 %, −1e20 % to 1e20 %, kept ±999999999 as a fraction; Delay's
Strength is 50 % with limits 0 % to 100 %), Selection (text, empty), the Deformation drop-down
(`deformation`, Off; options Off, Object, Point, Polygon) and the effector's Field list (not a scalar, not
recorded). Maximum (`maximum`, 100 %) and Minimum (`minimum`, 0 %) exist on Plain, Step, Formula and Time
with limits ±1e20 % (kept ±999999999 as a fraction). The Parameter-tab block shared by Plain, Step,
Formula and Time: Transform Mode (`transform_mode`, Relative; Relative, Absolute, Remap), Transform Space
(`transform_space`, Node; Node, Effector, Object), Position/Rotation/Scale on-off, P / R / S vectors, Color
Mode (`color_mode`; options Off, Effector Color, Custom Color, Fields Color), Use Alpha/Strength (twice, the
second `use_alpha_strength_1071`), Blending Mode (Plain only), Weight Transform, U Transform, V Transform and
Modify Clone (0 %, ±1e20 %), Visibility (off). P and S are in cm / as a fraction shown in the vector fields; R is in
degrees. The vectors P, R, S are hidden until their on-off box is on (the app hides them; they can still be set).
The Custom Color swatch (hidden until Color Mode is Custom Color) defaults to white (1, 1, 1).

### Plain
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Transform Mode | `transform_mode` | Relative | – | Relative, Absolute, Remap |
| Position | `position` | on | – | – |
| P | `p` | (0, 100, 0) | −1e20 each | 1e20 each |
| Rotation | `rotation` | off | – | – |
| R | `r_deg` | (0, 0, 0)° | −1e20° each | 1e20° each |
| Scale | `scale` | off | – | – |
| Uniform Scale | `uniform_scale` | off | – | – |
| Absolute Scale | `absolute_scale` | off | – | – |
| S | `s` | (0, 0, 0) | −1e20 each | 1e20 each |
| Scale (value under Uniform Scale, hidden) | `scale_1018` | 0 | −1e20 (kept ±999999999) | 1e20 |
| Color Mode | `color_mode` | Fields Color | – | Off, Effector Color, Custom Color, Fields Color |
| Blending Mode | `blending_mode` | Default | – | Default, Add, Subtract, Multiply, Divide |

### Step
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Spline | – | (a curve, not recorded) | – | – |
| Step Gap | `step_gap` | 0 | 0 | 2147483647 (kept 999999999) |
| Position | `position` | off | – | – |
| Rotation | `rotation` | off | – | – |
| Scale | `scale` | **on** | – | – |
| Uniform Scale | `uniform_scale` | on | – | – |
| Absolute Scale | `absolute_scale` | off | – | – |
| Scale (value) | `scale_1018` | 1 | −1e20 (kept ±999999999) | 1e20 |
| Color Mode | `color_mode` | Off | – | as above |
| Use Alpha/Strength (Weight tab block) | `use_alpha_strength_1071` | **on** | – | – |

P (0, 0, 0), R (0, 0, 0), S (0, 0, 0) when switched on.

### Formula
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Formula | `formula` | `sin(((id / count) + t) * f * 360.0)` | – | – |
| t - Project Time | `t_project_time` | 1 | −1e20 (kept ±999999999) | 1e20 |
| f - Frequency | `f_frequency` | 1 | −1e20 (kept ±999999999) | 1e20 |
| Position | `position` | on | – | – |
| P | `p` | (50, 0, 0) | −1e20 each | 1e20 each |
| Rotation | `rotation` | off | – | – |
| Scale | `scale` | on | – | – |
| Uniform Scale | `uniform_scale` | on | – | – |
| Scale (value) | `scale_1018` | 1 | −1e20 | 1e20 |
| Color Mode | `color_mode` | Off | – | as above |

Six read-only text rows list the formula variables (`px,py,pz`, `rx,ry,rz`, `sx,sy,sz`, `u,v,w`, `id`, `count`, `falloff`); not recorded.

### Target
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Target Mode | `target_mode` | Object Target | – | Object Target, Look At Camera, Next Node, Previous Node, Field Direction |
| Target Object | `target_object` | empty | – | – |
| Use Pitch | `use_pitch` | on | – | – |
| Reverse Heading | `reverse_heading` | off | – | – |
| Up Vector | `up_vector` | None (as written to case.toml; the raw stored value is 7, which is not the list position) | – | None, Object, X+, X-, Y+, Y-, Z+, Z- |
| Repel | `repel` | off | – | – |

Target has no Maximum, Minimum or Parameter tab.

### Delay
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Strength | `strength` | 50 % | 0 % | 100 % |
| Mode | `mode` | Even | – | Even, Blend, Spring |
| Position | `position` | on | – | – |
| Rotation | `rotation` | on | – | – |
| Scale | `scale` | on | – | – |

### Time
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Position | `position` | off | – | – |
| Rotation | `rotation` | **on** | – | – |
| R | `r_deg` | (90, 0, 0)° | −1e20° each | 1e20° each |
| Scale | `scale` | off | – | – |
| Color Mode | `color_mode` | Off | – | as above |

## Cases

All cases: Linear cloner, Count 5, P per step (0, 50, 0); undisturbed clones sit at y 0, 50, 100, 150, 200. Clone data in `samples.json` (matrix, colour, weight, visible), `mesh.obj` for the cloned cubes.

| Case | Effector | Changed from default | Notes |
|---|---|---|---|
| plain-scale | Plain | Position off, Scale on, Uniform Scale on, Scale 2 | Clone x axis length **3.0** for all five clones: in Relative mode the value is added to the clone's scale of 1. Positions unchanged |
| plain-rotation | Plain | Position off, Rotation on, R.H 90° | Every clone rotated: first clone x axis (0, 0, 1), z axis (0, 0, −1) (cosine 6e-17). Positions unchanged |
| plain-color | Plain | Color Mode Custom Color, colour (1, 0, 0) | "Color Mode On" does not exist; Custom Color was used. All five clones colour (1, 0, 0). **Position was left on** (default), so clones sit at y 100…300 |
| plain-strength-50 | Plain | Strength 50 % (Position (0, 100, 0) is the default) | Clones at y 50, 100, 150, 200, 250: half of the effector's 100 |
| step-defaults | Step | – | Scale on by default: clone scale 1, 1.25, 1.5, 1.75, 2. Position unchanged. Colours white |
| step-position | Step | Position on, P (0, 100, 0) | y 0, 74.9999, 150.0001, 225.0001, 300; scale as in step-defaults. Values as the app returned them (74.9999, not 75) |
| formula-defaults | Formula | – | Formula `sin(((id / count) + t) * f * 360.0)`; frame 0 (t = 1 as the t field reads, f = 1). Clone x: 0, 47.5528, 29.3893, −29.3893, −47.5528; scale 1, 1.9511, 1.5878, 0.4122, 0.0489 |
| formula-frame-10 | Formula | – | Document at frame 10 (30 fps). Clone x: 43.3013, −10.3956, −49.7261, −20.3368, 37.1572; scale 1.866, 0.7921, 0.0055, 0.5933, 1.7431. The formula's own `t` field still reads 1: the result changes with the document time anyway |
| target-defaults | Target | Null at (0, 0, 300) as Target Object | Positions unchanged. Rotation changes: the last clone's y axis is (0, 0.8321, 0.5547), z axis (0, −0.5547, 0.8321), the first clone's axes are unchanged (identity). Null made through its menu command |
| time-frame-30 | Time | Position on, P (0, 100, 0) | Document at frame 30. Clones y 100…300. Rotation (default on, 90°) is also in the data |
| delay-skip | Delay | – | **No case written.** The clone data (position and the three axes of every clone) with a Delay effector at frame 0 are identical to the same cloner without any effector (largest difference 0.0) |
| two-effectors | Plain P (0, 100, 0), then Plain Scale 2 | – | The cloner's effector list reads Plain, Plain.1 (the second effector is named "Plain.1"). Clones y 100…300, scale 3 |
| two-effectors-swapped | Plain Scale 2, then Plain P (0, 100, 0) | – | List Plain, Plain.1 again, but now the first (named "Plain") is the scale one. **Samples are identical to two-effectors** (largest difference 0.0): the order does not matter for position and scale here |

## Notes
- Each effector adds a field to the cloner named after it, as in O8; the second Plain makes a second one (`plain_1_percent` next to `plain_percent`).
- Hidden parameters (P, R, S, the Custom Color swatch, Scale value, Absolute Scale) are not written to `case.toml` unless the on-off switch is on, because the app hides them; the defaults are in the tables above.
- Not checked: Step with a changed spline or Step Gap; Formula with another formula, t or f; Target modes other than Object Target; Delay on animated frames (the effector depends on previous frames); Time at other frames; Absolute and Remap Transform Mode; effectors with fields.
