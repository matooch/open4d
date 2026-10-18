# Tube primitive — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Exporter settings: [export-settings.md](export-settings.md) (note any deviation here).
Folder: `tests/golden/tube/<case>/`.

## Where it lives
Menu path / how it is created: TODO (new scene each case: File > New).

## Parameters (as shown in the app)

Fill every parameter of the object's main tab (label, default, min/max found by typing a
huge and a negative value, unit). Use lowercase snake_case keys of your choice; the
keys used in the cases below are suggestions; rename them if the app's label suggests
a better name, and use the same keys in `case.toml` `[params]`.

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| TODO | TODO | | | | | | |

Enum options (label → our key): TODO

## Cases

Change only what the row lists. Values are chosen to be **unequal and odd** where that
helps tell index orders apart; if the app refuses a value, pick the nearest allowed one
and note it. If a listed parameter doesn't exist, skip the row and say so; if the
object has an important parameter not listed (e.g. a slice/partial option), add a case
for it.

| Case | Parameters changed from default | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | | |
| radii | inner_radius = 30, outer_radius = 120, height = 300 | | |
| segments | rotation_segments = 7, cap_segments = 2, height_segments = 3 | | |
| fillet | fillet on (defaults) | | |
| orientation-pos_x | orientation = +X | | |
| slice | slice on, from 0° to 270° | | |

## Behaviour notes
- Which parameters are greyed out or hidden by others: TODO
- Behaviour at the minimum segment counts: TODO
- Anything surprising (welded seams, n-gon caps vs triangle fans, poles): TODO
