# Progress

One page to check where the project stands. The planner updates it after every merged run.
Details live in the files linked at the bottom; when this page and those files disagree, the
linked files win.

**Last updated:** 2026-11-27, after 233 (the owner's GUI session applied), 046 (Null display), the run 10 checkpoint of 214 (Tier 1 at 100 fps) and the new area P (behaviours, live input, play mode; briefs 215–222).

## At a glance

| | Now |
|---|---|
| Implementation briefs | **206 of 222 done** (93 %) · 214 in progress · 15 ready · 231 waits on O64 |
| Golden reference cases | 1,117 (from the reference app, via the observer) |
| Tests in the gate | 2,500 passing, 0 failing (Linux, macOS, Windows CI) |
| Acceptance scenarios | S1–S9 pass (S6 with own-design Pose Morph, parity-pending) · S10: Tier 1 meets 60 fps, Tier 2 open (214) · S11 new (area P) |
| Release | No release cut yet (v1.0 checklist is brief 185, last) |
| Next | Implementer run 10 continues 214, then 213, then area P (215–222) |

## Acceptance scenarios (the "is it usable" test)

Each scenario is a real end-to-end task, run as an automated test. v1.0 = all eleven pass on
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
| S11 | Behaviours: Luau-built cloner with a stateful behaviour, gamepad + MIDI (injected), live play, recorded, rendered byte-identically, scrubbed | ⚪ not started | Briefs 215–222 (area P, D21) |

## Areas

| Area | Done | What's left |
|---|---|---|
| A · Automation (CLI, RPC, MCP, scripting) | 9 / 9 | – |
| B · Modeling tools, selections, splines | 18 / 18 | Interactive feel (GUI session G3) |
| C · Primitives and generators | 17 / 17 | Parity-pending: Null display own-design details (O20b), X-Ray drawing |
| D · Deformers and fields | 15 / 15 | Parity-pending: Bevel deformer rounding, Displacer UVW, Polygon Reduction positions |
| E · Clone toolset (MoGraph) | 17 / 17 | – |
| F · Volumes | 10 / 10 | – |
| G · Animation | 13 / 13 | – |
| H · Rigging, tags, constraints | 15 / 15 | Parity-pending: Pose Morph (own design D20, O37) |
| I · Dynamics and particles | 8 / 8 | Parity-pending: particles, forces and their point-source use (own design D20, O40; statistical tests) |
| J · Materials, shaders, UVs | 13 / 13 | Hands-on checks in GUI session G7 |
| K · Rendering and viewport | 11 / 12 | 142 GPU path tracer (queued) |
| L · UI and UX | 22 / 26 | Owner's first hands-on use (2026-11-26, D22): 223 nav 1/2/3 + E/R/T, 224 keyframe toggle, 232 keyframe drafts from the gizmo, Coordinates and inspector shown in the viewport (owner note N10; the frame-change and selection discard rules parity-pending G5), 225 one mode UI and 229 object manager cues are done (owner's feel check on the Mac pending; placements and numbers parity-pending G1/G2/G5); 233 applied the owner's Cowork session (observed key map with two-key sequences, 20 px object rows with boxed plus/minus, stacked visibility dots, four key diamond states, chevron number fields, Shift-click tab stacking, 1920 x 1080 window metrics; sequence timing, drop markers and scrub modifiers stay parity-pending); open: 226 docking, 227 icons and reference layout, 228 clone toolset at top level, 230 script lookup, 231 settings in tabs (waits on observation O64). GUI sessions G1/G2 give the exact layout |
| M · File exchange | 12 / 12 | Done: S9 passes (171). Open: glTF skin-pose KNOWN_MISMATCH, PLY/USD/morph not observed |
| N · Platform, packaging, docs | 13 / 14 | 185 v1.0 release checklist (last) |
| P · Behaviours, live input, play mode (Luau, Rust; D21) | 0 / 8 | Briefs 215–222 queued after 213: Luau runtime and builder scripts, behaviour tag with start/update and a per-frame cache, inputs (keyboard, gamepad, MIDI, OSC) through named actions, input tracks, play mode, bulk clone API, Rust behaviours, `forge play` and S11 |
| R · Real-time GPU renderer | 13 / 16 | All three tiers built: raster + post (198, 199), hybrid RT (201), real-time path tracing (202), RT on Metal (211) and Vulkan (212, matches compute on lavapipe; RTX 3090 run pending), cross-tier tests (203), S10 benchmark (204). **214 performance in progress** (S10 on M5 Pro: Tier 1 now 100 fps, target 60, met; Tier 2 0.77 fps of 30, diffuse GI 1,141 ms is next), 213 Tier 1/2 quality gaps (mat-glossy), 210 DX12 shadows |

## Next implementer run (run 10)

Run 9 finished: ray tracing (200, 211, 201, 202, 203, 204, 212).

1. 214 · Real-time performance (continuing: Tier 1 done; Tier 2 diffuse GI on the GPU next)
2. 213 · Tier 1/2 quality gaps (mat-glossy specular under hybrid shadows)
3. UI round (D22): 223, 224, 225, 229, 226, 227, 228
4. Area P, behaviours: 215, 216, 217, 218, 220, 221, 219, 222 (S11), then 230

Then only 210 (DX12 shadows, your Windows PC) and 185 (v1.0 release) remain.

## Blocked: waiting on you (owner)

- **DX12 shadows (210):** real-time shadows fail on DirectX 12 (Windows CI, WARP); Windows CI is red on that one test until it is fixed. Needs a run on your Windows PC (steps in the brief).

| Item | Why it's stuck | What unblocks it |
|---|---|---|
| GUI sessions G1–G9 | Feel, layout, shortcuts, viewport look can only be checked by a person at the reference app | An hour or two per session: [parity/GUI-SESSIONS.md](parity/GUI-SESSIONS.md) |
| Hands-on use of the editor | Nobody has used the editor for real work yet; that's where the remaining usability bugs are | Build a small scene in it and report what gets in the way |
| S10 + R8 numbers | Need real GPUs | Run the benchmarks on the M5 Pro and RTX 3090 |

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
