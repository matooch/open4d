# 020 · Tool framework + Extrude and Extrude Inner

Status: done · Area: B · Depends on: 011 · Observations: O6, O11, O25 · Touches: modeling (new), ui, geometry

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
- Result: tests added: `crates/modeling/tests/tools_parity.rs` (11 golden cases), `crates/modeling/tests/tools.rs` (9: defaults, validation and clamping, unsupported options, bad selections, closed meshes stay closed, subdivision counts, Preserve Groups angle split, mean normal around a bend, rectangle inset, interior points under inset), `crates/app/tests/tools.rs` (4: one undo step with selection, selection default and node override, errors change nothing, schemas), `crates/ui/tests/tools.rs` (5: descriptors, drag preview then apply as one undo step, Reset Values, no-target errors, Mesh menu + attribute panel + Apply via egui_kittest), one Rhai test in `crates/script/tests/script.rs`. **Parity 11/11 exact** (`UNSUPPORTED` and `KNOWN_MISMATCH` both empty), compared on the raw file and after `export_weld`. The other 23 `modeling-tools` cases are Bevel (021) and the mesh commands (022).
- Rules derived (spec docs/specs/MODELING.md §1-2), with the proving case:
  - New points appended after the originals, ascending original point index per group (`extrude-top`, `extrude-two`); one group's points before the next group's (`extrude-two-separate`).
  - Selected polygons are replaced in place by their caps, sides appended after all polygons (`extrude-top`, `extrude-two`).
  - Side face per boundary edge a->b is `[a', a, b, b']`, edges in group polygon / corner order, shared edges give none (`extrude-top`, `extrude-two`); negative offset keeps the faces (`extrude-top-negative`).
  - Preserve Groups: edge-connected selected polygons form one group, off = one group per polygon (`extrude-two` vs `extrude-two-separate`).
  - Subdivision N: N column points per boundary-edge start point, top (cap side) first, columns numbered after the group's new points in edge order; rows `[L_j, L_j+1, R_j+1, R_j]`, inner rows rotated to start at `R_j+1` (`extrude-subdiv`; `inset-subdiv` has no inner row).
  - Inset: new points only for boundary points; mitre `p + offset (i1+i2)/(1+i1.i2)` (`inset-top`, `inset-two`, `inset-subdiv`).
  - No effect seen: `create_ngons` (`extrude-ngon`, `inset-ngon`), `bevel` at Subdivision 0 (`extrude-bevel`).
  - **Parity-pending** (one data point or none): the Maximum Angle test for groups and the mean-normal direction of non-flat groups (every golden group is flat), column order for groups with several boundary loops, inner-row rotation for N >= 3 (data: N = 2 only), interior group points under inset, `bevel` with Subdivision > 0, the drag scale.
  - `Unsupported` (no golden data): Offset Variation, Rotation, Create Caps.
- Deviations from the design decisions:
  - Tool settings are `ParamDesc` lists in `forge-modeling` (`ToolDesc`, `Settings`) instead of two hand-written structs, so the command schema, the attribute rows, defaults and range checks come from one definition; angles stay plain degrees under the observed key names (`maximum_angle_deg`).
  - Command arguments are `{node?, polys?, settings?}` (settings nested, like `[params.settings]` in the cases), not flat. `mesh.extrude` / `mesh.extrude_inner` are implemented in `forge-command` (it now depends on `forge-modeling` and `forge-geometry`).
  - The `Tool` trait lives in `forge-ui/src/tools.rs` as designed, but `preview` takes `(mesh, polys, settings)`; the viewport draws the preview through `Scene::with_params` on a scene copy, so the document is never touched. The command also writes the new polygon selection and clears the stale point and edge sets in the same undo step.
  - The brief's `inset` spelling in two case files is accepted as `extrude_inner`.
  - `subdivision` is capped at 1e6 (observed 1e20).
- Files changed: crates/modeling/** (new), crates/command/{Cargo.toml,src/builtin/mod.rs,src/builtin/tools.rs}, crates/ui/{Cargo.toml,src/{app,attributes,commands,lib,state,tools}.rs,tests/tools.rs}, crates/app/tests/tools.rs, crates/script/tests/script.rs, Cargo.lock, docs/specs/{MODELING,COMMANDS}.md, docs/manual/reference/{commands,mcp}.md (regenerated), docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open questions or data that would settle them:
  - Extrude of a non-flat group (a bent 2-polygon selection, and one around a convex corner) with Preserve Groups on, to settle the offset direction and the Maximum Angle test.
  - Extrude with Subdivision 3 or more, to settle the rotation of inner rows.
  - A group with a hole or two boundary loops with Subdivision 1 (column order), and an inset of a 3 x 3 patch (interior points).
  - Create N-gons on a bent group, Create Caps, Rotation and Offset Variation (seeded?).
  - A GUI observation (G3) of the interactive drag: offset per pixel and whether the tool keeps its values after a release.
- The last lines of the gate output: `GATE: 584 tests passed, 0 failed` / `cleanroom: ok` / `ok: 471 cases` / `GATE: all passed`
