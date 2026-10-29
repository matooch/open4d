# Effectors II — observation (O14)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [cloner.md](cloner.md) (O8):
- a cloner made through its menu command, set to Linear mode with Count 5 and P per step
  (0, 50, 0), with a 20 cm cube child;
- the effector made through its menu command while the cloner is selected;
- clone data and `mesh.obj` in `tests/golden/cloner/<case>/`, with
  `observation = "docs/parity/effectors-2.md"`.

Unblocks: P3 effectors.

## Parameters (fill once per effector)

For each effector, list the Effector tab and the Parameter tab, as in cloner.md.

### Plain
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Step
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Formula
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Target
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Delay
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Time
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Effector | Changed from default | Notes |
|---|---|---|---|
| plain-scale | Plain | Position off, Scale on, uniform scale 2 (or the closest settings) | |
| plain-rotation | Plain | Position off, Rotation on, H 90° | |
| plain-color | Plain | Color Mode On, colour (1, 0, 0) | Record the clone colours |
| plain-strength-50 | Plain | Position (0, 100, 0), Strength 50 % | |
| step-defaults | Step | – | |
| step-position | Step | Position on, P (0, 100, 0) | |
| formula-defaults | Formula | – | Note the formula text; at frame 0 |
| formula-frame-10 | Formula | – | Document time set to frame 10 |
| target-defaults | Target | a Null at (0, 0, 300) as the target object | |
| time-frame-30 | Time | Position on, P (0, 100, 0) | Document time set to frame 30 |
| delay-skip | Delay | – | Note only whether it changes anything on a still frame. No samples needed if it doesn't |
| two-effectors | Plain P (0, 100, 0), then Plain Scale on, 2 | – | Effector order as listed. Does the order matter? Add a second case with the order swapped, named `two-effectors-swapped` |

## Notes
TODO
