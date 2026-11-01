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
3. **A registry command** (`mesh.extrude`, `mesh.extrude_inner`), so the CLI, RPC, MCP and
   Rhai can model without a window (COMMANDS.md §6).

`Settings` is a map from param id to `Value`. `Settings::from_json(desc, object)` starts from
the defaults, validates every given key with its `ParamDesc` (unknown key, wrong type,
non-finite value: error; numbers are clamped into the declared range), and `to_json()`
writes it back. Angles are plain degrees under the keys of the observation (`maximum_angle_deg`
...), not radian parameters, so command arguments, `case.toml` and the UI agree.

### 0.1 Command contract
Arguments: `{ "node"?: id, "polys"?: [index], "settings"?: {param: value} }`, no other keys.

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
    fn preview(&self, mesh: &PolyMesh, polys: &IndexSet, s: &Settings)
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
- Menu: Mesh > Extrude, Extrude Inner (activate the tool). The commands are also listed with
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

## 3. Parity (`crates/modeling/tests/tools_parity.rs`)
Each case of `tests/golden/modeling-tools/` whose `tool` is `extrude`, `extrude_inner` or
`inset` is rebuilt: the default cube with `segments`, made editable (our generator's mesh has
the reference's point and polygon order, so selection indices apply), `select_polys`, the
`[params.settings]` table; the result is compared with `compare_meshes` against the golden
mesh (strict: same counts, identical face lists including start corners, positions within the
case tolerance), both raw and after `export_weld`. `UNSUPPORTED` and `KNOWN_MISMATCH` are
asserted exactly. The tool name `inset` is the same tool as `extrude_inner` (two spellings in
the case files).

Result: see docs/parity/STATUS.md.

## 4. Checks
- `crates/modeling/tests/tools_parity.rs`: the 11 golden cases.
- `crates/modeling/tests/tools.rs`: settings parsing and clamping, unsupported options, bad
  selections, preserve-groups angle split, a closed mesh stays closed (extrude and inset of a
  cube face leave every edge used exactly twice), non-flat group directions, inset of a
  non-square polygon, subdivision counts.
- `crates/app/tests/tools.rs` (and one Rhai script in `crates/script/tests/script.rs`): both commands headless: node default and override, selection
  default and override, one undo step restores mesh and selection, errors change nothing, schema
  validation of the sample arguments.
- `crates/ui/tests/tools.rs`: activate Extrude, drag, apply; one undo step; Reset Values;
  preview does not touch the document; attribute rows come from the descriptors.
