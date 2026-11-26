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

## Follow-up (planner, 2026-11-25, D20): capture particle statistics through an export

The emitter's particle list can't be read by script, and we now build particles as our own
design with statistical tests (DECISIONS D20). Try these, in order, and record what works:

1. **Alembic export of the emitter** at frames 10, 20 and 30 (the exporter's particle/points
   option on). If the file holds points, write `samples.json` with per frame: particle count, mean
   and standard deviation of position (x, y, z), of speed (from consecutive frames if no velocity
   is stored) and, if present, age.
2. **A Matrix object in Object mode on the emitter**, or a **Tracer** with the emitter as its
   source: if either exposes one point or matrix per particle through its documented data, read
   them per frame and write the same statistics.
3. If neither works, write "not capturable" with what each attempt returned.

Cases: the form's baseline emitter (default settings) and the gravity case. Same frames, same
seed. No per-particle data is needed, only the statistics above.

### Observer result (2026-11-26, unattended run): both cases not capturable

Tried in the follow-up order, one attempt at a time, scripted throughout (no dialog opened).

1. **Alembic export: the export works, reading the points back does not.** Fresh document,
   30 fps, Emitter alone (defaults) and Emitter + Gravity, both stepped frame 0 to 30 one
   frame at a time; a single-frame Alembic archive written at frames 10, 20 and 30 with the
   exporter options Particles, Particles (Geometry) and Points Only (Geometry) set to on
   (scripted save with the scene-saver filter, no dialog). Sizes in bytes - defaults:
   1716 / 1812 / 1908, gravity: 1716 / 1812 / 1908 (frames 10/20/30); the gravity files
   differ from the defaults files byte for byte, so the force changed the cloud while the
   counts stayed equal. Each file contains an AbcGeom_Points object named `EmitterCloud`
   (strings in the file also show a `.velocities` property; no age property appears).
   Re-import of the archive gives an `Alembic Generator` whose cache is a `TP Geometry`
   object: it has no point access at all (`GetPointCount` / `GetAllPoints` do not exist on
   it). The importer option Points = "As Polygon Object" was set on the plugin data and as
   world plugin data and still yields TP Geometry; `Current State to Object` on the
   generator produces an empty Null; the loaded document particle system reports
   `NumParticles() = 0`; sending the TP get-particles message to the generator and to its
   cache returns an empty container. No particle count, position, speed or age could be
   read, so no statistics could be written.
2. **Matrix object in Object mode:** the source link is settable (container assignment;
   Mode reads back as Object), but the evaluated cache is null after stepping frames 0-10.
   A control Matrix fed a Cube is null too, while an unconfigured Grid Matrix caches a
   6400-point polygon, so evaluation works in general - Object mode yields nothing a
   script can read here.
3. **Tracer with the emitter as source:** the Trace Link accepts the emitter through
   SetParameter with the user-interaction flag, but the Tracer cache is null after
   stepping frames 0-10. No data.

| Case | Result |
| --- | --- |
| defaults | not capturable: Alembic export succeeds and the file holds the particle points, but every script read path returns nothing (TP Geometry with no point API, empty particle system, empty TP messages); Matrix Object mode and Tracer both evaluate to an empty cache |
| gravity | not capturable: same path and same failures as defaults (export ok and differs from the defaults files, re-import gives TP Geometry, NumParticles 0) |

No golden folder and no samples were written: the statistics the form asks for could not
be observed. The exporter recipe (which options, which filter, frame stepping) is in
`~/forge-observer/notes.md` for a later run if a read path appears.
