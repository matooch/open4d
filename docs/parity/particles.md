# Particles and forces — observation (O40)

Status: blocked, 0 of 10 cases · App version: R25 25.010 · OS: macOS · Observer: Hermes observer, scripted unattended run · Date: 2026-11-20
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
| Birthrate Editor | `birthrate_editor` | 10 | not reported | not reported |
| Birthrate Renderer | `birthrate_renderer` | 10 | not reported | not reported |
| Visibility | `visibility_percent` | 100 % | not reported | not reported |
| Seed | `seed` | 0 | not reported | not reported |
| Relative Speed | `relative_speed` | off | not reported | not reported |
| Lifetime Variation | `lifetime_variation_percent` | 0 % | not reported | not reported |
| Speed | `speed` | 150 | not reported | not reported |
| Speed Variation | `speed_variation_percent` | 0 % | not reported | not reported |
| Rotation | `rotation_deg` | 0 deg | not reported | not reported |
| Rotational Variation | `rotational_variation_percent` | 100 % | not reported | not reported |
| End Scale | `end_scale` | 1 | not reported | not reported |
| Scale Variation | `scale_variation_percent` | 0 % | not reported | not reported |

## Cases

| Case | Emitter | Forces | Notes |
|---|---|---|---|
| defaults | defaults | none | Birthrate 10 editor and 10 renderer, Speed 150, Seed 0 (label table above). not observed: the emitter exposes no particle list to a script (no particle method, no tag, no cache), so no particle position, velocity or age can be read. |
| rate-50 | birthrate (editor and render) 50 | none | not observed: the emitter exposes no particle list to a script. A probe stepped the document to frame 10 and tried the object's particle methods (none exist on the object), its tags (it has none), and its cache (empty), so no particle position, velocity or age can be read. The emitter parameter table above is observed and complete. |
| speed-var | speed 300, variation 50 % | none | not observed: the emitter exposes no particle list to a script. A probe stepped the document to frame 10 and tried the object's particle methods (none exist on the object), its tags (it has none), and its cache (empty), so no particle position, velocity or age can be read. The emitter parameter table above is observed and complete. |
| gravity | defaults | a Gravity force object, default | not observed: the emitter exposes no particle list to a script. A probe stepped the document to frame 10 and tried the object's particle methods (none exist on the object), its tags (it has none), and its cache (empty), so no particle position, velocity or age can be read. The emitter parameter table above is observed and complete. |
| wind | defaults | Wind, default, rotated so it blows +X | not observed: the emitter exposes no particle list to a script. A probe stepped the document to frame 10 and tried the object's particle methods (none exist on the object), its tags (it has none), and its cache (empty), so no particle position, velocity or age can be read. The emitter parameter table above is observed and complete. |
| attractor | defaults | Attractor at (0, 200, 300), default strength | not observed: the emitter exposes no particle list to a script. A probe stepped the document to frame 10 and tried the object's particle methods (none exist on the object), its tags (it has none), and its cache (empty), so no particle position, velocity or age can be read. The emitter parameter table above is observed and complete. |
| deflector | defaults, emitter pointing down onto a Deflector plane at y −100 | Deflector | not observed: the emitter exposes no particle list to a script. A probe stepped the document to frame 10 and tried the object's particle methods (none exist on the object), its tags (it has none), and its cache (empty), so no particle position, velocity or age can be read. The emitter parameter table above is observed and complete. |
| friction | speed 300 | Friction force, default | not observed: the emitter exposes no particle list to a script. A probe stepped the document to frame 10 and tried the object's particle methods (none exist on the object), its tags (it has none), and its cache (empty), so no particle position, velocity or age can be read. The emitter parameter table above is observed and complete. |
| turbulence | defaults | Turbulence, default | Random: compare statistically |
| seed | defaults with the random seed changed (if the field exists) | none | not observed: the emitter exposes no particle list to a script. A probe stepped the document to frame 10 and tried the object's particle methods (none exist on the object), its tags (it has none), and its cache (empty), so no particle position, velocity or age can be read. The emitter parameter table above is observed and complete. |

## Notes
TODO

## Notes

Attempted in an unattended window, 2026-11-20; nothing could be observed, so no case folder and no
samples were written.

- The standard Emitter object is a menu command away and its whole parameter table reads back (see
  the table above): birthrate 10 editor and renderer, seed 0, speed 150, no variations, lifetime and
  other defaults as listed. That part of the form is observation.
- The particle data itself is not reachable from a script in this build. The emitter object carries
  no particle method, no tag, and no cache at frame 10; the particle classes the scripting module
  offers are plain data records without any way to ask the emitter for its particles. Nothing in the
  document, the object or its children hands out a particle count, position, velocity or age.
- The three probes used here kept to the allowed surface (new document, menu command, parameter
  read-back, frame stepping, read-only cache inspection). No baking and no export that opens a
  dialog was tried: Bake Particles and the Alembic point export both need a human.
- Consequence: every case in this form needs the particle access the scripting surface lacks, so the
  job is blocked rather than partial. If a later build exposes the particle list, all ten cases can
  be built with the same scene recipe: new document, 30 fps, emitter with the case's settings, the
  named force objects created by their own menu commands, then frames 0 to 30 stepped one at a time.
- For the record, the force objects the cases name (Gravity, Wind, Attractor, Deflector, Friction,
  Turbulence) were not built, because no case could be sampled even with them in place.
