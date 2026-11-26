# Progress

One page to check where the project stands. The planner updates it after every merged run.
Details live in the files linked at the bottom; when this page and those files disagree, the
linked files win.

**Last updated:** 2026-11-26, after Hermes run 9 (ray tracing: 200, 211, 201–204, 212), particles (119–121) and Pose Morph/S6.

## At a glance

| | Now |
|---|---|
| Implementation briefs | **199 of 204 done** (98 %) · 4 ready · 1 waiting |
| Golden reference cases | 1,117 (from the reference app, via the observer) |
| Tests in the gate | 2,500 passing, 0 failing (Linux, macOS, Windows CI) |
| Acceptance scenarios | S1–S9 pass (S6 with own-design Pose Morph, parity-pending) · S10 needs a measurement |
| Release | No release cut yet (v1.0 checklist is brief 185, last) |
| Next | Implementer run 6 continues from 197 (see below) |

## Acceptance scenarios (the "is it usable" test)

Each scenario is a real end-to-end task, run as an automated test. v1.0 = all ten pass on
all three OS. Definitions: [roadmap/README.md](roadmap/README.md).

| | Scenario | Status | What's left |
|---|---|---|---|
| S1 | CLI script: cube, cloner + random effector, bend, frame 30, glTF + PNG | ✅ passes | – |
| S2 | MCP agent drives a running editor (key a sphere, read frame 15) | ✅ passes | – |
| S3 | Model a chair in the editor, save, reopen, export OBJ | ✅ passes | Feel checks in GUI sessions G1/G2/G4 |
| S4 | Motion graphics: text → fracture → cloner on spline → field effectors → 1080p sequence | ✅ passes (205) | Spline-source cloner is own design (parity-pending) |
| S5 | Volumes: 3 spheres + spline tube → builder → smooth → mesh → editable | ✅ passes (206) | – (208: mesher stays manifold on thin tubes) |
| S6 | Rigging: joints, auto weights, IK + pole, **pose morph**, playback | ✅ passes (114) | Pose Morph is own design (D20), parity-pending O37; GUI capture G9 checks it |
| S7 | Animation polish: dope sheet, broken/weighted tangents, easing, loop | ✅ passes | – |
| S8 | Look-dev: area light, DOF camera, node material, CPU render + AOVs | ✅ passes (146) | Cross-OS reference check confirmed by CI |
| S9 | Import FBX / glTF / Alembic from the reference app, matches | ✅ passes (171): 6 cases × FBX, GLB, Alembic, STL through the CLI | glTF skinned-arm poses at f15/f30 pinned KNOWN_MISMATCH (≤ 5e-4) |
| S10 | 1 M polygons + 10 k clones, ≥ 20 fps, 30-min fuzz without crash | 🔴 measured (204): M5 Pro Tier 1 0.86 fps, Tier 2 0.21 fps | Brief 214 (persistent GPU scene, RT effect budgets); RTX 3090 run |

## Areas

| Area | Done | What's left |
|---|---|---|
| A · Automation (CLI, RPC, MCP, scripting) | 9 / 9 | – |
| B · Modeling tools, selections, splines | 18 / 18 | Interactive feel (GUI session G3) |
| C · Primitives and generators | 16 / 17 | 046 Null/Instance display (waits on GUI session G8) |
| D · Deformers and fields | 15 / 15 | Parity-pending: Bevel deformer rounding, Displacer UVW, Polygon Reduction positions |
| E · Clone toolset (MoGraph) | 17 / 17 | – |
| F · Volumes | 10 / 10 | – |
| G · Animation | 13 / 13 | – |
| H · Rigging, tags, constraints | 15 / 15 | Parity-pending: Pose Morph (own design D20, O37) |
| I · Dynamics and particles | 8 / 8 | Parity-pending: particles, forces and their point-source use (own design D20, O40; statistical tests) |
| J · Materials, shaders, UVs | 13 / 13 | Hands-on checks in GUI session G7 |
| K · Rendering and viewport | 11 / 12 | 142 GPU path tracer (queued) |
| L · UI and UX | 16 / 16 | Hands-on checks in GUI sessions G1, G2, G4 |
| M · File exchange | 12 / 12 | Done: S9 passes (171). Open: glTF skin-pose KNOWN_MISMATCH, PLY/USD/morph not observed |
| N · Platform, packaging, docs | 13 / 14 | 185 v1.0 release checklist (last) |
| R · Real-time GPU renderer | 13 / 16 | All three tiers built: raster + post (198, 199), hybrid RT (201), real-time path tracing (202), RT on Metal (211) and Vulkan (212, matches compute on lavapipe; RTX 3090 run pending), cross-tier tests (203), S10 benchmark (204). **Open: 214 performance** (S10 on M5 Pro: Tier 1 0.86 fps of 60, Tier 2 0.21 of 30; CPU scene packing dominates), 213 Tier 1/2 quality gaps (mat-glossy), 210 DX12 shadows |

## Next implementer run (run 10)

Run 9 finished: ray tracing (200, 211, 201, 202, 203, 204, 212).

1. 214 · Real-time performance: persistent GPU scene, RT effect budgets (S10 targets)
2. 213 · Tier 1/2 quality gaps (mat-glossy specular under hybrid shadows)

Then only 210 (DX12 shadows, your Windows PC), 046 (GUI session G8) and 185 (v1.0 release) remain.

## Blocked: waiting on you (owner)

- **DX12 shadows (210):** real-time shadows fail on DirectX 12 (Windows CI, WARP); Windows CI is red on that one test until it is fixed. Needs a run on your Windows PC (steps in the brief).

| Item | Why it's stuck | What unblocks it |
|---|---|---|
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
