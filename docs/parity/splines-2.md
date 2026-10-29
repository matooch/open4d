# Spline primitives II — observation (O17)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
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

## Parameters (fill once per spline, as in r25-defaults.md)

List each spline's Object tab: label, our key, default, min and max (the ±999999999
assignment). Record Plane, Reverse and the Intermediate Points block for each one.

### Arc
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### n-Side
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Star
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Flower
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### 4-Side
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Cogwheel
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Cycloid
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Profile
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Cissoid
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Formula
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Spline | Changed from default |
|---|---|---|
| arc-defaults | Arc | – |
| arc-angles | Arc | start 30°, end 300° |
| arc-sector | Arc | type Sector (or the option that closes it to the centre) |
| nside-defaults | n-Side | – |
| nside-rounding | n-Side | Rounding on, radius 20 |
| nside-8 | n-Side | sides 8 |
| star-defaults | Star | – |
| star-twist | Star | twist 30 % (or the closest setting), points 5 |
| flower-defaults | Flower | – |
| fourside-defaults | 4-Side | – |
| fourside-types | 4-Side | the second Type option |
| cog-defaults | Cogwheel | – |
| cycloid-defaults | Cycloid | – |
| cycloid-epi | Cycloid | type Epicycloid (or the second option) |
| profile-defaults | Profile | – |
| cissoid-defaults | Cissoid | – |
| formula-defaults | Formula | – (record the three formula texts) |
| rectangle-rounding | Rectangle | Rounding on, radius 50 |
| circle-ellipse | Circle | Ellipse on, radii 200 / 100 |
| circle-ring | Circle | Ring on, inner radius 100 |
| helix-uniform | Helix | the R25 default intermediate points, Uniform (8) |
| circle-uniform | Circle | the R25 default intermediate points, Uniform (8) |

For an open spline, the side wall is a strip, not a loop. That's fine: export it as it is.
For a spline that makes several segments (Ring, a Profile type), note the segment count.

## Notes
TODO
