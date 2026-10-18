# Spline primitives (export probe) — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Exporter settings: [export-settings.md](export-settings.md) (note any deviation here).
Folder: `tests/golden/splines/<case>/`.

## Where it lives
Menu path / how it is created: TODO (new scene each case: File > New).

Goal: find out how splines export, so spline inputs above can be rebuilt. Export each spline alone as OBJ. If the file holds lines (`l` records) or points, record them; if it holds nothing, note it and try making the spline editable first.

## Parameters (as shown in the app)

Fill every parameter of the generator's main tab (label, default, min/max found by
typing a huge and a negative value, unit, enum options). Use lowercase snake_case keys
(suggestions below); use the same keys in `case.toml` `[params]`. Record each
**input** object's type, parameters and transform in `case.toml` too, as sub-tables
`[params.input]`, `[params.input2]`, … in child order (e.g. `type = "cube"`,
`pos = [150.0, 0.0, 0.0]`, `size_x = 50.0`; spline params with their own keys), so the
input can be rebuilt exactly. For mesh inputs also export them as `input.obj`,
`input2.obj`, … in the case folder.

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| TODO | TODO | | | | | | |

Enum options (label → our key): TODO

## Cases

Export the **generator** (with its children) selected, so the file holds the generated
result. If the export instead writes the children separately or unchanged, note it and
try making the generator editable (Current State to Object) first, then export that.

| Case | Inputs and parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| circle | Circle spline, defaults | | |
| circle-radius | Circle spline, radius 50 | | |
| rectangle | Rectangle spline, defaults | | |
| rectangle-size | Rectangle spline, width 100, height 200 | | |
| helix | Helix spline, defaults | | |
| intermediate | Circle spline, intermediate points = Uniform, number = 4 (record the default interpolation first) | | |

## Behaviour notes
- Which parameters are greyed out or hidden by others: TODO
- Anything surprising (welding, caps, seams, order of copies): TODO
