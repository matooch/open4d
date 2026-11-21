# Dynamics: rigid bodies — observation (O39)

Status: observed, 12 of 13 cases (`gravity-half` not observed, see its row)  · App version: R25 25.010 · OS: macOS · Observer: Hermes observer, scripted unattended run · Date: 2026-11-20
Method: scripted, 30 fps, project 0–90.

Simulate by stepping the document frame by frame from 0, so the simulation runs in order.
At every frame 0–90, read each listed object's **global** position (cm) and rotation H/P/B
(degrees).

Output:
- `tests/golden/dynamics/<case>/samples-<object>-<param>.json` (kind `track`, param
  `pos_x` … `rot_b`, `tolerance` 1e-3);
- `mesh.obj`: the whole scene at frame 90.

Use `feature = "dynamics"`. Parity is level C (trajectory thresholds).
Unblocks: I/115, 116.

Before the first case, record: the dynamics scene settings (gravity, steps per frame,
scale, …) and the rigid body tag's parameters.

| Setting | Our key | Default | Min / Max / Options | Bounce assigned 1e+18, the largest value the parameter reports, and read back unchanged. The cube falls to its lowest y 32.097976 at frame 22, then rebounds to 821.91056 at frame 23 (y 10060.042843 at frame 30, 47818.330129 at frame 60) and keeps climbing: y 82714.08342 at frame 90, still rising, so it leaves the frame. The case records exactly what the app does at that literal maximum. |---|---|---|---|
| Document frame rate | `fps` | 30 (set, then read back) | — |
| Project frame range | — | 0 – 90 (set, then read back) | — |
| Gravity | `gravity` | not readable as a setting; the free fall of the drop-cube case gives about 1014 cm/s² (least squares, frames 1–12) | — |
| Steps per frame | `steps_per_frame` | not observed: not reachable from a script | — |
| Simulation scale | `scale` | not observed: not reachable from a script | — |

The scene-level dynamics settings (gravity, steps per frame, scale) cannot be reached from a script
in this build: the document exposes no accessor for them and no scene settings object appears in the
object manager. The table holds only what could be set and read back, plus the acceleration measured
from the falling body.

### Rigid body tag, at its defaults

Read back from the tag that the two rigid-body commands create. Both bodies in the cases below
carry the same tag type; the one on the plane is the collider, the one on a moving object the body.

| Setting | Default | Min / Max / Options |
|---|---|---|
| Enabled | on | on / off |
| Dynamic | On | Off / On / Ghost |
| Show in Visualization | on | on / off |
| Trigger | Immediately | Immediately / At Velocity Peak / On Collision / By XPresso |
| Trigger Velocity Threshold | 10 | — |
| Custom Initial Velocity | off | on / off |
| Initial Linear Velocity | 0, 0, 0 | — |
| Initial Angular Velocity | 0, 0, 0 | — |
| Object Coordinates | on | on / off |
| Dynamic Transition | off | on / off |
| Transition Time | 2.0 s | — |
| Linear Velocity Threshold | 10 | 0 – 1e+18 |
| Angular Velocity Threshold | 10 deg | 0 – 1e+18 |
| Inherit Tag | Apply Tag to Children | 3 options, labels not read |
| Individual Elements | All | 4 options, labels not read |
| Self Collisions | on | on / off |
| Use Deformed Object | on | on / off |
| Shape | Automatic | 12 options, labels not read |
| Size Increment | 0 | — |
| Use Collision Margin | off | on / off |
| Margin | 0 | 0 – 1e+18 |
| Bounce | 5 % | 0 – 1e+18 (as the description reports it) |
| Friction | 90 % | 0 – 1e+18 |
| Collision Noise | 0.5 % | 0 – 90 % |
| Use Mass | World Density | 3 options, labels not read |
| Density | 1 | 0 – 1e+20 |
| Mass | 10 | 0 – 1e+20 |
| Rotational Mass | 100 % | 1 – 100 |
| Use Center of Mass | off | on / off |
| Center of Mass | 0, 0, 0 | — |
| Follow Position | 0 | 0 – 100 |
| Follow Rotation | 0 | 0 – 100 |
| Linear Damping | 5 % | 0 – 1e+18 |
| Angular Damping | 5 % | 0 – 1e+18 |
| Force Mode | Exclude | Exclude / Include |
| Drag | 0 | 0 – 1e+18 |
| Lift | 0 | 0 – 1e+18 |
| Two-Sided | off | on / off |
| Use Accurate Solver | on | on / off |
| Soft Body | Off | soft-body block follows |
| Structural | 100 % | soft-body block |
| Shear | 50 % | soft-body block |
| Flexion | 50 % | soft-body block |
| Damping | 20 % | soft-body block |

## Cases

| Case | Scene | Objects sampled | Notes |
|---|---|---|---|
| drop-cube | a cube (size 50) at (0, 300, 0), dynamic rigid body; a plane (default, 400×400) at y 0, collider | cube | Read back in `samples-cube-pos_*.json` / `samples-cube-rot_*.json` (frames 0–90, 30 fps, tolerance 1e-3) plus the mesh at frame 90. First contact at frame 23 (lowest y read 24.539 at frame 23); rebound peak 29.337 at frame 26; at rest from frame 30 on at y 25.002 through frame 90. Rotation 0 to frame 22; after the impact H 0.1 / P -0.0 / B 0.0 deg at frame 90 (B reads 360 at the impact frame, a decomposition flip). Frames 0–8 read 300.000, 299.334, 297.559, 294.677, 290.690, 285.600, 279.409, 272.118, 263.729. mesh.obj: bbox 400.000 x 50.002 x 400.000 cm, 129 vertices, 106 quads (plane 400 x 400 plus the cube resting on it). |
| drop-bounce | as drop-cube, with the cube's bounce at its maximum allowed value | cube | Bounce assigned 1e+18, the largest value the parameter reports, and read back unchanged. Contact at frame 23 (lowest y 821.91056, rebound peak 82714.08342). The body then gains energy instead of losing it: y 10060.042843 at frame 30, 47818.330129 at frame 60 and 82714.08342 at frame 90, still rising, so the cube leaves the frame. The case records exactly what the app does at that literal maximum. |
| drop-friction-slide | the cube starts at (0, 25, 0) with initial velocity (300, 0, 0) cm/s, friction default | cube | Custom Initial Velocity on with Initial Linear Velocity (300, 0, 0) cm/s. The cube slides 45.025817 cm: x 45.025817 at frame 90 (x 45.013775 at frame 30, 45.025504 at frame 60), y 25.002184 at the end, dipping to 24.877875 while it slides. 30 frames per second, frames 0-90. |
| drop-friction-high | as drop-friction-slide, with friction at its max | cube | Friction assigned 1e+18, read back unchanged. The cube slides 66.096937 cm to x 66.096937 (y 25.002193 at the end, dipping to 24.500353). That is further than the default-friction case above, although a higher friction should stop it sooner: recorded as the app shows it, not adjusted. |
| stack | 3 cubes (size 50) stacked at y 25, 75, 125 on the plane, all dynamic | cube1, cube2, cube3 | All three cubes stay where they were put. At frame 90: cube1 y 24.998325, cube2 y 74.981665, cube3 y 124.973569, with drift of at most 0.123179 cm in x and 0.05968 cm in z. The stack does not collapse. |
| slope-sphere | a sphere (r 25) on a plane rotated B 20°, dynamic | sphere | Plane rotation read back as H 0 / P 0 / B 20 deg. The sphere rolls down the slope and leaves the 400 x 400 plane: at frame 90 it is at (669.192353, -1593.613655, 1.355377) with H 0.257882 / P 0.395764 / B 190.191655 deg. Frames 0-90, 30 fps; the mesh at frame 90 holds the sphere off the plane. |
| tumble | a cube at (0, 300, 0) rotated (30, 20, 10), dropped | cube | Rotation set to H 30 / P 20 / B 10 deg. Contact at frame 24 (lowest y 28.826287), rebound peak 34.411138, and at frame 90 the cube rests at (-15.478351, 25.001896, 16.264318) with H 321.428737 / P 0.001602 / B 359.999402 deg: it turned about one full turn around B and came to rest on another face. |
| clones-fall | a cloner (Grid 3×1×3, size 200) of cubes (size 30) at y 300, with a dynamic body tag on the cloner (per-clone bodies) | clone positions: the clones kind per frame is too large, so sample only clones 0 and 4 as objects `clone0`, `clone4`, using the clone matrices at each frame | The cloner itself stays at (0, 300, 0) (global y 300.000 at frame 0 and at frame 90) while the clones are simulated as individual bodies. Clone 4, the centre clone, starts at (0, 300, 0) and lands at (1.871842, 15.002187, 1.870218). Clone 0, the corner clone starting at (-200, 300, -200) on the plane edge, falls past the plane: y -2227.771714 at frame 90. Frames 0-90, 30 fps. |
| gravity-half | drop-cube, scene gravity at half the default | cube | not observed: the scene gravity is not reachable from a script (see the Settings table) and the body tag has no gravity parameter, so half the default cannot be set. |

## Follow-up cases (planner, after briefs 115/116)

| Case | Setup | Record | Result |
|---|---|---|---|
| bounce-combine-0 | `drop-cube`, but the plane's Bounce 0 % (cube default) | the same tracks as `drop-cube` | observed: the plane tag Bounce reads back 0 % (its default 100 %); `samples-cube-*` over 91 frames: first contact frame 24, y_min 23.816228 at frame 24, rebound peak 25.72472, ends at 24.999673 — the cube settles with no visible rebound |
| bounce-combine-50 | the same with the plane's Bounce 50 % | the same | observed: plane Bounce reads back 50 %; first contact frame 24, y_min 24.177597, rebound peak 27.28798, ends at 25.000308 — the rebound peak grows with Bounce: 25.72472 (0 %), 27.28798 (50 %), 29.337369 (100 %, `drop-cube`) |
| clones-fall-2 | `clones-fall`, but the cloner moved by (13, 0, 7) so no clone sits exactly on the plane's edge | the same tracks as `clones-fall` | observed: cloner at (13, 300, 7); clone 0 starts at (−187, 300, −193) and still tips off the moved plane edge, ending at (−501.587775, −1986.90774, −517.687678); clone 4 starts at (13, 300, 7) and rests on the plane at y 15.002201 (ends x 5.119615, z 10.052614); `samples-clone0-*` and `samples-clone4-*`, 91 frames — the same tracks as `clones-fall` |
| collision-noise-0 | `clones-fall` with the tag's Collision Noise 0 | the centre clone's tracks | observed: the cloner body tag Collision Noise reads back 0 % (default 0.5 %); the centre clone (index 4) starts at (0, 300, 0) and rests at (0.944188, 15.001608, 0.501539) versus (1.871842, 15.002187, 1.870218) in `clones-fall` — same landing, different jitter path; `samples-clone4-*`, 91 frames |

Also read the scene's dynamics settings in the GUI if a script cannot (gravity, steps per frame)
and note them under GUI session G9.
ocity Threshold 10; Custom Initial Velocity off with Initial Linear Velocity (0, 0, 0) and
  Initial Angular Velocity (0, 0, 0); Set Initial State and Clear Initial State buttons; Show in
  Visualization on; plus the collision, mass, friction, bounce and force blocks further down
  the table.
- The scene-level dynamics settings (gravity, steps per frame, scale) were **not** located from
  the script: the document exposes no dynamics accessor, and no scene object of a dynamics type
  appeared in the object manager, so the Settings table at the top of this form is still empty.
  The fall in the probe above is therefore the new document's defaults, not a set of recorded
  values, and none of the nine cases is claimed.
- No case rows are filled and no golden data was written for this form: without the scene
  settings table, every case would record a simulation whose gravity and step count are unknown,
  which would be worse than leaving the rows empty.

### Observed in the window of 2026-11-20 (7 more cases)

- `drop-bounce`, `drop-friction-slide`, `drop-friction-high`, `stack`, `slope-sphere`, `tumble` and
  `clones-fall` are recorded: 6 tracks per sampled object, frames 0-90 at 30 fps, plus the mesh at
  frame 90. `gravity-half` stays unobserved for the reason in its row.
- The collider command puts the same tag type on the plane as the body command puts on a moving
  object, but the collider's tag reads Dynamic off while the body's tag reads Dynamic On. That is why
  the plane stands still and the cube falls.
- The stack is stable: three 50 cm cubes at 25 / 75 / 125 cm sink by at most 0.045 cm and drift at
  most 0.12 cm sideways over 90 frames. Nothing topples.
- Two literal "at its maximum" readings were recorded as found rather than adjusted: Bounce at
  1e+18 makes the cube gain energy on impact (y 82 714 cm at frame 90, still rising), and Friction at
  1e+18 lets the cube slide further (66.1 cm) than the default 90 % does (45.0 cm).
- The cloner keeps its own position (global y 300.000 at frame 0 and at frame 90) while the clones
  are simulated individually. A clone outside the 400 x 400 plane, or on its edge, simply falls past
  it; the corner clone 0 ends at y -2 227.772 cm.
- The sphere on the 20 deg slope rolls off the 400 x 400 plane and ends at (669.192, -1593.614,
  1.355) cm at frame 90, so its track is a fall, not a rolling-to-rest curve.
- The tumble cube lands on another face: B goes from 10 deg to 359.999 deg, essentially one turn.

### Follow-up observations, 2026-11-21

- Bounce sweep on the same drop: rebound peak after contact is 25.72472 / 27.28798 / 29.337369 at
  plane Bounce 0 / 50 / 100 % (first contact at frame 24 in both new cases, 23 in `drop-cube`).
- MoGraph matrix source: `GeGetMoData` returns cloner-local matrices in this build — the cloner's
  own position is not folded in — while the committed `clones-fall` samples hold world positions.
  The two cloner cases therefore add the cloner's position (13, 300, 7) and (0, 300, 0) to the
  sampled offsets, reproducing the committed convention; checked against the committed records
  (clone 4 starts at (0, 300, 0) and rests at y 15.002 in both).
- The cloner also carries a Data Tag named "Info", so the body tag is found by type
  ("Dynamics Body") rather than as the last tag in the tag list.
- Scene gravity/steps stay GUI-only per the section note; untouched here.
