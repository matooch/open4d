# Text spline — observation (O48)

Status: observed (9 of 9 cases; font family not scriptable on R25, see Notes) · App version: Cinema 4D R25 (version number 25010) · OS: macOS (Darwin 25.6.0) · Observer: observer session (MCP) · Date: 2026-11-04
Method: as [splines-2.md](splines-2.md), followed as planned: a Text spline under an Extrude
with Offset 20 and caps off, exported to `tests/golden/text/<case>/mesh.obj`. Scripted
([agents/OBSERVER.md](../agents/OBSERVER.md)). Level B, font-dependent — see Notes for how
the font question resolved.

Unblocks: C/039, E/075. Level B (font-dependent).

Record the Text spline's parameter table once.

## Parameters (all defaults as they read back on R25)

The Text spline lives in the Create > Spline menu; it is built by that menu command. The
font parameter holds a font-picker data type; its value reads back as a font data object
wrapping a small container (display name, family, style, PostScript name strings).

| Label | Our key | Default | Min | Max |
|---|---|---|---|---|
| Text | `text` | `Text` | – | text |
| Font (family/style/file) | `font` | `default` (empty font container) | – | font picker |
| Align | `align` | Left | – | options: Left, Middle, Right |
| Height | `height` | 200 | -1e+20 | 1e+20 |
| Horizontal Spacing | `horizontal_spacing` | 0 | -1e+20 | 1e+20 |
| Vertical Spacing | `vertical_spacing` | 0 | -1e+20 | 1e+20 |
| Separate Letters | `separate_letters` | off | – | – |
| Plane | `plane` | XY | – | options: XY, ZY, XZ |
| Reverse | `reverse` | off | – | – |
| Intermediate Points | `intermediate` | Adaptive | – | options: None, Natural, Uniform, Adaptive, Subdivided |
| Number | `intermediate_number` | 8 | 0 | 5000 |
| Angle | `intermediate_angle` | 5 | 0 | 90 |
| Maximum Length | `intermediate_max_length` | 5 | 0.01 | 1e+20 |

Default Intermediate Points is **Adaptive** (Angle 5°), like most spline primitives.

## Cases

| Case | Text | Changes | Mesh |
|---|---|---|---|
| defaults | the default text (`Text`) | – (default font; see Notes) | ok |
| a | `A` | font: script-set fonts have no effect on this build — measured with the default font | ok |
| o | `O` | the same (two contours) | ok |
| word | `Forge` | the same | ok |
| height | `A` | height 50 | ok |
| align-center | `AB` | alignment Middle | ok |
| spacing | `AB` | horizontal spacing 20 | ok |
| two-lines | `A` + newline + `B` | vertical spacing default; a plain newline in the text is stored and produces a second line | ok |
| intermediate | `S` | Intermediate Points Uniform, number 4 | ok |

## Follow-up cases (planner, after brief 039)

Brief 039 matched every font-independent rule above. Glyph shapes need a font both sides can
use: in GUI session G1 or G4, pick **DejaVu Sans** (freely licensed; installable from the
DejaVu project) in the font picker, confirm it reshapes the text, then export these like the
cases above. If the picker cannot reshape the text either, say so in the Notes and skip the
`dv-` rows.

| Case | Text | Changes | Mesh |
|---|---|---|---|
| dv-a | A | Font DejaVu Sans | extruded as above |
| dv-o | O | Font DejaVu Sans | extruded as above |
| dv-word | Forge | Font DejaVu Sans | extruded as above |
| dv-two-lines | A (newline) A | Font DejaVu Sans | extruded as above |
| align-right | AB | Align Right | extruded as above |
| vspacing | A (newline) A | Vertical Spacing 50 | extruded as above |
| kerning | AV | – (default kerning) | extruded as above |
| height-negative | A | Height −200 | extruded as above |
| separate-letters | AB | Separate Letters on (record the object tree) | extruded as above |

## Notes

- **Fonts are not scriptable on this build (R25.010, macOS).** The font parameter reads
  and writes a font data object: we can store a family/style/display/PostScript name, and
  it reads back unchanged. But the generated outline never changes for **any** font we
  tried — Helvetica, Helvetica Neue, Arial (+`ArialMT` PostScript), Times, Bodoni 72,
  Menlo, Courier New (+`CourierNewPSMT`), bold, and unknown names all produced identical
  contour points and bounding boxes for the same string, even with forced re-layouts
  (changing the text string between font writes so the glyph tables regenerate). The
  script-set name is stored in the object but the shaping engine ignores it and keeps the
  app's default font. The default font container is empty, and no public API returns the
  resolved default font's family — that is recorded as `font = "default"` in `case.toml`
  (family/style/file: TODO, needs the GUI font picker, and a GUI session should confirm
  whether a GUI-picked font reshapes the spline).
- Because of this, every case above was measured with the **default font**: the meshes pin
  the default-font glyphs. An implementer should replay with the default font too. When a
  font picker is available to the app's GUI, re-observe cases `a`, `o`, `word` with a
  freely licensed font before extending them.
- Geometry sanity readbacks (all against the expectations of the form): height 50 scales
  the glyph to 1/4 (bbox y 0–35.86 at height 50 vs 0–143.46 at 200); Align Middle centers
  the two letters (bbox x -127.93..127.93); horizontal spacing 20 widens `AB` from 128 to
  275.8; the two-line text extends below the baseline to y -245.996; `S` with Uniform
  intermediate points number 4 yields 28 spline points in 1 segment, vs 1 segment of
  Adaptive points for the letters in the other cases.
- The side walls of the export hold every spline point twice: once at z 0 and once at
  z -20 (the Extrude grows toward -Z here, unlike the splines-2 convention where the
  offset appeared at +20). The export settings pipeline and case flags
  (`flip_z = false`) are the same as splines-2 ([export-settings.md](export-settings.md)).

- Batch-03 `dv-a`: not observed: needs the GUI: planner explicitly requires DejaVu Sans chosen and visually confirmed in font picker; unattended font selection prohibited..

- Batch-03 `dv-o`: not observed: needs the GUI: planner explicitly requires DejaVu Sans chosen and visually confirmed in font picker; unattended font selection prohibited..

- Batch-03 `dv-word`: not observed: needs the GUI: planner explicitly requires DejaVu Sans chosen and visually confirmed in font picker; unattended font selection prohibited..

- Batch-03 `dv-two-lines`: not observed: needs the GUI: planner explicitly requires DejaVu Sans chosen and visually confirmed in font picker; unattended font selection prohibited..

- Batch-03 `align-right`: Observed; vertices: 300 (unused 0); faces: 150 (4-gons 150); bbox min: (-255.859375, 0, -20); bbox max: (0, 143.457031, 0); size: (255.859375, 143.457031, 20); centre: (-127.9296875, 71.7285155, -10); normals: 148; uvs: 294; complete measured parameters in case.toml.

- Batch-03 `vspacing`: Observed; vertices: 44 (unused 0); faces: 22 (4-gons 22); bbox min: (2.929688, -295.996094, -20); bbox max: (130.957031, 143.457031, 0); size: (128.027343, 439.453125, 20); centre: (66.9433595, -76.26953150000001, -10); normals: 12; uvs: 22; complete measured parameters in case.toml.

- Batch-03 `kerning`: Observed; vertices: 36 (unused 0); faces: 18 (4-gons 18); bbox min: (2.929688, 0, -20); bbox max: (263.671875, 143.457031, 0); size: (260.742187, 143.457031, 20); centre: (133.3007815, 71.7285155, -10); normals: 15; uvs: 34; complete measured parameters in case.toml.

- Batch-03 `height-negative`: Observed; vertices: 22 (unused 0); faces: 11 (4-gons 11); bbox min: (-130.957031, -143.457031, -20); bbox max: (-2.929688, 0, 0); size: (128.027343, 143.457031, 20); centre: (-66.9433595, -71.7285155, -10); normals: 11; uvs: 22; complete measured parameters in case.toml.

- Batch-03 `separate-letters`: Observed; vertices: 300 (unused 0); faces: 150 (4-gons 150); bbox min: (2.929688, 0, -20); bbox max: (258.789062, 143.457031, 0); size: (255.859374, 143.457031, 20); centre: (130.859375, 71.7285155, -10); normals: 148; uvs: 294; complete measured parameters in case.toml.

## Batch-03 checkpoint (2026-11-18)

Hermes; public scripted bridge, version 25010. New mesh cases: 5; form-only observed rows: 0; not observed rows: 4. Full golden check blocked by unrelated restrict-bend duplicate params table; no commit or push. Queue remains partial, collection and publication tracked separately. Original observations and fixtures preserved.

### Batch-03 Separate Letters readback

Source hierarchy and evaluated Text cache hierarchy, with Separate Letters on:

```json
{
  "object_tree": [
    {
      "name": "Extrude",
      "type": "Extrude",
      "position": [
        0.0,
        0.0,
        0.0
      ],
      "children": [
        {
          "name": "Text Spline",
          "type": "Text Spline",
          "position": [
            0.0,
            0.0,
            0.0
          ],
          "children": []
        }
      ]
    }
  ],
  "evaluated_text_tree": [
    {
      "name": "Text Spline",
      "type": "",
      "position": [
        0.0,
        0.0,
        0.0
      ],
      "children": [],
      "point_count": 150
    }
  ]
}
```

The source remains a Text spline child of Extrude; its evaluated text cache was one point object, 150 points, no children. No per-letter child objects were observed. All five new scriptable cases leave the font at the menu default; resolved family/style/file remain not observed. Height −200 reads back unchanged and mirrors both glyph axes in its mesh. The four `dv-*` rows remain genuine GUI-only skips, not a claim that the picker was tried.

Two initial Align Right preparations failed before export: the visible string label is Text Spline, not Text, and the font picker requires its typed parameter access rather than a scalar-key access. Both were discarded; the third fresh preparation exported successfully without changing the font.
