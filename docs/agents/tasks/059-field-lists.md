# 059 · Field lists: layers, blend modes, groups, Formula and Shader fields

Status: done · Area: D · Depends on: 057, 052 · Observations: O13, O23 · Touches: fields

## Goal
Field lists with ordered layers, blend modes and strengths, plus Group, Formula and Shader fields, match O13/O23 (overlapping blends).

## Inputs
- Area design: [../../roadmap/areas/D-deformers-fields.md](../../roadmap/areas/D-deformers-fields.md)
- O13: [../../parity/fields-2.md](../../parity/fields-2.md) and its golden cases
- O23: [../../parity/fields-3.md](../../parity/fields-3.md) and its golden cases
- docs/parity/fields-2.md (a layer inserted later goes on top), fields-3.md (blend-* cases on overlapping spheres)

## Design decisions (planner)
- `FieldList { layers: Vec<Layer { field, blend, strength, enabled }> }`, evaluated bottom to top. Implement the 9 blend modes per the observed data. The Shader field uses `forge-shaders` (125); pin it until then.

## Planner note (after 061)
Brief 061 stored a list row's strength as a field-node param `layer_strength`, because a
link list had no per-row data. Give field-list rows their own storage (strength, blend mode,
enabled) as part of this brief, migrate `layer_strength` into it, and keep the O24 and
effector cases passing.

## Done when
- [x] FIELDS.md §lists (F7).
- [x] O23 blend cases match.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: tests added; parity 16/17 exact at level B (tolerance 1e-5, max error < 1e-12): blend modes 9/9, `blend-normal-50`, `three-layers`, `two-layers-add`, `two-layers-max` (layers 3/3 beyond the modes), `group-field` 1/1, `formula-defaults` and `formula-custom` 2/2; `shader-noise` 0/1 pinned `UNSUPPORTED` (the Shader field needs the shaders of brief 125). `KNOWN_MISMATCH` empty. Tests: `crates/fields/tests/fields_parity.rs` (52 exact cases of the fields feature), `crates/fields/tests/lists.rs` (12), `crates/app/tests/fields.rs` (+3), `crates/core/tests/native_format.rs` (v11 fixtures, v10 fixtures still load, field list migration, row round trip), `crates/ui/tests/clone_ui.rs` (+1, rows), a degrees test in `forge-expr`.
- Rules derived (FIELDS.md F7), each proved on the 729 samples: Normal `top` (`blend-normal`, `three-layers`); strength `acc + (blend - acc) s` (`blend-normal-50`); Max, Min, Add `clamp(a+b, 0, 1)`, Multiply, Screen `a+b-ab`; Subtract is `base - top` clamped (`blend-subtract`; the reverse fails 102 samples); Overlay branches on the base `base < 0.5` (`blend-overlay`; branching on the top fails 34); Clip equals Min (`blend-clip`; `a*b` fails 28). Group = nested row list, then its own Remapping (off by default) (`group-field`). Formula: forge-expr in degrees mode, `id`/`count` = sample index and batch size, `px` the position, result limited to 0..1 before Remapping (`formula-defaults`: `0.5 + 0.5 sin(i/729 * 360 deg)`; `formula-custom`: `px/100` zero for px <= 0 even with clamps off). Parity-pending: Clip as a separate mode, non-Normal modes at strength below 100 %, Add / Subtract limits, a non-Normal bottom layer, colour / direction blending, Group with remapping on or empty, Formula `d`, `subfields`, space of `px`, `t`, `f` (table in FIELDS.md F7).
- Deviations from the design decisions: (1) rows are `Value::LinkRows` with their own storage on the list owner, so the native format is now 11 (migration 10 -> 11 turns every `fields` link list into rows with the target's `layer_strength` and `object.enabled`, drops `layer_strength`); (2) `Layer` has no `enabled` field, off rows are left out when the list is built; (3) the per-row enable box is the row's `enabled` (a field node's `object.enabled` still removes the layer); (4) `forge-expr` got `Expr::parse_degrees` (EXPR.md E5) because the Formula field's default text only matches in degrees; (5) the Shader field node type is registered (plumbing) but building it is Unsupported.
- Files changed: crates/core (param.rs, document.rs, depgraph.rs, scene.rs, file.rs, lib.rs, tests/native_format.rs, fixtures scene-v11*.forge), fuzz/corpus/scene_file (v11 copies), crates/expr, crates/fields (blend.rs, group.rs, formula.rs, shader.rs, lib.rs, node.rs, random.rs, tests), crates/clone-engine (plain.rs, effector.rs), crates/modifiers (lib.rs, tests), crates/command (param.rs, browser.rs, arrange.rs), crates/app (query.rs, schedule.rs, tests), crates/ui (link_list.rs, attributes.rs, widgets/mod.rs, icon_table.rs, three icons, tests), docs/specs (FIELDS, CORE, EXPR, MODIFIERS, MOGRAPH, UI), docs/parity/STATUS.md, docs/roadmap/BACKLOG.md.
- Open questions or data that would settle them: the follow-up cases listed in FIELDS.md F7 (Clip with partial strength and values below the lower layer; Add / Multiply at 50 %; Subtract chains; a non-Normal bottom layer; two layers with different colours through a Plain effector; Group with remapping on, moved and empty; Formula `px/100` on a moved and rotated field, `d` and `subfields` as formulas, `t` at 0 s and 1 s; Shader field on a finer grid once 125 exists).
- Gate: see the commit message / final report.
