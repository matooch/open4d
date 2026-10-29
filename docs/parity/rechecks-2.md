# Re-checks II — observation (O15)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Folder
`tests/golden/rechecks/<case>/`, `feature = "rechecks"`,
`observation = "docs/parity/rechecks-2.md"`.

## 1. Sweep caps on an open path

O11's sweep had no caps because its Rectangle path is closed. Use an open path instead:

| Case | Scene (menu commands) | Cap faces seen (count, corners) | Caps Type read |
|---|---|---|---|
| sweep-open | Sweep, child 1: default Circle (profile); child 2: a Helix with default values (path) | | |
| sweep-open-triangles | The same, with Caps Type set to the triangle option | | |

## 2. Extrude bevel needs subdivision?

| Case | Scene | Changed |
|---|---|---|
| extrude-bevel-subdiv | 1/1/1 cube made editable, top polygon | Extrude, offset 50, Bevel 10, Subdivision 2 |

Compare with `modeling-tools/extrude-subdiv` (Bevel 0): note any difference in vertex
count or bbox.

## Notes
TODO
