# Parity status
| Feature | Spec | Golden | Test | Notes |
|---|---|---|---|---|
| Phase 0: clean-room gate | docs/specs/INFRA.md §2 | n/a (synthetic) | tools/cleanroom/tests | Passing; CI green (run 39030302799) |
| Phase 0: golden layout + harness | docs/specs/INFRA.md §3–4 | crates/parity-harness/tests/fixtures (synthetic) | crates/parity-harness/tests | Passing; CI green (run 39030302799); no reference golden cases yet |
| Phase 1: core (params, registry, scene, undo, dirty/cache, file) | docs/specs/CORE.md | n/a (internal architecture) | crates/core/tests | Passing; CI green (run 39031237650). Transforms deferred until docs/parity/transform.md exists |
| Phase 2a: geometry (mesh, half-edge, normals, selections, splines) | docs/specs/GEOMETRY.md | n/a (published literature) | crates/geometry/tests | Passing locally; CI pending. Parity-pending defaults listed in spec. Phase 2 stays open: subdivision, booleans, transforms await observations |
