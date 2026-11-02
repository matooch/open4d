# 058 · Field types II: Cylinder, Cone, Torus, Capsule, Radial, Random

Status: done · Area: D · Depends on: 057, 063 · Observations: O13, O23 · Touches: fields, noise

## Goal
The O13/O23 field types match. Random is compared statistically unless the noise matches.

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O13: [../../parity/fields-2.md](../../parity/fields-2.md) and its golden cases
- O23: [../../parity/fields-3.md](../../parity/fields-3.md) and its golden cases
- docs/parity/fields-2.md, fields-3.md

## Design decisions (planner)
- One file per shape. Radial: angles, transitions, iterations, axis. Random: modes random, sorted and noise, through `forge-noise`.

## Done when
- [x] FIELDS.md extended; cases match, or are pinned.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: new node types `forge.field.cylinder`, `.cone`, `.torus`, `.capsule`, `.radial`, `.random` (one source file each in `crates/fields/src/`, registered by `FieldToolset`, so `forge_app::default_registry` has them); `FieldList::sample_many` now samples whole batches per layer (needed by Sorted). Parameters: Cylinder height, radius, direction; Cone height, radius, direction; Torus radius, thickness, direction (X+/Y+/Z+); Capsule top and bottom radius, height, direction; Radial start/end angle and transition, iterations, offset, axis; Random mode, seed, noise type (31), space, scale, relative scale, animation speed, and stored-only randomized sample positions, sample position seed, color shift, color alpha; all with Clip to Shape (not Random), Color, Layer Strength and the Remapping tab. Tests: `fields_parity.rs` (36 exact cases, 27 old + 9 new; the Random statistical test), `shapes.rs` (12 behaviour tests: directions, torus axes, capsule radii, clip, radial axes and transitions, random determinism, spaces, sorted, animation, unsupported types), `crates/app/tests/fields.rs` (+1, 6 total). Parity: 9/9 exact (cylinder-defaults, cylinder-size, cone-defaults, torus-defaults, capsule-defaults, radial-defaults, radial-half, radial-transition, radial-iterations) at tolerance 1e-5; 4/4 Random cases statistical per NOISE.md N7 (random-defaults, random-seed, random-global, random-random-mode); `KNOWN_MISMATCH` and `UNSUPPORTED` empty. The 17 remaining fields cases (blend, layers, group, formula, shader) are 059.
- Rules derived (FIELDS.md F6): Cylinder `d = max(r/radius, |a|/height)` (`cylinder-defaults`, `cylinder-size`); Cone: base at `-h/2`, apex at `+h/2`, radius `radius * clamp((h/2 - a)/h, 0, 1)`, `d = max(r/R(a), |a|/h)` (`cone-defaults`); Torus `d = hypot(r - R, a)/thickness` (`torus-defaults`, 4/7 at distance 25); Capsule: distance to the segment of length `height`, divided by the radius interpolated bottom to top (`capsule-defaults`); Radial: right-handed angle about the axis (`atan2(-z, x)` for Y), half-open range `[0, end - start)`, a full circle is 1 everywhere, transitions lie inside the range and ramp linearly, e.g. `(180 - theta)/45` (`radial-half`, `radial-transition`, `radial-defaults`, `radial-iterations`); Random: unit Perlin through forge-noise at 100 units x Scale with a seed-derived lattice offset (the reference value at the origin is not 0.5), Random mode a hash of seed and position (uniform), Global = Field when the field is at the origin unrotated (`random-global` equals `random-defaults` exactly in the reference and in ours). Parity-pending (no case tells apart): Cone extent and taper (one case; the fit is the zero pattern, the 0.5 plateau at the base and at the apex), Clip to Shape on the new shapes, unequal Capsule radii and Y-, Torus metric, Radial start transition, offset sign, iterations on a partial range, X / Z axes, overlapping transitions, Random feature size / octaves / seed hashing, Space None (constant), Sorted (ascending in sample order), Animation Speed, 16 noise types with no public definition (Unsupported in Noise mode).
- Deviations: none from the design decisions. Random with a noise type without a definition returns `Unsupported` only in Noise mode. The viewplane and display-box parameters of O13 are not parameters (editor previews).
- Files changed: crates/fields (Cargo.toml, src/{lib,node,shape,capsule,cone,cylinder,radial,random,torus}.rs, tests/{fields_parity,shapes}.rs), crates/app/tests/fields.rs, Cargo.lock, docs/specs/FIELDS.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, docs/roadmap/areas/D-deformers-fields.md, this brief.
- Open data (settles the pending rules; tabulated in FIELDS.md F6): Cone with another Height and Radius, spacing 25, and Y-; each new shape with Clip to Shape and Invert on; Capsule with Top 100 / Bottom 50; Radial with Start Angle 45, Start Transition 45, End Angle 270, Offset 30, Iterations 2, axes X and Z; Random Perlin sampled on a 1-unit line for two seeds and Scale values; Space None, Sorted on a cloner, Animation Speed 100 % at two times; one line per undefined noise type.
- Gate (last lines): `GATE: 1061 tests passed, 0 failed`; `cleanroom: ok`; `ok: 471 cases`; `GATE: all passed`.
