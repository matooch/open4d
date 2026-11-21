# 196 · OBJ texture coordinates (`vt`) import and export

Status: ready · Area: M · Depends on: 004, 128 · Observations: – (OBJ is a public format; UV layouts from O28 are already matched) · Touches: io (and tests)

## Goal
Brief 128 added per-corner UVs (`forge_geometry::CornerUvs`, beside the mesh) and all 10 primitives
match the reference's UVs. But OBJ import and export ignore `vt` (docs/specs/IO.md: "we have no UVs
yet, so the `vt` part is empty"). Read and write them.

## Inputs
- `docs/specs/IO.md` (OBJ section), `docs/specs/UV.md` (CornerUvs layout)
- `crates/io/src/` (the OBJ reader and writer), `crates/geometry/src/` (`CornerUvs`)
- the existing OBJ tests in `crates/io/tests/`

## Design decisions (planner)
- **Export:** when a mesh has CornerUvs, write one `vt u v` per distinct UV (keep first-seen order,
  deduplicated exactly) and faces as `v/vt` or `v/vt/vn`. Without UVs, the output must stay
  byte-identical to today's (the golden export tests depend on it).
- **Import:** read `vt` lines and the `vt` index of each face corner into CornerUvs. Faces without a
  `vt` index on some corners: no UVs for that mesh (all or nothing), and say so in IO.md.
- Round-trip test: export each primitive with UVs, import it, and compare the per-corner UVs exactly.
- Update IO.md first (replace the "no UVs yet" sentence with the rules above).

## Done when
- [x] Round trip of all 10 primitives keeps UVs exactly; the existing OBJ export tests are unchanged.
- [x] Import of a hand-written OBJ with `v/vt`, `v/vt/vn` and negative `vt` indices.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

Status: done

Added `uvs: Option<CornerUvs>` to `ObjObject`, updated the OBJ writer to emit `vt` lines
(deduplicated exactly, first-seen order) and faces as `v/vt/vn` (or `v/vt` without normals),
and the reader to parse `vt` lines and per-corner `vt` indices into `CornerUvs`. Objects
without UVs produce byte-identical output. `export_scene` reads UVs from `forge.uvw` tags;
`import_into` writes them back, skipping the tag silently only when the UVW tag type is not
registered (e.g. smoothing golden files with dummy `vt`). Added unit tests for UV round-trip,
primitive UV round-trip, hand-written `v/vt/vn` and negative `vt` indices, partial UVs, and
no-UV output. Updated IO.md and STATUS.md.

Co-Authored-By: kimi-k2.7-code <kimi-k2.7-code@nousresearch.com>
