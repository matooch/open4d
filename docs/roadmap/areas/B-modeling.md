# B · Modeling: polygon tools, selections, editable splines

## Scope (v1)

- [x] Polygon object, Make Editable, Current State to Object (brief 03).
- [x] Point/edge/polygon modes, selection, component transform (05, 06).
- [ ] **Tool framework:**
  - tools with settings descriptors (like node params);
  - live preview while dragging; apply as a command;
  - the Attribute panel shows the active tool;
  - Reset Values.
- [ ] **Tools:**
  - Extrude, Extrude Inner, Bevel (chamfer and solid; fixed, radial and proportional
    offset; subdivision; depth; mitering; endings; partial rounding; N-gon options);
  - Subdivide, Triangulate, Untriangulate, Reverse Normals, Align Normals, Optimize;
  - Weld, Melt, Dissolve, Split, Disconnect, Connect Points/Edges;
  - Mirror, Center Axis, Array command, Duplicate, Arrange, Randomize, Transfer.
- [ ] **Interactive tools:** Loop/Path Cut, Knife (line, plane, loop), Polygon Pen,
  Bridge, Stitch and Sew, Close Polygon Hole, Slide, Edge Cut.
- [ ] **Selection:**
  - Grow, Shrink, Convert (with tolerant/strict), Invert, Select Connected;
  - Loop, Ring, Fill, Phong-break, Path;
  - Set Selection (selection tags); soft selection with falloff modes; symmetry
    modeling.
- [ ] **Data tags:**
  - point, edge and polygon selection tags;
  - vertex map;
  - restriction (feeds deformers and tools).
- [ ] **Editable splines:**
  - interpolation types (linear, cubic, Akima, B-spline, Bezier);
  - point and tangent editing;
  - segments and closed flags.
- [ ] **Spline tools:** Chamfer, Create Outline, Cross Section, Spline Union / Subtract /
  Intersect / And / Or, Join Segment, Break Segment, Reverse Sequence, Set First Point,
  Project Spline, Subdivide Spline.
- [ ] **Snapping 2.0:** points, edges, polygons, axes, grid, workplane; quantize;
  axis/pivot edit mode.
- [ ] **Multi-object editing** and numeric component coordinates in the Coordinates
  panel.

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O6 | [modeling-tools.md](../../parity/modeling-tools.md) | done (14) |
| O11 §4 | [followups-1.md](../../parity/followups-1.md) | done (5) |
| O15 §2 | [rechecks-2.md](../../parity/rechecks-2.md) | done |
| O25 | [modeling-tools-2.md](../../parity/modeling-tools-2.md) | partial (16/24): interactive tools need the GUI |
| O26 | [selection-ops.md](../../parity/selection-ops.md) | partial (6/11): grow, loop, ring need the GUI |
| O46 | [spline-tools.md](../../parity/spline-tools.md) | todo |
| O3 | [make-editable.md](../../parity/make-editable.md) | done (follow-up brief 030) |
| G3 | [GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md) §G3: interactive modeling tools and selection behaviour | gui-only |

## Implementation design

- **`crates/modeling` (`forge-modeling`):** pure functions on `PolyMesh` plus a selection.
  - Each tool is `fn apply(mesh, sel, settings) -> Result<(PolyMesh, Selection), ToolError>`.
  - Reference vertex/face numbering is reproduced where the goldens prove it (level A).
    Otherwise use geometric parity with the numbering noted as a known mismatch.
- **Topology helpers in `forge-geometry`:** the half-edge mesh already exists. Add:
  - edge loops and rings;
  - boundary loops;
  - face-fan triangulation that keeps the reference's corner order (brief 033).
- **Tool framework in `forge-ui` (`tools.rs`):**
  - `Tool` trait: `settings() -> Vec<ParamDesc>`, `preview(state, drag) -> PolyMesh`,
    `commit(tx)`;
  - the Attribute panel renders `settings()` when a tool is active (O25 shows the
    reference does this too);
  - interactive tools (knife, loop cut) get viewport hit testing on edges and faces from
    `forge-viewport::picking`.
- **Commands:** every tool is a registry command (`mesh.extrude`, args = settings and
  selection), so the CLI and agents can model.
- **Splines:**
  - `forge-geometry::spline` already has the curve maths;
  - new node type `forge.spline` (an editable spline: points, tangents, segments,
    interpolation);
  - Make Editable on spline primitives produces it (O3 `spline` case: 4 points for a
    default circle);
  - spline booleans use a 2D polygon clipping library (`i_overlay`, MIT) on the spline's
    plane.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 020 | Tool framework + Extrude / Extrude Inner | 011 | O6, O11, O25 |
| 021 | Bevel (all modes) | 020 | O6, O25, O15 |
| 022 | Mesh commands: Subdivide, Triangulate, Untriangulate, Reverse, Optimize, Weld, Melt, Dissolve, Split, Disconnect, Connect, Mirror, Center Axis | 020 | O6, O11, O25 |
| 023 | Selection commands: grow, shrink, convert, invert, connected, loop, ring | 011 | O26, G3 |
| 024 | Interactive cutting: Loop/Path Cut, Knife, Slide | 020, 023 | G3 |
| 025 | Bridge, Stitch and Sew, Close Polygon Hole, Polygon Pen | 020 | G3 |
| 026 | Soft selection and symmetry modeling | 006, 023 | G3 |
| 027 | Selection tags, vertex map tags, restriction tags | 023 | O54 |
| 028 | Editable spline object and point/tangent editing | 003 | O3, O46 |
| 029 | Spline tools and spline booleans | 028 | O46 |
| 030 | Make Editable follow-ups (extrude caps as children, CSTO selection, UVW tag, splines) | 003, 028 | O3 |
| 031 | Duplicate, Arrange, Randomize, Transfer, Array command | 011 | O25 (array) + G3 |
| 032 | Snapping 2.0 and axis/pivot editing | 006 | G3 |
| 033 | N-gon handling: reference triangulation order, edge visibility | – | O6, O18 (dodecahedron) |
| 034 | Multi-object component editing, numeric component coordinates | 006 | G3 |

## Done when
S3 passes. Every O6/O25/O26 case is matched exactly or listed as a known mismatch with a
reason. Every tool is reachable from the menus, the command palette, the CLI and MCP.
