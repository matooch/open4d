# Sweep generator — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-18
Exporter settings: [export-settings.md](export-settings.md) (no deviation).
Folder: `tests/golden/sweep/<case>/`.

## Where it lives
Create > Generators > Sweep. The splines are created first; the path is dragged onto the
Sweep first and the profile second, so the Objects list reads Sweep > profile, path.
defaults / end-scale / end-rotation share one scene (each value restored after its
export); helix and rectangle-profile are new scenes. Attributes tabs: Basic, Coord.,
Object, Caps, Phong.

Inputs: profile first, path second: a Circle spline radius 20 (profile) and a Circle spline
radius 200 (path), both defaults otherwise (Plane XY, Intermediate Points Adaptive 5°).
`[params.input]` = profile, `[params.input2]` = path. No `input.obj` (splines don't
export).

**What the export contains:** the result directly, `o Sweep` (+ `o Cap_1`, `o Cap_2` when
the path is open). No Current State to Object needed.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| Isoparm Subdivision | `isoparm_subdivision` | int | 5 | not tested | not tested | – | viewport only |
| End Scale | `end_scale` | float | 100 | not tested | not tested | % | profile scale at the path end |
| End Rotation | `end_rotation` | float | 0 | not tested | not tested | ° | profile twist at the path end |
| Start Growth | `start_growth` | float | 0 | not tested | not tested | % | |
| End Growth | `end_growth` | float | 100 | not tested | not tested | % | |
| Parallel Movement | `parallel_movement` | bool | off | – | – | – | |
| Constant Cross Section | `constant_cross_section` | bool | on | – | – | – | |
| Banking | `banking` | bool | on | – | – | – | |
| Keep Segments | `keep_segments` | bool | off | – | – | – | |
| Use Rail Direction | `use_rail_direction` | bool | on | – | – | – | |
| 2-Rail | `two_rail` | bool | on | – | – | – | |
| Use Rail Scale | `use_rail_scale` | bool | on | – | – | – | |
| Flip Normals | `flip_normals` | bool | off | – | – | – | |
| Stick UVs | `stick_uvs` | bool | off | – | – | – | |
| Details (collapsed group) | – | – | – | – | – | – | scale / rotation / rail curves; not opened |
| Caps tab | `cap_start`, `cap_end`, `cap_type`, … | – | – | – | – | – | same layout as Extrude's Caps tab; only used for open paths |

## Cases

| Case | Inputs and parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | profile and path as above | 4624 / 4624 (4-gons 4624) / size (440.0466003417969, 440.0466003417969, 40), centre (0, 0, 0); normals 4624, uvs 4761 | 68 path steps × 68 profile points (both circles sample to 68 Adaptive points); a closed torus-like tube, no caps |
| helix | path = a Helix spline (defaults) | 13668 / 13596 (3-gons 132, 4-gons 13464) / size (440.0201721191406, 440.0180358886719, 239.87485694885254), centre (0.0015716552734375, 0.00189208984375, -100.00156688690186) | 201 path steps × 68; open path → `Cap_1` and `Cap_2` with 66 triangles each; the helix rises along **−Z** (centre Z −100) |
| end-scale | end scale = 50 % | 4624 / 4624 (4-gons 4624) / size (435.04078674316406, 430.0349578857422, 40), centre (2.5029067993164063, 2.5029067993164063, 0) | profile shrinks along the path |
| end-rotation | end rotation = 90 | 4624 / 4624 (4-gons 4624) / size (440.04833984375, 440.0520324707031, 40.02155685424804), centre (-0.0008697509765625, 0.0077362060546875, 0) | |
| rectangle-profile | profile = Rectangle spline (width 40, height 20) | 272 / 272 (4-gons 272) / size (440.0466003417969, 440.0466003417969, 20), centre (0, 0, 0); normals 138, uvs 345 | 68 × 4; width maps to the radial direction, height to Z |

## Behaviour notes
- Which parameters are greyed out or hidden by others: none seen on the Object tab.
- Anything surprising: a closed path gives a closed tube without caps; an open path
  (helix) gets triangulated caps. The default Helix (Subdivision 100, 720°) produces 201
  path steps.
