# 01 · Deformer framework, Bend and Twist

Status: done · Branch: `observations-7` · Planner: main session · Implementer: local model

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
- [x] MODIFIERS.md §0–2 and §9 written, citing cases.
- [x] Parity test covers every bend, twist and deformer-common case;
      `UNSUPPORTED` / `KNOWN_MISMATCH` explained in the spec.
- [x] `sh scripts/gate.sh` prints `GATE: all passed`.
- [x] `docs/parity/STATUS.md` rows added.
- [x] Committed on the branch with your trailer; report below filled.

## Out of scope
Taper, Bulge, Shear, Spherify (task 02); falloff/fields; Fit to Parent; UI beyond what
the registry gives for free; viewport gizmo for deformer boxes.

## Report (implementer fills in)

**Status:** done

**Result:** 23/23 golden cases exact — bend 10/10, twist 7/7, deformer-common 6/6.
`UNSUPPORTED`: none. `KNOWN_MISMATCH`: none. All three sets in the spec (§9) match the
lists in `crates/modifiers/tests/deformers_parity.rs`.

**Rules derived** (proving case in brackets; all cited in MODIFIERS.md §0–2):
- Box `size_y` centred on the deformer, `t = (y + size_y/2)/size_y`, anchored at the box
  bottom; the mode gate is on **y** only [`deformer-common/on-plane`: the plane is wider
  than the box in x/z and still deforms].
- `limited`: below the box unchanged, inside deformed, above carried rigidly along the
  tangent at the box top [`bend/strength`: bbox max (359.155, 259.155, 100)].
- `within_box`: outside unchanged; `unlimited`: `t` extrapolates below and above
  [`bend/mode-box`, `bend/mode-unlimited` min y −299.40, `twist/mode-unlimited` −27° turn].
- Twist `p' = (R(φ)(x, z), y)`, `R(θ)(x, z) = (x·cosθ − z·sinθ, x·sinθ + z·cosθ)`,
  `φ = strength-free angle·t` [`twist/angle`: (−100, 0, −100) → (0, 0, −141.42) fixes the
  sense; `twist/angle-big` (360°) confirms it].
- Bend neutral-line radius `R = h/strength` with Keep Y-Axis Length **on**, and
  `h/strength + size_x/2` **off** [`bend/keep-length` (234.155, 134.155) vs
  `bend/strength` (359.155, 259.155), `bend/strength-180` centre x 204.577,
  `bend/size` (277.324, 177.324), `deformer-common/on-plane` (−58.19, 217.35)].
- Bend's `angle` β turns the bend plane: `p' = R(β)·bend(R(−β)·p)`
  [`bend/angle`: x and z extent 353.96, max y 300.58].
- Points go into the deformer's space with the inverse of its transform **relative to
  the deformed object**, and come back [`bend/moved`, `twist/moved`: box at world
  y −25…225; `bend/rotated`: bank 90°].
- Deformers apply in object-tree order, first child first
  [`stack-bend-twist` ≠ `stack-twist-bend`].
- A deformer under a geometry-less parent (Null) deforms that parent's mesh children
  [`sibling` = `bend/strength`]; a deformer under a generator runs after it
  [`generator`: 2560 quads with `bend/strength`'s bbox].
- `enabled = false` does nothing [`disabled` = `input.obj`]; zero strength does nothing
  [`bend/defaults`].

**Parity-pending rules** (implemented, only one configuration observed):
- The `size_x/2` term of `R`: every case has `size_x = size_z`, so `size_x/2`,
  `size_z/2` and `(size_x + size_z)/4` are indistinguishable. `size_x/2` is used.
- Sibling deformers only observed *below* the object (`Null > Cube, Bend`); the
  implementation applies them regardless of position.
- `within_box` only gates on y (x/z gating never observed for that mode).
- Zero `strength` with a non-zero `angle` was never observed; treated as no deformation.

**Files changed:**
- `docs/specs/MODIFIERS.md` — new §0 (box, modes, frame, stacking, enabled, evaluation),
  §1 Bend (with the fitted-radius proofs), §2 Twist, §9 parity; the original checklist
  stays on top.
- `docs/parity/STATUS.md` — rows for bend, twist and deformer-common.
- `crates/modifiers/src/common.rs` — `Placement`/`placement`, `BendParams`,
  `TwistParams` (radians), param readers, unit tests.
- `crates/modifiers/src/bend.rs`, `twist.rs` — the pure deformers on `&mut [DVec3]`,
  unit tests (anchors, radius, carry, zero-size guards).
- `crates/modifiers/src/lib.rs` — `forge.bend` / `forge.twist` node types with the
  planner's params plus the common `enabled` flag, the `Deformers` plugin,
  `is_deformer`, `apply_deformer` (relative transform → deform → transform back).
- `crates/modifiers/tests/deformers_parity.rs` — the 23-case parity test (rebuilds the
  scene from `[params]`, `[params.input*]`, `[params.generator]`, including `parent`,
  `hierarchy`, `rot` in degrees HPB and vec3 `size`; `level` maps to `level_view`).
- `crates/generators/src/lib.rs` — `evaluate` now wraps `evaluate_geometry` and applies
  `deformer_children` (own deformer children, plus the deformer children of a
  geometry-less parent) via `produces_geometry`.
- `crates/generators/src/{subdivision,modeling,boole}.rs` — input probes call
  `evaluate_geometry` (presence only), the mesh fetches keep `evaluate`.
- `crates/generators/Cargo.toml`, `crates/modifiers/Cargo.toml`, `apps/studio/*`,
  `Cargo.lock` — dependency `generators → modifiers`, dev-dependency
  `modifiers → generators` for the parity test, `Deformers` installed in the app
  registry.
- Removed the never-committed `crates/modifiers/tests/bend.rs` stub from the earlier
  attempt (its tests live in the module `#[cfg(test)]` blocks now).

**Open questions for the planner:**
- A bend/twist case with `size_x ≠ size_z` and non-zero strength would settle which
  half-size belongs in `R` (currently `size_x/2`).
- A `Null > Bend, Cube` case (deformer above its sibling object) would settle the
  sibling ordering rule.
- A `within_box` case with an object wider than the box in x/z would settle whether that
  mode gates on all three axes.

**Gate output** (`sh scripts/gate.sh`):
```
GATE: 254 tests passed, 0 failed
cleanroom: ok
ok: 190 cases
GATE: all passed
```