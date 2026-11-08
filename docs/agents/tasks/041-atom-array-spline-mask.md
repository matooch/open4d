# 041 · Atom Array and Spline Mask

Status: done · Area: C · Depends on: 029 · Observations: O20 · Touches: generators

## Goal
Atom Array (spheres at points, cylinders on edges) and Spline Mask (spline booleans as a generator) match O20.

## Inputs
- Area design: [../../roadmap/areas/C-objects.md](../../roadmap/areas/C-objects.md)
- O20: [../../parity/generators-3.md](../../parity/generators-3.md) and its golden cases
- docs/parity/generators-3.md

## Design decisions (planner)
- Atom Array: sphere and cylinder sub-meshes built with the existing primitive builders, merged in the reference order (derive it from atom-array/cube).
- Spline Mask: wraps the 029 boolean functions, with mode as a param.

## Done when
- [x] GENERATORS3.md.
- [x] O20 atom-array and spline-mask cases match.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/generators/tests/generators3_parity.rs` added; parity atom-array 2/2, spline-mask 2/2 exact; `UNSUPPORTED` and `KNOWN_MISMATCH` empty. Unit tests in `atom_array.rs`.
- Rules (GENERATORS3.md): Atom Array = spheres (Sphere primitive, segments = Subdivisions) at the points, then cylinders (n = Subdivisions sides, no caps) on the unique edges in first-appearance order, ring frame `u = unit(X x d)` (Y if d is along X), `w = u x d` (`atom-array/cube` proves all six axis directions; `atom-array/radii` the counts for 6). Spline Mask = boolean of the **sampled outline polygons** (not the Bezier curves of SPLINES.md 8.3: `spline-mask/union` 48 nodes, `subtract` 38) with the 8.3 loop rules, output as straight pieces `[a, a, b, b]` sampled Uniform 8 (`nodes * 9` points; the `3t^2 - 2t^3` chord parametrisation removes a 1e-2 error seen with thirds handles).
- Parity-pending: oblique-edge frame and the axis-parallel test of the Atom Array, odd Subdivisions, Single Elements (Unsupported); mask modes And, Or, Intersection, B subtract A, Axis other than XY and Create Cap (Unsupported), non-circle inputs. Follow-up case suggestions are in the STATUS rows.
- Deviations: Spline Mask is not a wrapper of the 8.3 Bezier result as the design decision assumed; it reuses `spline_tools::boolean` on Linear polygon operands. No new commands (nodes are created with `object.create`); icons `atom_array` and `spline_mask` added.
- Files: docs/specs/GENERATORS3.md, crates/generators/src/{atom_array,spline_mask,lib,modeling,splines}.rs, crates/generators/tests/generators3_parity.rs, crates/ui/src/icon_table.rs, crates/ui/icons/{atom_array,spline_mask}.svg, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief, regenerated reference docs if the gate changed them.
