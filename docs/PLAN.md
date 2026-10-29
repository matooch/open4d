# Plan

Parity target: the reference app **R25** (R25.010, macOS, observed through the MCP
bridge: [agents/OBSERVER.md](agents/OBSERVER.md)). Golden data before batch 08 came from
R20 on Windows. O1 showed R25 builds the **same geometry** (9/13 byte-equivalent after
parsing; the rest differ only by changed defaults), so the R20 cases stay. Tests read the
app version from each case's observation file and apply that version's defaults (brief
07).

Work runs on two tracks. **Usability first** (milestone M1), then parity breadth. Tick a
box only when its tests pass (parity items: when their parity test passes).

## Parity levels

Not everything needs exact face-order parity. Each spec says which level it targets.

| Level | Applies to | Pass means |
|---|---|---|
| **A · exact** | primitives, generators, deformers, anything that exports a mesh | `compare_meshes`: same counts, same face lists, positions within 1e-4 (or `compare_geometric` where the spec says so) |
| **B · numeric** | clone matrices, field values, keyframe curves, MoData, dynamics at fixed seeds | `samples.json` values within the case's tolerance (see brief 07) |
| **C · behaviour** | UI layout, tools, shortcuts, viewport look, renders | the spec's behaviour list holds (UI tests); renders by PSNR threshold |

## Done so far (phases 0–7, partial)

- [x] 0 Workspace, CI, parity harness, golden layout, cleanroom gate.
- [x] 1 core: scene graph, params, undo, file, cache, registry, transforms.
- [~] 2 geometry: mesh, half-edge, normals, selections, subdivision, booleans, splines (UVs open).
- [~] 3 viewport: camera, picking, gizmos, snapping, grid, shading modes.
- [~] 4 ui: dock, object tree, attributes, coordinates, timeline placeholder, theme.
- [~] 5 primitives (all solid primitives; spline primitives circle/rectangle/helix only).
- [~] 6 generators II: SDS, Boole, Symmetry, Array, Connect, Instance, Extrude, Lathe, Loft, Sweep (Metaball etc. open).
- [~] 7 deformers: Bend, Twist, Taper, Bulge, Shear, Spherify (rest open).

## Track U · Milestone M1 "usable on Linux" (current)

Goal: model, animate, shade and render a small scene, save it, export it. Behaviour
follows the reference app where it matters for muscle memory (level C), our own design
elsewhere, marked **parity-pending** for later observation.

- [ ] U1 Editable meshes (batch 08): polygon object, Make Editable / Current State to
      Object (03); OBJ import/export + File menu (04); point/edge/polygon modes and
      selection (05); component move/rotate/scale (06).
- [ ] U2 Animation basics (batch 09): `samples.json` level-B golden format (08); keyframe
      tracks + interpolation, checked against O2/O12 (09); timeline with keys,
      record, autokey, playback (10). Briefs written; 08 running.
- [ ] U3 Familiar layout (batch 10): reference-like arrangement (tool palette left,
      Object manager top right, Attribute manager below it, Coordinates/Materials
      bottom), own icons, shortcut map (11).
- [ ] U4 Modeling tools (batch 11): extrude, inset, bevel, loop cut, knife, bridge,
      weld/merge, delete, optimize, normals flip/align; selection grow/shrink/loop/ring.
- [ ] U5 Look-dev (batch 12): camera + light objects, basic PBR material + material tag,
      material manager, viewport shows material colour.
- [ ] U6 Rendering (batch 13): `render-api` trait, CPU path tracer (BVH, area/point/
      sun lights, PBR), render settings, render to PNG/EXR and image sequence,
      render view.
- [ ] U7 Packaging: AppImage build in CI, crash-safe autosave, preferences.

## Track P · Parity breadth (after M1, or in parallel when the implementer waits)

- [ ] P1 Deformer debt: Bend `within_box` 3-axis gate (3 known mismatches), the
      MODIFIERS.md parity-pendings; then the remaining deformers (Squash & Stretch,
      Wrap, Wind, Melt, Explosion, Formula, Displacer, Smoothing, Jiggle, FFD, Spline,
      …) and Fit to Parent.
- [ ] P2 Fields (level B, sampled through the MCP): Linear, Spherical, Box, Cylinder,
      Cone, Torus, Capsule, Random, Formula, Shader, Group; layers, blend modes,
      remapping; deformer falloff via fields.
- [ ] P3 MoGraph (docs/specs/MOGRAPH.md): Cloner (linear, radial, grid, honeycomb,
      object), MoData, effectors (Plain, Random, Shader, Step, Delay, Formula, Target,
      Time, Spline, Push Apart), Matrix, Fracture/Voronoi Fracture, MoText, MoSpline,
      Tracer, MoInstance.
- [ ] P4 Materials and shaders beyond U5: node material, noise/gradient/fresnel/
      layer shaders, UV tools, texture tags.
- [ ] P5 Animation beyond U2: F-curve editor, tangent modes, tags/constraints
      (target, align to spline, …), node expressions, dynamics (rapier), caches.
- [ ] P6 Rendering beyond U6: AOVs, IPR, GPU path (render-gpu, wgpu).
- [ ] P7 io: glTF, FBX, Alembic, USD.
- [ ] P8 Scripting (Rhai/Python), plugin API stabilisation, Flatpak.

Parked: external renderer bridge ([REDSHIFT.md](REDSHIFT.md)). R25 doesn't bundle it,
so it isn't a parity item. Revisit after P6.

## Observations queue

The live queue, with statuses, is [parity/QUEUE.md](parity/QUEUE.md): the observer runs
it unattended (OBSERVER.md §6). Done so far: O0–O9, O11. Next: O12–O15.

Owner action still open: confirm the reference app's licence allows scripted use
(OBSERVER.md §1 step 6) and note it in r25-drift.md §0 (add a row).
