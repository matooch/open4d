# 026 · Soft selection and symmetry modeling

Status: done (parity-pending: no golden data; G3 item 6 unanswered) · Area: B · Depends on: 023 · Observations: G3 · Touches: ui, modeling

## Goal
Component transforms fall off smoothly around the selection, and edits mirror across a symmetry plane.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- GUI session G3 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- crates/ui/src/component_edit.rs (brief 06)

## Design decisions (planner)
- Soft selection: radius, falloff (linear, dome, bell, spline), surface vs volume distance. Weights feed `transform_points` as per-point blend factors.
- Symmetry: X, Y or Z plane in object space, with a tolerance. Each moved point moves its mirror partner.

## Done when
- [x] EDITABLE.md §5.
- [x] Tests: a move with radius 100 on a plane moves neighbours by the expected falloff fraction; symmetric moves stay symmetric.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: parity 0/0 (no golden data; G3 item 6 unanswered), all rules own design and **parity-pending**.
  Tests added: `crates/modeling/tests/soft.rs` (16: falloff values, spline curve and validation, an 11-point
  row moved with radius 100 gives exactly the falloff fraction for every falloff, nearest-selected distance,
  rotation/scale blending, surface vs volume distance across a gap and around an L, partners within the
  tolerance, one-sided and two-sided symmetric moves/rotations/scales stay symmetric, on-plane points stay on
  the plane, soft + symmetry across the plane, settings JSON errors change nothing),
  `crates/app/tests/soft_commands.rs` (8: settings commands, radius-100 move on a plane, editor settings vs
  `soft: false`, stored selection of the mode, symmetry on a cube, pivot rotate/scale, errors change nothing,
  schemas), `crates/ui/tests/component_edit.rs` (+1: a drag with symmetry and with soft selection, one undo step).
- Settings and commands: `edit.soft_selection {enabled, radius, falloff linear|dome|bell|spline, distance
  volume|surface, curve}` and `edit.symmetry {enabled, axis x|y|z, offset, tolerance}` (editor settings in
  `App::component_settings`, not undoable, not saved); `mesh.transform {node?, points?, move?, rotate{axis,angle}?,
  scale?, pivot?, soft?, symmetry?}` (one undo step; `soft`/`symmetry` take false, true or an object overriding the
  editor settings). `CommandContext` got `component_settings()`/`component_settings_mut()`. Gizmo drags
  (`ComponentDrag`) snapshot the settings and build a `ComponentPlan` at drag start; the header has Soft and Sym
  controls in component modes.
- Rules (EDITABLE.md §5): weight `w(t)`, `t = d/radius`: linear `1-t`, dome `sqrt(1-t^2)`, bell
  `0.5(1+cos(pi t))`, spline = Catmull-Rom through user samples; selected points `w = 1`; `p' = p + w (m p - p)`.
  Volume distance = Euclidean to the nearest selected point; surface = Dijkstra along edges. Symmetry plane
  `x[axis] = offset`, partner = nearest mirror image within the tolerance (ties lowest index); on-plane points
  average their displacement with its reflection; for a pair, the point with the larger weight (tie: lower index)
  drives the other with the reflected displacement.
- Deviations: symmetry applies to component transforms only. The brief names no modeling tool for it, so extrude,
  bevel and the others are not mirrored (Mirror/Center Axis stay the tools for that). `forge-fields` is an empty
  stub, so the falloff shapes are implemented in `forge-modeling` instead of reused. Soft selection is not shown as
  a heat map in the viewport, and there is no Attribute-panel page for it (a header bar only).
- Files: crates/modeling/src/{soft,lib}.rs, crates/command/src/{builtin/components,builtin/mod,context}.rs,
  crates/app/{Cargo.toml,src/lib.rs}, crates/ui/src/{component_edit,app,commands}.rs, tests
  crates/modeling/tests/soft.rs, crates/app/tests/soft_commands.rs, crates/ui/tests/component_edit.rs,
  crates/command/tests/registry.rs, docs/specs/{EDITABLE,COMMANDS}.md, docs/manual/reference/*,
  docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, Cargo.lock.
- Open data that would settle the pending rules (G3 item 6): where soft selection lives and its mode names; the
  falloff profile of each mode, sampled on a plane (move one point of a 21x21 plane by 100 with radius 100 and
  export, per mode: linear, dome, bell, spline default); the default radius and falloff; whether the distance is
  volume or surface by default and how surface treats separate pieces; whether the radius is the full falloff
  extent; how symmetry picks its partner (tolerance, plane choices incl. offset/world vs object space), what
  happens to on-plane points under a move off the plane, and which side drives when both sides are selected; and
  whether symmetry also mirrors the modeling tools (extrude, bevel, ...).
- Last gate lines: `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`.
