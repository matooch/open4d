# Effectors III — observation (O22)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: exactly as [effectors-2.md](effectors-2.md) (O14):
- Linear cloner, count 5, P per step (0, 50, 0), with a 20 cm cube child.
- The effector is made through its menu command with the cloner selected.
- Save the clone data and `mesh.obj` to `tests/golden/cloner/<case>/`, with
  `observation = "docs/parity/effectors-3.md"`.

Unblocks: P3 effectors in depth.

All 17 cases wrote a `samples.json` and a `mesh.obj`. Percent values are written in percent, as in effectors-2.md.
Hidden parameters are not written to `case.toml` (the app hides them). Min and Max: declared limit, and in
brackets what a ±999999999 assignment kept, where that differs.
Extra tables in `case.toml`: `[params.effector.field]` (a field made with the effector selected: it joined the
effector's field list by itself), `[params.effector.shader_object]`, `[params.effector.spline_object]`,
`[params.effector.volume_cube]` and `[params.effector.inherit_cloner]` hold the parameters of the helper object.

## Parameters

Shared by all six, as in effectors-2.md: Strength (100 %), Selection (empty), Deformation (Off) and a Field list.
Maximum / Minimum (limits ±1e20 %) exist on Random, Shader and Volume only. Transform Mode / Transform Space / Position /
Rotation / Scale / Color Mode / Use Alpha/Strength / Weight, U, V Transform / Modify Clone / Visibility are as in Plain
(effectors-2.md) on Random, Shader and Volume; Spline, Push Apart and Inheritance have the short sets below.

### Random: Random Mode options and settings
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Random Mode | `random_mode` | Random | – | Random, Gaussian, Noise, Turbulence, Sorted |
| Synchronized | `synchronized` | off | – | – |
| Indexed | `indexed` | off | – | – |
| Seed | `seed` | 12345 | 0 | 1e20 (kept 999999999) |
| Space | `space` | Global | – | Global, UV |
| Animation Speed | `animation_speed_percent` | 100 % | 0 % | 1e20 % (kept 999999999) |
| Scale (noise scale) | `scale_percent` | 100 % | 0 % | 1e20 % (kept 999999999) |
| Maximum | `maximum_percent` | 100 % | −1e20 % | 1e20 % |
| Minimum | `minimum_percent` | **−100 %** | −1e20 % | 1e20 % |
| Position | `position` | **on** | – | – |
| P | `p` | (50, 50, 50) | −1e20 each | 1e20 each |
| Rotation | `rotation` | off | – | – |
| Scale | `scale` | off | – | – |
| Color Mode | `color_mode` | Off | – | Off, Effector Color, Custom Color, Fields Color |
| Use Alpha/Strength (Weight tab block) | `use_alpha_strength_1071` | on | – | – |

"Indexed" is a switch next to Random Mode, not an entry of its list. Uniform Scale and the Scale value (`uniform_scale`, `scale_1018`) appear once Scale is on; with Scale on the observed values were Uniform Scale on, Scale 1 (the value set by the probe; the default value of the hidden field was not read).

### Shader
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Position | `position` | off | – | – |
| Rotation | `rotation` | off | – | – |
| Scale | `scale` | on | – | – |
| Uniform Scale | `uniform_scale` | on | – | – |
| Scale (value) | `scale_1018` | 0.5 | −1e20 (kept ±999999999) | 1e20 |
| Color Mode | `color_mode` | Effector Color as the file reads it (the stored value is 3; the drop-down list order differs from the stored numbers) | – | Off, Effector Color, Custom Color, Fields Color |
| Blending Mode | `blending_mode` | Default | – | Default, Add, Subtract, Multiply, Divide |
| Channel | `channel` | Color (stored 830); reads "Custom Shader" once a shader is inserted | – | Color, Luminance, Transparency, Environment, Bump, Alpha, Specular, Displacement, Diffusion, Normal, Custom Shader |
| Shader | `shader` | empty | – | – |
| OffsetU / OffsetV | `offsetu`, `offsetv` | 0 / 0 | −1e20 | 1e20 |
| LengthU / LengthV | `lengthu_percent`, `lengthv_percent` | 100 % / 100 % | −1e20 % | 1e20 % |
| Tile | `tile` | on | – | – |
| Use | `use` | Gray (as `case.toml` reads it with the shader inserted; the stored value 0 was not mapped to a name before) | – | Alpha, Gray, Red, Green, Blue |
| Invert | `invert` | off | – | – |
| Maximum / Minimum | `maximum_percent`, `minimum_percent` | 100 % / 0 % | ±1e20 % | |

### Spline
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Mode | `mode` | Step | – | Step, Falloff, Relative |
| Spline | `spline` | empty | – | – |
| Up Vector | `up_vector` | (0, 0, 0) | −1e20 each | 1e20 each |
| Rail | `rail` | empty | – | – |
| Target Rail | `target_rail` | on | – | – |
| Offset | `offset_percent` | 0 % | −1e20 % | 1e20 % |
| Start | `start_percent` | 0 % | −1e20 % | 1e20 % |
| End | `end_percent` | 100 % | −1e20 % | 1e20 % |
| Clamp | `clamp` | off | – | – |
| Segment Mode | `segment_mode` | Use Index | – | Use Index, Even Spacing, Random, Full Spacing |
| Segment | `segment` | 0 | 0 | 2147483647 (kept 999999999) |
| Transform Mode | `transform_mode` | **Absolute** | – | Absolute, Relative to Node, Relative to Spline |
| Position | `position` | on | – | – |
| Rotation | `rotation` | on | – | – |
| R | `r_deg` | (0, 0, 0)° | −1e20° each | 1e20° each |

Spline has no Maximum, Minimum or Selection.

### Push Apart
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Mode | `mode` | Push Apart | – | Hide, Push Apart, Scale Apart, Along X, Along Y, Along Z |
| Radius | `radius` | 100 | 0 | 1e20 (kept 999999999) |
| Iterations | `iterations` | 10 | 0 | 1000 |

### Inheritance
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Inheritance Mode | `inheritance_mode` | Direct | – | Direct, Animation |
| Object | `object` | empty | – | – |
| Morph Motion Object | `morph_motion_object` | off | – | – |
| Falloff Based | `falloff_based` | off | – | – |
| Transform Space | `transform_space` | Generator | – | Generator, Node |
| Animation | `animation` | To (In) | – | To (In), From (Out) |
| Start / Step Gap / End | – | time values, **not read** (see Notes) | – | – |
| Loop Animation | `loop_animation` | off | – | – |
| Position | `position` | on | – | – |
| Rotation | `rotation` | on | – | – |
| Scale | `scale` | on | – | – |

### Volume
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Volume Object | `volume_object` | empty | – | – |
| Maximum / Minimum | `maximum_percent`, `minimum_percent` | 100 % / 0 % | ±1e20 % | |
| Position | `position` | off | – | – |
| Rotation | `rotation` | off | – | – |
| Scale | `scale` | on | – | – |
| Uniform Scale | `uniform_scale` | on | – | – |
| Scale (value) | `scale_1018` | **−1** | −1e20 (kept ±999999999) | 1e20 |
| Color Mode | `color_mode` | Off | – | as above |

## Cases

All cases: 5 clones (Grid Array cases: 27), no clone hidden. Linear cloner clones undisturbed at y 0, 50, 100, 150, 200.
Positions in cm as `samples.json` holds them (rounded to 4 decimals here).

| Case | Effector | Changed from default | Notes |
|---|---|---|---|
| random-gaussian | Random | Random Mode Gaussian (second option) | Positions (−1.9902, −0.4242, 3.2007), (13.1269, 62.6804, 0.2784), (6.3014, 110.6711, −10.1449), (−7.6037, 155.3329, −7.7324), (6.5115, 188.5316, 0.1440). Scale and rotation unchanged |
| random-noise | Random | Random Mode Noise | (−15.8305, −16.2701, −16.3022), (−15.4115, 41.0546, −2.5604), (−7.5005, 97.6297, 2.3605), (−3.7251, 149.0249, 1.4394), (1.2433, 202.4048, 2.8472) |
| random-indexed | Random | **Indexed** switched on (no "Indexed" entry exists in the Random Mode list; Random Mode stays Random) | (−34.2326, −49.9704, 11.0361), (−16.36, 5.5578, −46.2975), (−30.4909, 50.4172, −35.5901), (−23.8553, 135.6844, −40.4519), (−22.5259, 190.2888, −9.464) |
| random-scale | Random | Position off, Scale on, Uniform Scale on, Scale 1 | Positions unchanged. Clone x axis length 1.1395, 0.9261, 1.6769, 1.8708, 1.3097 (all in 0..2: value ±1 around the base scale 1) |
| shader-noise | Shader | A Noise shader (type Noise, seed 665, octaves 5, space Texture, global scale 100 %, all other values default, listed in `[params.effector.shader_object]`) in its Shader slot; Position on, P (0, 100, 0) | x and z unchanged; y 35.3211, 97.379, 135.2113, 207.3293, 243.7372. Scale 1.1766, 1.2369, 1.1761, 1.2866, 1.2187. Clone colours grey 0.7715, 0.7507, 0.7719, 0.7554, 0.7539. **The shader was built by a script and inserted into the slot** (a shader has no menu command; the GUI picks it from the Shader drop-down) |
| spline-helix | Spline | Spline: a default Helix (made through its menu command; its parameters are in `[params.effector.spline_object]`) | Positions (200, 0, 0), (−161.7736, 117.5975, −39.996), (61.7075, −190.2417, −79.992), (61.9465, 190.1637, −119.988), (−161.9205, −117.3935, −159.984). Clones rotated: first clone x axis (−1, −0.0075, −0.0066), second (0.7854, −0.5895, 0.1889). Scale unchanged |
| spline-helix-segment | Spline | The same, Mode Falloff (Step is already the default, so the second option was used) | **All five clones at the same point** (199.9998, −0.2514, −199.9799), same x axis (−0.574, 0.0694, 0.8159). Segment Mode is a separate setting and stayed Use Index |
| push-apart | Push Apart | Grid Array 3×3×3, Size (30, 30, 30), Per-Step (instead of Linear); 27 clones | The clones end far from the grid: the 8 corners at (±251.6499, ±251.6499, ±251.6499), the 12 edge clones at ±277.7005 on two axes and 0 on the third, the 6 face clones at ±308.0827 on one axis, the centre at (0, 0, 0). Scale and rotation unchanged. The undisturbed grid was not read |
| push-apart-iterations | Push Apart | The same, Iterations 5 | Corners ±314.9785, edge clones ±348.4721, face clones ±387.5348, centre (0, 0, 0): **larger than with 10 iterations** |
| inheritance | Inheritance | A second, Radial cloner (Count 5, Radius 50, plane XZ, cube child 20 cm, name "Cloner.1") as Object | Both cloners' setups are in the case (`[params.effector.inherit_cloner]`). **No visible effect at frame 0:** positions y 0, 50, 100, 150, 200, scale and rotation as without effector. The mesh holds both cloners' cubes |
| volume | Volume | A cube made through its menu command, size set to 100 (the form says "scaled to 100"; read as Size 100 × 100 × 100, object scale left at 1), at (0, 0, 0), as Volume Object; Position on, P (0, 100, 0) | Clone 0 (inside the cube) moved to (0, 100, 0) and its scale became 0 (x axis length 0: the effector's Scale default is −1). Clones 1…4 unchanged (y 50, 100, 150, 200, scale 1). The mesh also holds the cube |
| plain-field-spherical | Plain, P (0, 100, 0) | A Spherical field (default size 100) at (0, 100, 0), made with the effector selected | The effector's field list holds 1 layer. y 0, 150, 200, 250, 200. Clone 0 and 4 white; clones 1…3 colour (0.0231, 0.205, 0.364) (the field's colour) |
| plain-field-linear | Plain, P (0, 100, 0) | A Linear field (length 100) in its field list. **The default direction is X+**, so Direction was set to Y+ as the form asks | 1 layer. y 50, 125, 200, 250, 300. Colours (0.596, 0.717, 0.6168), (0.394, 0.5755, 0.4252), then three clones (0.192, 0.434, 0.2336) |
| plain-absolute | Plain | Transform Mode Absolute, P (0, 100, 0) | y 100, 150, 200, 250, 300: **same as Relative** here |
| plain-remap | Plain | Transform Mode Remap, P (0, 100, 0) | y 100, 150, 200, 250, 300: same as Relative and Absolute |
| plain-effector-space | Plain | Transform Space Effector, effector rotated H 90° (written as `object_rotation_deg` in the effector table), P (0, 100, 0) | y 100, 150, 200, 250, 300, x and z 0: no difference to Node space, because P is along the rotation axis (Y). A different P direction would show the rotation; not tried |
| plain-weight | Plain | Weight Transform 50 %, Position off | Positions unchanged (y 0, 50, 100, 150, 200). **Clone weight 0.5 for all five** |

## Notes
- Each effector adds a field to the cloner named after it, as in O8 (`random_percent`, `push_apart_percent`, …).
- The Random effector's defaults differ from Plain's: Position on with P (50, 50, 50), Minimum −100 %.
- Shader and Volume: Color Mode, Use and Channel are read back with the names the file shows; the stored numbers are not in the repo.
- Inheritance: the Start, Step Gap and End values are time values; the probe reads only numbers and text, so they are `TODO` (not read). The case ran in Direct mode only.
- Not checked: Random Mode Turbulence and Sorted, Synchronized, Space UV, Seed changes; Shader with another shader type or channel; Spline modes Relative and the Rail; Push Apart modes other than Push Apart; Inheritance in Animation mode and on animated frames; Volume with other objects; the Remap mode with changed Minimum/Maximum; field layers beyond one.
