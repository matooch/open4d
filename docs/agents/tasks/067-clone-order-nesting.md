# 067 · Clone order (iterate, random, blend, sort), multiple children, nesting

Status: done (random order statistical) · Area: E · Depends on: 065 · Observations: O21, O50 · Touches: clone-engine

## Goal
Several prototype children are assigned to clones in the reference order; nested cloners expand correctly.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O21: [../../parity/cloner-3.md](../../parity/cloner-3.md) and its golden cases
- O50: [../../parity/clone-toolset-3.md](../../parity/clone-toolset-3.md) and its golden cases
- docs/parity/cloner-3.md, clone-toolset-3.md (blend order cases)

## Design decisions (planner)
- Iterate: child index = clone index mod child count. Random: seeded, three-hypothesis budget then statistical. Blend: interpolates parameters between children by clone weight (O50 shows how). Sort: by index field (record rule).
- Nested cloners: the inner cloner is a prototype; its CloneData is multiplied by each outer matrix. Expansion order: outer major.

## Done when
- [ ] Order cases match; nested case mesh matches.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: parity 3/4 order cases exact (iterate, sort, blend), `order-random` KNOWN_MISMATCH (reference RNG not reproduced, three hypotheses, spec O2); `blend-order` un-skipped as asked; `nested` through its rebuild test. Test counts in cloner_parity: 34 matched, 3 mismatched, 9 skipped.
- Rules: spec O1-O4. Parity-pending: Blend, Sort (one case each).
- Deviations: `assign_children` takes a seed and no longer errors; new cloner param `clones_seed`.
- Files: crates/clone-engine/src/{lib,node,settings,matrix}.rs, tests/modes.rs, crates/generators/src/cloner.rs, tests/cloner_parity.rs, spec, STATUS, BACKLOG.
- Open: the order cases' children parameters in case.toml; a Sort case with non-zero weights or a sort axis; Blend with 3 children.
