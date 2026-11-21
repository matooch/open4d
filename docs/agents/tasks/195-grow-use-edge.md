# 195 · Grow Selection: Use Edge option for polygons

Status: ready · Area: B · Depends on: 023 · Observations: O26 (`grow-poly`, `grow-point`) · Touches: geometry, command (and tests)

## Goal
O26 shows that with Grow Selection's **Use Edge** on, polygons grow only across shared **edges**:
front-face polygon 5 of a 4×4×4 cube grows to 1, 4, 5, 6, 9 (`tests/golden/selection-ops/grow-poly`).
Our `select.grow` grows across shared **points** (9 polygons), so the case is pinned in
`crates/app/tests/selection_ops_parity.rs` (KNOWN_MISMATCH). EDITABLE.md §2.5 already records the
observed rule.

## Inputs
- `docs/specs/EDITABLE.md` §2.5 (the observed rule), `docs/specs/GEOMETRY.md` §4 (grow/shrink)
- `crates/geometry/src/selection.rs` (`grow_polys`, `poly_neighbors`)
- the `select.grow` command in `crates/command/src/builtin/object.rs`
- `crates/app/tests/selection_ops_parity.rs`

## Design decisions (planner)
- Add an optional `use_edge` argument to `select.grow` (and `select.shrink`, same meaning), default
  **true** for polygons, because that is the reference's default for the observed case. Write the
  default into EDITABLE.md §2.5 and mark the default value parity-pending (only one case observed).
- For points, Use Edge means neighbours along edges: that is what `grow_points` already does
  (`grow-point` matches); leave it unchanged.
- Add an edge-adjacency neighbour function next to `poly_neighbors` (polygons sharing an edge), with a
  unit test on a small grid.
- The parity test passes `use_edge` from the case's `use_edge` key. Remove `grow-poly` from
  KNOWN_MISMATCH once it matches.
- Regenerate the command reference (`cargo run -p forge-cli -- docs --out docs/manual/reference`).

## Done when
- [ ] `grow-poly` and `grow-point` both match; KNOWN_MISMATCH in that test is empty again.
- [ ] Unit tests for edge-adjacent polygon grow and shrink.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
