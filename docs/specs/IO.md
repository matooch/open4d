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

**Export frame (all formats).** `file.export`, `forge export` and `forge convert` export the scene
**evaluated at the export frame** (the current frame in the editor, `--frame` on the CLI):
generators and deformers are baked at that frame. For the animation-preserving formats (glTF,
USD, Alembic, FBX) the evaluated scene still carries its tracks, so node transform animation is
written as well; deformation is not animated (glTF has no deformers). Fixed after S1 showed a
GLB with the bend at its stored value instead of frame 30.

Sources: own design (area M, `docs/roadmap/areas/M-io.md`: "Import/export dialogs with
options; batch export through the CLI"). The reference app's dialog layouts have no
observation, so **the dialog layout, the option names and their order are parity-pending**.
The options themselves are the ones §1 already defines. Every action is a registry command
(COMMANDS.md §6), so everything works headless; the dialog only collects arguments.

### 3.1 Format table (`forge_io::formats`)
`FormatInfo { id, label, extensions, import, export }`. `formats()` lists them in a fixed
order: `obj`, "Wavefront OBJ", extensions `["obj"]`; `stl` and `ply` (§4); `gltf` (§5); `usda` (§6);
`fbx` (§8; extension `fbx`); and `abc` (§7). All import and export.
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
| fbx | import | `scale` | Scale | float | 1.0 | multiplier on top of the file's unit, which is converted to centimetres (§8.2) |
| fbx | export | `scale` | Scale | float | 1.0 | file units per centimetre, declared as the file's unit (§8.8) |

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
Camera, light and morph exchange is outside this brief; reject morph targets
explicitly. Brief 134 maps glTF core materials and images to node materials
(see [MATERIAL §M9](MATERIAL.md)); external resources are not fetched by the byte reader.
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

## 8. FBX (`forge_io::fbx`, briefs 165 and 166)

Sources: the public ufbx documentation and header ([ufbx.github.io](https://ufbx.github.io/),
MIT or public domain, used through the `ufbx` crate, no vendored copy), the public
descriptions of the FBX file structure (the node/property layout, the `Lcl Translation`,
`Lcl Rotation`, `Lcl Scaling`, `PreRotation`/`PostRotation` and `Pivot` properties, the
`Deformer`/`SubDeformer` skin clusters and the `LayerElement` mapping modes), and
[O44](../parity/exchange.md): `tests/golden/exchange/*/scene.fbx` (binary FBX 7.7 written
by the reference exporter, 6 cases) with the scene each `case.toml` records and the
`mesh.obj` of the same scene as the geometry truth. Use `file.import` or `forge convert`
with format `fbx` (extension `.fbx`). Brief 165 is the import (§8.1 to §8.7); the ASCII
writer of brief 166 is §8.8.

### 8.1 Pipeline and options
`forge_io::fbx::read(bytes, scale) -> Result<ImportScene, Error>` parses binary or ASCII
FBX (any version ufbx reads) into an engine-neutral `ImportScene` (nodes, meshes,
materials, skins, tracks). `ImportScene::apply(tx)` installs it as one transaction and
returns the new roots, like the glTF, USD and Alembic importers. `ImportScene::warnings()`
lists the content that was skipped (§8.6). The only import option is `scale` (float,
default 1, range 1e-9 to 1e9): a multiplier applied after the unit conversion of §8.2.
The unit and the axes are converted by our own change of basis (§8.2), not by ufbx, so the
same rule covers nodes, geometry, animation and skin bind matrices. ufbx is asked for
`geometry_transform_handling = ModifyGeometry`, `pivot_handling = AdjustToPivot` and
`inherit_mode_handling = Compensate`, so that every node is a plain translate, rotate,
scale node with normal inheritance, and pivots and geometric transforms are baked into the
vertices. The first animation stack is used. Files above 128 MiB are rejected.

### 8.2 Unit and axis rule (observed)
- **Units.** The file's own unit (`UnitScaleFactor`, centimetres when 1) is converted to
  our centimetres: coordinates are multiplied by `file metres per unit / 0.01 * scale`
  (`k`). *Observed:* all six O44 files declare 0.01 m per unit, and `static-mesh` has the
  same cube corners (+-100) as its OBJ.
- **Axes.** FBX has no fixed axes: the header names them (`settings.axes`: right, up and
  front; a file without them is right-handed Y-up). The change of basis is
  `C = k S P`: `P` the signed permutation that takes the file's right, up and front axes to
  +X, +Y and +Z, `S = diag(1, 1, -1)` the Z reflection into our left-handed world (the same
  rule as glTF section 5 and Alembic section 7). Every point `p` becomes `C p`; every node
  or bind matrix `M` becomes `C M C^-1`, so a rotation about a file axis becomes the
  rotation about the image of that axis, written as HPB (TRANSFORM.md: `Ry(-h) Rx(-p)
  Rz(-b)`), a scale is permuted by `|S P|`, a translation is `C t`. Polygon corner order is
  reversed after the first corner (`[v0, v1 .. vn] -> [v0, vn .. v1]`) when `C` mirrors
  (`det(S P) < 0`: every right-handed file), so the polygons keep facing outwards; a
  left-handed file keeps them. For the usual right-handed Y-up file `C` is `S`:
  `(x, y, z) -> (x, y, -z)`, a rotation about X or Y by `a` becomes `-a` in the
  right-handed reading, which HPB turns back into `+a` for P and H, and a rotation about
  Z stays, which HPB writes as `-B`.
  *Observed:* `camera-light` camera at FBX `(0, 200, +600)` is `case.toml`
  `(0, 200, -600)`; `omni` `(300, 400, +300)` is `(300, 400, -300)`; `hierarchy` Root
  Euler Y `+45` is H `45`; `skinned-arm` J2 Euler Z `-90` is B `+90`; the geometry of all
  six cases matches the OBJ in position and in plane orientation (`compare_geometric`
  distinguishes a face from its reverse). A Z-up file in metres (`fbx.rs`
  `axes_and_units_are_converted`, hand-written ASCII) imports to the same objects as the
  same scene written Y-up in centimetres: file `(x, y, z)` Z-up becomes `(x, z, y) * 100`;
  its animation (`animation_of_a_z_up_file_is_converted`: a move along file Y and a turn
  about the file's up axis) becomes a move along our Z and H.
- **Cameras** look along their local +X with +Y up (and +Z to the right) in FBX and along
  +Z (+X right) in our scene, so the node rotation is multiplied from the right by
  `Ry(-90)` (the frame change that takes our look axis to the FBX one) before the change of
  basis. *Observed:* the `camera-light` camera gets `rotation_degrees = [0, -15, 0]`
  (P -15) exactly as in `case.toml`.
- **Lights** shine along `light.local_direction` (FBX default: local -Y) and ours along
  +Z, so the node rotation is multiplied by the shortest rotation that takes -Z to that
  direction (`Rx(-90)` for -Y) before the change of basis. *Observed:* FBX `Spot` and
  `Omni` both carry Rx(90) and both become the identity rotation in `case.toml`.

### 8.3 Content
- **Hierarchy.** One engine object per FBX node (the invisible scene root is not
  imported), same names and parent/child order. A node with a mesh is a polygon object,
  with a camera a camera, with a light a light, a bone (or any node a skin cluster
  names as its bone) a joint, and anything else a null.
- **Polygons.** Positions, n-gons (all corner counts of 3 or more; points and lines are
  dropped), per-corner UVs on a UVW tag (V unchanged: FBX and our UV origin are both
  bottom-left). The engine has no authored-normal channel, so normals are not stored;
  a smoothing tag with default values is added unless the file has normals and every
  corner normal equals its polygon normal (a faceted mesh).
- **Materials.** One node material per FBX material, named like it: base colour =
  diffuse colour times diffuse factor, alpha `1 - transparency factor` (alpha mode
  `BLEND` when below 1), emission = emission colour times factor, roughness
  `sqrt(2 / (shininess + 2))` for a Phong material and 1 for a Lambert one (own rule,
  not observed). Textures are not read (O44 writes none: `materials/Checker Texture` is
  an ordinary 0.8 grey material in the file). A mesh with one material gets one texture
  tag; with several, one polygon selection tag per used slot (named after the material)
  and one texture tag per slot whose `selection` names it, so each polygon keeps its
  material. *Observed:* `materials` imports "Red Matte" `(1, 0, 0)`, "Blue Glossy"
  `(0, 0, 1)` and "Checker Texture" `(0.8, 0.8, 0.8)` on the cubes at x = -150, 0, 150.
- **Cameras.** `sensor_size` = aperture width in mm (inches times 25.4),
  `fov_horizontal` = `2 atan(sensor / (2 focal))` radians, near and far plane and
  `FocusDistance` in our units (`near_clipping`, `far_clipping`, both enabled), an
  orthographic camera is `parallel` with `zoom = 1000 / width` (**parity-pending**).
  *Observed:* focal 36 mm and sensor 36 mm give the 53.130102 degrees of `case.toml`.
  A file may carry more cameras than the scene description lists (the exporter also
  writes its viewport camera); every camera node is imported.
- **Lights.** Point to `omni`, spot to `spot` (inner and outer cone angle in radians),
  directional to `infinite`, area to `area`, volume to `omni`; colour; intensity (FBX
  100 is 1.0). Decay and shadow settings are not mapped.
- **Animation.** The scene's first stack is baked by ufbx (`bake_anim`, keys kept at
  their own times, curves with non-linear interpolation resampled at 30 per second,
  steps become 1 ms ramps) into local translation, rotation and scale per node, in
  seconds (our tracks are in seconds as well). Channels that never change become the
  node's static transform; the others become `transform.pos.*`, `transform.rot.*` and
  `transform.scale.*` tracks with linear keys (only the axes that change); rotation
  tracks are quaternion tracks with the Euler values made continuous. Property
  animation (lens, light, material) is not imported. *Observed:* `anim-transform` has 31
  linear keys per curve (one per frame); frame 30 gives position `(200, 100, 0)`, H `90`,
  scale `1.5`.
- **Skins.** The first skin deformer of a mesh becomes a weight tag (joints in cluster
  order, one weight row per point), a bind pose and a skin deformer, like the glTF and
  rigging objects. The bind matrix of a joint is the cluster's bone transform at bind
  time in the mesh's space, `inverse(geometry_to_bone)`, changed to our basis.
  FBX linear skinning is the weighted sum of the cluster transforms with the unweighted
  remainder (`1 - sum`) staying at the bind position (ufbx `get_skin_vertex_matrix`), so
  the imported skin deformer has `normalise` off. *Observed:* `skinned-arm` has three
  clusters (49, 88 and 49 weights over 106 points); the exporter quantises weights to 16
  bits (rows sum to `1 + 1.5e-5`), and only without renormalising do the posed bounds of
  frames 15 and 30 match `f15_bbox` and `f30_bbox` to 1e-4 (renormalised, the minimum X is
  off by `3.6e-4` and `4.2e-4`).

### 8.4 Errors
`Error::Invalid(String)` for a file ufbx cannot read (the message is ufbx's), a file above
the size limit or a non-finite or non-positive `scale`. Unsupported content is skipped
with a warning (§8.6), never an error.

### 8.5 Parity (O44)
All six `scene.fbx` files are imported and compared with their `mesh.obj` through
`compare_geometric` (points, planes with orientation, closedness; position tolerance of
the case), and with the values of `case.toml`: node names and local transforms
(`static-mesh`, `hierarchy`, `anim-transform`, `camera-light`), the tracks sampled at the
recorded frames (`anim-transform` frames 0 and 30), the materials of `materials`, and for
`skinned-arm` the weights of the recorded points, the J2 rotation at frames 15 and 30 and
the skinned bounds at frames 15 and 30 against `f15_bbox` and `f30_bbox`. Results are
listed in `docs/parity/STATUS.md`.

### 8.6 Not imported (reported by `warnings()`)
NURBS curves and surfaces, line curves, procedural geometry, blend shapes (morphs), cache
deformers, constraints, additional animation stacks, more than one skin deformer per mesh,
texture files, property animation. The affected node is still imported as a null (or as its
mesh without the unsupported part).

### 8.7 Checks
`crates/io/tests/fbx.rs`: the six O44 scenes (geometry, transforms, animation, cameras and
lights, materials, skin), the axis and unit rules on hand-written ASCII files (Z-up in
metres, left-handed Y-up, `scale`, animation), UV, material and shading rules, bad input,
reported skips and undo. `crates/io/tests/formats.rs`: the format table.
`apps/forge/tests/convert.rs`: `forge convert` of an `.fbx`.

### 8.8 Export: ASCII FBX 7.4 (brief 166)

Sources: the public descriptions of the FBX file structure (ASCII layout, `FBXHeaderExtension`,
`GlobalSettings`, `Definitions`, `Objects`, `Connections`; `Properties70` entries; `Geometry`
with `Vertices`, `PolygonVertexIndex` where the last index of a polygon is stored as `-index - 1`,
`LayerElementNormal`/`UV`/`Material` with their mapping and reference modes; `Deformer` skin and
cluster objects with `Indexes`, `Weights`, `Transform`, `TransformLink`; `AnimationStack`,
`AnimationLayer`, `AnimationCurveNode` and `AnimationCurve` with `KeyTime` in 1/46186158000 s) and
our own importer (§8.1 to §8.7) as the check. The writer is our own code
(`forge_io::fbx::write`): no SDK and no text copied from any file. Use `file.export` or
`forge convert --format fbx` (extension `.fbx`); option `scale` (float, default 1, range 1e-9
to 1e9) is the number of file units per centimetre, declared in the file as the unit
(`UnitScaleFactor = 1 / scale` centimetres), so scale 1 writes centimetres and 0.01 metres.
The result is UTF-8 text and deterministic: the same scene gives the same bytes.

- **Scene.** The scene given to the writer is the one evaluated at the export frame (§3, "Export
  frame"): generators and deformers are baked at that frame, skinned meshes are written as their
  bind mesh with the skin, and the tracks of every object are kept. The whole scene (or the
  selected objects with their descendants) is written; an object whose parent is not selected
  becomes a root with its world transform (an animated one is an error). Consumed inputs of
  generators and skin deformers are not objects. Nothing to write is `NothingToExport`.
- **Axes and unit** are the inverse of §8.2: right-handed Y-up (`UpAxis 1`, `FrontAxis 2`,
  `CoordAxis 0`), points `(x, y, z) -> (x, y, -z) * scale`, node and bind matrices `D M D^-1`
  with `D = scale * S`, polygon corners `[v0, v1 .. vn] -> [v0, vn .. v1]`. A rotation is
  written as XYZ Euler degrees (`RotationOrder 0`: `R = Rz Ry Rx`) of the reflected rotation
  matrix, after the inverse frame change of §8.2 for cameras (`Ry(+90)`) and lights
  (`Rx(+90)`).
- **Objects.** Null, polygon object, camera, light and joint become `Model` of class `Null`,
  `Mesh`, `Camera`, `Light`, `LimbNode`; every other object type is written as a null.
  Hierarchy and names are kept. `NodeAttribute`: camera (`FocalLength` from the horizontal
  field of view and sensor, `FilmWidth` in inches and a 16:9 `FilmHeight`, `ApertureMode 3`,
  `NearPlane`, `FarPlane`, `FocusDistance`; a projection other than perspective is written as
  `CameraProjectionType 1` with `OrthoZoom = width / 30` in file units, 30 being the unit ufbx
  assumes for files from other writers, **parity-pending**; the six axis views are written as
  such a parallel camera with the object's own rotation, they ignore it when rendering),
  light (`LightType` 0 point for omni and IES, 1 directional for infinite and parallel, 2 spot
  for all spot kinds, 3 area; `Color`; `Intensity` = 100 times ours; `InnerAngle` and
  `OuterAngle` in degrees for spots), joint (`LimbNode` skeleton).
- **Polygons.** `Geometry` of class `Mesh`: all points, all polygons (n-gons included),
  `LayerElementNormal` per corner from our shading rule (the smoothing tag's angle, faceted
  without a tag), `LayerElementUV` per corner (deduplicated values, `IndexToDirect`) for an
  object with a UVW tag, `LayerElementMaterial` (`AllSame` or `ByPolygon`).
- **Materials.** `Material` with `ShadingModel` `lambert` (roughness 1) or `phong` with
  `Shininess = 2 / roughness^2 - 2` (the inverse of §8.3), `DiffuseColor` = base colour,
  `EmissiveColor`, `TransparencyFactor = 1 - alpha`; metallic, normal, occlusion and texture
  slots are not written (no textures: the format option is out of scope). The material slots of
  an object are its texture tags in tag order; a tag with a `selection` names a polygon
  selection tag of the object and covers those polygons, the last tag without selection covers
  the rest; polygons no tag covers get an extra material "Default" (our default surface) when
  another polygon has a material. Materials shared by objects are written once.
- **Skin.** A mesh with a weight tag and a skin deformer gets a `Deformer` skin and one
  cluster per joint (`Indexes`, `Weights` of the points that use it; `Transform` = the mesh
  node's matrix relative to the joint at bind time, the inverse of our bind matrix, which is
  how ufbx reads that field; `TransformLink` = the joint's world matrix at bind time, the
  mesh's current world matrix times our bind matrix; all reflected and scaled), the joint
  `Model` connected to its cluster. Rows are written as the engine
  evaluates them: normalised when the skin deformer normalises, as stored when it does not
  (FBX skinning is the unnormalised weighted sum, §8.3).
- **Animation.** One `AnimationStack` ("Take 001", `LocalStart` 0 or the first key to the last
  key) with one `AnimationLayer`; per animated object and parameter one `AnimationCurveNode`
  (`Lcl Translation`, `Lcl Rotation`, `Lcl Scaling`) with an `AnimationCurve` per axis. Keys
  are written at the union of the key times of the parameter's three tracks, plus every
  document frame between the first and last key when a track is spline-interpolated and for
  every rotation (FBX interpolates rotation channels as Euler angles, never as orientations,
  so a quaternion track is sampled on every frame); the values are the track values there
  (a rotation: the Euler angles of the interpolated orientation, made continuous).
  Keys are linear (`KeyAttrFlags 4`), or constant (`2`) when every key of the parameter is a
  step. Mixed step and continuous interpolation in one parameter is an error. Key times are
  `round(t * 46186158000)`. The document frame rate is `TimeMode` (or 14 with
  `CustomFrameRate`).
- **Not written:** textures and texture-related material slots, blend shapes, cameras'
  stereo settings, light shadow and decay settings, bind pose objects (the clusters carry the
  bind matrices), property animation (lens, light, material), splines and other non-polygon
  geometry.

**Checks** (`crates/io/tests/fbx_export.rs`): the six O44 scenes are imported, exported as
ASCII FBX, imported again and compared object by object (names, parents, kinds, local transforms
at several times, world meshes with `compare_geometric`, materials, camera and light settings,
skinned bounds at the recorded frames); scenes built in code cover a hierarchy with a rotation
about three axes, animation with linear, spline and step keys, cameras and lights with every
exported setting, materials with selections, a skin with metre units (scale 0.01 then 100),
selected roots, determinism and the file header.
## 9. Cameras, lights and materials across formats (brief 170)

Sources: O44 `camera-light` and `materials` ([exchange.md](../parity/exchange.md)) and the
format sections above. The reference's own per-format behaviour is the mapping:

| Format | Camera | Light | Material |
|---|---|---|---|
| glTF/GLB | yes: a `cameras[]` entry (perspective `yfov` vertical in radians, `znear`, `zfar`, `aspectRatio`) and a node `camera` index; transform keeps the glTF Y-up/flip-Z convention | **no** light export option, so lights are omitted | base colour and the metallic/roughness pair (brief 134) |
| Alembic | yes: the camera's transform and static lens (§7) | no light nodes | names are not recoverable |
| FBX (§8) | yes, both ways (import §8, export §8.8) | yes, both ways (Omni/Spot; direction rule §8) | diffuse colour both ways; roughness ↔ Phong shininess (own, §8.8) |
| USD (our own) | parity-pending (no USD case in O44; the ASCII writer still omits cameras and lights) | parity-pending | brief 168 |

- glTF import creates a `forge.camera` for a node with a camera: sensor 36,
  `fov_horizontal` from `yfov * aspectRatio`, and near/far clipping only when the file
  differs from glTF's 0.01 / 1e10 defaults. glTF export skips `forge.light` nodes, allows
  a file with cameras and no meshes (no BIN chunk), and uses the scene's stored render
  aspect else 16:9. The scene's default near clipping (enabled at 0) exports as 0.01.
- The O44 golden `scene.glb` declares 472 bytes but is 464 (the missing BIN chunk header);
  the importer accepts such a file as it is: when the declared length is exactly 8 bytes
  more than the file and the JSON chunk ends at the end of the file, it reads the file with
  its real length (users import files from that exporter; S9). Any other length mismatch is
  still an error. Our own writer always declares the true length.
- Checks (`crates/app/tests/gltf_exchange.rs` and `crates/io/tests/alembic.rs`): the
  camera-light scene imports the camera at (0, 200, -600) with a 53.13° horizontal field;
  our export has one perspective camera, translation (0, 200, 600), `yfov` 31.417°,
  `znear` 0.01, `zfar` 1e10, aspect 16:9, no meshes and no lights; the Alembic case keeps
  the camera and no lights.
- S9 acceptance (brief 171, `apps/forge/tests/s9_exchange.rs`): every O44 case imported from
  every reference file the exporter wrote (`scene.fbx`, `scene.glb`, `scene.abc`, `scene.stl`)
  through the `forge` CLI only (`convert --format forge --option scale=1`, `info`, `eval
  --export`, `export --format gltf --frame N`, `cmd`), compared with `mesh.obj` and `case.toml`.
  No new rule: the importers of §5, §7 and §8 already match; the per-format absences of the table
  above are asserted (STL: one object, no skin, no camera or material; Alembic: no joints,
  lights or materials; glTF: no lights, no arm animation). STL is compared triangle for
  triangle, because plane grouping is not stable on a triangle soup of nearly planar quads.
