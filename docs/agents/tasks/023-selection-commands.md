# 023 · Selection commands: grow, shrink, convert, invert, connected, loop, ring

Status: done · Area: B · Depends on: 011 · Observations: O26, G3 · Touches: geometry, ui

## Goal
The selection commands match the six observed O26 cases. Loop, ring and grow follow our own design until G3 is answered.

## Inputs
- Area design: [../../roadmap/areas/B-modeling.md](../../roadmap/areas/B-modeling.md)
- O26: [../../parity/selection-ops.md](../../parity/selection-ops.md) and its golden cases
- GUI session G3 ([../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md)), own design until it is answered: mark those rules **parity-pending**
- tests/golden/selection-ops/ (each case has `selection.json`)
- crates/geometry/src/selection.rs (grow/shrink already exist from brief 05)

## Design decisions (planner)
- Commands `select.grow|shrink|invert|connected|convert {to, mode: tolerant|strict}|loop|ring`.
- Parity test: rebuild the start selection from `case.toml`, run the command, compare with `selection.json` exactly.
- Convert rules from O26: points → polygons strict (all corners), with tolerant as an option. A polygon needs all four of its edges (convert-edge-to-poly).
- Loop and ring use edge loops and rings on the half-edge mesh (quads: across the opposite edge; stop at poles and N-gons). Own design, parity-pending.

## Done when
- [x] EDITABLE.md §2.8 (selection commands; §4 was taken by animation tracks).
- [x] All 6 O26 cases match.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: parity 6/6 exact (level A, selection indices), `UNSUPPORTED` and `KNOWN_MISMATCH` empty.
  Tests added: `crates/app/tests/selection_ops_parity.rs` (the 6 O26 cases through the real
  `select.*` commands on a polygon object holding the golden mesh, plus a check that our
  editable cube numbering equals the golden meshes), `crates/app/tests/select_commands.rs`
  (3: one undo step, no step when unchanged, argument and mode checks),
  `crates/geometry/tests/selection_ops.rs` (7: tolerant convert, connected, edge grow/shrink,
  loop, ring, strips, closing).
- Commands: new `select.connected`, `select.convert {to, mode: strict|tolerant}`,
  `select.loop {across?}`, `select.ring {across?}`; `select.invert|grow|shrink` kept (edge
  grow/shrink moved to `forge_geometry::selection_ops`). Select menu entries added. Trait
  method `CommandContext::select_extra` (default: error) carries the new ones.
- Rules and proving cases: shrink-poly (shrink = keep polys whose edge-neighbours are all
  selected: 16 front polys give 5, 6, 9, 10); invert-poly (complement of the current mode's set
  only); convert-poly-to-point (all corner points, source stays selected);
  convert-point-to-poly (strict = all corners; side polys with 2 points not chosen);
  convert-edge-to-poly (all edges of a polygon needed; 8 border edges give none);
  select-connected (poly 16 alone once Disconnect gave it its own points).
  Parity-pending (own design, no observation): tolerant convert, grow (poly: shared edge;
  point: along edges; edge: shared point), connected through shared points, edge loop, edge
  ring, polygon loop and ring (strips, `across`), convert switching the edit mode to the target.
- Deviations: spec section is EDITABLE.md §2.8 (§4 already exists: animation tracks). Convert
  switches the edit mode to the target kind (not observed). Polygon ring is the strip across
  the other edge pair (no observation of what Ring means for polygons).
- Files: crates/geometry/src/{selection_ops.rs,lib.rs}, crates/app/src/{select.rs,lib.rs},
  crates/command/src/{state.rs,context.rs,lib.rs,builtin/object.rs}, crates/ui/src/{app.rs,
  commands.rs}, tests above, crates/app/Cargo.toml (dev-dep `toml`), docs/specs/{EDITABLE,
  COMMANDS,GEOMETRY}.md, docs/manual/reference/*, STATUS.md, BACKLOG.md.
- Open data that would settle pending rules (for follow-up forms): (1) grow-poly and
  grow-point output (Grow Selection dialog, settings shown; start: one middle front polygon
  and one middle front point of a 4/4/4 cube; read back sets); (2) loop-edge, loop-poly,
  ring-edge from the GUI (G3 item 4: the steps, then the resulting indices on the 4/4/4 cube,
  incl. what a polygon loop's direction depends on and where loops stop at poles); (3)
  convert tolerant on (points to polys, edges to polys) with the 2/2/2 starts of the observed
  cases; (4) select-connected with Disconnect + Preserve Groups, and a case where polygons
  share only a point (does Select Connected cross it?) and a shared edge with separate points;
  (5) whether Convert changes the edit mode (G3 item 5 asks the related mode-switch question).
- Gate (last lines): `GATE: 639 tests passed, 0 failed`; `cleanroom: ok`; `ok: 471 cases`; `GATE: all passed`.
