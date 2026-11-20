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
second `use_alpha_strength_weight`), Blending Mode (Plain only), Weight Transform, U Transform, V Transform and
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
| Scale (value under Uniform Scale, hidden) | `scale_value` | 0 | −1e20 (kept ±999999999) | 1e20 |
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
| Scale (value) | `scale_value` | 1 | −1e20 (kept ±999999999) | 1e20 |
| Color Mode | `color_mode` | Off | – | as above |
| Use Alpha/Strength (Weight tab block) | `use_alpha_strength_weight` | **on** | – | – |

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
| Scale (value) | `scale_value` | 1 | −1e20 | 1e20 |
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

## Follow-up cases (planner, after brief 070)

Brief 070 matched every case above (clone data); these pin the rules it had to guess. Same
cloner as the cases above; write `frame` and `fps` into every case.toml; Delay needs the
timeline played from frame 0 to the case frame before export.

| Case | Effector | Changed from default | Notes |
|---|---|---|---|
| step-count-9 | Step | cloner count 9, Position on, P (0, 100, 0) | Observed; vertices: 72 (unused 0); faces: 54 (4-gons 54); bbox min: (-20, -10, -20); bbox max: (20, 520, 20); size: (40, 530, 40); centre: (0, 255, 0); normals: 14; uvs: 4; baseline maximum difference 100.0; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| step-spline | Step | Position on, P (0, 100, 0), spline curve changed to an ease-in | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-20, -10, -20); bbox max: (20, 320, 20); size: (40, 330, 40); centre: (0, 155, 0); normals: 10; uvs: 4; baseline maximum difference 100.0; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| step-gap-1 | Step | Step Gap 1, Position on | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-20, -10, -20); bbox max: (20, 220, 20); size: (40, 230, 40); centre: (0, 105, 0); normals: 6; uvs: 4; baseline maximum difference 1.0; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| step-gap-2 | Step | Step Gap 2, Position on | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-17.500009, -10, -17.500009); bbox max: (17.500009, 217.500009, 17.500009); size: (35.000018, 227.500009, 35.000018); centre: (0, 103.7500045, 0); normals: 10; uvs: 4; baseline maximum difference 0.7500009170091844; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| formula-project-time-2 | Formula | t - Project Time 2, frame 10 | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-44.641016, -1.339746, -19.945219); bbox max: (69.671314, 212.079117, 19.945219); size: (114.31233, 213.418863, 39.890438); centre: (12.515148999999997, 105.3696855, 0); normals: 10; uvs: 4; baseline maximum difference 49.72609476841367; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| formula-frequency-2 | Formula | Frequency 2, frame 10 | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-44.641016, -1.339746, -19.945219); bbox max: (69.671314, 219.945219, 19.945219); size: (114.31233, 221.284965, 39.890438); centre: (12.515148999999997, 109.30273650000001, 0); normals: 10; uvs: 4; baseline maximum difference 49.72609476841367; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| formula-px-falloff | Formula | formula `px*falloff`, a Spherical field (size 200) | not observed: needs interactive export: scripted evaluation left it unchanged against unaffected cloner baseline; baseline maximum difference 0.0 |
| time-frame-15 | Time | Position on, P (0, 100, 0), frame 15 | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-14.142136, 40, -14.142136); bbox max: (14.142136, 260, 14.142136); size: (28.284272, 220, 28.284272); centre: (0, 150, 0); normals: 6; uvs: 4; baseline maximum difference 50.0; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| time-frame-60 | Time | the same, frame 60 | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-10, 190, -10); bbox max: (10, 410, 10); size: (20, 220, 20); centre: (0, 300, 0); normals: 6; uvs: 4; baseline maximum difference 200.0; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| target-reverse | Target | as `target-defaults`, Reverse Heading on | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-10, -10, -13.867505); bbox max: (10, 213.867505, 13.867505); size: (20, 223.867505, 27.73501); centre: (0, 101.9337525, 0); normals: 24; uvs: 4; baseline maximum difference 2.0; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| target-no-pitch | Target | Use Pitch off | not observed: needs interactive export: scripted evaluation left it unchanged against unaffected cloner baseline; baseline maximum difference 0.0 |
| target-up-y | Target | Up Vector Y+ | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-10, -10, -13.867505); bbox max: (10, 213.867505, 13.867505); size: (20, 223.867505, 27.73501); centre: (0, 101.9337525, 0); normals: 24; uvs: 4; baseline maximum difference 2.0; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| target-next-node | Target | Next Node | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-10, -10, -10); bbox max: (10, 210, 10); size: (20, 220, 20); centre: (0, 100, 0); normals: 6; uvs: 4; baseline maximum difference 1.0; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| target-strength-50 | Target | Strength 50 % | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-10, -10, -12.468762); bbox max: (10, 212.468762, 12.468762); size: (20, 222.468762, 24.937524); centre: (0, 101.234381, 0); normals: 24; uvs: 4; baseline maximum difference 0.2897841486884301; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| target-offaxis | Target | Null at (200, 100, 300) | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-15.148226, -12.308854, -15.788587); bbox max: (15.148226, 212.308854, 15.788587); size: (30.296452, 224.617708, 31.577174); centre: (0, 100, 0); normals: 30; uvs: 4; baseline maximum difference 0.5547001962252293; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |
| delay-blend-f5 | Delay | after a Time effector (Position on, P (0, 100, 0)); Mode Blend; frame 5 | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-11.873896, 3.443433, -11.873896); bbox max: (11.873896, 223.443433, 11.873896); size: (23.747792, 220, 23.747792); centre: (0, 113.443433, 0); normals: 10; uvs: 4; evaluated-cache snapshot export matches measured clones; direct procedural export was rejected after numeric mismatch; full validation blocked |
| delay-even-f5 | Delay | the same, Mode Even, frame 5 | not observed: needs interactive export: scripted evaluation left it unchanged against Time-only baseline; baseline maximum difference 16.666666666666664; Delay versus Time-only maximum difference 0.0 |
| delay-spring-f5 | Delay | the same, Mode Spring, frame 5 | Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-12.29342, 7.082914, -12.29342); bbox max: (12.29342, 227.082914, 12.29342); size: (24.58684, 220, 24.58684); centre: (0, 117.08291399999999, 0); normals: 10; uvs: 4; baseline maximum difference 17.082913757077506; Delay versus Time-only maximum difference 0.41624709041084884; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture |

## Notes
- Each effector adds a field to the cloner named after it, as in O8; the second Plain makes a second one (`plain_1_percent` next to `plain_percent`).
- Hidden parameters (P, R, S, the Custom Color swatch, Scale value, Absolute Scale) are not written to `case.toml` unless the on-off switch is on, because the app hides them; the defaults are in the tables above.
- Not checked: Step with a changed spline or Step Gap; Formula with another formula, t or f; Target modes other than Object Target; Delay on animated frames (the effector depends on previous frames); Time at other frames; Absolute and Remap Transform Mode; effectors with fields.

- Follow-up `target-up-x`: Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-10, -10, -13.867505); bbox max: (10, 213.867505, 13.867505); size: (20, 223.867505, 27.73501); centre: (0, 101.9337525, 0); normals: 24; uvs: 4; baseline maximum difference 2.0; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture

- Follow-up `delay-blend-f0`: not observed: needs interactive export: scripted evaluation left it unchanged against Time-only baseline; baseline maximum difference 0.0; Delay versus Time-only maximum difference 0.0

- Follow-up `delay-blend-f1`: Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-10.258598, -8.331667, -10.258598); bbox max: (10.258598, 211.668333, 10.258598); size: (20.517196, 220, 20.517196); centre: (0, 101.66833299999999, 0); normals: 6; uvs: 4; baseline maximum difference 1.6683333333333508; Delay versus Time-only maximum difference 1.665; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture

- Follow-up `delay-blend-f2`: Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-10.633109, -5.830001, -10.633109); bbox max: (10.633109, 214.169999, 10.633109); size: (21.266218, 220, 21.266218); centre: (0, 104.16999899999999, 0); normals: 6; uvs: 4; baseline maximum difference 4.169999166666685; Delay versus Time-only maximum difference 2.496667500000001; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture

- Follow-up `delay-blend-f10`: Observed; vertices: 40 (unused 0); faces: 30 (4-gons 30); bbox min: (-13.450648, 20.009876, -13.450648); bbox max: (13.450648, 240.009877, 13.450648); size: (26.901296, 220.000001, 26.901296); centre: (0, 130.0098765, 0); normals: 10; uvs: 4; baseline maximum difference 30.00987636983892; Delay versus Time-only maximum difference 3.323456963494415; complete measured parameters in new case.toml; full validation blocked by unrelated existing fixture

### Authorized full-run follow-up checkpoint (2026-11-17)

- Observer: Hermes, scripted public bridge; app version 25010; original observation metadata above preserved. Licence permission is owner-confirmed in observer state, not an independent licence review.
- New measured cases: 19. Safe-scope follow-ups exhausted; full validation/publication remain blocked by the unrelated existing restrict-bend duplicate parameter table. No commit or push.
- Every new case records measured visible parameters and exact input readbacks. Scratch probes and raw descriptions remain outside the repository.
- Outstanding: `formula-px-falloff`, `target-no-pitch`, `delay-blend-f0`, `delay-even-f5`. See case rows for measured failure/no-change reasons.
- Literal Target rows use the same target Null as target-defaults unless a different mode or position is requested. Extra X+ and Blend frames 0, 1, 2, 10 implement the cases explicitly requested in Notes. Step ease-in was chosen as a two-knot quadratic; points and tangent vectors are measured in its manifest. Step Gap rows leave the default P unchanged. No-change cases are not evidence of a broken bridge: px is zero on this input and disabling pitch may legitimately yield identity. Such unchanged outputs were withheld under the observer rule. Delay exports use evaluated cache snapshots because procedural export dropped Delay; the first new Blend-f5 fixture was corrected before this checkpoint, with its rejected export preserved in scratch.
