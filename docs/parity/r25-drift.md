# R25 drift check — observation (O1)

Status: observed · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-10-28
Bridge: MCP plugin version 0.2.0 ("cinema4d-mcp (sdimaging fork)", scripting runtime Python 3.9) · Unblocks: trust in the R20 golden data for the R25
target; the R25 exporter settings.

Every golden case before batch 08 came from R20 on Windows. This form re-exports a sample
of them in R25 (scripted, [agents/OBSERVER.md](../agents/OBSERVER.md)) and records whether
each file is identical. Nothing in `tests/golden/<feature>/` is changed by this form.

## 0. Smoke test (O0, OBSERVER.md §2)

| Step | Result (ok / error text) |
|---|---|
| `ping` | ok (plugin 0.2.0) |
| `get_capabilities`: app version string, plugin version | ok: app 25010 (R25.010), plugin 0.2.0 ("cinema4d-mcp (sdimaging fork)"), Python 3.9.1+, platform darwin, safe mode off, **auth token now required** (`auth_token_required: true`, host 127.0.0.1, port 5555); renderer Standard; 108 commands, 46 marked safe |
| `execute_python_script` returns version and `1 + 1` | ok: `{'version': 25010, 'sum': 2}` |
| probe: default cube → OBJ export | ok, but **not identical** to the R20 file with the §1 settings: no normals at all (see below) |
| probe: key position X at 0 and 30, read the value at 15 | ok: keys 0 and 300 gave 150.00043 at frame 15 (default key interpolation is not exactly linear; set linear on both keys if O2 needs exact values) |
| licence / EULA | edition not readable from the scripting API (no licence-type call in R25's Python); EULA wording **not checked** by the observer (OBSERVER.md §2 / parity/README.md §0 ask the owner to confirm scripted observation and export are allowed). Owner to fill in: edition ______, EULA allows scripted observation: yes / no |

Cube export vs `tests/golden/cube/defaults/mesh.obj` (200 cm cube, Scale 1 cm, Flip Z off, Material None, UVs Original, Normals Vertices):

- A cube made by the Create menu command exports with the same vertices, the same faces in the same order, 4 UVs and **6 normals**, like R20: `golden info` is identical. `cmp` still differs on text only: the header says "Cinema 4D" (R20: "CINEMA 4D") and numbers are written with six decimals (`-100.000000`, R20: `-100`). Parse the numbers, don't compare bytes.
- Gotcha for the probe: an object made by the scripting API directly (not the menu command) starts with different values than the menu one, and a bare cube then exports **no normals** (`f v/vt`). Build every case with the menu command (the probes do), then set values.
- Generator results exported with no normals: Subdivision Surface over a cube (98 vertices, 96 faces, same positions as R20, but 0 normals; R20 had 98).

## 1. R25 OBJ exporter options

List every option of the R25 OBJ export dialog (or what the probe can set), as in
[export-settings.md](export-settings.md). Mark options that are new since R20 and
defaults that changed. Use the R20 values for every option R20 had; for new options
write what you chose and why.

| Option label | Value used | Default | New since R20? | Set from probe how (plain words) |
|---|---|---|---|---|
| Presets | Default (reset by the probe to the settings below) | Default | changed list: Default, 3ds Max, Adobe Dimension, Blender, Maya, Red Giant Trapcode, Zbrush, Custom | not touched; the probe sets each option directly on the exporter's settings |
| Scale + unit | 1, Centimeters | 1, Centimeters | same | set the scale number and unit on the exporter's settings |
| UVs | Original | Original | **yes, now a list**: None, Original, Flip U, Flip V, Flip UV (R20: checkbox) | set the option to "Original" |
| Normals | Vertices | Vertices | list now None / Vertices (R20: Skip / Vertices) | set to "Vertices" (see §0: writes nothing without a smoothing tag) |
| Vertex Colors | off | off | **yes** | left at default |
| Flip X / Flip Y | off | off | no | left at default |
| Flip Z | off | **on** | no | switched off |
| Swap X-Y / X-Z / Y-Z | off | off | no | left at default |
| Flip Faces | off | off | no | left at default |
| Resolve N-gons | off | off | **yes** | left at default (quads stay quads in the cube) |
| Objects as Groups (R20: Export Objects as Groups) | off | off | renamed | left at default |
| Material | None | **Name and Default** | list now None / Name and Default / Original Paths / Local Paths (R20: No Material / Export Name and Default / Export Materials) | set to "None" |
| Default Material | White | White | same | left at default |
| Invert Transparency | off | off | no | left at default |
| Animation: Frame Range, Start Frame, End Frame, Frame Step | No Animation, 0, 90, 1 | same | **yes** (animation export) | left at default; Frame Range list: No Animation, Manual, Preview, All Frames, All Animated Frames |
| Shared Material File | off | off | **yes** | left at default |
| Export as UTF-8 | off | off | **yes** | left at default |
| Export Duplicate Vertices | not found | – | R20 option is gone | – |

## 2. Cases

"Identical" here means the same vertex positions and faces after parsing the numbers (±0.01); byte-for-byte `cmp` fails for every file because of the header and number format (§0). Kept files were exported with R25's own defaults for every parameter the case does not list, since that is what a user sees.

For each case: rebuild it from its `case.toml` in R25, export with §1's settings to
`tests/golden/r25-drift/<feature>-<case>/mesh.obj`, then compare with the R20 file:
`cmp old new` (identical?) and `golden info` on both. **If identical, delete the R25
export** (don't commit duplicates). If different, keep it, write a `case.toml` copied
from the original with `feature = "r25-drift"`, `observation = "docs/parity/r25-drift.md"`,
and describe the difference (counts, bbox, a few differing lines).

| R20 case | Identical? | If not: counts old → new, bbox old → new, what differs |
|---|---|---|
| calibration/offset-cube | yes (geometry) |  |
| cube/defaults | yes (geometry) |  |
| cube/fillet | yes (geometry) | normals 104 (R20: 105) |
| sphere/defaults | **no, R25 default differs** | vertices/faces 266/288 → 114/128; bbox same ±100. Cause: default Segments 16 (R20: 24). With Segments 24 typed: identical geometry (normals 266 vs 267). Kept: `r25-drift/sphere-defaults` |
| sphere/type-icosahedron | **no, R25 default differs** | 162/320 → 92/180; bbox ±100 → ±97.94. Same cause (Segments 16); with 24 typed: identical. Kept: `r25-drift/sphere-type-icosahedron` |
| cylinder/fillet | **no, R25 default differs** | 434/468 → 178/192; bbox same. R25 defaults: Height Segments 4 (R20 1), Rotation Segments 16 (R20 36), Fillet Segments 3 (R20 5), Fillet Radius 3 (R20 20). With R20 values typed: identical, new and legacy ('use new version' off) alike. Kept: `r25-drift/cylinder-fillet` |
| torus/slice | **no, R25 default differs** | 666/680 → 528/540; bbox ±250 → ±150. R25 defaults: Ring Radius 100 (R20 200), Ring Segments 32 (36), Pipe Segments 16 (18). With R20 values typed: same vertices (as a set), same counts; only vertex order and normals (689 vs 687) differ. Kept: `r25-drift/torus-slice` |
| subdivision/cube-2 | yes (geometry) | normals 0 (R20: 98), see §0 |
| boole/mode-union | **no** | R20 20 vertices/18 faces → R25 26/12 (six 6-gons) with R25 defaults, 26/18 with Hide new edges off. Default Boolean Type is union in R25 (R20: A subtract B); Hide new edges default on (R20 off). The 26 points hold the same 20 positions (six duplicates). bbox same. Kept: `r25-drift/boole-mode-union`, `r25-drift/boole-mode-union-r20values` |
| extrude/rectangle-caps-quadrangles | **no, R25 default differs** | bbox depth 20 → 100: default Offset is 100 (R20 Movement Z 20; the field is now named Offset, with a Direction option, default Auto). With Offset 20: same vertices and faces, different vertex order. Kept: `r25-drift/extrude-rectangle-caps-quadrangles` |
| bend/strength | yes (geometry) | max vertex difference 7e-6 (digit rounding); normals 21 both |
| twist/angle | yes (geometry) | normals 146 both |
| transform/ (any one case with all three rotations) | yes (geometry) | rot-combined (30, 45, 60) |

## 3. Notes
- **The primitives' defaults changed in R25**, not their geometry (once the R20 values are typed in, sphere, cylinder, torus and extrude match R20). R25 defaults seen: sphere Segments 16; cylinder Height Segments 4, Rotation Segments 16, Fillet Segments 3, Fillet Radius 3; torus Ring Radius 100, Ring Segments 32, Pipe Segments 16; extrude Offset 100; Boolean Type union, Hide new edges on. Every R20 default recorded in `docs/parity/*.md` for these objects needs a re-check before the R25 target uses it.
- The cylinder and torus have a new "use new version" switch (on by default in R25); the geometry was the same either way for these cases.
- Exporter: new options and the changed lists are in §1. Normals: 1 fewer than R20 in several files (sphere 266 vs 267, cube fillet 104 vs 105); R25 writes none for a Subdivision Surface result.
- Keys: the default key interpolation is not linear (150.0004 at frame 15 between 0 and 300), see §0.
- Boole: the same positions appear 26 times instead of 20 (duplicate points); compare it geometrically, not by index.
- Not checked: the R25 EULA wording (the playbook asks for it before the first run), Windows vs macOS differences, any case other than the 13 above.
