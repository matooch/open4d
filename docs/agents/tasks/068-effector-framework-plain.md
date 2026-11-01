# 068 · Effector framework + Plain effector (modes, spaces, colour, weight, min/max)

Status: done · Area: E · Depends on: 065, 057 · Observations: O8, O14, O22 · Touches: clone-engine

## Goal
The effector trait, the effector list on a cloner (in list order) and the Plain effector with all common parameters, matching O8/O14/O22 samples.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O8: [../../parity/cloner.md](../../parity/cloner.md) and its golden cases
- O14: [../../parity/effectors-2.md](../../parity/effectors-2.md) and its golden cases
- O22: [../../parity/effectors-3.md](../../parity/effectors-3.md) and its golden cases
- docs/parity/cloner.md, effectors-2.md, effectors-3.md
- brief 057 (deformer/effector common falloff) for the strength path

## Design decisions (planner)
- `trait Effector { fn apply(&self, data: &mut CloneData, falloff: &FieldList, ctx: &EvalCtx); }`; effectors are `Category::Effector` nodes linked from the cloner list. Apply in list order (O14 two-effectors cases).
- Common parameters: strength; transform mode relative/absolute/remap; space node/effector/object; P/S/R with uniform/absolute scale; colour mode (off/on/field); weight transform; U/V; visibility; min/max remap.
- Rotation composition order and space conversion must come from the samples; write each rule down with its case.

## Done when
- [x] Plain effector cases from O8/O14/O22 pass at level B.
- [x] MOGRAPH.md §Effectors documents the common parameter rules.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Field falloff beyond the effector default (072).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: **14/14** Plain cases match (clone data level B 1e-4, expanded mesh level A strict): `linear-plain`, `plain-absolute`, `plain-color`, `plain-effector-space`, `plain-field-linear`, `plain-field-spherical`, `plain-remap`, `plain-rotation`, `plain-scale`, `plain-strength-50`, `plain-weight`, `two-effectors`, `two-effectors-swapped` through the case files; `linear-plain-field` through its own test because its `case.toml` has no field table (only the description). `UNSUPPORTED` / `KNOWN_MISMATCH` empty. The earlier cloner test and its pinned counts (19 matched, 35 effector cases, 12 skipped) are unchanged. Tests added: `plain_effector_cases_match_the_reference`, `linear_plain_field_matches_with_the_default_linear_field`, `the_effector_list_applies_in_list_order` (generators), `crates/clone-engine/tests/effectors.rs` (8), app and script tests that create and link an effector by commands.
- Rules (MOGRAPH.md E1-E10): the effector list is a link list applied in list order; `w = strength x field value` (`plain-strength-50`, `plain-field-*`); Relative adds P, S (1 + 2 = 3: `plain-scale`), Rot(R) (`plain-rotation`), weight (`plain-weight`); Absolute and Remap equal Relative for position (`plain-absolute`, `plain-remap`); the position delta is in the space axes without the clone's scale (`two-effectors-swapped`); Custom Color and Fields Color blend `c + (target - c) x value` (`plain-color`, `plain-field-linear`, `plain-field-spherical`); the field's colour channel is its Color. Parity-pending (own design, no sample tells): Node / Object / Effector space with a rotated clone, Absolute and Remap for rotation/scale/weight/UV and Minimum/Maximum, Absolute Scale, colour at strength < 100 % and Use Alpha/Strength, U/V, Visibility threshold (w < 0.5 hides), Modify Clone (stored only), the per-effector strength on the cloner. Unsupported (returns an error): Effector Color, Blending Mode other than Default, Deformation other than Off.
- Deviations: (1) `Effector::apply` takes `&EffectorCtx` (eval ctx + cloner world matrix) instead of `&EvalCtx`, and returns `Result`, because fields are sampled at world positions and options can be unsupported. (2) New core value kind `Value::LinkList` / `ParamKind::LinkList` / `ParamDesc::link_list` (ordered, unique, category-checked, `param.set` takes an array of ids), the linking model for the cloner's `effectors` and an effector's `fields`. The scene file version stays 5 (bumping would change existing version assertions); an older build reads a file with a link list as a parse error. (3) Fields gained a constant `color` parameter and `FieldSample.color` returns it (FIELDS.md F2). (4) Effectors do not use the cloner's per-effector strength field ("Plain 100 %").
- Files: `crates/core/src/{param,document,scene}.rs`, `crates/command/src/builtin/param.rs`, `crates/app/src/query.rs`, `crates/ui/src/attributes.rs`, `crates/fields/src/{lib,node}.rs`, `crates/clone-engine/src/{effector,plain,node,lib}.rs` + `Cargo.toml`, `crates/generators/src/cloner.rs` + `Cargo.toml`, tests in `crates/{clone-engine,generators,app,script}/tests`, `docs/specs/{MOGRAPH,FIELDS,CORE}.md`, `docs/parity/STATUS.md`, `docs/roadmap/BACKLOG.md`.
- Open data (MOGRAPH.md E10): rotated-clone cases for each space (Step Rotation H 90 degrees with P (50, 0, 0); effector rotated with P (50, 0, 0)); Absolute mode with R, S, weight; Remap with Minimum 20 % / Maximum 80 %; Custom Color at Strength 50 % with Use Alpha/Strength on and off; Blending Mode Add/Multiply; U/V with Display = UV; Visibility with a Linear field; Absolute Scale; two effectors whose order changes the result; re-record `linear-plain-field` with the field table and its Color; the cloner's per-effector strength at 50 %.
- Gate: see the commit (last lines below).
