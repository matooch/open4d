# Deformer common behaviour — observation

Status: observed · App version: Cinema 4D R20.059 (Build RB272827) · OS: Windows 11 Pro 10.0.26200 · Observer: observer session · Date: 2026-10-20
Exporter settings: [export-settings.md](export-settings.md) (no deviation). All six exports contain the deformed result directly; no Current State to Object was needed.
Folder: `tests/golden/deformer-common/<case>/`.

Setup as in [bend.md](bend.md) (cube 200 × 400 × 200, segments 4 × 8 × 4) unless the row
says otherwise. Record every deformer as `[params.input2]`, `[params.input3]`, … in
**object-tree order** with `type = "bend"` etc., its params and `pos`/`rot`, and say in
the notes where each sits in the hierarchy (child of the cube, or sibling).

## Cases

| Case | Setup | `golden info` output (vertices / faces / size) | Notes |
|---|---|---|---|
| stack-bend-twist | Bend (strength 90°) then Twist (angle 90°), both children of the cube, Bend first in the tree | 162 / 160 (4-gons 160) / size (361.3762664794922, 459.1549377441406, 497.13555908203125), centre (55.26091766357422, 29.577468872070313, 120.48785400390625); bbox min (-125.42721557617188, -200, -128.07992553710938), max (235.9490509033203, 259.1549377441406, 369.0556335449219) | tree Cube > Bend, Twist (both children of the cube; `input2` = Bend, `input3` = Twist). **Bend applied first, then Twist**: the bent shape is twisted about Y, so it spreads into Z (size Z 497.1); the top-left corner (−100, 200, −100) ends at (100, 259.15, 359.15) |
| stack-twist-bend | the same two, Twist first in the tree | 162 / 160 (4-gons 160) / size (468.66019439697266, 468.6601867675781, 282.84271240234375), centre (124.8248405456543, 34.33009338378906, 0); bbox min (-109.50525665283203, -200, -141.42135620117188), max (359.1549377441406, 268.6601867675781, 141.42135620117188) | tree Cube > Twist, Bend (`input2` = Twist, `input3` = Bend). Twist first (Z stays within ±141.42 as in `twist/angle`), then the twisted column is bent in XY: the same corner ends at (359.15, 59.15, −100). Differs from stack-bend-twist, so order matters |
| sibling | Bend (strength 90°) as a **sibling** of the cube under a Null, cube first (does it deform?) | 162 / 160 (4-gons 160) / size (459.1549377441406, 459.1549377441406, 200), centre (129.5774688720703, 29.577468872070313, 0); normals 21, uvs 45 | tree Null > Cube, Bend (Bend is the cube's sibling; both `parent = "null"`). **It deforms**: byte-identical to `bend/strength`. Made by grouping the cube into a Null (Alt+G) and moving the Bend from the cube to the Null below the cube |
| generator | Bend (strength 90°) as a child of a **Subdivision Surface** whose child is the cube (does it bend the subdivided result?) | 2562 / 2560 (4-gons 2560) / size (459.1549377441406, 459.1549377441406, 200), centre (129.5774688720703, 29.577468872070313, 0); normals 1081, uvs 561 | tree Subdivision Surface > Cube, Bend (defaults: Subdivision Editor 2). **The subdivided result is bent**: 2560 quads (160 × 16) and the same bbox as `bend/strength`. Whether the bend acts on the cage before subdivision or on the subdivided mesh was not determined; the export is one `o Cube` group |
| disabled | Bend (strength 90°) child of the cube but **disabled** (green tick off) | 162 / 160 (4-gons 160) / size (200, 400, 200), centre (0, 0, 0); normals 6, uvs 45 | tree Cube > Bend with the Bend's green tick off (`enabled = false`): **no effect**, byte-identical to the undeformed cube (`input.obj`) |
| on-plane | a Plane primitive (defaults) with a Bend (strength 90°) child | 441 / 400 (4-gons 400) / size (282.84272384643555, 282.8427200317383, 400), centre (83.22706031799316, 75.92788314819336, 0); normals 10, uvs 441 | tree Plane > Bend; `input` = default Plane (`input.obj` = `plane/defaults/mesh.obj`). The plane lies at y = 0, the middle of the box (t = 0.5), so it is only **rotated by 45°** (half the strength) about Z and moved, not curved: first vertex (−200, 0, −200) → (−58.19, 217.35, −200); size X = size Y = 400·cos 45° = 282.84 |

## Behaviour notes
- Order rule (which deformer applies first): the deformers under one object apply in
  object-tree order, top first (stack-bend-twist ≠ stack-twist-bend; the coordinates
  match "first listed, first applied").
- Does a deformer affect siblings, a parent generator's output, or only its parent: a
  deformer affects its parent and also its siblings under a Null (sibling case = same as
  a direct child); under a Subdivision Surface it bends the generator's subdivided
  output. A disabled deformer does nothing. All results are in the OBJ export itself (no
  Current State to Object needed).
