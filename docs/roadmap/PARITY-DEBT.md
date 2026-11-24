# Parity debt: triage of pinned cases (2026-11-24)

Result of the planner's parity-debt pass (subagent). Pins in class **a** were fixed and unpinned; **b** are reference-data limits or decisions (stay pinned); **c** need more data or a dedicated brief. Use this list to pick follow-up observations or briefs. Re-run the triage when pins change.


Class: a = fixable in our code from the golden data, b = reference-data limitation / decision, c = unclear (tried or too deep).

| Pin | Class | Outcome |
|---|---|---|
| rigging/j2-bend-dq (UNSUPPORTED, Spherical) | a | FIXED: Spherical = dual quaternion skin (already in code, only never wired in the test); exact |
| rigging/j2-bend-90-blended (UNSUPPORTED) | a | FIXED: Blended = (1-blend) linear + blend dual quaternion, blend 0.5 (bbox x min -57.337782 = mean of -47.50042 and -67.175144); exact |
| splines-2/formula-defaults (UNSUPPORTED, "needs expression library") | a | FIXED: forge-expr exists now; samples+1 points of (x_t,y_t,z_t) over [t_min,t_max]; exact |
| splines-2/cog-defaults | c | tooth outline not derived (spec 6.3); big multi-rule item, not attempted |
| rigging KNOWN_MISMATCH_FRAMES 15 | b | animation noise of the reference |
| ik reach-near/-side/-straight | b | reference stops at its 0.01 threshold, residual up to 0.04 (our result is exact) |
| ik four-joints, four-joints-2 | c | redundancy resolution; 3 hypotheses already spent in RIG.md 10 (CCD x2, Jacobian pseudo-inverse, FABRIK; Jacobian-transpose flow fits one of the two) |
| platonic bucky x4 | b | export is an irregular fixed table (layers with equal coordinates, radii 96.7..99.8, asymmetric z layers 91.857 ... -92.13); no formula |
| spline-mask/cap | b | near-cocircular Delaunay tie on 432 points decided by sub-float32 noise (GEOMETRY.md 6.1 R1) |
| caps-bevel/tri-extrude-ngon | b | regular hexagon, exact cocircular tie, same reason |
| caps-bevel/bevel-* (6), sweep-caps-bevel | c | studied (see below); numbering is shape-specific and the Round profile angles (29.2776 / 60.7224 deg) have no rule yet |
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
| boole r25-* (8) | c | new-point numbering, face order of the union family, split-polygon order and start corners all unknown at once (BOOLE.md 5a/5b); a strict match needs all four |
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
  The Round profile points sit on the circle at 29.2776 and 60.7224 deg (not 30/60, not a Bezier-sampled circle with k = 0.5523);
  no rule found, so no code was written.
- Outline: moved distance d*1.00013035 for both radius 200/d 20 and radius 100/d 10; a 100-chord polyline length of a k = 0.55228 Bezier quarter
  gives 1.00012999, close but not equal; no rule adopted.
