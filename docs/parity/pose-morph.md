# Pose Morph — observation (O37)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted. Unblocks: H/107. Level A.

## Setup

- **Mesh:** a sphere (radius 100, segments 16), Made Editable.
- **Tag:** add a **Pose Morph** tag with point morphing enabled. The tag's base pose is the
  sphere as made.
- **Morph A:** move every point with y > 50 by (0, 50, 0).
- **Morph B:** scale every point's x by 1.5.

Set the morph points through the script, in the tag's edit mode or its scripting
interface. Record the exact point displacement rules in `case.toml`.

Output: `tests/golden/pose-morph/<case>/mesh.obj`, with `feature = "pose-morph"`.

## Parameters (fill once)
| Label | Our key | Default | Options / Min / Max |
|---|---|---|---|

## Cases

| Case | Strengths | Mode | Notes |
|---|---|---|---|
| base | A 0 %, B 0 % | relative | |
| a-50 | A 50 % | relative | |
| a-100 | A 100 % | relative | |
| a-150 | A 150 % (if allowed) | relative | Extrapolation |
| ab-mix | A 100 %, B 50 % | relative | |
| absolute | A 100 %, B 50 % | absolute (or the other mode) | |
| psr-morph | a second tag with position/rotation morphing: morph C rotates the object H 90° | 50 % | Record the resulting object matrix as clones samples too |

## Notes
TODO
