# 02 · Taper, Bulge, Shear, Spherify

Status: done · Branch: `observations-7` · Planner: main session · Implementer: local model

## Goal
The four remaining deformers of this batch match their golden cases, reusing task 01's
framework (box, modes, frame, stacking).

## Inputs
- Observations: `docs/parity/taper.md`, `bulge.md`, `shear.md`, `spherify.md`.
- Golden cases: `tests/golden/{taper,bulge,shear,spherify}/` (8 + 7 + 7 + 5).
- Task 01's spec (MODIFIERS.md §0) and parity test.

## Design decisions (planner)
- Same crate, same plugin, ids `forge.taper`, `forge.bulge`, `forge.shear`,
  `forge.spherify`. Taper/Bulge/Shear params: `size`, `mode`, `strength` (percent, 0),
  `curvature` (percent, **100**, min 0), `fillet` (false); Shear also `angle` (angle, 0).
  Spherify: `radius` (200, min 0), `strength` (percent, **50**, clamp 0–100); no box, no
  mode. Percent params are stored as the number shown (50 = 50 %), like `end_scale` in
  GENERATORS2 §10.
- MODIFIERS.md §3 Taper, §4 Bulge, §5 Shear, §6 Spherify; extend the task-01 test.

## Hints (planner, from the observer's fits: verify)
- Taper: X and Z scaled by `1 − strength·f(t)`, Y unchanged; t from the box bottom.
  f = curvature-blend of linear and quadratic: `c·t² + (1 − c)·t`; with fillet,
  `3t² − 2t³` (`taper/fillet`). Limited holds the top value above the box.
- Bulge: X and Z scaled by `1 + strength·g(u)`, u = y/(size_y/2) (centred);
  g = `c·(1 − u²) + (1 − c)·(1 − |u|)`; fillet: smoothstep of `1 − |u|`.
  Unlimited extrapolates (the scale goes below 1 beyond the box).
- Shear: offset along +X turned by `angle` about Y, magnitude
  `strength·(size_y/2)·f(t)` with Taper's f.
- Spherify: `p + strength·(r·p/|p| − p)` in deformer space (centre = deformer origin),
  for every point.

## Done when
- [x] MODIFIERS.md §3–6 and the updated §9 written, citing cases.
- [x] Parity test covers every taper, bulge, shear and spherify case;
      `UNSUPPORTED` / `KNOWN_MISMATCH` explained in the spec (both empty).
- [x] `sh scripts/gate.sh` prints `GATE: all passed`.
- [x] `docs/parity/STATUS.md` rows added.
- [x] Committed on the branch with your trailer; report below filled.

## Out of scope
Falloff/fields; other deformers.

## Report (implementer fills in)

**Status:** done

**Result:** 50/50 golden cases exact — the 23 of task 01 (bend 10, twist 7,
deformer-common 6) plus taper 8/8, bulge 7/7, shear 7/7, spherify 5/5. `UNSUPPORTED`:
none. `KNOWN_MISMATCH`: none. All three sets in MODIFIERS.md §9 match the lists in
`crates/modifiers/tests/deformers_parity.rs`.

**Rules derived** (proving case in brackets; all cited in MODIFIERS.md §3–6):
- Percent params (`strength`, `curvature`) carry `Unit::Percent` and store **the number
  the app shows** (50 = 50 %), divided by 100 in the maths — the `end_scale` convention
  (GENERATORS2 §10).
- Taper: `x` and `z` scaled by `1 − s·f(t)`, y unchanged, anchored at the box bottom;
  `f(t) = c·t² + (1 − c)·t` blends the quadratic and linear profiles
  [`taper/strength` rows 0.995/0.955/0.875/0.755/0.595 = `1 − 0.5·t²` at
  t = 0.1/0.3/0.5/0.7/0.9; `taper/curvature` (c = 1/2) 0.9725/0.8125/0.5725 =
  `1 − 0.5·(0.5t² + 0.5t)`]; `fillet` replaces the blend with `3t² − 2t³`
  [`taper/fillet`: 0.986/0.75/0.514 at the same rows].
- Bulge: the same scale about `1 + s·g(u)`, `u = y/(size_y/2)` **centred**;
  `g(u) = c·(1 − u²) + (1 − c)·(1 − |u|)`, fillet `smoothstep(1 − |u|)`; `g(±1) = 0`,
  so the box ends are untouched [`bulge/strength` rows −118/−142/−150 at
  u = −0.8/−0.4/0; `bulge/curvature` −114/−136; `bulge/fillet` −105.2/−132.4 =
  `1 + 0.5·smoothstep(0.2)`/`smoothstep(0.6)`; `bulge/mode-box` byte-identical to
  `bulge/strength`].
- `limited` above the box holds the value at `t = 1` for the scaling/offset deformers
  [`taper/strength`: rows 0.5 above y = 125; `shear/strength`: +62.5 there].
- `unlimited` extrapolates `t` (and with it `u`) below and above the box
  [`taper/mode-unlimited` y ±200 → 0.955/0.155 (t = ∓0.3/1.3);
  `bulge/mode-unlimited` u = −1.6 → scale 0.22, u = −1.2 → 0.78;
  `shear/mode-unlimited` +5.625/+105.625].
- Shear offsets every point by `m = s·(size_y/2)·f(t)` along
  `R(angle)·(1, 0, 0) = (cos angle, 0, sin angle)` — **Taper's `f`** — y unchanged;
  turning the offset is turning the frame (`R(β)(R(−β)p + m·x̂) = p + m·R(β)x̂`)
  [`shear/strength` rows 0.625/15.625/50.625/62.5 = `62.5·t²`;
  `shear/angle` 45°: X and Z both up to `62.5·cos 45° = 44.19` (bbox 244.19);
  `shear/curvature` 3.4375 = `62.5·0.055`; `shear/fillet` 1.75 = `62.5·0.028`].
- Spherify has **no box and no mode**: `p' = p + (strength/100)·(radius·p/|p| − p)` in
  the deformer's space for every point, no falloff, and the origin stays put
  [`spherify/defaults`: corner (−100, −200, −100) → (−90.82, −181.65, −90.82), halfway
  to its projection (−81.65, −163.30, −81.65); `spherify/strength-full`: every point at
  exactly 200; `spherify/radius` at exactly 100; `spherify/moved`: the sphere centred on
  (50, 100, 0), so §0's relative transform decides the centre]. Its `strength` is
  clamped to 0…100 by the param range (the observation typed 99999 → 100).

**Parity-pending rules** (implemented, only one configuration observed):
- `fillet` together with `curvature < 100`: the one fillet case per deformer has
  curvature 100, so "fillet replaces the blend" and "fillet blends into it" cannot be
  told apart (the replacement is what the numbers show).
- The curvature blend extrapolated *below* the box in `unlimited`: every unlimited case
  has curvature 100, so only the quadratic term was ever seen outside the box.
- Spherify at the deformer origin and at `radius` 0 (the `|p| = 0` guard).
- Task 01's pendings stand unchanged (the `size_x/2` term of `R`, sibling deformers
  below the object, `within_box` gating on y only, zero strength with a non-zero
  `angle`).

**Files changed:**
- `docs/specs/MODIFIERS.md` — intro and §0 updated (Carry = `t = 1` for the scaling and
  offsetting deformers, the percent-param convention), new §3 Taper, §4 Bulge, §5 Shear,
  §6 Spherify (each with the fitted-number proofs and a case table), §9 rewritten for
  the 50 cases and the new pendings.
- `docs/parity/STATUS.md` — rows for taper, bulge, shear and spherify.
- `crates/modifiers/src/common.rs` — `Placement::t()`, `smoothstep`, `taper_curve`
  (shared by Taper and Shear), `bulge_curve`, the four param structs, curve unit tests.
- `crates/modifiers/src/taper.rs`, `bulge.rs`, `shear.rs`, `spherify.rs` — the pure
  deformers on `&mut [DVec3]` with unit tests (curves, box anchor, hold above the box,
  angle direction, zero-size/origin guards).
- `crates/modifiers/src/lib.rs` — the four node types (percent `strength`/`curvature`,
  `fillet`, Shear's `angle`, Spherify's `radius` with a clamped `strength` and no box or
  mode), the new `Job` variants and their dispatch.
- `crates/modifiers/tests/deformers_parity.rs` — the four features in `FEATURES`, the
  27 cases, count assertion 50.
- `docs/agents/tasks/02-taper-bulge-shear-spherify.md` — this report.

**Open questions for the planner:**
- A Bulge case with a non-default box (`size`) would show `u` move with `size_y` the
  way `taper/size` does for `t`; only the default box was observed.
- A Spherify case with a *rotated* deformer would confirm the projection runs in the
  deformer's own frame (only a moved one was observed, which §0 already covers).

**Gate output** (`sh scripts/gate.sh`):
```
GATE: 278 tests passed, 0 failed
cleanroom: ok
ok: 190 cases
GATE: all passed
```
