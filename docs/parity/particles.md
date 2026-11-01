# Particles and forces — observation (O40)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted, 30 fps.

Use the app's standard **particle Emitter** object, not the advanced particle system,
which is out of scope (DECISIONS D13).

Step the document frame by frame from 0. At frames 10, 20 and 30, read every live
particle's position, velocity and age, through the emitter's documented particle access.

Output:
- `tests/golden/particles/<case>/samples-f<frame>.json`, kind `clones`:
  - one entry per particle, with pos = position;
  - x = velocity (in the x slot, since clones has no velocity field: note this);
  - y = (age, 0, 0); z = (0, 0, 1);
  - colour (1, 1, 1), weight 1, visible true;
- `mesh.obj`: a small cube emitted as the emitter's child, exported at frame 30.

Use `feature = "particles"`. Parity is level C (statistics). Unblocks: I/119, 120.

Record the emitter's full parameter table once.

| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Emitter | Forces | Notes |
|---|---|---|---|
| defaults | defaults | none | Record the birthrate, speed and seed |
| rate-50 | birthrate (editor and render) 50 | none | |
| speed-var | speed 300, variation 50 % | none | |
| gravity | defaults | a Gravity force object, default | |
| wind | defaults | Wind, default, rotated so it blows +X | |
| attractor | defaults | Attractor at (0, 200, 300), default strength | |
| deflector | defaults, emitter pointing down onto a Deflector plane at y −100 | Deflector | |
| friction | speed 300 | Friction force, default | |
| turbulence | defaults | Turbulence, default | Random: compare statistically |
| seed | defaults with the random seed changed (if the field exists) | none | |

## Notes
TODO
