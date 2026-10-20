# 02 · Taper, Bulge, Shear, Spherify

Status: ready (after 01 is done) · Branch: `observations-7` · Planner: main session · Implementer: local model

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
Same checklist as task 01 (spec §3–6, parity cases, gate, STATUS rows, commit, report).

## Out of scope
Falloff/fields; other deformers.

## Report (implementer fills in)
