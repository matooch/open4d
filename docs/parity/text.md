# Text spline — observation (O48)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: as [splines-2.md](splines-2.md): a Text spline under an Extrude with Offset 20 and
caps off, exported to `tests/golden/text/<case>/mesh.obj`.

**Fonts:** use a font that ships with the OS or is freely licensed. Record the exact font
family, style and file name in `case.toml` (`font = "…"`). Do not commit font files.

Unblocks: C/039, E/075. Level B (font-dependent).

Record the Text spline's parameter table once.

## Cases

| Case | Text | Changes |
|---|---|---|
| defaults | the default text | – (record the default font) |
| a | `A` | font: a plain sans available on macOS (e.g. Helvetica) |
| o | `O` | the same font (two contours) |
| word | `Forge` | the same font |
| height | `A` | height 50 |
| align-center | `AB` | alignment Center |
| spacing | `AB` | horizontal spacing 20 |
| two-lines | `A` + newline + `B` | vertical spacing default |
| intermediate | `S` | Intermediate Points Uniform, number 4 |

## Notes
TODO
