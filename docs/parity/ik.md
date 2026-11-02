# Rigging II: IK — observation (O35)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted. Unblocks: H/104. Level B.

## Setup

- **Joints:** a 3-joint chain J1 (0, 0, 0) → J2 (0, 100, 0) → J3 (0, 200, 0), made through
  the joint command.
- **IK:** add an **IK tag** on J1 with its tip set to J3, and a goal Null made through the
  tag's "add goal" function or the Null command.
- **Pole:** for the pole cases, a pole Null at the given position, linked as the pole.

Output per case:
- `tests/golden/ik/<case>/samples.json`, kind `clones`, one entry per joint (J1, J2, J3)
  holding each joint's **global** matrix (pos, x, y, z), with colour (1, 1, 1), weight 1
  and visible true;
- `mesh.obj`: a cube of size 10 at each joint, made with a cloner in Object mode on the
  joints, or the scene's joints exported. If joints don't export, add three small cubes as
  children of the joints and export those.

Use `feature = "ik"`.

## Parameters (fill once)

### IK tag
| Label | Our key | Default | Options / Min / Max |
|---|---|---|---|

## Cases

| Case | Goal position | Pole | Notes |
|---|---|---|---|
| reach-straight | (0, 200, 0) | none | Unchanged chain |
| reach-bend | (100, 100, 0) | none | |
| reach-near | (0, 50, 0) | none | Chain folds |
| reach-far | (0, 400, 0) | none | Out of reach: does the chain stretch, or point at the goal? |
| reach-side | (150, 50, 50) | none | |
| pole-x | (100, 100, 0) | pole Null at (0, 100, 200) | |
| pole-z | (100, 100, 0) | pole Null at (0, 100, −200) | |
| four-joints | a 4-joint chain, spacing 100, goal (150, 150, 0) | none | Multi-joint solver |
| twist | reach-bend, plus the IK tag's twist/rotation parameter at 45° (if present) | none | |

## Notes
TODO
