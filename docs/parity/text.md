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
