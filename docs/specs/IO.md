# File formats and the File menu (`crates/io`, package `forge-io`; `crates/ui`)

Sources: the exporter conventions in [docs/parity/export-settings.md](../parity/export-settings.md)
(calibration case `tests/golden/calibration/offset-cube`, settings used for every golden
`mesh.obj`), the harness conventions in INFRA.md §3–4 (`parse_obj`, `export_weld`,
`[import] flip_z`), the Wavefront OBJ format as publicly documented, and
[r25-drift.md](../parity/r25-drift.md) (a newer exporter writes the same geometry with other
header text and six decimals, so we compare parsed numbers, never bytes). The File menu
is our own design (UI.md §9 lists the menu bar). Anything not cited is ours.

## 1. OBJ (`forge_io::obj`)

### 1.1 Types
- `ObjObject { name: String, mesh: PolyMesh, world: DMat4, smoothing_angle: Option<f64> }`.
  `mesh` is in the object's own space and `world` takes it to world space; the writer
  writes world space. `smoothing_angle` (radians) is the smoothing tag's angle
  (SMOOTHING.md §2); `None` is faceted. The reader fills `world` with the identity.
  (`smoothing_angle` is a field the writer needs for corner normals.)
- `WriteOptions { normals: bool, weld: bool }`, `Default` = both `true`.
- `ReadOptions { flip_z: bool }`, `Default` = `false`.
- `ObjError { line, message }` (1-based line).

### 1.2 Writer: `write(objects: &[ObjObject], opts: &WriteOptions) -> String`
Follows the observed exporter settings (Scale 1, Flip X/Y/Z off, no swaps, no flipped
faces, no materials, no UVs).
- **Handedness.** Coordinates are written exactly as they are in our scene; no axis is
  flipped and faces keep their winding (counter-clockwise from outside, matching the
  normals). This is the observed "Flip Z off" file: offset-cube position `(10, 20, 30)`
  comes out as centre `(10, 20, 30)`. A default-settings export from the reference
  flips Z and reverses each face (export-settings.md), so importing such a file needs
  `ReadOptions { flip_z: true }` (§1.3).
- **Header and names.** First line `# Forge3D OBJ export`. Each object starts with
  `o <name>`. Names are written as given with line breaks replaced by a space. No
  `mtllib` or `usemtl` line (the reference writes `usemtl default`; the parsers ignore it).
- **Layout per object**, in this order: `o` line, its new `v` lines, its new `vn` lines,
  its `f` lines. Indices are 1-based and global to the file. OBJ allows later sections
  to refer back to earlier points, which welding uses.
- **Points.** Each object's points are transformed with `world` and written in point
  order. Numbers use Rust's shortest round-trip `f64` formatting (`-0` is written `0`),
  so `read(write(x))` returns the same bits.
- **Weld** (default on). The reference exporter merges exactly equal points across the
  whole file (GENERATORS2.md §0; `export_weld` is the tolerant form the tests apply to
  both sides). With `weld` on, a point equal (bitwise, after the world transform and
  `-0` to `0`) to an earlier point of the file reuses it and is not written again;
  new points are numbered in first-occurrence order over objects and point order.
  Unused points stay (they are points of the object). With `weld` off every object
  writes all its points.
- **Normals** (default on). The world-space mesh gets `forge_geometry::normals::corner_normals`
  with `Weighting::Angle` and the object's smoothing angle (`None`: angle 0, so only
  polygons of equal normal share a corner normal). One `vn` per corner, deduplicated:
  a normal equal (bitwise) to an earlier one in the file reuses its index. Faces are
  written `f v//vn v//vn ...`. With normals off, faces are `f v v v ...`. The reference
  writes `v/vt/vn`; we have no UVs yet, so the `vt` part is empty.
- A polygon keeps its corner order and its first corner (the harness compares face
  lists exactly).

### 1.3 Reader: `read(text: &str, opts: &ReadOptions) -> Result<Vec<ObjObject>, ObjError>`
- `v x y z [w]`: a point (w ignored). `f` corners: `i`, `i/t`, `i//n`, `i/t/n`;
  1-based; negative means relative to the number of `v` lines read so far. `vn`, `vt`
  and every other statement (`s`, `usemtl`, `mtllib`, `l`, `p`, ...) and comments are
  ignored without error. Lines may end in `\r\n`, and `\` at the end of a line joins
  the next line.
- Errors (with the line): a number that does not parse or is not finite, a face with
  fewer than 3 corners, an index of 0 or out of range.
- `o <name>` and `g <name>` start a new object. If the current object has no `v` and no
  `f` yet, the line only renames it. A name may contain spaces (the rest of the line,
  trimmed). Before any `o`/`g` the object is named `Object`; an `o`/`g` without a name
  gives `Object`. Objects without polygons are dropped (their points stay in the shared
  list, so later faces can still use them).
- **Points per object.** The global point list is shared by the file. An object's
  points are the points declared inside its section plus every point its faces
  refer to, in ascending global order, re-indexed from 0. So a file written with weld
  off returns each object's points in the written order including unused ones, and a
  file that declares all points first and then splits faces by `o`/`g` gets, per
  object, only the points it uses (in the file's order).
- Faces are kept as read (no triangulation, repeated corners kept).
- `smoothing_angle` of read objects is 40 degrees (the default smoothing tag); file
  normals are ignored.
- **`flip_z`.** Negates every z and reverses the winding of every polygon (the harness's
  `[import] flip_z`, INFRA.md §3): corners `[a, b, c, d]` become `[d, c, b, a]`.

### 1.4 Scene side
- `export_scene(scene, roots: Option<&[NodeId]>) -> Vec<ObjObject>`. With `None` the
  walk covers every root; with `Some(nodes)` it covers each listed node and, depth
  first, its descendants (a node reached twice is taken once). Nodes are visited in
  pre-order, children in order. A node is exported if it is a hierarchy node, is not a
  consumed input of a generator (`forge_generators::is_consumed`) and
  `forge_generators::evaluate` returns a mesh with at least one polygon (so
  deformers, Nulls, splines and nodes whose build fails or is unsupported are
  skipped; deformer children are already applied by `evaluate`). `world` is
  `scene.world_matrix(id)`; `smoothing_angle` is `forge_core::smoothing::smoothing_angle`.
- `import_into(tx, objects) -> Result<Vec<NodeId>, EditError>` creates one root
  `forge.polygon` per object, in order: named after the object, identity transform, the
  mesh with `world` applied to the points, and one `forge.smoothing` tag with default
  params. Polygons with fewer than 3 distinct indices are dropped (a polygon object's
  mesh must be valid, CORE.md §1). Objects left without polygons are skipped. One call
  is one transaction step of the caller's `Tx`.

### 1.5 Checks
- `crates/io/tests/obj.rs`: `read(write(x))` is exact with weld off (points, polygon
  lists, names); weld merges equal points and keeps face corners; negative indices; the
  `v`, `v/vt`, `v//vn`, `v/vt/vn` forms; `o` and `g` split; `flip_z`.
- `crates/io/tests/export_parity.rs`: every primitive and generator golden case that
  matches in `crates/generators/tests/parity.rs` and `generators_parity.rs` is built as a
  scene, run through `export_scene`, `write`, the harness's `parse_obj` and compared
  against `mesh.obj` with the case's tolerance and comparison mode. Exceptions are listed
  in the test and below. The offset-cube calibration case has a non-zero position. The
  smoothing golden cases check the written normals with `parse_obj_corner_normals`
  and `max_normal_error`.
- Results and exceptions: the table in the brief report and `docs/parity/STATUS.md`.

## 2. File menu (own design; menu bar: UI.md §9)

Items: **New**, **Open...**, **Save**, **Save As...**, separator, **Import OBJ...**,
**Export OBJ...**, **Export Selected as OBJ...**.

- **Shortcuts.** Cmd/Ctrl+N is not bound. Cmd/Ctrl+O opens, Cmd/Ctrl+S saves,
  Cmd/Ctrl+Shift+S saves as (egui `Modifiers::COMMAND`). Like the other shortcuts they
  are ignored while a text field has the keyboard.
- **Native file.** Extension `.forge`; content is `forge_core::file::save` (CORE.md §6).
  A file name without an extension gets `.forge` (OBJ exports get `.obj`).
- **Current file and dirty marker.** The editor remembers the path and the scene
  revision at the last save or load (`Scene::revision`). The scene is *dirty* when its
  revision differs. A new or opened scene starts clean. The window title is
  `<file name>[*] - Forge3D`; an unsaved scene is called `Untitled`. The title is sent
  to the viewport only when it changes.
- **Save** writes to the current path, or asks for one when there is none. **Save As**
  always asks, then the new path becomes the current file. A successful save clears the
  dirty marker. Writing failures go to the status bar (the marker stays).
- **Open** reads, parses and loads the file first; only if that works is the document
  replaced (selection and undo history are reset, the dock layout and camera kept). A
  failure leaves the document alone and the message (`Open failed: ...`) goes to the
  status bar.
- **New** replaces the document with an empty one; current path is cleared.
- **Unsaved changes.** New and Open on a dirty scene first ask: **Save**, **Don't
  Save**, **Cancel**. Save saves (asking for a path when needed; if the user cancels
  that dialog the whole action is cancelled) and continues. Cancel stops.
- **Import OBJ** reads the file with `ReadOptions::default()`, then creates the objects
  with `import_into` in one undo step labelled "Import OBJ"; they become the selection.
  There is no flip-Z checkbox in the menu yet: a default-settings reference export needs
  `flip_z`, which only the API offers (parity-pending UI).
- **Export OBJ** writes `export_scene(scene, None)`, **Export Selected as OBJ** writes
  `export_scene(scene, Some(selected))` (disabled without a selection), both with
  `WriteOptions::default()`. An empty result is not written; the status bar says
  `Nothing to export`.
- **Messages.** The status bar shows errors (red): `Save failed: ...`, `Open failed: ...`,
  `Import failed: ...`, `Import: no polygons found`, `Export failed: ...`,
  `Nothing to export` (no dialog is opened then). Successful actions leave it empty,
  except an Open whose load had warnings (they are joined there).
- **Dialogs behind a trait.** `Dialogs { pick_open(kind), pick_save(kind, default_name),
  confirm_unsaved(name) }` abstracts the native dialogs (the `rfd` crate, which uses the
  XDG desktop portal on Linux). Tests inject a `Dialogs` that returns scripted answers.
  `FileKind` is `Scene` (`.forge`) or `Obj` (`.obj`).
- **Scripting for tests.** The menu items and shortcuts call the same methods of
  `Editor` (`new_scene`, `open_scene`, `save`, `save_as`, `import_obj`, `export_obj`),
  so tests drive them without clicking.
