# 065 · Clone data, cloner node, Linear / Radial / Grid / Honeycomb modes, instance modes

Status: done · Area: E · Depends on: 050 · Observations: O8, O11, O21 · Touches: clone-engine, plugin

## Goal
A cloner node whose Linear, Radial, Grid Array and Honeycomb modes produce the same clone matrices as the reference, plus the instance modes. All matching O8/O11/O21 clone samples pass at level B; the expanded mesh goldens match.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O8: [../../parity/cloner.md](../../parity/cloner.md) and its golden cases
- O11: [../../parity/followups-1.md](../../parity/followups-1.md) and its golden cases
- O21: [../../parity/cloner-3.md](../../parity/cloner-3.md) and its golden cases
- docs/parity/cloner.md, cloner-3.md (parameter tables, samples)
- docs/specs/MOGRAPH.md (existing spec; extend it)
- crates/clone-engine/src/lib.rs (current stub)
- crates/parity-harness/src/samples.rs (kind `clones`)
- tests/golden/cloner/

## Design decisions (planner)
- `CloneData` as in the area design (struct of arrays: matrices, colors, weights, visible, uvw, index, time). Effectors (068) mutate it later; keep it `Clone` and cheap to copy.
- `Cloner` is a generator node (`Category::Generator`) registered through `forge-plugin`; its children are the prototypes. Evaluation: evaluate children once, then produce `CloneData`, then expand to geometry for export/Make Editable in clone order (the goldens show the order).
- Modes: Linear (count, offset, mode per step / end point, step rotation/scale, cumulative), Radial (count, radius, plane, start/end angle, offset, offset variation with seed), Grid (count xyz, size per step / endpoint, forms cubic/sphere/cylinder, fill), Honeycomb (width/height count, size, orientation, offset direction). Take the exact formulas from the samples; record each in MOGRAPH.md with the case that proves it.
- Instance modes: Instance, Render Instance, Multi-Instance only change how the viewport/renderer consumes clones; geometry export is the same. Store the mode; the viewport hook is 079.
- Add `samples::clones` comparison support for any new fields you need, without loosening tolerances.

## Done when
- [x] MOGRAPH.md updated with every rule and its case.
- [x] All O8/O11/O21 linear/radial/grid/honeycomb samples pass, or are listed in `KNOWN_MISMATCH` with a reason.
- [x] Cloner registered as a node; undo, serialization and attribute UI derive from its descriptors.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Object mode (066), clone order with several children (067), effectors (068+).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: parity **19/19** at level B (clone data) and level A (expanded mesh): linear 5, radial 5, grid 6, honeycomb 3; `UNSUPPORTED` and `KNOWN_MISMATCH` empty. Tests added: `crates/generators/tests/cloner_parity.rs` (2), `crates/generators/tests/cloner.rs` (9), `crates/clone-engine/tests/modes.rs` (16), `crates/app/tests/cloner.rs` (5), one Rhai test in `crates/script/tests/script.rs`. Not run (pinned by count): 35 effector cases (068+), 8 Object-mode cases (066), `order-*` (067); `nested` matches in a separate rebuild test.
- Rules derived (MOGRAPH.md C4-C7), with the proving case: Linear per step is linear in the index, scale `1 + i(S-1)` (`linear-scale-step`: 1, .9, .8 ...), Cumulative affects only the rotation, `i(i+1)/2` (`linear-cumulative`), Step Rotation H is `Ry(-H)` (`linear-count-step`), End Point puts P at the last clone (`linear-mode-endpoint`), the Transform tab P is added to every position (`linear-count-step`); Radial angle `start + offset + i(end-start)/count`, also for partial arcs (`radial-plane-start-end`), frames per plane (`radial-defaults`, `radial-plane-start-end`, `radial-xy`), Align off keeps the identity frame (`radial-align-off`), offset (`radial-offset`); Grid x fastest, centred, Per-Step `size`, Endpoint `size/(n-1)` (`grid-3x2x4`, `grid-endpoint`), Sphere 33 and Cylinder (axis Y) 65 visible by normalised radius `<= 1`, Fill hides `r < 1 - fill` (`grid-fill-50`: 32); Honeycomb staggered lattice, `ceil(N/2)` / `floor(N/2)` clones per line, odd lines shifted by `offset`, centred on the half-cell lattice (`honeycomb-defaults` 50, `honeycomb-small` 3, `honeycomb-offset` width direction); mesh expansion is visible clones in list order, each with the prototype's own vertex/face order, prototypes consumed (all meshes). Parity-pending (seen in one configuration or not at all): see MOGRAPH.md C3.1, C4, C5, C6, C7.
- Deviations from the design decisions: (1) the node type, its `Plugin` (`CloneToolset`) and the pure math live in `forge-clone-engine`, installed by `forge_app::default_registry`; the `forge-plugin` crate is an empty stub and the `Plugin` trait lives in `forge-core`, so nothing was registered there. (2) Geometry expansion lives in `forge-generators` (`cloner.rs`, like the other generators with child inputs) because the generators crate would otherwise form a cycle with the clone engine; the engine has no scene access. (3) `CloneData` has one extra array, `child` (prototype per clone), for brief 067. (4) No change to `samples::clones` was needed. (5) `mode = object` is selectable and returns `Unsupported` until 066; Honeycomb Circle/Spline and Grid Object forms are `Unsupported`.
- Files changed: crates/clone-engine/{Cargo.toml,src/*,tests/modes.rs}, crates/generators/{Cargo.toml,src/{lib,cloner}.rs,tests/{cloner,cloner_parity}.rs}, crates/app/{Cargo.toml,src/lib.rs,tests/cloner.rs}, crates/script/tests/script.rs, Cargo.lock, docs/specs/MOGRAPH.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open data that would settle the pending rules (one small case each, details in MOGRAPH.md C11): Transform-tab R / S / colour / weight and P on a rotated clone; Linear Offset 1 and 1.5, Amount 50 %, Step Size 50 %, R (per step) alone and with Step Rotation, End Point with S and R set; Radial Offset Variation (two seeds, statistical); Grid Cylinder with Fill 50 %, Sphere with unequal sizes or counts; Honeycomb orientations X and Y, End Point, both variations (two seeds), odd/even counts in both directions, Circle and Spline forms; Reset Coordinates off with a displaced child; the per-clone UVW.
- Gate: see the commit message.
