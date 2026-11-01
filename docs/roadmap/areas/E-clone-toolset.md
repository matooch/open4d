# E · Clone toolset (cloners, effectors, fracture, matrix, text, tracer)

The reference app calls this toolset by its brand name. In code and UI we say "clone
toolset", "cloner", "effector", "clone data" (DECISIONS, cleanroom gate).

## Scope (v1)

- [ ] **Clone data:** per-clone arrays for matrix, colour, weight, visibility, index, UV,
  time, flags. The cloner owns them, effectors modify them, and consumers read them
  (instancing, dynamics, tags).
- [ ] **Cloner:**
  - modes Object (vertex, edge, polygon centre, surface, volume, axis; selection; seeds),
    Linear (per step, end point, cumulative step), Radial, Grid Array (cubic, sphere,
    cylinder, object forms; fill; endpoint), Honeycomb;
  - clone order: iterate, random, blend, sort;
  - instance modes: instance, render instance, multi-instance;
  - fix texture;
  - nested cloners.
- [ ] **Effectors:** Plain, Random (random, gaussian, noise, turbulence, indexed), Step,
  Formula, Time, Delay, Target, Shader, Spline, Push Apart, Inheritance, Volume, Group,
  Reeffector.
- [ ] **Effector common:**
  - strength;
  - transform modes (relative, absolute, remap) and spaces (node, effector, object);
  - position, rotation, scale (uniform/absolute); colour modes; weight transform;
    U/V transform; visibility;
  - min/max;
  - deformation mode (object, point, polygon);
  - field falloff.
- [ ] **Generators:** Matrix, Fracture (explode segments, connected, sorted), Voronoi
  Fracture (point sources, seeds, inner faces, offset), Text (clone-toolset text:
  letters, words, lines as clones), Tracer (connect objects, vertices; trace modes;
  interval), Spline generator (simple, spline, turtle modes), Instance-over-time.
- [ ] **Tags:** clone selection tag, clone weight tag, clone cache (bake).
- [ ] **Viewport:** GPU instancing for clones, matrix display, clone indices display.

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O8, O11 §2, O14, O21, O22 | cloner and effectors I–III | done (66 clone sample files) |
| O31 | [clone-generators-2.md](../../parity/clone-generators-2.md) | todo |
| O50 | [clone-toolset-3.md](../../parity/clone-toolset-3.md): spline generator, tracer variants, instance-over-time, selection/weight tags, blend order | todo |
| O27 §target | Target tag | todo |

## Implementation design

- **`crates/clone-engine` (`forge-clone`):**
  - `CloneData { matrices: Vec<DMat4>, colors, weights, visible, uvw, index, time }`;
  - `Cloner::generate(mode, params, inputs, ctx) -> CloneData`;
  - `Effector::apply(&mut CloneData, falloff: &FieldList, ctx)`.
  - **Parity:** level B against `samples.json` (kind `clones`). The mesh golden checks
    instancing: the order and placement of instanced children.
- **Evaluation:** a cloner node evaluates its children once, as prototypes, then
  instances them by matrix. For OBJ export and Make Editable, the instances are expanded
  into geometry (the golden meshes show the expanded order).
- **Randomness:** the reference's random sequences can't be reproduced bit for bit unless
  the observations reveal the generator. Random-dependent cases are compared
  statistically (distribution moments, ranges), unless the implementer finds an exact
  rule within the three-hypothesis budget. Seeds are deterministic in our implementation
  either way.
- **Effector order:** in list order. O14 `two-effectors` and `two-effectors-swapped`
  prove that order matters.
- **Fracture and Voronoi:**
  - Voronoi cells come from 3D clipping of the input mesh by bisector planes (own code,
    exact predicates via `robust`);
  - inner faces are capped with planar polygons;
  - parity is geometric (level A-geometric), with the counts reported.
- **GPU instancing** lives in the viewport (K/144). Clone colours feed a per-instance
  colour buffer.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 065 | Clone data, cloner node, Linear / Radial / Grid / Honeycomb modes, instance modes | 050 | O8, O11, O21 |
| 066 | Object-mode distributions with seeds and selections | 065, 027 | O21 |
| 067 | Clone order (iterate, random, blend, sort), multiple children, nesting | 065 | O21, O50 |
| 068 | Effector framework + Plain (modes, spaces, colour, weight, min/max) | 065, 057 | O8, O14, O22 |
| 069 | Random effector (all modes) | 068, 063 | O8, O11, O22 |
| 070 | Step, Formula, Time, Delay, Target effectors | 068, 052 | O14 |
| 071 | Shader, Spline, Push Apart, Inheritance, Volume, Group effectors | 068, 125 | O22 |
| 072 | Effector falloff via field lists | 068, 059 | O22 |
| 073 | Matrix object | 065 | O31 |
| 074 | Fracture and Voronoi Fracture | 065, 033 | O31 |
| 075 | Clone-toolset Text | 039, 065 | O31, O48 |
| 076 | Tracer and the clone-toolset spline generator | 065, 028 | O31, O50 |
| 077 | Instance-over-time generator | 065, 088 | O50 |
| 078 | Clone selection and weight tags, clone cache | 065, 027 | O50 |
| 079 | Clone UI: effector lists, clone display, GPU instancing hookup | 065, 144 | G4 |

## Done when
S4 passes. Every clone sample case matches (level B) or has a statistical comparison with a
documented reason.
