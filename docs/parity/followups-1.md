# Follow-ups to O5–O9 — observation (O11)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-29
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Each section adds cases next
to the job it follows up, in that job's folder and with that job's `case.toml` rules.
Don't edit or delete the existing cases. Add the new rows to **this** file, not to the
original form. All 22 cases were exported; every `case.toml` points here.

## 1. Fields (follows O7, `tests/golden/fields/`, `observation = "docs/parity/followups-1.md"`)

The O7 cases `spherical-inner`, `box-inner` and `spherical-interp-linear` repeated their
defaults. Replace their purpose with:

Each case: a field made through its menu command, the one change below, sampled as in
[fields.md](fields.md) (729 points, value channel). Counts are over the 729 samples.

| Case | Field | Changed from default | Read back | Samples: exactly 1 / between 0 and 1 / exactly 0 |
|---|---|---|---|---|
| spherical-inner-0 | Spherical | Inner Offset 0 % | 0 % | 1 / 6 / 722 |
| spherical-inner-80 | Spherical | Inner Offset 80 % | 80 % | 7 / 0 / 722 |
| box-inner-0 | Box | Inner Offset 0 % | 0 % | 1 / 26 / 702 |
| linear-inner-50 | Linear | Inner Offset 50 % | 50 % | 405 / 81 / 243 |
| spherical-steps-4 | Spherical | Contour Mode Step, Steps 4 | Step, 4 | 1 / 6 / 722 |
| spherical-quantize-4 | Spherical | Contour Mode Quantize, Steps 4 | Quantize, 4 | 1 / 6 / 722 |
| spherical-strength-50 | Spherical | Remapping Strength 50 % | 50 % | 0 / 7 / 722 (largest value 0.5) |
| spherical-scaled | Spherical | object scale (2, 1, 1) | (2, 1, 1) | 3 / 14 / 712 |

Notes:
- Contour Mode options in the order the app lists them: None, Quadratic, Step, Quantize, Curve.
- `spherical-steps-4` and `spherical-quantize-4` are byte-identical to each other. Both differ from `spherical-defaults`.
- `spherical-inner-80` has no samples strictly between 0 and 1: all 729 samples are 0 or 1.
- `spherical-scaled`: `case.toml` carries a `scale` key (not present in the older field cases) next to `position` and `rotation_deg`.

## 2. Cloner (follows O8, `tests/golden/cloner/`)

Each case: a Cloner made through its menu command with a default 20 cm cube child, as in [cloner.md](cloner.md). The Random effector was made through its menu command.

| Case | Mode | Changed | Effector | Read back / mesh |
|---|---|---|---|---|
| grid-random-seed-7 | Grid Array | – | Random, Seed 7 (other values default) | Seed 7. 9 clones, 72 vertices, 54 faces, bbox (−259.974, −40.171, −245.624)–(253.716, 54.621, 228.591) |
| grid-random-rot | Grid Array | – | Random, Position off, Rotation on (45°, 45°, 45°) | R (45°, 45°, 45°). 9 clones, 72 vertices, 54 faces, bbox (−216.684, −17.148, −216.611)–(215.405, 17.148, 214.877) |
| radial-xy | Radial | Plane XY (the first option) | – | Plane XY. 5 clones, 40 vertices, 30 faces, bbox (−60.154, −54.419, −10)–(60.154, 60, 10) |
| linear-mode-endpoint | Linear | Count 5, Mode End Point (the first option; Per Step is the default), P (0, 200, 0) | – | Count 5, End Point, P y 200. 5 clones, 40 vertices, 30 faces, bbox (−10, −10, −10)–(10, 210, 10) |

Notes:
- Cloner Linear Mode options: End Point, Per Step (default). In End Point mode the P vector (default (0, 50, 0)) is the position of the last clone; the clone Offset is 0.
- Cloner Radial Plane options: XY, ZY, XZ (default).
- The Random effector's rotation vector appears only after Rotation is switched on.
- The new `case.toml` files follow the existing cloner files, including their repeated `position` and `scale` keys (cloner-level and effector-level).

## 3. Deformers II (follows O9)

Each case: the same 200 × 400 × 200 cube with 4/8/4 segments as in [deformers-2.md](deformers-2.md); all 162 vertices and 160 faces.

| Case (folder) | Deformer | Changed | Read back / bbox |
|---|---|---|---|
| wrap/spherical | Wrap | Wrap Spherical (the first option) | bbox (−212.132, −212.132, −100)–(212.132, 212.132, 150); differs from wrap/defaults |
| wrap/width-90 | Wrap | span of 90°: Longitude End 270° (Start stays 180°) | End 270°. bbox (−277.164, −200, −77.164)–(−38.268, 200, 161.732); differs from wrap/defaults |
| squash-stretch/expand-0 | Squash & Stretch | Factor 50 %, Expand 0 % | bbox (−100, −149.9975, −100)–(100, 149.9975, 100) |
| squash-stretch/expand-200 | Squash & Stretch | Factor 50 %, Expand 200 % | bbox (−300, −149.9975, −300)–(300, 149.9975, 300) |

Notes:
- The Wrap has no width angle field. The Longitude span is End − Start, so "width angle 90°" was set as End 270°. That is an interpretation of the row, not a field of that name.
- Expand 0 % leaves the x and z extent at ±100, Expand 200 % widens it to ±300. The y extent is ±149.9975 in both.

## 4. Modeling tools (follows O6, `tests/golden/modeling-tools/`)

| Case | Cube | Selection | Tool + settings | Result |
|---|---|---|---|---|
| optimize-real | 1/1/1 editable (8 points, 6 polygons) | top polygon (4), then the Disconnect command | Optimize on everything with Tolerance 0.01, Optimize Points, Optimize Polygons and Delete Unused Points on | Disconnect: 12 points, 6 polygons. Optimize: 8 points, 6 polygons; mesh equals the plain cube. bbox ±100 |
| extrude-bevel | 1/1/1 | top polygon: polygon 4 | Extrude, offset 50, Bevel 10 | 12 vertices, 10 faces, bbox (−100, −100, −100)–(100, 150, 100): same size as extrude-top |
| extrude-subdiv | 1/1/1 | polygon 4 | Extrude, offset 50, Subdivision 2 | 20 vertices, 18 faces, bbox (−100, −100, −100)–(100, 150, 100) |
| inset-subdiv | 1/1/1 | polygon 4 | Extrude Inner, offset 20, Subdivision 1 | 16 vertices, 14 faces, bbox ±100 |
| bevel-poly-extrusion0 | 1/1/1 | polygon 4 | Bevel (polygon mode), offset 20, Extrusion 0 | 12 vertices, 10 faces, bbox ±100: the top stays at y = 100 (with the default Extrusion 10 it moves to 110, see bevel-poly) |

Notes:
- optimize-real: the Optimize command run with an **empty** settings container (as in O6) merged nothing: the 12 points after Disconnect stayed 12. With the Tolerance and the three option switches set by the probe it merged back to 8 (tolerance 0.0 gave the same). The defaults of those four Optimize settings are still **TODO: not read**; the values in `case.toml` are the ones the probe set, not defaults. This also explains why `modeling-tools/optimize` changed nothing.
- optimize-real `case.toml` has `disconnect_first = true` and a `[params.settings]` table with `tolerance`, `optimize_points`, `optimize_polygons`, `delete_unused_points`.
- extrude-bevel: Bevel 10 changed no vertex count or bbox against extrude-top (Subdivision 0); whether Bevel only acts together with Subdivision was not tested.
- bevel-poly-extrusion0: the hidden field behind the +10 lift is the Extrusion field (default 10), confirmed: set to 0 the top stays at 100. The hidden second Depth (10) and two other hidden values at 10 had no effect on the top height. `case.toml` carries `extrusion = 0.0` under `[params.settings]`.

## 5. Sweep cap type (follows O5)

Sweep (menu command) with a default Circle as the profile (first child) and a default Rectangle as the path (second child); exported to `tests/golden/rechecks/sweep-defaults/mesh.obj`.

Result: 144 vertices, 144 faces, **all 4-gons**, bbox (−400, −400, −200)–(400, 400, 200), normals 130, uvs 185. **There are no cap faces**: no n-gon or triangle face is in the file, although Start Cap and End Cap read on and Caps Type reads N-gon. So this case does not confirm the cap type from a mesh. Why no cap appears (for example a closed path) was not tested: TODO.

## Notes
- 22 cases: fields 8, cloner 4, deformers 4, modeling tools 5, rechecks 1.
- Not done / TODO: the defaults of the Optimize settings (see section 4); the cap type of Sweep from a mesh (section 5).
- Probe scripts are in the observer workspace, not in the repo.
