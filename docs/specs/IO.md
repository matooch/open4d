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
  written `f v//vn v//vn ...` when the mesh has no UVs and `f v/vt/vn ...` when it has
  UVs. With normals off, faces are `f v v v ...` or `f v/vt v/vt ...`. A mesh keeps its
  corner order and its first corner.
- **Texture coordinates** (default on when present). A polygon object with a UVW tag
  (`forge.uvw`, `uvw` parameter as a flat `FloatList` of `u, v` pairs) exports its
  per-corner UVs. One `vt u v` line is written per distinct UV, kept in first-seen order
  and deduplicated exactly (bitwise equality). The face corner is `v/vt/vn` with normals
  on, `v/vt` with normals off. Objects without UVs write no `vt` lines, so their output
  is byte-identical to a no-UV export.
- A polygon keeps its corner order and its first corner (the harness compares face
  lists exactly).

### 1.3 Reader: `read(text: &str, opts: &ReadOptions) -> Result<Vec<ObjObject>, ObjError>`
- `v x y z [w]`: a point (w ignored). `vt u v [w]`: a texture coordinate (w ignored).
  `f` corners: `i`, `i/t`, `i//n`, `i/t/n`; 1-based; negative means relative to the
  number of `v`/`vt` lines read so far. `vn` and every other statement (`s`, `usemtl`,
  `mtllib`, `l`, `p`, ...) and comments are ignored without error. Lines may end in
  `\r\n`, and `\` at the end of a line joins the next line.
- UVs are read when **every** corner of **every** face of an object has a `vt` index;
  otherwise the object has no UVs. The imported UVs are stored on a `forge.uvw` tag
  (`uvw` parameter as a flat `FloatList`).
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
- **Import OBJ** (since brief 172 the menu item opens the options window of §3.5/UI.md §27 after the chooser; the text here is what OK runs with default options) reads the file with `ReadOptions::default()`, then creates the objects
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

## 3. Exchange options, generic commands, dialogs and batch conversion (brief 172)

Sources: own design (area M, `docs/roadmap/areas/M-io.md`: "Import/export dialogs with
options; batch export through the CLI"). The reference app's dialog layouts have no
observation, so **the dialog layout, the option names and their order are parity-pending**.
The options themselves are the ones §1 already defines. Every action is a registry command
(COMMANDS.md §6), so everything works headless; the dialog only collects arguments.

### 3.1 Format table (`forge_io::formats`)
`FormatInfo { id, label, extensions, import, export }`. `formats()` lists them in a fixed
order: `obj`, "Wavefront OBJ", extensions `["obj"]`; `stl` and `ply` (§4);
`gltf` (§5); and `abc` (§6). All import and export.
`format_for_path(path)` matches the extension case-insensitively; `format_by_id(id)` matches
the id exactly (lower case). New formats (area M briefs 164 to 171) add one entry, one pair of
option lists and one reader/writer arm each.

### 3.2 Options are parameter descriptors
`import_options(id)` and `export_options(id)` return `Vec<ParamDesc>` (CORE.md §2), so the
dialog, the command schema, `io.formats` and the CLI all derive from one definition. An unknown
id gives `None`.

| Format | Direction | Id | Label | Kind | Default | Meaning |
|---|---|---|---|---|---|---|
| obj | import | `flip_z` | Flip Z | bool | false | `ReadOptions::flip_z` (§1.3) |
| obj | export | `normals` | Write normals | bool | true | `WriteOptions::normals` (§1.2) |
| obj | export | `weld` | Merge equal points | bool | true | `WriteOptions::weld` (§1.2) |
| stl, ply | import | `scale` | Scale | float | 1.0 | multiplier of every coordinate (§4) |
| stl, ply | export | `binary` | Binary | bool | true | binary or ASCII file (§4) |
| stl, ply | export | `scale` | Scale | float | 1.0 | multiplier of every coordinate (§4) |

The defaults are the `Default` values of `ReadOptions` and `WriteOptions` (a test pins this).

`Options` is `BTreeMap<String, forge_core::Value>`. `resolve_options(descs, given: &serde_json::Map)
-> Result<Options, IoError>` starts from the defaults and applies `given`: a key that is not a
descriptor id is an error (`unknown option "x"`); a JSON value of the wrong type (a bool for a
bool, an integer for an int, a number for a float, an option id for an enum, a string for a
string) is an error (`option "x" must be a boolean`, ...); numbers are clamped by
`ParamDesc::validate`. `obj_read_options(&Options)` and `obj_write_options(&Options)` build
the structs of §1.1.

### 3.3 Text entry points
- `export_text(id, scene, roots, &Options) -> Result<(String, usize), IoError>`:
  `export_scene` (§1.4) followed by the format's writer; the count is the number of objects;
  nothing to export is `IoError::NothingToExport`.
- `read_text(id, text, &Options) -> Result<Vec<ObjObject>, IoError>`: the reader (§1.3) with the
  options.
- `IoError` has `UnknownFormat(String)`, `Unsupported(id, "import"|"export")`,
  `BadOption(String)`, `NothingToExport` and `Parse(String)`; its `Display` is the message the
  commands show.

### 3.4 Commands (COMMANDS.md §6)
- `file.import {path, format?, options?}` -> `{nodes, format}`; undoable ("Import <label>"
  such as "Import OBJ"). `format` defaults to the one matching the extension of `path`; an
  unknown extension is `Import failed: unknown format ...`. Same selection and messages as
  `file.import_obj` (§2), which stays and equals `file.import` with `format: "obj"` and no
  options.
- `file.export {path, format?, selected?, options?}` -> `{objects, format}`; not undoable;
  writes the scene evaluated at the current frame like `file.export_obj`, which stays.
- `io.formats {}` -> `{formats: [{id, label, extensions, import, export, import_options,
  export_options}]}`, each option `{id, label, kind, default, ...}` with `min`/`max` for
  numbers and `choices: [{id, label}]` for enums. Not undoable, read-only.
- `file.import_dialog {format?}` and `file.export_dialog {format?, selected?}` ask the editor
  window to run the dialog of §3.5 (UiRequest `ImportDialog`, `ExportDialog`); headless
  contexts refuse with "needs the editor window".
Errors keep the §2 wording: `Import failed: ...`, `Export failed: ...`, `Nothing to export`,
`Import: no polygons found`; a bad option is `InvalidArgs`.

### 3.5 Dialog (UI.md §27)
After the file chooser, the File menu items show an options window built from the descriptors
(a check box per bool, a drag value per number, a combo box per enum, a text box per string).
OK runs `file.import` / `file.export` with the chosen values; Cancel does nothing.

### 3.6 Batch conversion (CLI.md `forge convert`)
`forge convert <input> --out <dir> [--format obj] [--option key=value]... [--frame N] [--recursive]
[--force]` converts one scene file or every scene file of a folder. The glue is in `apps/forge`;
the formats and options are those above. Converting to `forge` imports each file with the options its own format knows (an option no importable format knows is an error). Rules in CLI.md.

### 3.7 Checks
- `crates/io/tests/formats.rs`: the format table, extension lookup (case-insensitive), option
  lists and defaults equal to the struct defaults, `resolve_options` (defaults, overrides, unknown
  key, wrong types), `export_text`/`read_text` equal `write`/`read` for the option combinations
  (normals off gives `f v v v`, weld off keeps all points, `flip_z` on read), unknown format.
- `crates/app/tests/io_commands.rs`: `file.import`/`file.export` round trip and equality with the
  `_obj` commands, inferred format, options applied, errors, one undo step, `io.formats` schema.
- `apps/forge/tests/convert.rs`: folder conversion, recursion, options, `--force`, skipped files,
  failures with exit code, `--format forge` from OBJ files, no window.
- `crates/ui/tests/io_dialog.rs`: the options window (UI.md §27).

## 4. STL and PLY (`forge_io::stl`, `forge_io::ply`; brief 169)

Sources: [exchange.md](../parity/exchange.md) (O44) and the files `scene.stl` of
`tests/golden/exchange/{static-mesh,hierarchy,anim-transform,skinned-arm,materials,camera-light}` (binary STL, the
exporter's default options: Scale 1, unit centimetres), the STL and PLY formats as publicly documented. **PLY was not observed**
(O44: no public export call found, no `scene.ply`), so everything in §4.2 is our own design, **parity-pending**.

Both formats are binary or text, so §3.3 gains the byte entry points `export_bytes(id, scene, roots, &Options) -> (Vec<u8>, usize)`
and `read_bytes(id, bytes, &Options) -> Vec<ObjObject>`; `export_text` / `read_text` call them and require UTF-8 (a binary result is
`IoError::Unsupported(id, "text export")`). The commands (`file.import`, `file.export`) and `App::export_at` use the byte forms and write
or read the file as bytes. Table additions: `stl` "STL" (`stl`), `ply` "PLY" (`ply`), both import and export, after `obj`.

### 4.1 STL (`crates/io/src/stl.rs`)
**Writer, binary** (default; the only form observed): 80-byte header (text `Forge3D STL export`, zero padded), `u32` triangle count, then
per triangle 50 bytes: unit normal (3 `f32`), three vertices (9 `f32`), attribute `u16` 0, little endian. Rules, each seen on all 6 files
(1 468 triangles: 12, 12, 36, 208, 272, and 0 for `camera-light`):
- **Axes.** A scene point `(x, y, z)` is written `(x, z, y)` (y and z swapped; our scene is Y-up, the format Z-up). This is a mirror, so
  the winding is reversed to keep the normals outward (next rule). Positions are the world-space evaluated points, no scale (Scale 1,
  centimetres) except the option `scale`.
- **Triangulation.** Per polygon, in face order: a triangle `[a, b, c]` is written `[a, c, b]`; a quad `[a, b, c, d]` is written as the two
  triangles `[a, c, b]`, `[a, d, c]` (the fan from the first corner, in reversed winding). Polygons with more corners are
  ear-clipped (`forge_geometry::triangulate::triangulate_poly`) and every triangle is reversed the same way (**parity-pending**: no n-gon was exported).
- **Normal.** The unit vector `(v1 - v0) x (v2 - v0)` of the written triangle (computed in `f64`, stored `f32`; zero for a degenerate triangle).
- **No sharing:** STL stores every triangle's vertices; objects are written one after the other in scene order (the objects of §1.4).
  A scene with no polygon is a file of header and count 0 (`camera-light`); `export_*` still reports `NothingToExport` before writing, as for OBJ.
**Writer, ASCII** (option `binary` off, own design, **parity-pending**): `solid <name>` per object, `facet normal nx ny nz`, `outer loop`, three
`vertex x y z` lines, `endloop`, `endfacet`, `endsolid <name>`; numbers in Rust shortest `f32` formatting.
**Reader:** a file is binary when its size is `84 + 50 * count`, otherwise ASCII (`solid` first; the whole file is parsed, `solid` blocks
become objects, names after `solid`). Triangles go back through the inverse of the rules: points `(x, y, z) -> (x, z, y)`, a triangle `[p0, p1, p2]` becomes the
polygon `[p0, p2, p1]`; points that are bitwise equal after the `f32` read are merged (STL repeats every shared vertex), in first-occurrence order. Normals and the attribute
are ignored. The binary file is one object `Object` (STL has no names). Errors: a truncated file, a count that does not fit the size, a non-finite number, an ASCII
facet without three vertices.
Options: export `binary` bool true, `scale` float 1.0 (a multiplier of every coordinate); import `scale` float 1.0 (multiplier). The observed
export table also lists a unit (`centimeters`); other units are not modelled (**parity-pending**).

### 4.2 PLY (`crates/io/src/ply.rs`, own design)
**Writer:** `format binary_little_endian 1.0` (option `binary` on, default) or `format ascii 1.0`. Header lines: `ply`, `format ...`,
`comment Forge3D PLY export`, `element vertex N`, `property float x`, `property float y`, `property float z`, `element face M`,
`property list uchar uint vertex_indices`, `end_header`. One vertex list for the whole file (objects concatenated, world space, scene axes, no swap,
`f32`), one face per polygon with its corners as written (n-gons kept, indices global); objects are not named (PLY has no objects). ASCII numbers use the shortest
`f32` formatting. Option `scale` (1.0).
**Reader:** `ply` magic, `format ascii 1.0` / `binary_little_endian 1.0` / `binary_big_endian 1.0`, `comment` and `obj_info` lines skipped, any number of elements; `vertex`
properties `x`, `y`, `z` of any numeric type are used and others skipped; the `face` element's list property named `vertex_indices` or `vertex_index`
gives the polygons (index type any integer type); other elements and properties are skipped by their declared size. Faces with fewer than 3 corners and an
index out of range are errors with the element number; the result is one object `Object`. Errors: a missing magic, an unknown format keyword, a
list property that is not a face list, a file shorter than its header declares. Option `scale` (1.0).

### 4.3 Checks
`crates/io/tests/exchange_stl_ply.rs`: for the six `exchange` cases the golden `scene.stl` equals our binary STL of the case's `mesh.obj` geometry
(triangle count, vertices and normals within `1e-3`, exact corner order), a read of the golden file gives back the OBJ's point set (within `1e-3`) and
polygons as triangles; round trips (`write` then `read` for STL binary, STL ASCII, PLY ASCII, PLY binary: points and polygons); the readers' error cases; the
 scene entry points (`export_bytes` of a cube scene, `read_bytes`), format table and option defaults. Level B (INFRA.md).

## 5. glTF 2.0 (brief 164)

Sources: [glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
and [O44](../parity/exchange.md). `forge_io::gltf` parses GLB or self-contained glTF
into an `ImportScene` before applying any edits. Write binary GLB and keep node
hierarchy, per-corner UVs, base colour and roughness, TRS animation, and linear skins.
Camera, light, texture and morph exchange is outside this brief; reject morph targets
explicitly. External resources are not fetched by the byte reader.
Use `file.import`/`file.export` or `forge convert` with format `gltf`. Import accepts
`.glb` and `.gltf`, export writes `.glb`; both expose a positive `scale` option.

- Positions and translations scale by 100 on import and 0.01 on export (metres to/from
  centimetres). Reflect Z, reverse triangle winding and conjugate rotations and
  inverse bind matrices by the axis change. Apply the unit conversion to bind
  translations too. O44 writes centimetre-valued GLBs, so parity imports use scale 1.
- Keep triangles (glTF has no polygons); split vertices for corner UVs when writing
  and recover UVs as corner values when reading. Reflect V between Forge and glTF
  UV origins. Mesh primitives with different materials remain separate.
- Key translation, quaternion rotation and scale at the glTF time in seconds;
  STEP holds, LINEAR is linear for position/scale and slerp for rotation. Unsupported
  interpolation is reported instead of silently changing the motion. Export bakes
  Forge spline and Euler rotation tracks at document frames and keeps source key times.
- Convert skin joint slots and inverse bind matrices to a weight tag and skin deformer.
  Export the host's bind mesh, not the evaluated skin mesh. The O44 arm GLB is static;
  restore its manifest's J2 keys when comparing the recorded f15/f30 poses.
  **Known mismatch:** O44's separately posed bounds differ at f15 and f30 by at most
  `4.2e-4`; its GLB has rows summing to about `0.9999847` and no authored animation.
  The reader uses normalized linear skinning and does not reconstruct authoring-only
  weights or interpolation. The independent GLB round trip retains both poses within `1e-4`.

## 6. ASCII USD (`usda`, brief 168)

**Parity-pending:** O44 has no USD export. Layout follows the public OpenUSD
[Mesh](https://openusd.org/release/api/class_usd_geom_mesh.html),
[Xformable](https://openusd.org/release/api/class_usd_geom_xformable.html) and
[UsdPreviewSurface](https://openusd.org/release/spec_usdpreviewsurface.html) schemas.

- Read and write a single UTF-8 `#usda 1.0` layer through File Import/Export or the CLI.
  Export Y-up, `metersPerUnit = 0.01`. Import Y-up, with centimetres as the USD fallback;
  convert units from explicit stage metadata. Reflect Z and reverse winding for
  right-handed meshes. Retain vertex and polygon order on a Forge round trip.
- Author polygonal `Mesh` prims with `subdivisionScheme = "none"`, points, face
  counts and indices, face-varying normals and `primvars:st` if present. Import
  vertex/varying, uniform, constant or face-varying ST, optionally indexed, as
  corner UVs. Authored normals are validated; Forge regenerates shading normals
  because its polygon node has no authored-normal channel. If the scheme is omitted,
  import the polygon cage; an explicitly requested subdivision scheme is unsupported.
- Preserve `Xform` and mesh hierarchy. The supported transform stack is translate,
  rotateXYZ and scale, in that order from least to most local (missing ops are
  identity). Reflect the rotation basis and convert USD degrees to Forge HPB.
  Use `timeCodesPerSecond` (or `framesPerSecond`, then 24) when reading; write
  time in document frames. Import linear keys. Export keyed values at key times
  and document frames; interpolation between baked samples need not match Forge
  spline interpolation.
- Bind meshes to an in-layer `Material` with a single `UsdPreviewSurface`, mapping
  diffuse color and roughness to the hosted Standard material. No texture graph.
  Scope prims organize material or geometry namespaces without adding transforms.
- Binary USD, composition arcs, cameras, lights, skeletons, Z-up, arbitrary
  transform stacks, and time-varying topology are outside this subset.
## 7. Alembic Ogawa (`.abc`, brief 167)
Sources: [O44](../parity/exchange.md), the public Alembic 1.8.9
[Ogawa container](https://github.com/alembic/alembic/tree/1.8.9/lib/Alembic/Ogawa),
[property and sampling layout](https://github.com/alembic/alembic/tree/1.8.9/lib/Alembic/AbcCoreOgawa),
and [geometry schemas](https://github.com/alembic/alembic/tree/1.8.9/lib/Alembic/AbcGeom).
Use `file.import` / `file.export` or `forge convert` with format `abc`.
- Accept frozen little-endian Ogawa archives, version 1, with Alembic core layout
  0. Checked lengths/pointers, sample limits and depth bounds reject malformed or
  oversized archives. Use the archive's property time sampling (uniform, cyclic
  or acyclic); sample runs can share storage.
- Read Xform schema local operation stacks as native position, HPB rotation and
  scale, with linearly baked tracks for changing samples. Import a shape below a
  named Xform on that object when unambiguous. Parent inheritance is required;
  shear, perspective matrices and changing inheritance cannot be authored as TRS.
- Read and write PolyMesh n-gons, per-corner UVs, and time-sampled positions on
  fixed topology. Indexed UVs are expanded on import. Mesh winding and corner
  order are kept: the Y-up Alembic convention and Z reflection together match the
  OBJ winding in O44. Reflect Z on points and conjugate local transforms by that
  reflection. The `scale` option defaults to 1 (cm-valued O44 archives) and applies
  to positions and translations.
- Read/write a perspective camera's transform and static focal length, aperture
  and focus. Unhandled object schemas are omitted or carried only as transforms.
  No HDF5, SubD control schema, changing topology/UVs, skin weights,
  light/material semantics, or animated lens properties are claimed.
- Export samples authored animation and evaluated geometry at project integer
  frames, not live dynamics or expression-tag state. Use the original hierarchy
  or selected roots and their descendants; a detached root bakes its world
  transform. Deterministically write standard Xform and PolyMesh properties and
  native Ogawa blocks; reject topology changes instead of discarding later faces.
