# Forge3D roadmap: from today to v1.0

This folder is the **whole project plan**. It is written so agents can run it without a
human relaying messages:
- **observers** produce reference data from the reference app;
- **reviewers** check that data and unlock work;
- **implementers** build from briefs;
- **leads** integrate and release.

Every unit of work is a file in this repository:
- observation **forms**: `docs/parity/*.md`, listed in [../parity/QUEUE.md](../parity/QUEUE.md);
- implementation **briefs**: `docs/agents/tasks/NNN-*.md`, listed in [BACKLOG.md](BACKLOG.md).

How the roles hand work to each other: [../agents/PIPELINE.md](../agents/PIPELINE.md).
Technology decisions the plan rests on: [DECISIONS.md](DECISIONS.md).

## The goal: Forge3D v1.0

A native desktop 3D modeling and animation application written in Rust that:
1. **Runs on Linux, macOS and Windows** from one code base, with release builds for each
   (AppImage/tarball, .app in .dmg, .zip/installer), built and tested in CI.
2. **Feels familiar to a user of the reference app** (R25):
   - the same window layout;
   - the same manager concepts (Objects, Attributes, Coordinates, Materials, Timeline);
   - the same object/generator/deformer/tag model;
   - matching default values and matching geometry (parity level A/B/C, see
     [../PLAN.md](../PLAN.md)).
3. **Covers the feature scope** in the [area files](#areas): object creation and
   manipulation, polygon and spline modeling, all primitives, generators and deformers,
   fields, the clone toolset (cloners, effectors, fracture, matrix, text, tracer), volume
   building and meshing, rigging (joints, skinning, IK, constraints, morphs), keyframe
   animation with dope sheet, value graph and speed graph, materials and UVs, dynamics,
   a CPU path tracer and fast viewport, and file exchange (OBJ, glTF, FBX, Alembic, USD,
   STL, PLY).
4. **Is fully scriptable and agent-controllable**:
   - a `forge` command-line tool for headless work;
   - a JSON-RPC server inside the running editor;
   - an MCP server so AI agents can build scenes and automate tasks;
   - an embedded scripting language.

   Every user action is a named, undoable command that all four reach.

### v1.0 acceptance scenarios

v1.0 ships when all of these pass on all three operating systems. The first two run
headless in CI; the rest are recorded as UI test scripts plus one manual run per release:

| # | Scenario |
|---|---|
| S1 | `forge` CLI builds a scene from a script (cube, cloner with random effector, bend deformer), evaluates frame 30, exports glTF and renders a PNG, headless |
| S2 | An MCP agent connects to a running editor, creates a sphere, keys its position at 0 and 30, plays the timeline, and reads back the evaluated position at frame 15 |
| S3 | Model a chair: primitives → make editable → extrude, bevel, loop cut, symmetry, subdivision surface → save, reopen, export OBJ |
| S4 | Motion graphics: a text object fractured into letters, cloned along a spline, effectors with a field falloff, animated over 90 frames, rendered as a 1080p image sequence |
| S5 | Volume: union of three spheres and a spline tube in a volume builder, smoothed, meshed, made editable |
| S6 | Rigging: a joint chain bound to a cylinder with automatic weights, IK with a pole target, pose morph, animated and played back in real time |
| S7 | Animation polish: dope sheet retime, value-graph tangent edits (broken, weighted), speed-graph easing, loop after |
| S8 | Look-dev: area light, physical camera with depth of field, a node material with noise and fresnel, CPU render with AOVs |
| S9 | Exchange: import FBX/glTF/Alembic exported from the reference app; geometry and animation match the reference within tolerance |
| S10 | Robustness: open a scene with 1 M polygons and 10 k clones, stay interactive (≥ 20 fps viewport on a mid-range GPU), no crash in a 30-minute fuzzed UI session |

## Areas

Each area file holds:
- the feature checklist;
- the **observation jobs** (what the observer records, linked to forms);
- the **implementation design** (crates, data model, algorithms, UI);
- the **briefs** with their dependencies;
- the area's definition of done.

| Area | File | Brief range | Depends on |
|---|---|---|---|
| A · Automation: commands, CLI, RPC, MCP, scripting | [areas/A-automation.md](areas/A-automation.md) | 011–019 | core |
| B · Modeling: polygon tools, selections, editable splines | [areas/B-modeling.md](areas/B-modeling.md) | 020–034 | A (commands) |
| C · Objects: primitives, spline primitives, generators | [areas/C-objects.md](areas/C-objects.md) | 035–049 | – |
| D · Deformers and fields | [areas/D-deformers-fields.md](areas/D-deformers-fields.md) | 050–064 | C |
| E · Clone toolset | [areas/E-clone-toolset.md](areas/E-clone-toolset.md) | 065–079 | D (fields) |
| F · Volumes | [areas/F-volumes.md](areas/F-volumes.md) | 080–086 | C |
| G · Animation: keys, dope sheet, value and speed graphs | [areas/G-animation.md](areas/G-animation.md) | 087–099 | A |
| H · Rigging, tags, constraints, node expressions | [areas/H-rigging.md](areas/H-rigging.md) | 100–114 | G |
| I · Dynamics and particles | [areas/I-dynamics.md](areas/I-dynamics.md) | 115–122 | E, G |
| J · Materials, shaders, UVs | [areas/J-materials.md](areas/J-materials.md) | 123–134 | – |
| K · Rendering and viewport | [areas/K-rendering.md](areas/K-rendering.md) | 135–146 | J |
| L · UI and UX: layout, managers, preferences | [areas/L-ui.md](areas/L-ui.md) | 147–162 | A |
| M · File exchange | [areas/M-io.md](areas/M-io.md) | 163–172 | G, H |
| N · Platforms, packaging, performance, docs | [areas/N-platform.md](areas/N-platform.md) | 173–185 | all |

## Order of work

```
          A (commands, CLI, RPC) ─────────────┬─────────────┐
          │                                   │             │
    C (objects) ── D (deformers, fields) ── E (clones) ── I (dynamics)
          │                                   │
          F (volumes)                         │
    B (modeling) ── L (UI layout) ─── G (animation) ── H (rigging) ── M (exchange)
    J (materials) ── K (rendering)
    N (platform) runs alongside everything: CI matrix first, packaging last
```

**Waves.** Implementers take the lowest-numbered `ready` brief in [BACKLOG.md](BACKLOG.md).
The numbering already follows this order:

| Wave | Contents | Milestone |
|---|---|---|
| W1 | 011–019 (automation core), 173–176 (CI matrix, cross-platform builds), 087 (keyframe rework), 020–024 (core modeling tools), 147–150 (layout) | **M2 "agent-drivable editor"**: S1, S2, S3 partly |
| W2 | 025–034, 035–049, 050–064 | **M3 "modeling and deformation parity"**: S3 |
| W3 | 065–079, 080–086 | **M4 "motion graphics and volumes"**: S4, S5 |
| W4 | 088–099, 100–114 | **M5 "animation and rigging"**: S6, S7 |
| W5 | 123–146, 115–122 | **M6 "look-dev, rendering, dynamics"**: S8 |
| W6 | 151–162, 163–172, 177–185 | **M7 "v1.0"**: S9, S10, release on all three OS |

Observers run the whole queue as early as possible, ahead of the implementers. Most
briefs need their observation to be done first, and the observer is the only role that
needs the reference app.

## Status

- Live brief status: [BACKLOG.md](BACKLOG.md).
- Observation status: [../parity/QUEUE.md](../parity/QUEUE.md).
- Parity results per feature: [../parity/STATUS.md](../parity/STATUS.md).
- Earlier phases and batches (briefs 01–10): [../PLAN.md](../PLAN.md).
