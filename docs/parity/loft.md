# Loft generator — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Exporter settings: [export-settings.md](export-settings.md) (note any deviation here).
Folder: `tests/golden/loft/<case>/`.

## Where it lives
Menu path / how it is created: TODO (new scene each case: File > New).

Inputs: two Circle splines as children: the first radius 200 at (0, 0, 0), the second radius 100 at (0, 200, 0), both in the XZ plane (Plane = XZ in the spline's settings).

## Parameters (as shown in the app)

Fill every parameter of the generator's main tab (label, default, min/max found by
typing a huge and a negative value, unit, enum options). Use lowercase snake_case keys
(suggestions below); use the same keys in `case.toml` `[params]`. Record each
**input** object's type, parameters and transform in `case.toml` too, under
`[input]` / `[input2]` (e.g. `type = "cube"`, `pos = [150.0, 0.0, 0.0]`, plus any
changed params), so the input can be rebuilt exactly.

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
| defaults | circles as above | | |
| mesh-subdivision | mesh subdivision U = 7, V = 3 (or the app's equivalents) | | |
| three | a third circle radius 150 at (0, 400, 0) | | |
| caps-off | start and end caps off | | |

## Behaviour notes
- Which parameters are greyed out or hidden by others: TODO
- Anything surprising (welding, caps, seams, order of copies): TODO
