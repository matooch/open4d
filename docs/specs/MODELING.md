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

## 4. Parity (`crates/modeling/tests/tools_parity.rs`)
Each case of `tests/golden/modeling-tools/` whose `tool` is `extrude`, `extrude_inner`, `inset` or
`bevel` is rebuilt: the default cube with `segments`, made editable (our generator's mesh has
the reference's point and polygon order, so selection indices apply), `select_polys`, the
`[params.settings]` table; the result is compared with `compare_meshes` against the golden
mesh (strict: same counts, identical face lists including start corners, positions within the
case tolerance), both raw and after `export_weld`. `UNSUPPORTED` and `KNOWN_MISMATCH` are
asserted exactly. The tool name `inset` is the same tool as `extrude_inner` (two spellings in
the case files).

Result: see docs/parity/STATUS.md.

## 5. Checks
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
