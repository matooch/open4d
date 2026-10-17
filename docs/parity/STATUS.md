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
| Cube primitive | docs/specs/PRIMITIVES.md §2 | tests/golden/cube (6) | crates/generators/tests/parity.rs | 4/6 exact; fillet (2 cases) and separate surfaces unsupported |
| Sphere primitive | docs/specs/PRIMITIVES.md §3 | tests/golden/sphere (15) | crates/generators/tests/parity.rs | 7/15 exact (standard, hemisphere); tetra/hexa/octa/icosahedron types (8 cases) unsupported |
| Smoothing (tag, 40° default, corner normals) | docs/specs/SMOOTHING.md §1–3 | tests/golden/smoothing (5) | crates/geometry/tests/smoothing_parity.rs | 5/5 exact (normals within 1e-5); weighting still parity-pending (cases are symmetric) |
| Viewport handedness (left-handed display) | docs/specs/SMOOTHING.md §4 | docs/parity/viewport.md (screen directions) | crates/viewport/tests/camera.rs | Passing; exact default camera pose not shown by the reference app |
| Rotate gizmo, object/world gizmo axes, coordinate panel, viewport picking | docs/specs/MANIPULATE.md | derived from tests/golden/transform conventions | crates/core/tests/hpb.rs, crates/viewport/tests/gizmo.rs, crates/ui/tests/manipulate.rs | Passing locally; gizmo look, sizes and keys are ours (parity-pending); "Object (Abs)" and "Size+" not implemented |
