# Parity debt: triage of pinned cases (2026-11-24)

Result of the planner's parity-debt pass (subagent). Pins in class **a** were fixed and unpinned; **b** are reference-data limits or decisions (stay pinned); **c** need more data or a dedicated brief. Use this list to pick follow-up observations or briefs. Re-run the triage when pins change.


Class: a = fixable in our code from the golden data, b = reference-data limitation / decision, c = unclear (tried or too deep).

| Pin | Class | Outcome |
|---|---|---|
| rigging/j2-bend-dq (UNSUPPORTED, Spherical) | a | FIXED: Spherical = dual quaternion skin (already in code, only never wired in the test); exact |
| rigging/j2-bend-90-blended (UNSUPPORTED) | a | FIXED: Blended = (1-blend) linear + blend dual quaternion, blend 0.5 (bbox x min -57.337782 = mean of -47.50042 and -67.175144); exact |
| splines-2/formula-defaults (UNSUPPORTED, "needs expression library") | a | FIXED: forge-expr exists now; samples+1 points of (x_t,y_t,z_t) over [t_min,t_max]; exact |
| splines-2/cog-defaults | c | round 2: geometry of the tooth found (fillet quarter circle, radial run, involute), exact spline control points still unknown (see "Round 2 notes") |
| rigging KNOWN_MISMATCH_FRAMES 15 | b | animation noise of the reference |
| ik reach-near/-side/-straight | b | reference stops at its 0.01 threshold, residual up to 0.04 (our result is exact) |
| ik four-joints, four-joints-2 | c | redundancy resolution; 3 hypotheses already spent in RIG.md 10 (CCD x2, Jacobian pseudo-inverse, FABRIK; Jacobian-transpose flow fits one of the two) |
| platonic bucky x4 | b | export is an irregular fixed table (layers with equal coordinates, radii 96.7..99.8, asymmetric z layers 91.857 ... -92.13); no formula |
| spline-mask/cap | b | near-cocircular Delaunay tie on 432 points decided by sub-float32 noise (GEOMETRY.md 6.1 R1) |
| caps-bevel/tri-extrude-ngon | b | regular hexagon, exact cocircular tie, same reason |
| caps-bevel/bevel-round, bevel-inner | a | FIXED (round 2): the Round profile is a rational quadratic quarter arc (weights 1, sqrt2/2, 1) sampled uniformly in the parameter; layout of a 4-point extrude derived, both strict (GENERATORS2.md 7b) |
| caps-bevel/bevel-step, -chamfer, -concave, sweep-caps-bevel | c | numbering of Step/Solid shapes and of the sweep circle profile still not derived |
| caps-bevel/caps-delaunay, caps-grid | c | order seen for one outline only / 730-point grid not derived |
| caps-bevel/tri-ellipse-loft, tri-loft-rotated | b | input not recorded in the case |
| caps-bevel/tri-slice-orient-x, tri-dodeca-seg2 | b | exported from editable polygon objects (Make Editable triangulation, sub-float noise) |
| spline-tools outline, outline-neg, outline-r100-d10 | c | moved distance is d * 1.00013035 (R independent: 220.002607/200 and 110.001303/100 give the same 1.1000130); the reference offset is numeric (open arc: radii 220.00011). No closed form found (see below) |
| spline-tools outline-arc-open | c | the result spline is recorded in case.toml (closed, 10 points: arc points then the offset points reversed), but its end offsets are off the radial direction by 0.067 and radii are 220.0001, numeric offset again |
| cloner honey-var, order-random, random-indexed | b | reference random sequence not reproducible / statistical |
| cloner linear-rstep | b | bad data (settled by linear-rstep-2) |
| cloner reset-coords-off | b | superseded by reset-coords-off-2 |
| cloner step-count-9, step-position (mesh level) | b | reference curve not an exact line (9e-7 weights), sums to 1.01e-4 |
| generators loft/defaults, three, caps-off | b | decision D18: R25 wins, no version switch |
| generators loft/mesh-subdivision, sweep/helix, scale-end, rail, r25-helix-radius, -sub4, -sub8, r25-rail-rectangle (+ the same 10 in io export_parity) | c | frame of the Banking-on sweep along a helix (>= 3 hypotheses already spent in GENERATORS2 10a); not retried |
| sweep/r25-scale-end-only | b | case records no path points |
| boole r25-* (8) | c | round 2: not solved; new findings in "Round 2 notes" below (numbering is mode independent, uneven vs default numbering disagree, face order is a dual-graph walk) |
| boole/quality-off | c | no data to derive the High Quality off result |
| metaball/editor-sub | b | reference mesh is open |
| tracer interval, vertices | b | the two cases disagree by 1e-3 on the same frame-1 position |
| landscape no-borders, spherical, sea-level-* | b | statistical noise |
| clone_text FONT_PIVOT_MISMATCHES | b | reference font advances unknown |
| material/app proj-frontal | b | recorded camera does not reproduce the reference |
| render area-soft, area-soft-2 | c | reference lights the cube's -Z face but leaves the plane black (render.png); the area light orientation/emission rule is not derivable from one image |
| render mat-checker-uv/-cubic, checker-plane | b | reference black squares are 0 |
| render mat-metal | b | silhouette not observable |
| camera ortho-zoom-2 | c | needs a third zoom value (510.8 wide at zoom 2, 1000 at zoom 1) |
| rigid drop-cube, drop-bounce, drop-friction-high, tumble (+ clones/collision noise) | b | engine contact/solver details, restitution 1e18, contact order |
| modeling-tools/melt-ring | c | the 8 ring polygons are re-created in a scrambled order with irregular start corners; no ordering rule visible |
| keyframes/linear-to-spline | b | superseded by linear-to-spline-2 |
| animation3 euler-cross, quaternion, generator-param | b | solver residual of the reference |
| volumes smooth-strength | c | iterated-box Gaussian rejected for 3 iterations (VOLUME.md 8.3, widths 3..6 scanned) |
| fields shader-noise | b | noise of the reference not reproducible (shaders S12) |
| fields mod-step | c | the Step layer depends on the bounds of the sampled set, not a field of position |
| fields decay/delay-frame20 | b | committed samples unreliable |
| io export_parity | (mirrors generators) | |
| shaders noise-* (9), gradient-turbulence, colorizer | b | reference noise basis not reproducible |
| shaders brick | b | observed pattern is noise-perturbed |
| ffd_formula_melt melt/* (7) | b | Melt noise (randomness above 0) not reproducible |
| modifiers spline-wrap/defaults | c | helix frame roll, 3 hypotheses spent |
| spline-wrap/arc | b | sampler last-digit error amplified 99 lengths |
| spline-deformer/defaults, modifier-moved, radius-60 | c | pairing / weights unresolved, hypotheses spent (MODIFIERS.md 20.2) |
| spline-rail/one-spline | b | reference read the unlinked helper of the scene |
| spline-rail/x-splines | c | data only shows z = 0; every z-source consistent with it, so under-determined |
| helper_parity mesh-deformer/cage, surface/defaults, collision/sphere, jiggle/* | b | the reference export did not deform |
| spline_sampling cog | see cog-defaults | |

## Notes on what was tried

- Bevel rings (caps-bevel): on the 400 x 400 rectangle extrude (size 10) the bevel is a profile swept around the cap outline:
  Step n: staircase (inset 10/n, depth 10/n per step), Solid: L shape of 3 + 3 equal segments, Round: arc of radius 10.
  Point numbering differs per shape: Round is corner-major (q3..q0, profile order outer -> inner), Chamfer ring-major, Step has a
  group of horizontal-move points corner-major with an irregular first pair, Solid starts with the cap ring in q0..q3 order.
  The Round profile points sit on the circle at 29.2776 and 60.7224 deg (not 30/60, not a Bezier-sampled circle with k = 0.5523).
  **Solved in round 2:** the points are the rational quadratic arc P0 (1,0), P1 (1,1), P2 (0,1), weights 1, sqrt2/2, 1 at s = 1/3, 2/3
  (the golden insets 1.277396 : 5.109584 are exactly 1 : 4, and tan(phi/2) = 0.2612, 2 - sqrt2 gave the weight away). Polynomial Bezier samples (k = 0.5, 0.5523) and
  arc-length variants do not lie on the circle. Implemented for the 4-point extrude (bevel-round, bevel-inner strict); Step/Solid/sweep not attempted further.
- Outline: moved distance d*1.00013035 for both radius 200/d 20 and radius 100/d 10; a 100-chord polyline length of a k = 0.55228 Bezier quarter
  gives 1.00012999, close but not equal; no rule adopted.

## Round 2 notes (parity debt, second pass)

### Boole `r25-*` (not solved)
Facts established from the eight goldens (comparison scripts read mesh.obj; positions by `a` = 100 (A's face), `b` = B's lower bound):
- The numbering is **independent of the mode**: `r25-intersect`, `-union`, `-a_minus_b`, `-a_without_b` carry the same new-point order (A: P1..P6 = aab, abb, aba, baa, bab, bba for B at (50,50,50); B: aab, bba, abb, bab, aba, baa). Survivors keep the input order (the removed corner is compacted away), the new points follow.
- The numbering of the **uneven** case (B at (30,50,70), no ties) is class-wise for A: first the three points where A's edges pierce B's planes (aab, aba, baa = A's edges 3-5, 4-5, 5-7 in index order, also ascending by edge length 130, 150, 170), then the three points where B's edges pierce A's faces in A's polygon order x+, z+, y+ (abb, bba, bab). B's numbering (uneven): B's edges first in B's edge order (0-1, 0-2, 0-6 = bab, abb, bba), then A's edge points in the same order as A's (aab, aba, baa). On the intersection loop (aab abb aba bba baa bab) these are "even positions then odd" for A. The **default** case (all keys tie) does not follow the same pattern (A: loop positions 0,1,2,4,5,3), so no single rule fits both; a hash/unstable-sort tie break is the likely cause, which cannot be derived.
- Hypotheses rejected: first use in a walk of A's faces x+, y+, z+ (fits default, not uneven); creation order over B's planes; sort by edge parameter t (from either end), by coordinates (lexicographic, either direction), by distance to the centres; per B-face walk (default B order P1,P6,P2,P5,P3,P4 has no face-walk reading).
- Face order of the union family: A `z-, x+, y-, z+, y+, x-` and B `z-, y-, x+, z+, x-, y+` are the same for default, uneven and a_minus_b (A), i.e. a function of the cut topology only; the R20 `hide-edges` case keeps the source order (`-z +x +z -x +y -y`), `intersect`, `inside` and the shift-x cases keep it as well (shift-x: source order minus the removed x+). Every consecutive pair in the scrambled orders shares an edge (a Hamiltonian walk of the face graph); DFS/BFS with neighbours by polygon edge order, by lowest/highest shared vertex index and a strip walk were tried by hand and none fits all steps. UV evidence (obj `vt`) only repeats the source face, no extra order information.
- A solution needs the walk rule above; the numbering needs the tie rule. Next attempt: new goldens with B shifted by distinct amounts per axis AND a different A size (so that A-edge lengths, B-edge lengths and loop order all differ), which would separate class order from loop order.

### caps-bevel (Round solved, rest not)
See the table row and GENERATORS2.md 7b: the Round profile is the rational quadratic quarter arc (weights 1, sqrt2/2, 1) at uniform parameter. Bevel Outside on moves the shell ring outward by the size and the cap ring sits on the original outline. Step, Solid, Curve and the sweep circle profile were not attempted (numbering of the other shapes still shape specific, per the notes above; the profile idea does not apply to Step/Solid).

### splines-2/cog-defaults (not solved; geometry understood)
Per tooth the export holds 52 samples, `tests/golden/splines-2/cog-defaults/mesh.obj` outline loop, starting at the root-land end (r 180, -3.14457 deg for the tooth at 4.5 deg):
- samples 0..17: the fillet, a quarter circle of radius 7.5 whose centre sits at r = 187.5 and the angle of its root tangent point (so the arc runs from the root circle to (r 187.5, tooth angle -5.354 deg)); the samples are NOT on that circle (deviation up to 0.017) so the reference stores a cubic approximation with handles not equal to the standard 0.5523 r;
- 17..24: radial run to the base circle (r 187.94) then the involute of the base circle (r_b = 200 cos 20 deg); the samples deviate from the exact involute by at most 7e-4 deg, so again a spline approximation;
- 24, 25, 26: the tip land with its own middle point (r 220, 220.00613 at +-1.81 deg), straight; the mirror flank 26..33, fillet 33..50, the root land 50, 51 (middle, 13.5 deg), 52 = next tooth sample 0.
- The 5 degree Adaptive rule of GENERATORS2.md 6 explains the fillet sampling (chord turn 5.1-5.5 deg per step, the short piece at index 14->15 is the "middle of previous sample and end" rule); the sample count per fillet is 17 intervals.
- Tried: single cubic with radial/tangential end tangents and two free handle lengths (best rms 6e-3 at lengths 3.65 / 4.55, with t = .0659, .1307, .1944, ... not on the 1/50 grid), single cubic with four free handle coordinates (optimiser not converged), samples at t = 0.06k (least squares x residual 0.056). So the fillet is not one cubic with those tangents; likely two segments around an unseen interior spline point (12 spline points per tooth = 8 seen + one per curved piece). Next: the editable spline of one tooth (point positions and tangents) as noted in GENERATORS2.md 6.3, or a fit with a two-segment model and the sampler run inside the optimiser.
