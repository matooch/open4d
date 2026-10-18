# Parity status
| Feature | Spec | Golden | Test | Notes |
|---|---|---|---|---|
| Phase 0: clean-room gate | docs/specs/INFRA.md §2 | n/a (synthetic) | tools/cleanroom/tests | Passing; CI green (run 39030302799) |
| Phase 0: golden layout + harness | docs/specs/INFRA.md §3–4 | crates/parity-harness/tests/fixtures (synthetic) | crates/parity-harness/tests | Passing; CI green (run 39030302799); no reference golden cases yet |
| Phase 1: core (params, registry, scene, undo, dirty/cache, file) | docs/specs/CORE.md | n/a (internal architecture) | crates/core/tests | Passing; CI green (run 39031237650). Transforms deferred until docs/parity/transform.md exists |
| Phase 2a: geometry (mesh, half-edge, normals, selections, splines) | docs/specs/GEOMETRY.md | n/a (published literature) | crates/geometry/tests | Passing; CI green (run 39033285128). Parity-pending defaults listed in spec. Phase 2 stays open: subdivision, booleans, transforms await observations |
| Phase 3a: viewport (camera, picking, snapping, grid, translate/scale gizmo, wgpu renderer) + n-gon triangulation | docs/specs/VIEWPORT.md, GEOMETRY.md §6 | n/a (published literature) | crates/viewport/tests, crates/geometry/tests/triangulate.rs | Passing; CI green incl. GPU tests on lavapipe (run 39035547589). Phase 3 stays open: rotate gizmo and object-local axes await docs/parity/transform.md |
| Phase 4a: editor shell (dock, object tree, attributes from descriptors, commands, timeline placeholder, theme, embedded viewport) | docs/specs/UI.md | n/a (own design) | crates/ui/tests (incl. egui_kittest UI tests) | Passing; CI green (run 39038146003). Phase 4 stays open: coordinate manager (needs transforms), material manager (Phase 9), saved layouts |

Observation kit and per-feature forms: [README.md](README.md).
| Object transforms (pos/rot HPB/scale, parent composition) | docs/specs/TRANSFORM.md | tests/golden/transform (8) + calibration (1), from docs/parity/transform.md | crates/core/tests/transform_parity.rs | Passing locally (all 9 cases within 1e-4). Other rotation orders and viewport handedness await observation |
| Plane primitive | docs/specs/PRIMITIVES.md §1 | tests/golden/plane (9) | crates/generators/tests/parity.rs | 9/9 exact |
| Cube primitive | docs/specs/PRIMITIVES.md §2 | tests/golden/cube (6) | crates/generators/tests/parity.rs | 6/6 exact (incl. fillet, PRIMITIVES.md §2.1); separate surfaces not observed |
| Sphere primitive | docs/specs/PRIMITIVES.md §3 | tests/golden/sphere (15) | crates/generators/tests/parity.rs | 15/15 exact (standard, hemisphere, tetra/hexa/octa/icosahedron: PRIMITIVES.md §3.1); frequency rule seen at segments 8 and 24 only |
| Smoothing (tag, 40° default, corner normals) | docs/specs/SMOOTHING.md §1–3 | tests/golden/smoothing (5) | crates/geometry/tests/smoothing_parity.rs | 5/5 exact (normals within 1e-5); weighting still parity-pending (cases are symmetric) |
| Viewport handedness (left-handed display) | docs/specs/SMOOTHING.md §4 | docs/parity/viewport.md (screen directions) | crates/viewport/tests/camera.rs | Passing; exact default camera pose not shown by the reference app |
| Rotate gizmo, object/world gizmo axes, coordinate panel, viewport picking | docs/specs/MANIPULATE.md | derived from tests/golden/transform conventions | crates/core/tests/hpb.rs, crates/viewport/tests/gizmo.rs, crates/ui/tests/manipulate.rs | Passing locally; gizmo look, sizes and keys are ours (parity-pending); "Object (Abs)" and "Size+" not implemented |
| Subdivision surface (Catmull–Clark, creases, reference vertex/face order) | docs/specs/SUBDIVISION.md | tests/golden/subdivision (11) | crates/generators/tests/subdivision_parity.rs | 10/11 exact (cube levels 1–3, cylinder, plane, pyramid, catmull_clark, adaptive, OpenSubdiv Catmull-Clark and bilinear: SUBDIVISION.md §3.1); Loop unsupported |
| Boolean generator (BSP split, winding-number classification, coplanar merge) + geometric comparison | docs/specs/BOOLE.md, docs/specs/INFRA.md §4.2 | tests/golden/boole (6), `compare = "geometric"` | crates/generators/tests/boole_parity.rs, crates/geometry/tests/boolean.rs, crates/parity-harness/tests/geometric.rs | 5/6 match geometrically (mode-union, mode-a_minus_b, mode-intersect, mode-a_without_b, hide-edges: same points, area per oriented plane, closedness, volume; face counts also equal); quality-off (High Quality off) unsupported. Reference numbering/quad split not reproduced (geometric parity only). Local run 2026-10-17: fmt, clippy -D warnings, FORGE_REQUIRE_GPU=1 cargo test --workspace (233 passed), cleanroom ok, golden check ok (61 cases) |
| Cylinder primitive | – | tests/golden/cylinder (10) | – | Observed (docs/parity/cylinder.md), not implemented |
| Cone primitive | – | tests/golden/cone (9) | – | Observed (docs/parity/cone.md), not implemented |
| Disc primitive | – | tests/golden/disc (6) | – | Observed (docs/parity/disc.md), not implemented |
| Torus primitive | – | tests/golden/torus (5) | – | Observed (docs/parity/torus.md), not implemented |
| Tube primitive | – | tests/golden/tube (6) | – | Observed (docs/parity/tube.md), not implemented |
| Capsule primitive | – | tests/golden/capsule (5) | – | Observed (docs/parity/capsule.md), not implemented |
| Pyramid primitive | – | tests/golden/pyramid (4) | – | Observed (docs/parity/pyramid.md), not implemented |
