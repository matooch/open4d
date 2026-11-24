# Progress

One page to check where the project stands. The planner updates it after every merged run.
Details live in the files linked at the bottom; when this page and those files disagree, the
linked files win.

**Last updated:** 2026-11-24, after Hermes run 3 (PR #45) and the reference-machines doc (PR #44).

## At a glance

| | Now |
|---|---|
| Implementation briefs | **171 of 196 done** (87 %) · 14 ready · 11 waiting |
| Golden reference cases | 1,117 (from the reference app, via the observer) |
| Tests in the gate | 2,500 passing, 0 failing (Linux, macOS, Windows CI) |
| Acceptance scenarios | S1, S2, S3, S7 pass · S8 running now · S4, S5 queued · S6, S9 blocked · S10 needs a measurement |
| Release | No release cut yet (v1.0 checklist is brief 185, last) |
| Running now | Implementer run 4 (8 briefs + 2 queued, see below) |

## Acceptance scenarios (the "is it usable" test)

Each scenario is a real end-to-end task, run as an automated test. v1.0 = all ten pass on
all three OS. Definitions: [roadmap/README.md](roadmap/README.md).

| | Scenario | Status | What's left |
|---|---|---|---|
| S1 | CLI script: cube, cloner + random effector, bend, frame 30, glTF + PNG | ✅ passes | – |
| S2 | MCP agent drives a running editor (key a sphere, read frame 15) | ✅ passes | – |
| S3 | Model a chair in the editor, save, reopen, export OBJ | ✅ passes | Feel checks in GUI sessions G1/G2/G4 |
| S4 | Motion graphics: text → fracture → cloner on spline → field effectors → 1080p sequence | 🟡 brief 205 queued (run 4) | Test + manual chapter |
| S5 | Volumes: 3 spheres + spline tube → builder → smooth → mesh → editable | 🟡 brief 206 queued (run 4) | Test + manual chapter |
| S6 | Rigging: joints, auto weights, IK + pole, **pose morph**, playback | 🔴 blocked | Pose Morph (107) has no reference data: **owner decision** |
| S7 | Animation polish: dope sheet, broken/weighted tangents, easing, loop | ✅ passes | – |
| S8 | Look-dev: area light, DOF camera, node material, CPU render + AOVs | 🟡 brief 146, running in run 4 | – |
| S9 | Import FBX / glTF / Alembic from the reference app, matches | 🔴 waits on FBX (165, 166) and 170, 171 | FBX import/export not started: **owner decision on timing** |
| S10 | 1 M polygons + 10 k clones, ≥ 20 fps, 30-min fuzz without crash | 🟡 benchmarks exist (181) | A measured run on a real GPU (your M5 Pro / RTX 3090) |

## Areas

| Area | Done | What's left |
|---|---|---|
| A · Automation (CLI, RPC, MCP, scripting) | 9 / 9 | – |
| B · Modeling tools, selections, splines | 18 / 18 | Interactive feel (GUI session G3) |
| C · Primitives and generators | 15 / 17 | 040 Metaball (run 4), 046 Null/Instance display (waits on GUI session G8) |
| D · Deformers and fields | 14 / 15 | 054 Smoothing, Displacer, Polygon Reduction, Bevel (run 4) |
| E · Clone toolset (MoGraph) | 16 / 17 | 205 S4 acceptance (queued) |
| F · Volumes | 8 / 9 | 206 S5 acceptance (queued) |
| G · Animation | 13 / 13 | – |
| H · Rigging, tags, constraints | 13 / 15 | 107 Pose Morph (blocked), 114 S6 acceptance (needs 107) |
| I · Dynamics and particles | 5 / 8 | 119, 120 particles + forces (blocked), 121 (needs 119) |
| J · Materials, shaders, UVs | 13 / 13 | Hands-on checks in GUI session G7 |
| K · Rendering and viewport | 10 / 12 | 146 S8 (run 4), 142 GPU path tracer (run 4) |
| L · UI and UX | 16 / 16 | Hands-on checks in GUI sessions G1, G2, G4 |
| M · File exchange | 10 / 12 | 170 (run 4), 171 S9 tests |
| N · Platform, packaging, docs | 13 / 14 | 185 v1.0 release checklist (last) |
| R · Real-time GPU renderer | 0 / 8 | 197, 198, 199 in run 4; then 200–204 (ray tracing on Vulkan + Metal) |

## Running now: implementer run 4

Queue and rules: [agents/HERMES-IMPLEMENTER.md](agents/HERMES-IMPLEMENTER.md) §2.

1. 146 · S8 acceptance and rendering docs
2. 040 · Metaball
3. 054 · Smoothing, Displacer, Polygon Reduction, Bevel deformers
4. 170 · Camera, light and material exchange (glTF, USD, Alembic)
5. 197 · R1: node materials → GPU shader code (start of the real-time renderer)
6. 142 · GPU compute path tracer
7. 198 · R2: real-time raster PBR
8. 199 · R3: post effects and the viewport "Render" mode
9. 205 · S4 acceptance
10. 206 · S5 acceptance

## Blocked: waiting on you (owner)

| Item | Why it's stuck | What unblocks it |
|---|---|---|
| Pose Morph (107) → S6 (114) | The reference app's morph strength can't be scripted, so there's no golden data (O37) | Either: build it as **own design** (parity-pending), or capture it by hand in a GUI session |
| Particles + forces (119, 120, 121) | The emitter exposes no particle list to scripts (O40) | Same choice: **own design**, or a GUI capture |
| FBX (165, 166) → S9 (171) | Ready, just not scheduled | Say when you want it; it's a good next run |
| GUI sessions G1–G9 | Feel, layout, shortcuts, viewport look can only be checked by a person at the reference app | An hour or two per session: [parity/GUI-SESSIONS.md](parity/GUI-SESSIONS.md) |
| Hands-on use of the editor | Nobody has used the editor for real work yet; that's where the remaining usability bugs are | Build a small scene in it and report what gets in the way |
| S10 + R8 numbers | Need real GPUs | Run the benchmarks on the M5 Pro and RTX 3090 |
| Reference app licence | Confirm scripted use is allowed (OBSERVER.md §1 step 6) | A one-line note in `docs/parity/r25-drift.md` |

## Observation data (golden cases from the reference app)

Live queue: [parity/QUEUE.md](parity/QUEUE.md). Most jobs are done or partial (the missing cases
can't be scripted and are listed for the GUI sessions). Open: O28 (one follow-up case), O37 Pose
Morph and O40 Particles (blocked, see above).

## Planned, not started

- Real-time renderer tiers 2–3: hardware ray tracing on Vulkan (RTX 3090) and Metal (M5 Pro),
  real-time path tracing with denoising, consistency tests against the CPU renderer (200–204).
- External renderer bridge (parked, [REDSHIFT.md](REDSHIFT.md)).

## Where the details live

- Every brief and its status: [roadmap/BACKLOG.md](roadmap/BACKLOG.md)
- Plan, areas, decisions, scenarios: [roadmap/README.md](roadmap/README.md), [roadmap/DECISIONS.md](roadmap/DECISIONS.md)
- Parity results per feature: [parity/STATUS.md](parity/STATUS.md)
- Observation queue: [parity/QUEUE.md](parity/QUEUE.md); GUI sessions: [parity/GUI-SESSIONS.md](parity/GUI-SESSIONS.md)
- Implementer queue and rules: [agents/HERMES-IMPLEMENTER.md](agents/HERMES-IMPLEMENTER.md)
- History (early phases): [PLAN.md](PLAN.md)
