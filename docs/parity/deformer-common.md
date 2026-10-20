# Deformer common behaviour — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Exporter settings: [export-settings.md](export-settings.md).
Folder: `tests/golden/deformer-common/<case>/`.

Setup as in [bend.md](bend.md) (cube 200 × 400 × 200, segments 4 × 8 × 4) unless the row
says otherwise. Record every deformer as `[params.input2]`, `[params.input3]`, … in
**object-tree order** with `type = "bend"` etc., its params and `pos`/`rot`, and say in
the notes where each sits in the hierarchy (child of the cube, or sibling).

## Cases

| Case | Setup | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| stack-bend-twist | Bend (strength 90°) then Twist (angle 90°), both children of the cube, Bend first in the tree | | |
| stack-twist-bend | the same two, Twist first in the tree | | |
| sibling | Bend (strength 90°) as a **sibling** of the cube under a Null, cube first (does it deform?) | | |
| generator | Bend (strength 90°) as a child of a **Subdivision Surface** whose child is the cube (does it bend the subdivided result?) | | |
| disabled | Bend (strength 90°) child of the cube but **disabled** (green tick off) | | |
| on-plane | a Plane primitive (defaults) with a Bend (strength 90°) child | | |

## Behaviour notes
- Order rule (which deformer applies first): TODO
- Does a deformer affect siblings, a parent generator's output, or only its parent: TODO
