# Re-checks from O1 and O4 — observation (O5)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted where possible ([agents/OBSERVER.md](../agents/OBSERVER.md)). Unblocks:
three planner rulings in brief 07 that rest on a single reading.

## 1. Slice "To" default

Make a cylinder, cone, disc, torus, tube and capsule with their menu commands. For each,
read Slice > To back through the scripting interface **and** through `get_parameter`, and
record both in degrees. If the two disagree, say so.

| Object | To (probe) | To (get_parameter) |
|---|---|---|
| cylinder | | |
| cone | | |
| disc | | |
| torus | | |
| tube | | |
| capsule | | |

## 2. Cap type of Lathe, Loft and Sweep

O4 reads N-gon caps for all three, while the R20 meshes show triangles. Settle it with meshes:

| Case | Scene (menu commands) | Export to | Cap faces seen (count, corners per face) |
|---|---|---|---|
| lathe-defaults | Lathe with a default Rectangle spline as its child | `tests/golden/rechecks/lathe-defaults/mesh.obj` | |
| loft-defaults | Loft with two default Circle splines as children, the second at position y 200 | `tests/golden/rechecks/loft-defaults/mesh.obj` | |

Lathe has no caps on a closed revolution: if the export has no cap faces, note that and
read the Caps tab defaults instead. `case.toml` lists every parameter you can read from
the generator, as OBSERVER.md §3 says. Use `feature = "rechecks"`.

## 3. Boole default type

Read Boolean Type after creating the Boole object through its menu command in each of
three ways: nothing selected, one cube selected, and two cubes selected.

| How created | Boolean Type read | Children it got |
|---|---|---|
| nothing selected | | |
| one cube selected | | |
| two cubes selected | | |

## Notes
TODO
