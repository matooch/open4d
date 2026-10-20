# 01 · Deformer framework, Bend and Twist

Status: ready · Branch: `observations-7` · Planner: main session · Implementer: local model

## Goal
Deformers exist as node types in `crates/modifiers`, apply to scene geometry during
evaluation in tree order, and Bend and Twist match their golden cases.

## Inputs
- Observations: `docs/parity/bend.md`, `twist.md`, `deformer-common.md` (read the
  observer's fitted formulas and behaviour notes: they are hypotheses, verify them).
- Golden cases: `tests/golden/bend/` (10), `tests/golden/twist/` (7),
  `tests/golden/deformer-common/` (6; this task: `stack-*`, `sibling`, `disabled`,
  `on-plane`, `generator`). Every folder has `input.obj` (the undeformed cube).
- Patterns: `docs/specs/GENERATORS2.md`, `crates/generators/src/modeling.rs`,
  `crates/generators/tests/generators_parity.rs`.

## Design decisions (planner)
- **Crate:** `crates/modifiers` (package `forge-modifiers`, already a stub). Pure
  functions `fn bend(points: &mut [DVec3], p: &BendParams)` etc. operating in the
  deformer's local space; node types with `Category::Deformer`, ids `forge.bend`,
  `forge.twist` (later `forge.taper`, `forge.bulge`, `forge.shear`, `forge.spherify`);
  a `Deformers` plugin registering them. Common params: `size` (vec3, 250³), `mode` enum
  `limited (default), within_box, unlimited`; Bend: `strength` (angle, 0),
  `angle` (angle, 0), `keep_y_axis_length` (false); Twist: `angle` (angle, 0).
- **Evaluation** (in `forge-generators::evaluate`, which then depends on
  `forge-modifiers`): after a node's mesh is built, apply its **enabled** deformer
  children in child order. A deformer whose parent has no mesh (e.g. a Null) applies to
  its parent's mesh children (the `sibling` case). A deformer under a generator applies
  to the generator's output (`generator` case: verify whether before or after
  subdivision by comparing both). Points go into the deformer's space with the inverse
  of its local transform, are deformed, and come back. Check `forge_core::Node` for an
  enabled flag; if there is none, add a common bool param `enabled` (default true) and
  note it in the spec.
- **Spec:** new `docs/specs/MODIFIERS.md` sections (keep the existing checklist at the
  top): §0 common (box, modes, frame, stacking, enabled), §1 Bend, §2 Twist, §9 parity.
- **Test:** new `crates/modifiers/tests/deformers_parity.rs` (or extend
  `generators_parity.rs` if the scene helpers are easier to reuse there): rebuild the
  scene from `[params]`, `[params.input*]`, `[params.generator]`; new keys this batch:
  `parent` ("input", "generator", "null"), `hierarchy` (a description: ignore), `rot`
  (degrees H, P, B → our transform rotation, radians), vec3 `size`. Compare with
  `export_weld` on both sides as in GENERATORS2 §0.

## Hints (planner)
- Box in deformer space: y from −size_y/2 to +size_y/2; the deformation is anchored at
  the **bottom** of the box (t = (y + size_y/2)/size_y). Limited: below the box t = 0,
  above it t = 1 applied rigidly; Within Box: points outside unchanged; Unlimited:
  extrapolate t.
- Twist: rotation about Y by angle·t (`twist/angle`: at y = 0 the corner (−100,0,−100)
  → (0,0,−141.42)). Check the rotation sense against that corner.
- Bend: circular arc in the deformer's XY plane turned by `angle` about Y, total turn
  `strength` over the box height; Limited carries the part above along the end tangent.
  `on-plane` (plane at t = 0.5 → rotated 45°) is a good small check. `keep_y_axis_length`
  changes the arc length (compare `bend/keep-length`). Start with `bend/strength`.
- Stack order: first child first (`stack-bend-twist` vs `stack-twist-bend`).

## Done when
- [ ] MODIFIERS.md §0–2 and §9 written, citing cases.
- [ ] Parity test covers every bend, twist and deformer-common case;
      `UNSUPPORTED` / `KNOWN_MISMATCH` explained in the spec.
- [ ] `sh scripts/gate.sh` prints `GATE: all passed`.
- [ ] `docs/parity/STATUS.md` rows added.
- [ ] Committed on the branch with your trailer; report below filled.

## Out of scope
Taper, Bulge, Shear, Spherify (task 02); falloff/fields; Fit to Parent; UI beyond what
the registry gives for free; viewport gizmo for deformer boxes.

## Report (implementer fills in)
