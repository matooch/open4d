# C · Objects: primitives, spline primitives, generators

## Scope (v1)

- [x] **Solid primitives:** plane, cube, sphere (all types), cylinder, cone, disc, torus,
  tube, capsule, pyramid. R25 defaults are applied (brief 07).
- [ ] **Remaining solid primitives:** Platonic (6 types), Oil Tank, Polygon, Figure,
  Landscape.
- [x] **Spline primitives (partial):** circle, rectangle, helix.
- [ ] **Remaining spline primitives:**
  - Arc (arc, sector, segment, ring), n-Side, Star, Flower, 4-Side, Cogwheel, Cycloid,
    Profile, Cissoid, Formula, Text;
  - Circle ellipse and ring options; Rectangle rounding;
  - every Intermediate Points mode (None, Natural, Uniform, Adaptive, Subdivided), with
    R25's per-spline defaults.
- [x] **Generators (partial):** Subdivision Surface, Boole, Symmetry, Array, Connect,
  Instance, Extrude, Lathe, Loft, Sweep.
- [ ] **Remaining generators:** Metaball, Atom Array, Spline Mask.
- [ ] **R25 generator options:**
  - Extrude Direction and Offset;
  - the shared Caps/Bevel tabs (Extrude, Lathe, Loft, Sweep);
  - Sweep rail, banking and end scale/rotation;
  - Loft linear interpolation;
  - Lathe partial angle and movement;
  - Boole all modes with R25 numbering;
  - Subdivision Surface types;
  - Symmetry planes and weld.
- [ ] **Scene objects as data:** Null (display options), Camera, Light, Environment,
  Floor, Sky. Their rendering is in K.

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O17 | [splines-2.md](../../parity/splines-2.md) | done (22) |
| O18 | [primitives-3.md](../../parity/primitives-3.md) | done (19) |
| O20 | [generators-3.md](../../parity/generators-3.md) | todo |
| O48 | [text.md](../../parity/text.md) | todo |
| O56 | [caps-bevel.md](../../parity/caps-bevel.md) | todo |
| O29 | [scene-defaults.md](../../parity/scene-defaults.md): camera, light, document | todo |
| O4/O5/O15 | defaults and cap re-checks | done |

## Implementation design

- **Primitives** follow the `round.rs` pattern: a `*_type()` descriptor plus a
  `build_*()` pure function in `forge-generators`, with parity cases in
  `tests/parity.rs`.
  - **Landscape:** its fractal noise is unlikely to match bit for bit. Use geometric and
    statistical comparison (same grid, height histogram within tolerance), and mark exact
    matching parity-pending.
  - **Figure:** the reference's figure is a fixed built-in model, so its mesh counts as an
    app asset. **Never copy or tabulate the golden mesh** (CLAUDE.md rule 2). Build our own
    procedural mannequin from boxes and capsules: same height and segment parameters, our
    own shape. Level C: bounding box and parameter behaviour only. The O18 golden is used
    just to check the size and the parameter ranges.
- **Spline primitives** live in `splines.rs`. Intermediate points are one shared module
  (`spline_sampling.rs`) that every spline uses.
- **Text:**
  - `ttf-parser` reads system fonts, with a bundled open font (DejaVu Sans, Bitstream Vera
    licence) as the default;
  - glyph outlines become Bezier segments, then go through spline sampling;
  - parity is level B: the reference uses the system font, so the observer records which
    font was used.
- **Metaball:** an implicit field from sphere and spline children plus marching cubes. It
  shares the mesher with F (volumes), so do F/080 first or factor the mesher out
  (`forge-geometry::isosurface`).
- **Caps/Bevel tab:** one shared module (`caps.rs`) used by Extrude, Lathe, Loft and
  Sweep: cap types (triangles, quads, N-gon, Delaunay, regular grid) and bevel shapes.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 035 | Platonic, Oil Tank, Polygon, Figure | – | O18 |
| 036 | Landscape | 063 (noise) | O18 |
| 037 | Spline primitives II | 038 | O17 |
| 038 | Intermediate Points modes and R25 per-spline defaults | – | O17, O4 |
| 039 | Text spline (fonts) | 038 | O48 |
| 040 | Metaball | 083 (mesher) | O20 |
| 041 | Atom Array, Spline Mask | 029 (spline booleans) | O20 |
| 042 | Sweep rail/banking/scale, Loft linear/multi, Lathe partial/movement, cap fixes | – | O20, O5, O15 |
| 043 | Boole R25 modes and numbering, Subdivision types, Symmetry planes | – | O20 |
| 044 | Shared Caps/Bevel tabs and Extrude R25 options (direction, offset) | 042 | O56 |
| 045 | Extrude R25 vertex order (brief 07 known mismatch) | 044 | O1 |
| 046 | Null display, Instance render modes, Connect weld options | – | O20 |
| 047 | Camera, Light, Environment, Floor, Sky as scene objects (data and defaults) | – | O29 |
| 048 | Generators in the clone and volume contexts (consumed-input rules) | 065, 081 | – |
| 049 | Create menu organisation (matches the reference menu grouping) | 148 | G1 |

## Done when
Every primitive, spline primitive and generator in the reference's Create menu (within
DECISIONS D13 scope) exists and has parity cases. Defaults match R25.
