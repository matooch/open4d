# Modeling tools (`crates/modeling`, package `forge-modeling`)

Sources: own design for the framework (docs/roadmap/areas/B-modeling.md, brief
docs/agents/tasks/020-tool-framework-extrude-inset.md); the Extrude and Extrude Inner rules
come from the parity observations docs/parity/modeling-tools.md (O6),
docs/parity/followups-1.md §4 (O11) and docs/parity/modeling-tools-2.md (O25), i.e. the cases
under `tests/golden/modeling-tools/`. Rules seen in one case only are marked
**parity-pending**. Mesh and selection types: GEOMETRY.md §1, §4; the polygon object and its
stored selection: EDITABLE.md §1–2; commands: COMMANDS.md.

## 0. Framework

A modeling tool is three things that share one definition:

1. **A pure function** in `forge-modeling`: `fn(&PolyMesh, &IndexSet, &Settings) ->
   Result<(PolyMesh, Selection), ToolError>`. It never touches a document.
2. **A descriptor** `ToolDesc { id, label, command, drag_param, params }`. `params` is a list
   of `ParamDesc` (id, kind, range, default), the same descriptors node types use, so the
   Attribute panel, the command schema, the defaults and range clamping all derive from it
   (CLAUDE.md "data-driven descriptors"). `tool_descs()` lists the tools in menu order.
3. **A registry command** (`mesh.extrude`, `mesh.extrude_inner`, `mesh.bevel`), so the CLI, RPC, MCP and
   Rhai can model without a window (COMMANDS.md §6).

The function type is `fn(&PolyMesh, &Selection, &Settings)`: Extrude and Extrude Inner read
`selection.polys`, Bevel reads the edges, else the polygons (§3.2). `tool_fn(id)` returns it.

`Settings` is a map from param id to `Value`. `Settings::from_json(desc, object)` starts from
the defaults, validates every given key with its `ParamDesc` (unknown key, wrong type,
non-finite value: error; numbers are clamped into the declared range), and `to_json()`
writes it back. Angles are plain degrees under the keys of the observation (`maximum_angle_deg`
...), not radian parameters, so command arguments, `case.toml` and the UI agree.

### 0.1 Command contract
Arguments: `{ "node"?: id, "polys"?: [index], "settings"?: {param: value} }`, no other keys.
`mesh.bevel` also takes `"edges"?: [id | [a, b]]` (edge ids of the mesh, or point pairs of an
existing edge) and `"points"?: [index]`. For Bevel, when any of `polys`, `edges`, `points` is
given only the given sets are used; when none is given the three stored sets are used.
Setting enums are option ids (`"chamfer"`); the schema lists them.

- `node`: a polygon object (`forge.polygon`); default: the active node. Anything else is
  `Failed`.
- `polys`: the polygons to work on; default: the stored `selection.polys` of the node (the
  selection override of the brief). Empty, or an index past the polygon count: `Failed`.
- `settings`: the tool's params; absent keys keep the factory defaults (not the values of an
  earlier run). Unsupported option values are `Failed("<tool>: unsupported ...")` and change
  nothing.
- One run is one `Document::transact` labelled with the tool name: it writes the node's `mesh`
  and its three stored selection sets (`selection.polys` = the resulting polygons, §1.5;
  `selection.points` and `selection.edges` cleared because their indices are stale) in the
  same step, so one undo restores mesh and selection. Result:
  `{ "node", "points", "polys", "selected": [index] }`.
- Edit mode and the object selection are not changed.

### 0.2 UI framework (`forge-ui`, `tools.rs`)
```rust
trait Tool {
    fn id(&self) -> &'static str;          // "extrude"
    fn label(&self) -> &'static str;
    fn command_id(&self) -> &'static str;  // "mesh.extrude"
    fn settings(&self) -> Vec<ParamDesc>;  // the attribute panel rows
    fn drag_param(&self) -> &'static str;  // the setting a viewport drag edits ("offset")
    fn preview(&self, mesh: &PolyMesh, sel: &Selection, s: &Settings)
        -> Result<PolyMesh, String>;       // never writes the document
}
```
- `ToolState { tool, values: Settings, drag: Option<f64> }` lives in `EditorState` (editor
  state, not document state: not saved, not undoable). `activate(id)` loads the factory
  defaults; `deactivate()`.
- **Attribute panel:** while a tool is active the panel shows the tool (its label, one row per
  setting built by the same widget code as node params, a "Reset Values" button, an "Apply"
  button). With no tool active it is unchanged. (O25 shows the reference does this too.)
- **Reset Values** restores the factory defaults of the descriptor (the observed defaults,
  §1.1, §2.1).
- **Preview:** `preview` runs the pure function on the target's stored mesh and polygon set
  with the current values; the viewport draws it in place of the stored mesh while a tool is
  active (live). Realtime Update off previews only on release. The document is untouched.
- **Interactive drag:** with a tool active, a primary drag that starts in the viewport (on
  no gizmo handle) sets the `drag_param` to `distance = vertical pixels dragged × scale`
  (up is positive; scale = the target's bounding-box diagonal / viewport height in pixels,
  so one screen height is about one object diagonal). The release applies.
- **Apply** (the button, or a drag release) builds the arguments from the values and runs the
  tool's command through `execute`: exactly **one** `mesh.extrude` command, one undo step.
  There is no intermediate document write during the drag. After applying, the tool stays
  active with the same values (the reference keeps the tool), and the resulting polygons are
  the new selection.
- Menu: Mesh > Extrude, Extrude Inner, Bevel (activate the tool). The preview and Apply use
  the stored selection (`tool_target` holds all three sets); enum settings are combo boxes. The commands are also listed with
  their ids in the generated reference (docs/manual/reference).
- **parity-pending:** the drag scale and the keeping of values after Apply are own design; the
  reference's interactive behaviour is a GUI observation (G3).

## 1. Extrude (`mesh.extrude`)

### 1.1 Settings (O6 "Extrude" table; defaults after Reset Values)
| id | label | kind | default | range |
|---|---|---|---|---|
| `offset` | Offset | float | 5 | ±1e20 |
| `offset_variation` | Offset Variation | float | 0 | 0..1e18 |
| `subdivision` | Subdivision | int | 0 | 0..1e6 |
| `create_ngons` | Create N-gons | bool | off | |
| `preserve_groups` | Preserve Groups | bool | on | |
| `create_caps` | Create Caps | bool | off | |
| `maximum_angle_deg` | Maximum Angle | float | 91 | 0..180 |
| `rotation_deg` | Rotation | float | 0 | ±1e20 |
| `quantize_rotation` | Quantize Rotation | bool | on | |
| `quantize_angle_deg` | Quantize Angle | float | 5 | 0..180 |
| `bevel` | Bevel | float | 5 | ±1e20 |
| `bevel_variation` | Bevel Variation | float | 0 | 0..1e18 |
| `realtime_update` | Realtime Update | bool | on | |

`subdivision` is capped at 1e6 here instead of the observed 1e20 so the resulting mesh stays
addressable. `realtime_update` only affects the UI preview.

### 1.2 Groups (case: `extrude-two`, `extrude-two-separate`)
- Preserve Groups **off**: every selected polygon is its own group.
- Preserve Groups **on**: selected polygons that share an edge and whose unit normals differ by
  at most `maximum_angle_deg` are one group (union-find over shared edges). **parity-pending**
  for the angle test: every golden group is flat (angle 0), so only "edge-connected selected
  polygons form a group" is proven (`extrude-two` vs `-separate`). The angle test is the
  public manual's description of Maximum Angle.
- Groups are processed in ascending order of their smallest polygon index; within a group the
  polygons are in ascending index order.

### 1.3 New points (cases: all)
- For each group, one new point per point of the group, in **ascending original point index**
  (`extrude-two`: copies of 1,3,5,7,9,11 are points 12..17; `extrude-two-separate`: group 6 gives
  12..15, group 7 gives 16..19, so a point shared by two groups is copied twice and the file
  keeps both).
- The original points stay where they are and keep their index. The new points are appended
  after all existing points, group after group.
- Position: `p + offset · d`, `d` the unit normal of the group's polygons at `p` (the
  normalised sum of the unit polygon normals of the group's polygons that use `p`; for a flat
  group this is the polygon normal). Cases `extrude-top` (+50: y 150) and `extrude-top-negative`
  (−50: y 50, same faces). **parity-pending** for non-flat groups.

### 1.4 Faces (all extrude cases)
- Polygons keep their index. Each selected polygon is **replaced in place** by its cap: the same
  corners with every point replaced by its new point (`extrude-top`: face 4 = `8 11 10 9`).
  Unselected polygons are unchanged, including the bottom faces after the top ones in
  `extrude-two`.
- A **boundary edge** of a group is an undirected edge used by exactly one polygon of the
  group. Interior (shared) edges make no side face (`extrude-two`: the edge between polygons 6
  and 7 has none).
- Side faces are appended after all original polygons, group after group; within a group in
  the order the boundary edges are met walking the group's polygons in ascending index and
  each polygon's corners in order (`extrude-two`: edges 1→11, 11→9, 3→1, 9→7, 7→5, 5→3).
  For a boundary edge `a→b` with new points `a'`, `b'` and Subdivision 0 the face is
  `[a', a, b, b']` (`extrude-top`: `8 1 7 11`). Winding follows the source polygon, so a
  negative offset keeps the faces (`extrude-top-negative`).

### 1.5 Subdivision (case: `extrude-subdiv`, N = 2)
Each side is cut into `N + 1` rows. Between `a'` and `a` there are `N` intermediate points
`c_1(a) … c_N(a)` at `a' + (a − a')·k/(N+1)`, `k = 1..N`, **top (cap side) first**
(`133.33`, then `116.67`).
- Column points are numbered after the group's new points. Columns are created in the order of
  the boundary edges, one column per edge start point `a` (each point once): `extrude-subdiv`
  columns of points 1, 7, 5, 3 (the polygon's corner order, not ascending order), two points
  each: 12,13 / 14,15 / 16,17 / 18,19. **parity-pending** for groups with several
  boundary loops (a start-point-only order is the simplest rule that fits; ascending order of
  the new points does not fit `extrude-subdiv`). A point that is only ever an edge end gets its
  column after all start-point columns.
- Rails: `L = [a', c_1(a)…c_N(a), a]`, `R = [b', c_1(b)…c_N(b), b]`. Row `j` (0 = cap side,
  `0..=N`) is `[L_j, L_{j+1}, R_{j+1}, R_j]`, except **inner rows** (`1 ≤ j ≤ N−1`) which are
  the same cycle started at `R_{j+1}`: `[R_{j+1}, R_j, L_j, L_{j+1}]` (`extrude-subdiv` face 7 =
  `15 14 12 13`, faces 10, 16 likewise; first and last rows are plain). **parity-pending**: the
  data have N = 2 only (one inner row); "inner rows are rotated" is the simplest rule that
  fits N = 2 here and N = 1 for `inset-subdiv` (no inner row). Rows of one edge come
  consecutively, top to bottom.

### 1.6 Other settings
- `create_ngons`: no effect seen. `extrude-ngon` equals `extrude-two` face for face, even
  though two of the side faces lie in one plane (the wall `11→9`, `9→7`) and are not merged.
  Ignored; **parity-pending**.
- `bevel`, `bevel_variation`: `extrude-bevel` (Bevel 10, Subdivision 0) equals `extrude-top`
  (Bevel 5): the value has no effect at Subdivision 0 without caps. Ignored in that situation;
  **parity-pending** for Subdivision > 0 (not observed).
- `realtime_update`, `quantize_rotation`, `quantize_angle_deg`: no geometry effect without a
  rotation.
- **Unsupported** (`ToolError::Unsupported`, never guessed; no golden case): `offset_variation
  ≠ 0`, `rotation_deg ≠ 0`, `create_caps` on.
- Resulting selection: the replaced polygons (the caps), same indices as the input.

## 2. Extrude Inner (`mesh.extrude_inner`)

### 2.1 Settings (O6 "Extrude Inner (Inset)" table)
`offset` (default 5), `offset_variation` (0), `subdivision` (0), `create_ngons` (off),
`preserve_groups` (on), `maximum_angle_deg` (**89**), `realtime_update` (on); labels, kinds and
ranges as in §1.1. The tool has no caps, rotation or bevel settings.

### 2.2 Rules
Same group rules as §1.2, same face rules as §1.4 and subdivision rules as §1.5 (cases
`inset-top`, `inset-two`, `inset-ngon`, `inset-subdiv`), with these differences:

- **New points** are made only for the *boundary points* of a group (points of a boundary
  edge), in ascending original index (`inset-two`: copies of 1,3,5,7,9,11 = 12..17). A group
  point not on the boundary keeps its index and position and the cap refers to it
  (**parity-pending**: no golden group has an interior point).
- **Position**: the point moves inwards in the polygon plane so that it lies `offset` away
  from both adjacent boundary edges (a mitre). With `i₁`, `i₂` the unit inward directions of
  the boundary edge arriving at and leaving the point (`n × d`, `n` the unit normal of the
  polygon owning the edge, `d` the edge direction):
  `p' = p + offset · (i₁ + i₂) / (1 + i₁·i₂)`. A straight boundary gives `p + offset · i`
  (`inset-two`: point 3 → (0, 100, −80)); a right-angle corner moves `offset` along both edges
  (`inset-top`: (−100, −100) → (−80, −80)). If `1 + i₁·i₂` is below 1e-9 (a hairpin) the move
  is `offset · i₁`. A boundary point with other than two boundary edges moves by `offset ·`
  the normalised sum of its inward directions (**parity-pending**).
  Positive offset is inward, negative outward.
- **Subdivision**: the intermediate points lie on the straight line from the new point to the
  original one (`inset-subdiv`: (−90, 100, −90) between (−80, …) and (−100, …)). The two
  rows of N = 1 are plain (`8 12 13 11`, `12 1 7 13`).
- `create_ngons`: no effect seen (`inset-ngon` equals `inset-two`). Ignored.
- **Unsupported**: `offset_variation ≠ 0`.
- Resulting selection: the inset (cap) polygons.

## 3. Bevel (`mesh.bevel`)

Sources: the Bevel table of docs/parity/modeling-tools.md (O6), the cases `bevel-poly`,
`bevel-edge`, `bevel-edge-sub`, `bevel-edges-top` (O6), `bevel-solid`, `bevel-proportional`,
`bevel-mitering-uniform` (O25), `bevel-poly-extrusion0` (docs/parity/followups-1.md §4) and
the Extrusion field confirmed there. The file counts in the observations are the exporter's
(it joins the app's triangulated n-gon pieces back into one n-gon); our meshes hold the
n-gons directly.

### 3.1 Settings (O6 "Bevel" table; defaults after Reset Values)
| id | label | kind | default | range / options |
|---|---|---|---|---|
| `bevel_mode` | Bevel Mode | enum | chamfer | chamfer, solid |
| `offset_mode` | Offset Mode | enum | fixed_distance | fixed_distance, radial, proportional |
| `offset` | Offset | float | 10 | ±1e20 |
| `subdivision` | Subdivision | int | 0 | 0..1e6 |
| `depth` | Depth | float | 1 | ±1e18 |
| `limit` | Limit | bool | on | |
| `shape` | Shape | enum | round | round, user, profile |
| `tension` | Tension | float | 1 | ±1e18 |
| `mitering` | Mitering | enum | default | default, uniform, radial, patch |
| `ending` | Ending | enum | default | default, extend, inset |
| `partial_rounding` | Partial Rounding | enum | none | none, full, convex |
| `corner_ngons` | Corner N-gons | bool | off | |
| `rounding_ngons` | Rounding N-gons | bool | off | |
| `phong_break_rounding` | Phong Break Rounding | bool | off | |
| `phong_break_miters` | Phong Break Miters | bool | off | |
| `realtime_update` | Realtime Update | bool | on | |
| `extrusion` | Extrusion | float | 10 | ±1e20 |

`extrusion` is the field the reference tool hides in edge mode (followups-1 §4): it is the lift
of the polygon mode and has no effect in edge mode. `subdivision` is capped at 1e6.

### 3.2 Mode and selection
The mode is the selection that is given: edges, else polygons, else points (when several are
non-empty, in that order). Edge ids are indices into `PolyMesh::edges()` (GEOMETRY.md §1); the
command also accepts point pairs `[a, b]`. The result selects the new polygons: the cap
polygons in polygon mode, every strip polygon in edge mode.
**Point mode is `Unsupported`**: no golden case.

### 3.3 Polygon mode (cases: `bevel-poly`, `bevel-poly-extrusion0`)
An inset (§2.2, same groups, mitre and numbering) plus a lift. Each new point is the inset
position of §2.2 plus `extrusion · d`, `d` the group direction of §1.3 (the polygon normal for
a flat group). The cap replaces the polygon in place, the side faces are appended per boundary
edge as `[a', a, b, b']`, exactly as Extrude (§1.4).
- `bevel-poly`: offset 20, extrusion 10: points 8..11 = (∓80, 110, ∓80), in ascending order of
  the original point (1, 3, 5, 7); cap 4 = `8 11 10 9`; sides `8 1 7 11`, `11 7 5 10`,
  `10 5 3 9`, `9 3 1 8`.
- `bevel-poly-extrusion0`: the same with `extrusion` 0: the cap stays at y = 100.
- Preserve Groups (on) and Maximum Angle (89) are the hidden values of the reference tool;
  they are fixed here. Interior points of a group (none in the data) move by the lift only
  (**parity-pending**).
- `subdivision > 0`, `bevel_mode` solid, `offset_mode` other than fixed: `Unsupported`.

### 3.4 Edge mode, chamfer (cases: `bevel-edge`, `bevel-edge-sub`, `bevel-edges-top`,
`bevel-mitering-uniform`, `bevel-proportional`)

**Supported topology** (anything else is `Unsupported`, never guessed): every selected edge
has exactly two polygons with opposite directions; every vertex of a selected edge is used by
exactly three polygons (a valence-3 vertex) and has one or two selected edges (three is a
corner patch, no data).

**Edge direction and sides.** Edges are processed in ascending order of their lowest half-edge
index, where the half-edge of corner `k` of polygon `p` is `Σ_{q<p} |poly q| + k`
(`bevel-edges-top`: 1, 5, 9, 13). `a -> b` is the direction of that half-edge. `fB` is the
polygon that holds `a -> b`, `fA` the one that holds `b -> a`. `A` denotes the side of `fA`,
`B` the side of `fB`. Strips run from `A` (level 0) to `B` (level `N + 1`), `N = subdivision`
(`bevel-edge`: `fB` = polygon 0 holding 1->3, `fA` = the top).

**Profile.** `u_A`, `u_B` are the unit in-plane directions from the edge into `fA`, `fB`
(`n × d` for the polygon normal `n` and the direction `d` of the polygon's own half-edge).
`d_off` = `offset` (fixed distance, §3.1) or `offset · |b − a|` (proportional:
`bevel-proportional`, 0.3 · 200 = 60). In the cross-section at `a`: `A = a + d_off u_A`,
`B = a + d_off u_B`, control point `K = a`, weight `w = cos(φ/2)` with `cos φ = −u_A·u_B`.
Level `j` (`t = j/(N+1)`) is the rational quadratic (a circle arc)
`Q_j = ((1−t)² A + 2w t(1−t) K + t² B) / ((1−t)² + 2w t(1−t) + t²)`. The golden points
(97.445208, −89.780834) and (89.780834, −97.445208) of `bevel-edge-sub` are exactly this at
`t = 1/3, 2/3` (they lie on the radius-20 circle but not at equal angles; equal angles would
give 97.32), and the 45-degree point (94.142136) of `bevel-edges-top` is `t = 1/2`.
The level line of the strip is `Q_j + s · (b − a)/|b − a|`.
**Tension 1 and Depth 1 only**; other values are `Unsupported` (no data). **parity-pending**
for angles other than 90 degrees (all data are cube edges).

**End points.**
- A vertex with one selected edge: the cap polygon is the polygon `(n1 -> v)` where `n1` is
  the next point after `v` in the polygon that holds `x -> v` (`x` the other end of the edge).
  Level point `j` is the intersection of the level line with the plane of the cap polygon
  (`bevel-edge`: x = ±100). The two ends are `p1` on edge `v–n1` and `p2` on `v–n2`
  (`n2` = next point after `v` in the polygon that holds `n1 -> v`); `p1` = level 0 (`A`) when
  `v = a`, level `N+1` (`B`) when `v = b`.
- A vertex with two selected edges `x`, `y` (they share the polygon `f0` between them):
  the corner points `C_0 … C_{N+1}` (`C_0` on `f0`) are the mid point of the closest approach
  of the level lines of the two strips at matching levels (level `j` of a strip whose `A`
  side is `f0`, level `N+1−j` otherwise); for the cube corners the lines intersect. The
  points are shared by both strips (`bevel-edges-top`: (−94.142, 94.142, −94.142)).

**Point order (new points).** Old points keep their slot unless they are removed; the new
points are created in two passes and appended, then the old points of every vertex of a
selected edge are removed one by one in ascending index with a **swap-remove** (the last point
moves into the freed slot). This is the only order that explains why the golden files have
new points at the slots of the removed points (`bevel-edge`: slots 1, 3 hold the points of
the x = 100 end).
1. End points. Walk the selected edges in the order above, `a` before `b`; the first time a
   vertex is met, create: one selected edge: `p1`, `p2`; two selected edges: `C_0`, `C_{N+1}`
   (`bevel-edge`: A1, B1, B3, A3 = 8..11; `bevel-edges-top`: top, vertical per corner =
   8..15 for the corners 1, 3, 5, 7).
2. Intermediate points, same walk: when a vertex is met first, create its levels `1..N` in the
   order `N … 1` if it is `a` of the edge, `1 … N` if it is `b` (`bevel-edge-sub`: slot 12, 13
   = the `a` end, then 14, 15; `bevel-mitering-uniform`: 16..23).

  After the swap-remove, the final index of a new point follows from the count (`bevel-edge`:
  A3 moves into slot 1, B3 into slot 3; `bevel-edges-top`: mids of corners 7, 5, 3, 1 end in
  slots 1, 3, 5, 7).

**Polygons.** Existing polygons keep their index; each corner at a vertex of a selected edge
is replaced (consecutive duplicates collapse):
- vertex with one selected edge `x`: the polygon that holds `x -> v` gets the end point on its
  outgoing edge; the polygon that holds `v -> x` gets the end point on its incoming edge; the
  cap gets the whole profile from the end point on its incoming edge to the one on its
  outgoing edge (`bevel-edge-sub` polygon 1: `2 10 1 3 11 5 4`, polygon 3: `6 7 8 13 12 9 0`).
- vertex with two selected edges: the polygon between them gets `C_0`; the two polygons with
  the unselected edge get `C_{N+1}`.
New polygons (strips) are appended: row `j` of an edge (`0 … N`) is
`[Q_j(a), Q_j(b), Q_{j+1}(b), Q_{j+1}(a)]` for `j = 0` and
`[Q_j(b), Q_{j+1}(b), Q_{j+1}(a), Q_j(a)]` for `j ≥ 1` (the same cycle started one corner
later: `bevel-edge-sub` faces 7, 8; `bevel-mitering-uniform` 10..17). Order: row 0 of every
edge in edge order, then, edge by edge, rows `1 … N` (`bevel-mitering-uniform`: faces 6..9 are
the row 0 of the four edges, 10, 11 are rows 1, 2 of the first edge).
**parity-pending**: rows `j ≥ 1` for `N ≥ 3` and the row order for more than three rows.

### 3.5 Edge mode, solid (case: `bevel-solid`)
`bevel_mode` solid keeps the old points and adds the same four end points as the chamfer, in
the same order (8..11), no point is removed. Only one selected edge per vertex, `subdivision`
0 and one selected edge per polygon are supported (`Unsupported` otherwise).
- The polygons `fA` and `fB` are cut by the line through their two new points. Let the cut
  edges be the polygon's edges next to the beveled edge, `X` on the earlier one, `Y` on the
  later one in the polygon's walk (indices `i < j`): the part that holds the polygon's first
  corner `c_0 … c_i, X, Y, c_{j+1} …` keeps the index, the other part `X, c_{i+1} … c_j, Y` is
  appended. In the case the top polygon `1 7 5 3` becomes the strip `1 8 11 3` in place and
  `8 7 5 11` is appended, the back polygon `0 1 3 2` becomes `0 9 10 2` and `9 1 3 10` is
  appended. Appended parts are in ascending order of the original polygon.
- The cap polygons get `[incoming end point, v, outgoing end point]` (`2 10 3 11 5 4`).

### 3.6 Settings without an effect or without data
- `limit`: no effect seen (all offsets are inside the geometry); ignored. **parity-pending**.
  (The note of `bevel-proportional` that offset 30 gave a degenerate mesh is consistent with
  Limit not clamping, but it does not prove it.)
- `phong_break_rounding`, `phong_break_miters`: change normals, not points or polygons;
  ignored.
- `mitering` default and uniform give the same points and polygons for the corner of
  `bevel-mitering-uniform` and `bevel-edges-top` (a cube corner with two beveled edges and
  Subdivision 1, 2); they are treated alike. **parity-pending**: the difference, if any, is
  not visible in a right-angle corner.
- **Unsupported** (no golden case): `shape` user/profile (out of scope, v1.1), `offset_mode`
  radial, `tension ≠ 1`, `depth ≠ 1`, `mitering` radial/patch, `ending` ≠ default,
  `partial_rounding` ≠ none, `corner_ngons`, `rounding_ngons`, `bevel_mode` solid together
  with `subdivision > 0` or in polygon mode, point mode, open or non-manifold edges, vertices
  that are not valence 3, three selected edges at a vertex.

## 4. Mesh commands (`mesh.subdivide` ... `mesh.center_axis`)

Sources: the O6 cases `subdivide-1`, `triangulate`, `reverse-normals`, `optimize`
(docs/parity/modeling-tools.md), `optimize-real` (docs/parity/followups-1.md) and the O25 cases
`subdivide-smooth`, `subdivide-2`, `untriangulate`, `weld-two-points`, `melt-two`, `split-top`,
`disconnect-top`, `connect-points`, `mirror-x`, `center-axis` (docs/parity/modeling-tools-2.md);
where there is no case the rule is our own design, marked **parity-pending**. All commands use
the framework of §0 (descriptor, pure function, one registry command, one undo step) and the
command contract of §0.1 with these differences:

- Where the table says "empty = all", a command given no polygons works on every polygon of the
  mesh (the reference commands act on the whole object without a selection). Indices past the
  polygon count are `Failed`.
- `mesh.weld`, `mesh.connect_points` and `mesh.dissolve` also take `edges` and `points` (as
  `mesh.bevel`, §0.1); the others take `polys` only.
- Removal of points **and of polygons** is always the **swap-remove** of §3.4: the items to
  remove are taken in ascending original index; each is replaced by the last item of the list
  (`weld-two-points`: removing point 1 moves point 7 into slot 1; `melt-two`: removing polygon 7
  moves the last polygon 9 into slot 7, so the bottom polygons come out as 9, 8). Polygons that
  lose corners keep their index.
- A polygon that has fewer than three distinct corners after a merge is dropped; consecutive
  duplicate corners (also across the wrap) collapse.
- Two commands do more than replace the mesh: `mesh.split` (a second object) and
  `mesh.center_axis` (the object position). Their pure functions are `split_copy` and
  `center_axis`; the registry function of the tool list (`tool_fn`) is the identity on the mesh
  for `split` and the re-centred mesh for `center_axis`, so the preview and the UI stay uniform.

Descriptors (id, label; the settings have no observed defaults unless stated):

| command | tool id | settings (default, range) | works on |
|---|---|---|---|
| `mesh.subdivide` | `subdivide` | `subdivisions` int (1, 1..8), `hyper` bool (off) | polys, empty = all |
| `mesh.triangulate` | `triangulate` | none | polys, empty = all |
| `mesh.untriangulate` | `untriangulate` | `angle_deg` float (1, 0..90), `ngons` bool (off) | polys, empty = all |
| `mesh.reverse_normals` | `reverse_normals` | none | polys, empty = all |
| `mesh.optimize` | `optimize` | `tolerance` float (0.01, 0..1e20), `optimize_points` (on), `optimize_polygons` (on), `delete_unused_points` (on) | the whole mesh |
| `mesh.weld` | `weld` | none | points |
| `mesh.melt` | `melt` | none | polys |
| `mesh.dissolve` | `dissolve` | none | edges, else points, else polys |
| `mesh.split` | `split` | none | polys |
| `mesh.disconnect` | `disconnect` | `preserve_groups` bool (on) | polys |
| `mesh.connect_points` | `connect_points` | none | points |
| `mesh.mirror` | `mirror` | `plane_point` vec3 (0,0,0), `plane_normal` vec3 (1,0,0), `duplicate` bool (off), `weld` bool (on), `tolerance` float (0.01, 0..1e20) | polys, empty = all |
| `mesh.center_axis` | `center_axis` | `mode` enum `bounding_box` / `point_average` (bounding_box) | the object |

The defaults of Triangulate, Untriangulate, Optimize, Weld, Melt, Split, Connect and Center Axis
could not be read in O6/O25 ("TODO: not readable"). The values above are our own, except
`optimize-real`'s Tolerance 0.01 with the three switches on, which is what the probe set (the
reference's own defaults are unknown) and `preserve_groups`, `plane_point`, `plane_normal`,
`duplicate` (off), `subdivisions`, `hyper` which the cases set. Hyper defaults to off because `subdivide-1` (O6) has no settings table and its description says smoothing off. **parity-pending** for all of
them. `tool_descs()` lists them after Bevel in this order. These tools have no drag parameter
(`drag_param` is empty), so a viewport drag does not apply them.

### 4.1 Subdivide (cases `subdivide-1`, `subdivide-smooth`, `subdivide-2`)
The command's order is **not** the Subdivision generator's (SUBDIVISION.md §2) for level 1 and
differs again at level 2. For an all-quad mesh, each quad `[c0, c1, c2, c3]` is a lattice of
`2^L x 2^L` cells (`L` = `subdivisions`): lattice point `(R, C)` with `R` along `c0 -> c3`,
`C` along `c0 -> c1`, `0 <= R, C <= 2^L`, corners `c0 = (0,0)`, `c1 = (0, 2^L)`,
`c2 = (2^L, 2^L)`, `c3 = (2^L, 0)`.
- **Points**: the originals, same order; then the edge points: for every edge in
  `PolyMesh::edges()` order, in the direction it is first met in a polygon walk (corner `k` to
  `k+1`), its `2^L - 1` inner points from the first to the second end; then, polygon by
  polygon, the inner lattice points `R = 1 .. 2^L-1` (outer), `C = 1 .. 2^L-1` (inner).
  `subdivide-1`: points 8..19 are the 12 edge points, 20..25 the face points. `subdivide-2`:
  8..43 are three points per edge (edge 3->2 gives y = 50, 0, -50), 44..97 nine per polygon
  (polygon 0: x-major, (-50,-50), (-50,0), ...).
- **Polygons**: the cells `[L(R,C), L(R,C+1), L(R+1,C+1), L(R+1,C)]`, row by row (`R` outer, `C`
  inner). The **last cell** (`R = C = 2^L-1`, next to `c2`) **replaces the polygon in place**;
  the other cells are appended, polygon after polygon, in that order. `subdivide-1`: face 0 =
  `20 9 3 10` (cell next to `c2`), faces 6, 7, 8 = `0 8 20 11`, `8 1 9 20`, `11 20 10 2`.
  `subdivide-2`: face 0 = `52 13 3 14`, faces 6.. = `0 8 44 19`, `8 9 45 44`, ... .
- **Positions**: Hyper **off** is bilinear (edge points at equal fractions, face points the
  bilinear patch); Hyper **on** is Catmull-Clark (SUBDIVISION.md §2) applied `L` times
  (`subdivide-smooth`: corners (-55.56, ...) = 100 x 5/9, edge points (-75, 0, -75), face
  points unchanged). The export is therefore rounded; its bounding box stays +-100 only because
  the face points do not move (the observation's "not rounded" reading was wrong). Smoothing is
  part of the mesh, not a display effect. The positions are those of the generator
  (`subdivide_levels`), which is also how the lattice is read (each generator cell tells its
  four lattice points).
- Meshes with a polygon that is not a quad (**parity-pending**, no data): `L` single steps, each
  with the points ordered originals, edge points (`edges()` order), face points, and the polygon
  `[c0 .. c_(n-1)]` replaced in place by `[c_(n-1), E(n-1,0), F, E(n-2,n-1)]`, the quads of
  corners `0 .. n-2` (`[c_k, E(k,k+1), F, E(k-1,k)]`) appended in order.
- A polygon selection that is neither empty nor all polygons is `Unsupported` (no data for
  partial subdivision). Result selection: all polygons.

### 4.2 Triangulate (case `triangulate`)
Every selected polygon with n > 3 corners becomes n-2 triangles, a fan from its first corner:
`[p0, p_i, p_(i+1)]`. The **first triangle keeps the polygon's index**, the others are appended
after all existing polygons, polygon after polygon (`triangulate`: quad `[0,1,4,3]` becomes
face 0 `0 1 4` and face 24 `0 4 3`). Points are unchanged. A polygon whose fan has a triangle
turned against the polygon normal (a concave polygon) uses `triangulate_poly` (ear clipping)
instead, the first ear in place. **parity-pending** for n > 4 and concave polygons. Triangles
stay triangles. Result selection: the resulting triangles.

### 4.3 Untriangulate (case `untriangulate`)
Pairs of triangles that share an edge and are coplanar (normals within `angle_deg`) are merged
into a quad when the quad is strictly convex.
- Triangles are visited in ascending index; an unmerged triangle takes the unmerged candidate
  neighbour with the **lowest index** (`untriangulate`: triangle 0 `[0,1,4]` pairs with 24
  `[0,4,3]` and not with 26 `[1,5,4]`, which would also be a convex coplanar quad).
- The merged polygon takes the index of the lower triangle (the other is removed, later indices
  close up) and is the boundary loop of the pair started at the first boundary half-edge of the
  lower triangle (the `merge` rule of §4.5): `[0,1,4]` + `[0,4,3]` gives `[0,1,4,3]`.
- `ngons` on: instead of pairs, every edge-connected coplanar group of triangles (same
  tolerance) with one simple boundary loop becomes one polygon (loop rule of §4.5; groups with
  holes or several loops stay as they are). **parity-pending**: the case has `ngons` off, the
  reference defaults (angle, n-gons) are unknown.
- Polygons that are not triangles are copied. Result selection: the merged polygons.

### 4.4 Reverse Normals (case `reverse-normals`)
Each selected polygon is reversed **keeping its first corner**: `[p0, p1, ..., p_(n-1)]`
becomes `[p0, p_(n-1), ..., p1]` (`reverse-normals`: `[1,7,5,3]` becomes `[1,3,5,7]`). Result
selection: the same polygons.

### 4.5 Merging polygons: Melt, Dissolve (case `melt-two`)
`merge(mesh, groups)` replaces each group of polygons by one polygon, its **boundary loop**:
the directed edges `a -> b` of the group's polygons whose reverse `b -> a` is not in the group,
chained `a -> b -> ...`. The loop starts at the origin of the first boundary edge met walking the
group's polygons in ascending index and each polygon's corners in order (`melt-two`: polygons 6
`[1,11,9,3]` and 7 `[3,9,7,5]` give `[1,11,9,7,5,3]`). The merged polygon replaces the lowest
polygon of the group, the others are removed (swap-remove). Points left without a polygon are removed
(swap-remove). A group whose boundary is not one simple loop (a hole, two loops, a vertex with
two outgoing boundary edges, inconsistent winding) is `Unsupported`.
- **Melt** (polys): groups are the edge-connected components of the selected polygons; a group
  of one polygon is left unchanged, and no group of two or more is `Invalid`. `melt-two`: the two top polygons become one 6-gon, the two
  inner points stay because the side polygons use them, so the point count stays 12.
  **parity-pending**: removal of interior points, the one-polygon case (the reference might
  delete it).
- **Dissolve** (own design, **parity-pending**; the O25 case `dissolve-edge` did not run):
  - edges selected: every selected edge used by exactly two polygons joins them; the groups are
    the connected unions; merged as above. Edges with other counts are ignored; no usable edge is
    `Invalid`.
  - else points selected: all polygons around a point join into one group; the point itself is
    dropped from the loop and removed. A point used by one polygon of at most three corners is
    `Unsupported`.
  - else polygons: the same as Melt.

### 4.6 Weld (case `weld-two-points`)
All selected points (at least two) become one point at their **average** (the case: points 0, 1
become (-100, 0, -100)). The merged point keeps the **lowest index**, the other points are
removed by swap-remove (point 7 moves to slot 1). Polygons are remapped (`[0,1,3,2]` becomes
`[0,3,2]`, `[6,7,1,0]` becomes `[6,1,0]`), polygons with fewer than three distinct corners are
dropped. No tolerance: the case welds points 200 apart. Selection of edges or polygons instead of
points is not used (`EmptySelection`).

### 4.7 Split and Disconnect (cases `split-top`, `disconnect-top`)
- **Split** (`split_copy`): the selected polygons are copied into a new object; the source keeps
  **all** of its polygons (`split-top`). The copy has the points of the selected polygons in
  ascending original index (the top polygon `[1,7,5,3]` of the cube uses points 1, 3, 5, 7, which
  become the copy's points 0..3, so the polygon is `[0,3,2,1]`; the file shows it as `9 12 11 10`). The command
  adds a polygon object right after the source (same parent, transform params, tags and a copy
  of the transform animation as Make Editable), named `<name>.1` (`.2`, ... if taken), and
  selects it. One undo step. The parity test compares source + copy as one mesh (the second
  object's indices offset by the source's point count), as the exporter writes the file.
- **Disconnect** (`disconnect`): groups are the edge-connected components of the selected
  polygons when `preserve_groups` is on, else each polygon is a group. For each group (ascending
  smallest polygon) every point **that is also used by a polygon outside the group** gets a new
  copy appended (ascending original index; one group before the next, as Extrude §1.3); the
  group's polygons are replaced in place (`disconnect-top`: points 8..11 copy 1, 3, 5, 7, polygon
  4 becomes `8 11 10 9`). Points not used outside the group are not copied (so disconnecting all
  polygons changes nothing). **parity-pending** for that exception (the case cannot tell).
  Result selection: the selected polygons.

### 4.8 Connect Points (case `connect-points`)
Each polygon that holds **exactly two** selected points that are not neighbours in it is cut
along the diagonal. With the points at corner indices `i < j`: the part `p[0..=i] + p[j..]`
keeps the polygon's index, the part `p[i..=j]` is appended (after all polygons, in polygon
order). `connect-points`: points 0 and 3 in `[0,1,3,2]` (`i = 0`, `j = 2`) give `[0,3,2]` and
the appended `[0,1,3]`. Polygons with more than two selected points are `Unsupported`
(no data); no cuttable polygon is `Invalid`. **Unsupported**: edge selection (the reference
connects edge midpoints, not observed). Result selection: the polygons that were cut (both
parts).

### 4.9 Optimize (cases `optimize`, `optimize-real`)
1. `optimize_points`: points within `tolerance` (distance) of an earlier point are merged into
   the lowest-index point of their cluster (a point is compared with the already chosen
   representatives, ascending); polygons are remapped, collapsed corners removed, polygons left
   with fewer than three corners dropped.
2. `optimize_polygons`: polygons that are degenerate (a corner repeated) or that repeat an
   earlier polygon (same corners, any start and direction) are dropped.
3. `delete_unused_points`: points no polygon uses are removed (swap-remove).
`optimize-real`: the 12 points after Disconnect merge back to the cube's 8, polygons and order
as the cube. `optimize`: a clean cube is unchanged. The selection is ignored.
**parity-pending**: the reference's defaults (the probe had to set all four), the survivor
order after a removal (all removed points were the last ones in the case), whether merged points
move to the cluster centre.

### 4.10 Mirror (case `mirror-x`)
Reflection across the plane through `plane_point` with unit normal `plane_normal`:
`p' = p - 2 ((p - q) . n) n`. A zero normal is `Invalid`.
- `duplicate` **off** (the case): the points of the selected polygons are reflected **in place**
  (indices kept) and the selected polygons are reversed keeping their first corner (§4.4), which
  restores the winding (`mirror-x`: `[0,1,3,2]` becomes `[0,2,3,1]`; a symmetric cube maps onto
  itself, so only the order of the corners changes). **parity-pending**: unselected polygons that
  share a reflected point move with it and keep their order.
- `duplicate` on (own design, **parity-pending**): the points of the selected polygons are
  reflected into new points appended in ascending original index; with `weld` on, a point within
  `tolerance` of the plane is not copied (the original is used); the reversed copies of the
  polygons are appended; the selection is the new polygons.

### 4.11 Center Axis (case `center-axis`)
`center_axis(mesh, mode)` returns the mesh with every point moved by `-c` and `c`: the centre of
the bounding box (or the point average, `point_average`) in object space. The command adds `c`
(transformed by the object's rotation and scale) to `transform.pos` and subtracts `c` from the
`transform.pos` of each direct child so that nothing moves in the world, in one undo step. The
file of `center-axis` holds world coordinates, which therefore do not change; the parity test
checks `c + mesh'` equals the moved cube and that `c` is (50, 0, 0), the position the
observation reports. An empty mesh is `EmptySelection`. **parity-pending**: the children rule,
other targets of the "Center Axis to" dialog.

### 4.12 Not covered
`Unsupported`, with the data that would settle it: partial Subdivide; Dissolve and Melt of groups
with holes; Connect on edges; Connect with three selected points in one polygon; Untriangulate
and Triangulate of n-gons. Close Polygon Hole, Bridge, Stitch and Sew are §6 (brief 025),
Loop/Path Cut, Knife and Slide are §5; the O25 cases that did not run (`loopcut-*`, `knife-line`,
`bridge-two`, `stitch`, `dissolve-edge`, `close-hole`, `smooth`, `array-command`) have no data.

## 5. Cutting tools (`mesh.loop_cut`, `mesh.knife`, `mesh.slide`; brief 024)

Sources: no golden data. O25 (docs/parity/modeling-tools-2.md) could not run `loopcut-mid`,
`loopcut-quarter` and `knife-line` (the scripted calls did nothing; the Knife needs a viewport
line), so there are no cases under `tests/golden/modeling-tools/` for these tools, and no Slide
case at all. Everything here is own design from the public meaning of the tools, and **all of it is
parity-pending** until GUI session G3 (items 1 and 2: how a cut is placed, how many cuts, how the
offset is set, the HUD, the knife modes) is answered. Parity: 0 of 0 cases; nothing is listed
`UNSUPPORTED` or `KNOWN_MISMATCH`.

The geometry is pure (`crates/modeling/src/cut.rs`, `slide.rs`); the commands take explicit
numbers so a script or agent cuts without a mouse; the viewport tools (`crates/ui/src/cutting.rs`)
only turn pointer positions into those numbers. One application is one `Document::transact`, one
undo step.

### 5.1 Loop cut (`loop_cut(mesh, start_edge, offsets)`)
1. The **ring** of the start edge: the edge, the edge opposite it in each quad it borders,
   and so on in both directions. The walk stops at a border edge, at a non-quad polygon, at a
   non-manifold edge (more than two polygons), or when it returns to the start edge (closed
   ring). The quads passed are the **strip**. A strip needs at least one quad, else `Invalid`.
2. Every ring edge is oriented so that parallel edges of a quad agree: the start edge runs from
   its lower to its higher point index; in a quad `[p0,p1,p2,p3]` the edge `p0 p1` is parallel to
   `p3 p2` (`p0` matches `p3`, `p1` matches `p2`).
3. `offsets` are fractions `t` in the open interval (0, 1) along that orientation
   (`point = from + (to - from) * t`). They are sorted ascending; an empty list, a value outside
   (0, 1) (or not finite) or a repeated value (within 1e-9) is `Invalid`. Up to 64 cuts.
   The command also takes `count = n`: the offsets `i / (n + 1)` for `i = 1..=n`.
4. New points are appended: ring edges in ring order (start edge first, then the walk forward
   through the start edge's lowest-index face, then the walk backward reversed into place), for
   each edge its offsets ascending.
5. Each strip quad is replaced by `k + 1` quads `[a_j, a_{j+1}, b_{j+1}, b_j]` (`a` the points on
   the first ring edge walked along the quad's winding, `b` the parallel points on its opposite
   edge). The first quad keeps the polygon's index, the others are appended in strip order.
6. A polygon next to a cut edge that is not in the strip (the neighbour across a ring end) gets
   the new points inserted into its edge, so the mesh stays closed (no T-junctions).
7. Result selection: the new edges (between the cut points), as edge ids of the new mesh; polygon
   and point sets are cleared.
Proof: unit tests on the 1/1/1 cube (one cut across the ring of four quads: 12 points, 10 polygons,
all cut points on the plane, still closed), 2/2/2 cube (the ring of a middle edge), two cuts,
a cylinder (a cut around the barrel), an open plane and a mesh with a triangle at the ring end.

### 5.2 Knife (`knife_plane`, `knife_line`)
Both cut polygons along the intersection with a plane and split them there.
1. The plane has a point and a normal. Per polygon the signed distance of each corner is
   computed; `|d| <= eps` (`eps = 1e-9 * (1 + bbox diagonal)`) counts as on the plane.
2. **Cut vertices** of a polygon: a new point on every edge whose end distances have opposite
   signs; and every corner on the plane where the polygon passes through (the nearest off-plane
   corners before and after it lie on opposite sides). A corner that only touches the plane,
   and a run of two or more on-plane corners, give no cut vertex.
3. The cut vertices are ordered along the cut line and paired (0-1, 2-3, ...); each pair is a
   chord. A chord whose ends are adjacent in the polygon is dropped. A convex polygon has one
   chord. Polygons are split chord by chord as in Connect (§4.8): the first part keeps the index
   (`p[0..=i] + p[j..]`), the second `p[i..=j]` is appended; polygons without a chord are
   unchanged.
4. New edge points are appended in ascending edge order, one per crossed edge; every polygon
   sharing a crossed edge gets the point (also polygons that are not cut). Only edges used by a
   chord that is carried out get a point.
5. Result selection: the chord edges as edge ids of the new mesh.
6. `polys` restricts the cut to those polygons (default: all).
**Plane mode**: the plane is given directly (`point`, `normal`); a zero normal is `Invalid`.
**Line mode**: `from` and `to` are the ends of the drawn segment in object space, and the surface
cut is the plane through them and the viewer: with `eye` (perspective) the plane through `eye`,
`from`, `to`; with `direction` (parallel view) the plane through `from` and `to` containing
`direction`. Exactly one of `eye` and `direction` is required. Only the part of the cut inside
the segment is made: a chord is carried out when both its ends project (from the eye, or along
`direction`) onto the segment's line at `s` in `[0, 1]` (tolerance 1e-9). With `visible_only`
(default on) polygons facing away from the viewer (`n . (centroid - eye) >= 0`, or `n . direction
>= 0`) are not cut. Degenerate input (zero-length segment, the plane's normal vanishes) is
`Invalid`.
**Loop mode** (`mesh.knife` with `mode = loop`): a loop cut with a single offset through the
edge under the cursor (`edge`, `offset`), the same function as §5.1.
Proof: unit tests on the cube (plane x = 0: 12 points, 10 polygons, closed), plane through
corners (no new points, polygons split along diagonals), a plane that misses the mesh (`Invalid`:
nothing to cut), a concave polygon with four crossings (two chords), a plane through a triangle
corner, `polys` restriction with a T-junction free neighbour, line mode extent (a chord is made only
when both its ends project onto the segment: a segment ending inside the cube cuts only the faces
whose whole chord it covers, otherwise `Invalid`), visible-only on a cube seen from a parallel view, a segment from the eye (perspective) matching the plane result.

### 5.3 Slide (`slide(mesh, sel, amount, toward)`)
1. **Edges** (the selected edges): each connected chain is oriented consistently (the lowest edge
   id runs from its lower to its higher point; at a point where exactly two selected edges meet
   the orientation continues). The polygon in which an edge runs `u -> w` is its **left** side,
   the polygon in which it runs `w -> u` its **right** side.
2. The **rail** of an end point on a side is the other edge of that side's polygon at the point:
   on the left side at `u` the edge to the previous corner, at `w` to the next corner; on the right
   side at `w` the previous, at `u` the next. A point reached by several edges uses the rail of the
   lowest edge id.
3. `amount` in [-1, 1]: positive slides toward the left side, negative toward the right. Every
   point of the selected edges moves to `p + (q - p) * |amount|`, `q` the far end of its rail
   (original positions). Topology, point count and the selection stay; `amount = 0` is `Invalid`,
   a side that does not exist (border edge) is `Invalid`.
4. **Points** (no edges selected): each selected point slides toward the point `toward` along the
   edge between them by `amount` in [0, 1] (clamped); a point not joined to `toward` by an edge is
   `Invalid`. `toward` is the setting `toward` (int, -1 = none).
5. The command keeps the stored edge and point selection (indices do not change).
Proof: unit tests on a 2/2/2 cube (a loop of four edges slides by 0.5 / -0.25 to the exact
positions, volume stays closed, the topology is unchanged), a single edge, a border edge on one
side only, point slide, errors.

### 5.4 Commands
| command | arguments | result |
|---|---|---|
| `mesh.loop_cut` | `node?`, `edge` (edge id, or a point pair `[a, b]`), `offsets?: [float]` or `count?: int` (exactly one; neither means `count = 1`) | `{node, points, polys, cut_edges: [id]}` |
| `mesh.knife` | `node?`, `polys?`, `mode`: `line` / `plane` / `loop`; plane: `point`, `normal`; line: `from`, `to`, one of `eye` / `direction`, `visible_only?` (default true); loop: `edge`, `offset?` (default 0.5) | `{node, points, polys, cut_edges: [id]}` |
| `mesh.slide` | `node?`, `edges?`, `points?`, `settings?: {amount, toward}` (as the other tool commands, §0.1) | `{node, points, polys, selected}` |
Vectors are three-number arrays in object space. Errors are `Failed` and change nothing. Nodes that
are not polygon objects are `Failed`. The stored selection after a cut holds the new edges.

### 5.5 Viewport tools (`crates/ui/src/cutting.rs`; own design, parity-pending)
Menu: Mesh > Loop/Path Cut, Knife Line, Knife Plane, Knife Loop, Slide (after the brief 020-022
tools). The Attribute panel shows the tool (settings from `ToolDesc`, Reset Values, Apply, Done);
Apply on Knife Plane cuts with the panel's `plane_point` and `plane_normal` (also previewed), on the
loop tools at the hovered edge. The tools reuse `ToolState`; hover and drag live in its
`interaction` field.
- **Loop/Path Cut**: while hovering, the edge nearest the cursor (picking, `pick_edge_param`) and the
  parameter along it (the closest point of the cursor ray on the edge) give the offset; with
  `count` > 1 the offsets are `i / (count + 1)` and the hover edge only sets the ring and the
  direction. The preview mesh (the result of the pure function) is drawn in place of the target
  and the HUD (status line) reads `Loop Cut: 1 cut, offset 0.50`. A click runs `mesh.loop_cut`.
  Settings: `count` (1..64, default 1), `lock_offset` (use `offset` instead of the cursor),
  `offset` (default 0.5). Path Cut (a cut along a path of polygons) is the Knife line.
- **Knife Line**: a drag draws the segment; the preview follows the pointer; the release runs
  `mesh.knife` (line mode, `eye` or `direction` from the camera). A drag shorter than 3 pixels
  does nothing. **Knife Plane**: the same drag, with an unbounded segment (plane mode: the plane
  through the eye and the dragged line). **Knife Loop**: hover and click as the loop cut with one cut (settings `lock_offset`, `offset`).
- **Slide**: the standard tool drag (§0.2) sets `amount` (one viewport height = 2.0); the release
  applies.
- Nothing is written during hover or drag; the target is the active polygon object; all three
  need a polygon object (else the tool shows nothing).

## 6. Topology tools (`mesh.bridge`, `mesh.close_hole`, `mesh.stitch`, `mesh.pen`; brief 025)

Sources: no golden data. O25 (docs/parity/modeling-tools-2.md) could not run `bridge-two`,
`stitch` and `close-hole` (the scripted calls returned false; GUI only), so
`tests/golden/modeling-tools/` has no case for these tools and nothing is listed `UNSUPPORTED` or
`KNOWN_MISMATCH`. Parity: 0 of 0 cases. Everything here is own design from the public meaning of the
tools and **parity-pending** until GUI session G3 answers it (the open data is in the brief 025 report).
The geometry is pure (`crates/modeling/src/build.rs`); the commands take explicit indices and numbers
(no mouse); the viewport tools (`crates/ui/src/building.rs`) only turn clicks into those arguments.
One application is one `Document::transact`, one undo step.

### 6.1 Border loops
An edge used by exactly one polygon is a **border edge**. Its **direction** is the one it has in that
polygon (`u -> v`). The border loop through it follows border edges in their direction: at each point
exactly one border edge must start (else `Invalid`: the border is pinched). The loop is the list of its
points `[u, v, ...]`. Existing polygons lie to the left of the loop, so a polygon that closes the hole
has to run the other way (its edges are the reverse of the loop's).

### 6.2 Bridge (`bridge(mesh, a, b, segments)`)
`a` and `b` each name an end: `Edge(e)` (the border loop through border edge `e`) or `Poly(p)`
(the polygon `p` is removed and its outline is the end; its loop is the polygon reversed, which is the
direction its neighbours give the new border). Rules:
1. Both loops must have the same number of points `n`, share no point and not be the same loop
   (else `Invalid`). `segments` is 1..=64.
2. **Matching**: loop `A` runs in its direction; loop `B` is paired in reverse, `P[i] = B_rev[(i + k) % n]`,
   which is the only orientation whose new quads are wound like the rest of the mesh (a closed
   tube stays an oriented surface). The rotation `k` is the one with the smallest total edge length
   `sum |A[i] - P[i]|`; ties choose the smallest `k` (a rotation must be smaller by more than `1e-9 (1 + cost)` to win,
   so rounding noise does not decide).
3. **Rings**: with `segments = s`, rings `1..s-1` are new points appended ring by ring, each in loop
   order: `A[i] + (P[i] - A[i]) * j / s`. Ring 0 is `A`, ring `s` is `P`.
4. **Polygons**: appended, segment by segment (`j = 0..s`), then along the loop (`i = 0..n`):
   `[r_j[i+1], r_j[i], r_{j+1}[i], r_{j+1}[i+1]]`. With `Poly` ends the two polygons are removed first
   (swap-remove, §4), so the new polygons come after the survivors.
5. Result selection: the new polygons; points and edges cleared.
Proof: `bridge.rs` tests: two open cube ends (top and bottom polygons deleted) give a closed tube
(the 4 side polygons plus 4 bridge quads: 8 polygons, every edge used twice), the same through
`Poly` ends of a two-cube stack, rotation picked by length (loops offered in a rotated start),
segments (ring positions, counts), errors (different sizes, same loop, interior edge, shared points).

### 6.3 Close Polygon Hole (`close_hole(mesh, edge)`)
The border loop through `edge` (§6.1) becomes one n-gon appended to the polygon list, wound as the
reverse of the loop (the corner order starts at the start of the given border edge). Result selection:
the new polygon. A non-border edge is `Invalid`. Proof: a cube with the top polygon deleted gets 6
polygons back and is closed; the polygon equals the deleted one up to the start corner; a hole of 6.

### 6.4 Stitch and Sew (`stitch(mesh, pairs, to)`, `stitch_edges(mesh, edge_pairs)`)
`pairs` are point pairs `(a, b)`; the points of every connected set of pairs are merged into the
lowest index of the set. `to` = `Middle` (default): the merged point sits at the average of the set's
positions; `First`: the lowest index keeps its position. Polygons are rewritten with the merged point;
a polygon left with fewer than three different corners is removed; the other merged points are removed
(swap-remove, §4). `a == b`, a missing point, or no pair is `Invalid`/`BadPoint`. Result selection: the
merged points (their new indices).
`stitch_edges` turns pairs of border edges into point pairs: the border edge `u -> v` and the border
edge `x -> y` face each other, so `u` joins `y` and `v` joins `x`; a join of a point with itself is dropped;
interior edges, the same edge twice, or nothing left are `Invalid`.
Proof: `bridge.rs`: a strip of two quads whose seam was opened (`disconnect`) is sewn back to the
original (counts and positions), midpoint vs first, chains of pairs, a collapsing polygon.

### 6.5 Polygon Pen (`pen(mesh, corners)`)
`corners` are `Point(i)` (an existing point: the snap) or `New(position)`. New points are appended in
corner order.
- one corner, `New`: adds a lone point (result selection: that point);
- three or more corners: adds the polygon with the corners in the given order (it is wound as given);
  the result selection is the new polygon;
- two corners, a repeated point, a missing point, a non-finite position, a zero-area outline
  (`|normal| <= 1e-12`), or a single `Point` are `Invalid`/`BadPoint`.
Proof: unit tests (snap reuses the point, new points appended, orientation, errors).

### 6.6 Commands
| command | arguments | result |
|---|---|---|
| `mesh.bridge` | `node?`, exactly one of `edges: [e, e]` (border edge ids or point pairs) and `polys: [p, q]`; `segments?` (1..64, default 1) | `{node, points, polys, new_polys: [id]}` |
| `mesh.close_hole` | `node?`, `edge` (id or point pair of a border edge) | `{node, points, polys, new_polys: [id]}` |
| `mesh.stitch` | `node?`, exactly one of `points: [[a, b], ...]` and `edges: [[e, e], ...]`; `position?`: `middle` / `first` | `{node, points, polys, merged: [id]}` |
| `mesh.pen` | `node?`, `corners: [{point: i} or {pos: [x, y, z]}]` | `{node, points, polys, new_points: [id], new_polys: [id]}` |
Positions are object space. Errors are `Failed` and change nothing. The stored selection afterwards
is the one named in each rule (polygons, points; edges cleared).

### 6.7 Viewport tools (`crates/ui/src/building.rs`; own design, parity-pending)
Menu: Mesh > Bridge, Close Polygon Hole, Stitch and Sew, Polygon Pen. Hover finds the border edge under
the cursor (`pick_edge_param`; a non-border edge is not accepted, the HUD says so); the target is the
active polygon object.
- **Close Polygon Hole**: hover shows the closed hole as preview, a click runs `mesh.close_hole`.
- **Bridge** and **Stitch and Sew**: the first click picks a border edge, the second runs the command
  (preview while the second is hovered). Apply runs the command with the two picked edges, or with
  exactly two selected border edges. Bridge setting: `segments`; Stitch setting: `position`.
- **Polygon Pen**: each click adds a corner, snapped to an existing point within the point pick radius
  (else a point on the plane through the camera target facing the viewer); a click on the first
  corner with three or more corners, or Apply, runs `mesh.pen`. Reset Values / Done discard the
  corners. The preview shows the polygon once there are three corners.
Nothing is written during hover.

## 7. Parity (`crates/modeling/tests/tools_parity.rs`)
Each case of `tests/golden/modeling-tools/` whose `tool` is `extrude`, `extrude_inner`, `inset` or
`bevel` is rebuilt: the default cube with `segments`, made editable (our generator's mesh has
the reference's point and polygon order, so selection indices apply), `select_polys`, the
`[params.settings]` table; the result is compared with `compare_meshes` against the golden
mesh (strict: same counts, identical face lists including start corners, positions within the
case tolerance), both raw and after `export_weld`. `UNSUPPORTED` and `KNOWN_MISMATCH` are
asserted exactly. The tool name `inset` is the same tool as `extrude_inner` (two spellings in
the case files).

The mesh commands (§4) have their own test, `mesh_commands_match_the_reference`, in the same
file: the 15 cases whose `tool` is `subdivide`, `triangulate`, `untriangulate`,
`reverse_normals`, `optimize`, `weld`, `melt`, `split`, `disconnect`, `connect_points`,
`mirror` or `center_axis`. Set-up steps the case files name are replayed with our own commands
(`triangulate_first`, `disconnect_first`, `move_points`); `split` is compared as source plus
copy with the copy's indices offset (the file holds both objects), `center_axis` as world
coordinates (centre + moved point) with the centre asserted to be (50, 0, 0). `UNSUPPORTED` and
`KNOWN_MISMATCH` (`COMMAND_*`) are empty.

Result: see docs/parity/STATUS.md.

## 8. Checks
- `crates/modeling/tests/tools_parity.rs`: the 11 Extrude/Inset cases, the 8 Bevel cases and the
  `rechecks/extrude-bevel-subdiv` case (Extrude with Bevel 10, Subdivision 2 equals
  `extrude-subdiv`, so the Bevel value has no effect there either).
- `crates/modeling/tests/bevel.rs`: defaults, closed meshes stay closed (edge, four edges,
  solid, polygon), removed volume of a fine chamfer and arc radius, edge order independent of the
  selection order, proportional = fraction of the edge length (non-cube box), polygon-mode
  lift, every unsupported option, bad selections, valence-4 vertex, three edges at a corner,
  open mesh, a non-right-angle edge.
- `crates/modeling/tests/tools.rs`: settings parsing and clamping, unsupported options, bad
  selections, preserve-groups angle split, a closed mesh stays closed (extrude and inset of a
  cube face leave every edge used exactly twice), non-flat group directions, inset of a
  non-square polygon, subdivision counts.
- `crates/app/tests/tools.rs` (and one Rhai script in `crates/script/tests/script.rs`): both commands headless: node default and override, selection
  default and override, one undo step restores mesh and selection, errors change nothing, schema
  validation of the sample arguments; `mesh.bevel` by point pair and by edge id, stored
  selection, errors, enum and component schema.
- `crates/ui/tests/tools.rs`: activate Extrude, drag, apply; one undo step; Reset Values;
  preview does not touch the document; attribute rows come from the descriptors; Bevel on a
  stored edge selection (preview, drag, enum settings, unsupported option) and its panel.
- `crates/modeling/tests/commands.rs`: Subdivide counts and closedness for 1..3 levels, Hyper
  volume, triangle and n-gon fallbacks, partial selection; Triangulate of a pentagon, a
  concave quad and a selected subset; Untriangulate round trip, N-gons, hinge, concave pair;
  Reverse twice; every Optimize switch and the tolerance edge; Weld with a collapsing polygon
  and the swap-remove order; Melt of an interior point, a hole, two groups, a lone polygon, a
  closed surface; Dissolve of edges, points, polygons; Split copy; Disconnect groups and the
  unshared-point rule; Connect on a hexagon; Mirror (plane, normal, duplicate, weld); Center
  Axis modes; settings validation of vectors.
- `crates/app/tests/mesh_commands.rs`: the 13 commands headless (one undo step each, undo
  restores the mesh), Melt selection, Split object naming and selection, Center Axis with a
  child, argument errors, Mirror duplicate, schemas.
- `crates/ui/tests/tools.rs`: the Mesh menu lists the commands, no drag for setting-less
  commands, vector rows, Apply from the panel.
- `crates/modeling/tests/cut.rs`: §5.1-5.3 as listed under each rule.
- `crates/app/tests/cut_commands.rs`: `mesh.loop_cut`, `mesh.knife`, `mesh.slide` headless (one undo
  step, stored selection, argument errors, schemas).
- `crates/ui/tests/cutting.rs`: hover previews without touching the document, click commits one
  undo step, knife drag, Slide drag, menu entries.
- `crates/modeling/tests/build.rs`: §6.1-6.5 as listed under each rule.
- `crates/app/tests/build_commands.rs`: `mesh.bridge`, `mesh.close_hole`, `mesh.stitch`, `mesh.pen`
  headless (one undo step, stored selection, argument errors, schemas).
- `crates/ui/tests/building.rs`: hover previews without touching the document, clicks commit one
  undo step, pen corners and snapping, menu entries.

## 9. Arrangement commands (`object.duplicate`, `object.arrange`, `object.randomize`, `object.transfer`, `mesh.array_command`; brief 031)

Sources: **own design, all parity-pending.** The only observation is the array note of
docs/parity/modeling-tools-2.md (case `array-command`, dropped): the probe's Array command on the
default cube with 3 copies and offset (300, 0, 0) produced 24 points and 18 polygons, but with
scaled and rotated copies and a third copy reaching x = 385 instead of 500, because the settings
fields were not mapped to the dialog labels. Nothing in it fixes a rule, so no golden case exists
and the parity count is 0/0. The settings are descriptors (`arrange_descs()`, the framework of
§0), so the panel, the schema, the defaults and the clamping derive from one definition. The pure
functions are in `crates/modeling/src/arrange.rs`; the commands in
`crates/command/src/builtin/arrange.rs` (objects) and `tools.rs` (`mesh.array_command`, a
normal mesh tool command, §0.1).

### 9.1 Step rule (`copy_step`)
Shared by Duplicate and the Array command. Settings: `copies` (int, 1..1000, default 1 for
Duplicate, 3 for Array), `offset` (vec3, distance), `rotation_deg` (vec3 heading, pitch, bank in
degrees), `scale` (vec3, per-axis factor, 0.001..1000, default 1), `mode` (`step` default | `total`).
Copy `k` (1-based) uses the fraction `f = k` in `step` mode (each copy adds the settings once more:
offset `k·offset`, rotation `k·rotation`, scale factor `1 + k·(scale − 1)` per axis) and `f = k / copies` in `total`
mode (the settings are the totals reached by the last copy). Position delta `f·offset`, rotation
delta `f·rotation`, scale factor `max(0.001, 1 + f·(scale − 1))` per axis. The scale is linear in the
index, as the observed Linear mode of the clone toolset (MOGRAPH.md, `1 + i(S − 1)`), and clamped so a
shrinking series never reaches zero or turns negative. Equal formulas for objects and meshes keep the
two commands interchangeable.

### 9.2 Duplicate (`object.duplicate`)
Arguments (explicit numbers, no other keys): `nodes?` (default: the selection), `copies`,
`offset`, `rotation_deg`, `scale`, `mode` (§9.1).
- Nested nodes are dropped as in `object.group`; the rest are processed in the order given (the
  selection is in ascending id order).
- A copy is a deep copy: the node, its tags, its children recursively, all params (links that point
  inside the copied subtree are remapped to the copies; links to other nodes are kept) and
  transform tracks. Copy `k` of a node is inserted in the same parent at `index_of_source + k`
  (copies follow their source); the source stays.
- Transform of the copy's root: the source's local transform plus the §9.1 deltas, i.e.
  `pos + Δpos`, `rot + Δrot`, `scale ⊙ factor` (all in the parent's space; heading/pitch/bank add).
- Name: `<source name>.N` with the lowest free `N` over the whole scene (the Split rule, §4.7);
  children and tags keep their names. The transform tracks of the copy's root are not copied (its
  placement is the step); every other track is.
- One transaction "Duplicate", one undo step. The copies become the selection (the last node's last
  copy is active). Result `{created: [id…], count}`. No selection and no `nodes` is `Failed`.

### 9.3 Arrange (`object.arrange`)
Sets the local position of the nodes only; rotation and scale are unchanged. Arguments: `nodes?`,
`mode` (`line` default | `grid` | `circle`), `spacing` (vec3, default (200, 0, 0)), `row_spacing`
(vec3, default (0, 0, 200)), `columns` (int ≥ 1, default 3), `radius` (default 200), `plane`
(`xz` default | `xy` | `zy`), `start_angle_deg` (default 0). Nodes are used in the order given
(selection: ascending id), nested nodes dropped. With `n` nodes and `a` the first node's position:
- `line`: node `i` at `a + i·spacing`.
- `grid`: node `i` at `a + (i mod columns)·spacing + (i div columns)·row_spacing`.
- `circle`: centre `c` = mean of the nodes' positions; angle `θi = start_angle + 360°·i/n`; the plane
  axes are `(u, v)` = (X, Z) for `xz`, (X, Y) for `xy`, (Z, Y) for `zy`; node `i` at
  `c + radius·(cos θi · u + sin θi · v)`; the component along the plane's normal keeps `c`'s value.
Positions are local values, so nodes with different parents are only aligned if the parents are.
One transaction "Arrange". Result `{arranged: n}`. No nodes: `Failed`.

### 9.4 Randomize (`object.randomize`)
Arguments: `nodes?`, `seed` (int, default 1), `position` (vec3 ± amplitude, default (50, 50, 50)),
`rotation_deg` (vec3 ± degrees, default 0), `scale` (vec3, 0..0.999, default 0), `uniform_scale`
(bool, default off). Deterministic: for each node a SplitMix64 stream (public-domain algorithm, local to
the module) starts at `seed ⊕ node_id·0x9E3779B97F4A7C15`;
it is advanced for nine draws in a fixed order (position x y z, rotation h p b, scale x y z), each
mapped to `u ∈ [−1, 1)` by `2·(bits >> 11)/2^53 − 1`. All nine are always drawn, so changing one
range never shifts the others, and a node's result depends on its id and the seed only, not on
the order or the other nodes. New values: `pos += u·position`, `rot += radians(u·rotation_deg)`,
`scale ⊙= 1 + u·scale` per axis (with `uniform_scale` all three axes use the factor `1 + u_x·scale_x`).
The offsets are added to the current values, so a second run with another seed accumulates. One
transaction "Randomize", result `{randomized: n}`.

### 9.5 Transfer (`object.transfer`)
Copies the placement of one object to another. Arguments: `target` (node id, required), `source?`
(default: the active node), `position`, `rotation`, `scale` (bools, default on), `space` (`world`
default | `local`). `local` copies the chosen local params. `world` sets the target so that its
world matrix equals the source's world matrix (`target_local = inverse(parent_world(target)) ·
source_world`, then position = translation, rotation = `hpb_from_rotation` of the normalised
columns, scale = column lengths); only the ticked components are written. Source = target, a
missing node, a tag: `Failed`. One transaction "Transfer"; result `{node}`.

### 9.6 Array command (`mesh.array_command`)
A mesh tool of the framework (§0.1, settings under `settings`): the polygon object gets, after
its own points and polygons, `copies` transformed copies of the whole mesh (the polygon selection
argument is ignored). Copy `k` is `T(f·offset) · R(f·rotation) · S(max(0.001, 1 + f·(scale − 1)))` applied to every point
in object space, with `f` from §9.1 and the transform convention of TRANSFORM.md (heading about Y,
pitch about X, bank about Z, applied bank, pitch, heading). Copy points follow copy after copy in
original order; copy polygons likewise (the same corner order, so winding is kept). Result
selection: the polygons of the copies. Defaults: `copies` 3, `offset` (200, 0, 0), `rotation_deg`
0, `scale` 1, `mode` `step`. Whether the reference's "copies" counts the original (the probe's 24
points would then be 3 instances) is **open**; `copies` here is the number of added copies. More
than 4 million resulting points: `ToolError::Invalid`. One undo step "Array command" restores the
mesh and selection.

### 9.7 UI
The Mesh menu lists Duplicate, Arrange, Randomize, Transfer and Array command after the building
tools. Choosing one activates the tool: the Attribute panel shows its descriptor rows with
Reset Values and Apply (§0.2). Apply runs the command with the panel values (Array: under
`settings`; the object commands: the values as top-level arguments, plus `target` for Transfer,
which is the id typed in the panel, or when -1 the first selected object other than the active
one). There is no viewport preview and no drag for the object commands; the Array command previews
live. Own design, parity-pending.

### 9.8 Checks
`crates/modeling/tests/arrange.rs` (step rule both modes, array counts, order, winding, closed
cube copies, offsets, rotation/scale, the point cap), `crates/app/tests/arrange_commands.rs` (the five
commands headless, one undo step each, deep copies, determinism of Randomize, argument errors,
schemas), `crates/ui/tests/arrange.rs` (menu entries, panel, Apply).
