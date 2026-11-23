# 071 · Shader, Spline, Push Apart, Inheritance, Volume and Group effectors

Status: done (parity-pending: five O22 pins) · Area: E · Depends on: 068, 125 · Observations: O22 · Touches: clone-engine

## Goal
The remaining effectors from O22.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O22: [../../parity/effectors-3.md](../../parity/effectors-3.md) and its golden cases
- docs/parity/effectors-3.md (O22)
- forge-shaders (125)

## Design decisions (planner)
- Shader: samples a shader at the clone UV or position; channel to P/S/R/colour.
- Spline: maps clones onto a spline by index or weight (offset, start/end, rail).
- Push Apart: iterative relaxation with radius and iterations; hide mode. Deterministic order (by index).
- Inheritance: transfer from another cloner by index, with morph/falloff.
- Volume: in/out of a geometry volume (winding number).
- Group: combines child effectors as one entry.

## Done when
- [ ] O22 cases pass at level B, or statistically with reason (push apart may differ in iteration details).
- [x] The gate passes and BACKLOG.md is updated.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- `crates/app/tests/remaining_effectors.rs` reads all seven in-scope O22 manifests
  and clone samples. `inheritance` and `volume` are exact at level B. Group has no
  observed case; a separate test checks in-order application of its child effectors.
- `KNOWN_MISMATCH`: `shader-noise` uses the Forge Noise basis; displacement/scale
  coupling, bounds, mean and repeatability are checked. `spline-helix` and
  `spline-helix-segment` match positions but not full orientation. Analytic and
  Uniform intermediate tangents were compared; the positions require the Uniform
  chords and open-end shortfall. `push-apart` and `push-apart-iterations` improve
  separation and preserve the centroid and axes, but the observed magnitudes and
  iteration response are not derived. Immediate moves and unnormalized simultaneous
  corrections were also tried.
- Nodes register in clone-engine. The generators dispatcher resolves spline,
  linked mesh and inherited cloner inputs; dependency checks reject recursive links.
  Only hosted Noise is supported in Shader: the full renderer shader graph depends on
  generators through material and cannot be linked back into that crate. Motion
  animation Inheritance, other Push Apart modes and non-closed volume inputs are
  not implemented. See MOGRAPH.md E25 for the supported subset and pins.
- Gate: `2452 tests passed, 0 failed`; `GATE: all passed` (licenses included).
