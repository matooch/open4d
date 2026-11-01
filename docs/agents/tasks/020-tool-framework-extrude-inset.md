# 020 · Tool framework + Extrude and Extrude Inner

Status: ready · Area: B · Depends on: 011 · Observations: O6, O11, O25 · Touches: modeling (new), ui, geometry

## Goal
Modeling tools exist as a framework, with settings shown in the Attribute panel, a live preview, and an apply step that is a command. Extrude and Extrude Inner match the reference cases.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O6: [../../parity/modeling-tools.md](../../parity/modeling-tools.md) and its golden cases
- O11: [../../parity/followups-1.md](../../parity/followups-1.md) and its golden cases
- O25: [../../parity/modeling-tools-2.md](../../parity/modeling-tools-2.md) and its golden cases
- tests/golden/modeling-tools/ (O6, O11 §4, O25 cases) and the settings tables in docs/parity/modeling-tools.md and modeling-tools-2.md
- docs/specs/EDITABLE.md §2–3 (component modes, selection storage)

## Design decisions (planner)
- New crate `crates/modeling` (`forge-modeling`): pure functions `extrude(mesh, polys, &ExtrudeSettings) -> (PolyMesh, Selection)` and `extrude_inner(...)`. The settings structs mirror the observed tables (offset, variation, subdivision, create N-gons, preserve groups, create caps, maximum angle, bevel …).
- Tool framework in `forge-ui` (`tools.rs`): `trait Tool { fn id(); fn settings() -> Vec<ParamDesc>; fn preview(&self, mesh, sel, settings) -> PolyMesh; fn command_id() }`. While a tool is active, the Attribute panel shows its settings (O25 shows the reference does this). Reset Values restores the observed defaults. An "Apply" button and an interactive drag in the viewport (drag = offset) both end in one `mesh.extrude` command.
- Parity: rebuild each case from `case.toml` (cube, segments, `select_polys`, settings) in a parity test `crates/modeling/tests/tools_parity.rs`, then compare with `compare_meshes` after `export_weld`, as the generator tests do. Derive the new-point numbering and face order from the goldens. If the order can't be derived in three hypotheses, compare geometrically and pin a `KNOWN_MISMATCH` "numbering".
- Commands `mesh.extrude` and `mesh.extrude_inner`: args = settings + selection override, so agents can model by script.

## Done when
- [ ] Spec `docs/specs/MODELING.md` §0 (framework) and §1–2.
- [ ] Every extrude/inset case in O6/O11/O25 matches, or is pinned with a reason.
- [ ] A UI test: activate Extrude, drag, one undo step.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Bevel (021) and the other commands (022).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: tests added; parity N/M (`UNSUPPORTED` / `KNOWN_MISMATCH` with reasons).
- Rules derived, with the case that proves each; parity-pending rules.
- Deviations from the design decisions, and why.
- Files changed.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
