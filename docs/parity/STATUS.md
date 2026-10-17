# Parity status
| Feature | Spec | Golden | Test | Notes |
|---|---|---|---|---|
| Phase 0: clean-room gate | docs/specs/INFRA.md §2 | n/a (synthetic) | tools/cleanroom/tests | Passing; CI green (run 39030302799) |
| Phase 0: golden layout + harness | docs/specs/INFRA.md §3–4 | crates/parity-harness/tests/fixtures (synthetic) | crates/parity-harness/tests | Passing; CI green (run 39030302799); no reference golden cases yet |
| Phase 1: core (params, registry, scene, undo, dirty/cache, file) | docs/specs/CORE.md | n/a (internal architecture) | crates/core/tests | Passing; CI green (run 39031237650). Transforms deferred until docs/parity/transform.md exists |
| Phase 2a: geometry (mesh, half-edge, normals, selections, splines) | docs/specs/GEOMETRY.md | n/a (published literature) | crates/geometry/tests | Passing; CI green (run 39033285128). Parity-pending defaults listed in spec. Phase 2 stays open: subdivision, booleans, transforms await observations |
| Phase 3a: viewport (camera, picking, snapping, grid, translate/scale gizmo, wgpu renderer) + n-gon triangulation | docs/specs/VIEWPORT.md, GEOMETRY.md §6 | n/a (published literature) | crates/viewport/tests, crates/geometry/tests/triangulate.rs | Passing; CI green incl. GPU tests on lavapipe (run 39035547589). Phase 3 stays open: rotate gizmo and object-local axes await docs/parity/transform.md |
| Phase 4a: editor shell (dock, object tree, attributes from descriptors, commands, timeline placeholder, theme, embedded viewport) | docs/specs/UI.md | n/a (own design) | crates/ui/tests (incl. egui_kittest UI tests) | Passing locally; CI pending. Phase 4 stays open: coordinate manager (needs transforms), material manager (Phase 9), saved layouts |
