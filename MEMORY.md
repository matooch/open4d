# Memory

## Task 01 · Deformer framework, Bend and Twist — done (branch `observations-7`, commit `a433d57`)

Spec first, then tests, then code; committed with the brief's trailer. Clean room
throughout: everything below was derived from `docs/parity/{bend,twist,deformer-common}.md`
and `tests/golden/{bend,twist,deformer-common}/`, never from the reference app's files.

### What exists now

- **Spec** `docs/specs/MODIFIERS.md`: the original checklist stays on top; §0 common
  (box, modes, frame, stacking, `enabled`, where a deformer applies), §1 Bend, §2 Twist,
  §9 parity + parity-pending list.
- **Crate** `crates/modifiers` (`forge-modifiers`):
  - `common.rs` — `Placement`/`placement` (the `limited` / `within_box` / `unlimited`
    rules, gated on **y** only), `BendParams` / `TwistParams` (angles in **radians**),
    param readers.
  - `bend.rs` / `twist.rs` — pure deformers on `&mut [DVec3]` in the deformer's local
    space, with module unit tests.
  - `lib.rs` — node types `forge.bend` / `forge.twist` (`Category::Deformer`), the
    `Deformers` plugin, `is_deformer`, `apply_deformer` (inverse deformer transform
    relative to the target object → deform → transform back).
- **Evaluation hook** `crates/generators/src/lib.rs`: `evaluate` wraps the new private
  `evaluate_geometry` and applies `deformer_children` (the node's own deformer children
  plus, when its parent has no geometry of its own, that parent's deformer children — the
  Null sibling case). Input probes in `subdivision` / `modeling` / `boole` use
  `evaluate_geometry` so they don't redo the work.
- **Registration**: `registry.install(&Deformers)` in `apps/studio/src/main.rs`.
- **Test** `crates/modifiers/tests/deformers_parity.rs` — 23/23 golden cases exact
  (bend 10, twist 7, deformer-common 6); dev-dependency `modifiers → generators` (Cargo
  allows a dev-dependency cycle).

### Rules worth remembering (all proved by a golden case)

- `t = (y + size_y/2)/size_y`, anchored at the box bottom; `limited` carries the part
  above the box along the tangent at the top, `within_box` leaves it, `unlimited`
  extrapolates. The gate is on y only (`on-plane` has points outside the box in x/z that
  still deform).
- Twist: `p' = (R(φ)(x,z), y)` with `R(θ)(x,z) = (x·cosθ − z·sinθ, x·sinθ + z·cosθ)` —
  the *opposite* sense to the `Ry` used for array (`GENERATORS2` §2); `twist/angle`
  pins it.
- Bend neutral-line radius `R = h/strength + (keep_y_axis_length ? 0 : size_x/2)`, arc
  `A(φ) = (R − R·cosφ, −h/2 + R·sinφ)`, normal `N = (cosφ, −sinφ)`, tangent
  `T = (sinφ, cosφ)`; `p' = R(β)·bend(R(−β)·p)` for the `angle` param.
- Deformers apply in tree order; under a geometry-less parent they deform the siblings;
  under a generator they run **after** it (subdivided-then-bent).

### Parity-pending (one configuration observed only)

`size_x/2` in `R` (every case has `size_x = size_z`), sibling deformers only observed
*below* the object, `within_box` gating only on y, zero strength with non-zero `angle`.

## Task 02 · Taper, Bulge, Shear, Spherify — done (branch `observations-7`, uncommitted)

Same workflow, same framework; the four deformers are four small pure functions that
reuse §0's box, modes and relative transform.

### What exists now

- **Spec** `docs/specs/MODIFIERS.md` gained §3 Taper, §4 Bulge, §5 Shear, §6 Spherify
  (fitted-number proofs + case tables each); §0 gained `Placement::t()`'s rule (Carry =
  `t = 1` for the scaling/offsetting deformers) and the percent-param convention; §9 now
  lists 50 cases and the new pendings.
- **Code** `crates/modifiers/src/`: `common.rs` gained `smoothstep`, `taper_curve`
  (shared by Taper **and** Shear), `bulge_curve` and the four param structs; new
  `taper.rs`, `bulge.rs`, `shear.rs`, `spherify.rs`; `lib.rs` registers `forge.taper`,
  `forge.bulge`, `forge.shear`, `forge.spherify` with `Job` variants + dispatch.
- **Test**: the same parity test now covers 50 cases (23 + taper 8, bulge 7, shear 7,
  spherify 5); `UNSUPPORTED`/`KNOWN_MISMATCH` still empty.

### Rules worth remembering (all proved by a golden case)

- Percent params carry `Unit::Percent` and store **the number the app shows** (50 = 50 %),
  `/100` in the maths — the `end_scale` convention (GENERATORS2 §10). `ParamDesc::validate`
  *clamps* floats into range, so `.range(0.0, 100.0)` reproduces the app's clamping.
- Taper: x, z scaled by `1 − s·f(t)`; `f(t) = c·t² + (1 − c)·t`, fillet → `3t² − 2t³`.
- Bulge: centred `u = y/(size_y/2)`, `g(u) = c·(1 − u²) + (1 − c)·(1 − |u|)`, fillet →
  `smoothstep(1 − |u|)`; `g(±1) = 0`, hence `within_box` = `limited` for Bulge.
- Shear: offset `s·(size_y/2)·f(t)` along `(cos angle, 0, sin angle)` — Taper's `f`.
- Spherify: `p' = p + (s/100)(r·p/|p| − p)`, no box and no mode, `|p| = 0` stays.

### Parity-pending (one configuration observed only)

`fillet` + `curvature < 100`, the curvature blend extrapolated below the box in
`unlimited` (all such cases have curvature 100), Spherify at the origin / `radius` 0,
plus task 01's four.

### Gate

`sh scripts/gate.sh` → `GATE: 278 tests passed, 0 failed` / `cleanroom: ok` /
`ok: 190 cases` / `GATE: all passed` (run it with
`"C:\Program Files\Git\bin\bash.exe" scripts/gate.sh` — `sh` is not on the PowerShell
PATH).

### Next

No task 03 brief exists yet (`docs/agents/tasks/` holds only 01 and 02); the planner
writes the next batch. MODIFIERS.md's checklist (Falloff, Fit to Parent, the remaining
deformers) is the natural continuation, and each parity-pending rule above asks for one
more golden case.

## Observer session · bend follow-ups — done (branch `observations-7`, commits `3fef51b`, `8c5ddc1`)

Two observer loops settled three pendings; parity is now 52 matched exactly with 3
`KNOWN_MISMATCH` (`bend/mode-box-narrow{,-x,-z}`), gate green (`ok: 195 cases`).

### Derived rules (Reference App golden-proven)

- `R = h/α + size_x/2` — the half-size term is `size_x/2`, not `size_z/2` or
  `(size_x+size_z)/4` (`bend/size-nonuniform`, box (300,250,100) → max (384.155, 284.155)).
- **`within_box` gates per point on all three axes** (`|x| ≤ sx/2`, `|y| ≤ sy/2`,
  `|z| ≤ sz/2`) — `mode-box-narrow-x` leaves the x = ±100 columns unmoved,
  `mode-box-narrow-z` leaves all 90 vertices at |z| = 100 unmoved.
- **The whole deformation is suppressed when the box spans the parent's x/z extent in
  neither axis**: box (100,250,100) vs the 200×400×200 cube → the undeformed cube
  (`mode-box-narrow`); a 100×400×100 cube with box (120,250,150) bends, so the
  threshold is object-relative, not absolute. Our implementation still gates only on y —
  the three cases are pinned `KNOWN_MISMATCH` for the fix.
- α = 0 with angle 45 → the undeformed cube (`bend/strength-zero`): a zero strength is
  no deformation regardless of the angle.

### GUI doctrine refinements (all learned the hard way this session)

- With a modeling/selection tool active, the Attributes panel keeps showing the **tool**
  even when an object is selected: press `Esc` first, then select the object, and crop
  -verify the panel after every action.
- `Set-Field` types into whatever has focus; leaked `100` keystrokes hit the UI (`1`/`0`
  switch component modes, activate tools). Never type before crop-verifying focus.
- Measure field rects with a **labeled grid-overlay crop** (draw 10px lines + coords onto
  a screenshot crop) — eyeballed row positions were off by 10–20px twice. Current
  coords: cube Size X/Y/Z (1430, 686/706/726); bend Size X/Y/Z (1479/1550/1619, 686),
  Mode (1500,706) items Limited 728 / Within Box 748 / Unlimited 768, Strength
  (1475,726), Angle (1475,745).
- A freshly written golden folder can stay locked by the file watcher: renames/deletes
  fail but **file writes inside still work** — keep the original folder name instead of
  renaming.

### Remaining MODIFIERS.md pendings

Sibling deformers *below* the object (§0), `fillet` + `curvature < 100` (§3–§5), the
curvature blend extrapolated below the box in `unlimited` (§3–§5), Spherify at the
origin / `radius` 0 (§6).
