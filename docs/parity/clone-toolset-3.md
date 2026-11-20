# Clone toolset III — observation (O50)

Status: partial · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-20
Method: as [cloner.md](cloner.md) (O8) for clone data (`samples.json`, kind `clones`) and
meshes. Spline outputs go through an Extrude as in [splines-2.md](splines-2.md).

Folders:
- `tests/golden/cloner/<case>/` for cloner cases;
- `tests/golden/clone-spline/<case>/` for the spline generator;
- `tests/golden/tracer/<case>/` for Tracer.

Unblocks: E/067, 076, 077, 078.

## Cases

| Case | Setup | Output |
|---|---|---|
| cloner/blend-order | Linear, count 5; children a cube (size 20) and a sphere (r 10); clones mode Blend | samples + mesh |
| cloner/selection-tag | Linear, count 5, plus a clone selection tag selecting clones 1 and 3, and a Plain effector (P 0, 100, 0) restricted to that selection | samples + mesh |
| cloner/weight-tag | Linear, count 5, with a clone weight tag (weights painted or set 0, 0.25, 0.5, 0.75, 1 by script) and a Plain effector using weight | samples + mesh |
| cloner/on-spline | Object mode on a default Helix, count 10, distribution Count | samples + mesh |
| cloner/on-spline-step | the same, distribution Step 50 | samples + mesh |
| clone-spline/simple | the clone-toolset spline generator in Simple mode, defaults | extruded mesh |
| clone-spline/turtle | Turtle mode, defaults | extruded mesh |
| clone-spline/spline-mode | Spline mode with a default Circle as source | extruded mesh |
| tracer/vertices | Tracer, trace mode "vertices of object", on a cube keyed 0 → 300 in x over frames 0–30; exported at frame 30 through an Extrude | mesh |
| tracer/interval | the same, with the interval set to 5 | mesh |
| instance-time/defaults | the instance-over-time generator on a cube keyed as above, defaults, at frame 30 | samples + mesh |
| cloner-f/transform-rs | follow-up from brief 065: Linear cloner (count 3, offset (0, 0, 100)) of a 20 cm cube; Transform tab R (0, 30, 0), S (1.5, 1, 1), colour (1, 0, 0), weight 50 % | samples |
| cloner-f/linear-offset | Linear, count 5, P (0, 50, 0), Offset 1, then a second case `linear-offset-15` with Offset 1.5 | samples |
| cloner-f/linear-amount | Linear, count 5, P (0, 50, 0), Amount 50 % | samples |
| cloner-f/linear-rstep | Linear, count 5, P (0, 50, 0), per-step R (0, 20, 0) with Step Rotation H 10° | samples |
| cloner-f/linear-endpoint-sr | Linear End Point mode, count 5, P (0, 200, 0), S (0.5, 0.5, 0.5), R (0, 90, 0) | samples |
| cloner-f/radial-var-1 | Radial, count 8, Offset Variation 50 %, seed 1; and `radial-var-2` with seed 2 | samples |
| cloner-f/grid-cyl-fill | Grid Array, 5/5/5, form Cylinder, Fill 50 % | samples |
| cloner-f/grid-sphere-uneven | Grid Array, counts 7/5/3, size (300, 200, 100), form Sphere | samples |
| cloner-f/honey-orient-x | Honeycomb, orientation X (if offered), defaults otherwise; and `honey-orient-y` | samples |
| cloner-f/honey-endpoint | Honeycomb, End Point mode, defaults otherwise | samples |
| cloner-f/honey-var | Honeycomb, both variations 50 %, seed 1 | samples |
| cloner-f/honey-odd-even | Honeycomb, width count 5, height count 4 | samples |
| cloner-f/reset-coords-off | Linear, count 3, the child cube moved to (0, 50, 0) and rotated H 30°, Reset Coordinates off | samples + mesh |
| effector-f/space-node | follow-up from brief 068: Linear cloner (count 3, P (0, 0, 100)), Step Rotation H 90°; Plain effector P (50, 0, 0), space Node; then `space-object` and `space-effector` (effector rotated H 90°) | samples |
| effector-f/absolute-rs | Plain, transform mode Absolute, R (45, 0, 0), S 2 (uniform) | samples |
| effector-f/remap-minmax | Plain, mode Remap, P (0, 100, 0), Minimum 20 %, Maximum 80 %, with a Linear field | samples |
| effector-f/abs-scale | Plain, Scale 2 with Absolute Scale on; and `abs-scale-off` | samples |
| effector-f/color-alpha | Plain, Custom Color (1, 0, 0), Strength 50 %, Use Alpha/Strength on; and `-off` | samples |
| effector-f/blend-add | Plain, Custom Color (0.5, 0, 0), Blending Mode Add over a cloner colour (0, 0.5, 0); and `blend-multiply` | samples |
| effector-f/visibility | Plain, Visibility on, with a Linear field across the clones | samples |
| effector-f/order | two Plain effectors: A rotates R (90, 0, 0), B moves P (100, 0, 0); in order A, B and then `order-swapped` | samples |
| effector-f/per-effector-50 | Plain P (0, 100, 0), the cloner's strength for that effector set to 50 % | samples |
| effector-f/linear-plain-field | re-record O8 `linear-plain-field` with the field's full parameter table and Color in `case.toml` | samples |
| random-f/count50-s12345 | follow-up from brief 069: Linear cloner count 50, P (0, 0, 20); Random effector defaults (seed 12345); then `-s0` (seed 0), `-s1` (seed 1) and `-strength50` (Strength 50 %) | samples |
| random-f/indexed-50 | the same, Indexed on; then `indexed-5` and `indexed-8` with counts 5 and 8 | samples |
| random-f/gaussian-50 | the same, mode Gaussian | samples |
| random-f/noise-steps | Linear count 20, P (0, 0, 10); Random mode Noise; then `noise-scale50`, `noise-scale200`, `noise-speed0-f30` (Animation Speed 0 %, frame 30), `noise-uv` (Space UV) | samples |
| random-f/turbulence | Linear count 20, Random mode Turbulence | samples |
| random-f/synchronized | Linear count 20, Random, Synchronized on | samples |
| random-f/sorted | Linear count 20, Random mode Sorted (if offered) | samples |
| random-f/rot-h | Linear count 20, Random with Position off and Rotation on, R (90, 0, 0) | samples |

## Case results

| Case | State |
|---|---|
| `cloner/blend-order` | 358 points / 396 polygons; samples kind `clones`, 5 clones; clone positions (0, 0, 0) (0, 50, 0) (0, 100, 0) (0, 150, 0) (0, 200, 0) |
| `cloner/selection-tag` | not observed: the scripted clone selection never reaches the effector. `c4d.modules.mograph.GeSetMoDataSelection` on the MoGraph Selection tag (1021338) reads back correctly (clones 2 and 4 selected, and `GeGetMoDataSelection` on the cloner reports the same array), and the Plain effector's Selection field (param 1021) takes the tag as a link (`GetParameter` returns `MoGraph Selection`), yet the effector still offsets all five clones by (0, 100, 0) (3 attempts, see Notes) |
| `cloner/weight-tag` | not observed: `c4d.modules.mograph.GeSetMoDataWeights` on the MoGraph Weightmap tag (440000231) reads back 0, 0.25, 0.5, 0.75, 1, but the Plain effector with Weight Transform on offsets every clone by the full (0, 100, 0) instead of scaling by the weight, so the scripted weights do not reach the stored tag data (see Notes) |

## Notes
- Blend order: 5 clones of a cube (size 20) and a sphere (r 10) in Linear mode at the default (0, 50, 0) step bake to a 358-point / 396-polygon mesh; clone weights read 0 because no weight tag is present, as in the O8 cloner cases.
- Clone selection and clone weights are not scriptable through the Python API in this build. `GeSetMoDataSelection` and `GeSetMoDataWeights` update only the live MoGraph data: the corresponding getters read the values back, the cloner's own MoData reports the same selection array, and the Plain effector accepts the selection tag in its Selection field (1021), but the effector evaluation still treats every clone as selected and weight 1, so the clones are all offset. Neither tag exposes a writable per-clone array through its description (the Selection tag offers only Clear, 1000, and the Fields list, 1005; the Weightmap tag is the same shape), and painting the tag or filling its Fields list is a GUI action. Both cases are recorded as not observed rather than filing an unrestricted effector as the restricted case.
- Only the two cloner cases of this form were attempted in this run; the clone-spline, tracer, instance-time, cloner-f, effector-f and random-f rows remain open.

