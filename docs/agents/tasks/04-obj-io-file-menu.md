# 04 · OBJ import/export and the File menu

Status: done · Branch: `batch-08` · Planner: main session · Implementer: Claude Sonnet 5.5 (local, macOS)

## Goal
From the File menu you can Open, Save, Save As a native scene, export the scene (or the
selection) to Wavefront OBJ, and import an OBJ as polygon objects. Our OBJ export of a
golden case's scene reproduces the golden `mesh.obj` geometry.

## Inputs
- `docs/parity/export-settings.md` (the reference exporter's conventions; our writer
  follows them so exports compare 1:1), `docs/specs/INFRA.md` §3–4 (OBJ handling in the
  harness, `export_weld`, `[import] flip_z`).
- `crates/parity-harness/src/lib.rs` (`parse_obj`, `export_weld`, `compare_meshes`):
  read for conventions, **don't move or change it** in this task.
- `crates/core/src/file.rs` (native save/load), brief 03 (`forge.polygon`).
- `crates/ui/src/app.rs` (`menu_bar`, shortcuts).

## Design decisions (planner)
1. **Crate** `crates/io` (`forge-io`), module `obj`:
   - `pub fn write(objects: &[ObjObject], opts: &WriteOptions) -> String` with
     `ObjObject { name, mesh: PolyMesh, world: DMat4 }`. Points are written in world space.
     `WriteOptions { normals: bool, weld: bool }`: default normals on (corner normals
     from the smoothing tag rules, `forge-geometry::normals`), weld on (the reference
     exporter welds exactly equal points across the file, `export_weld` semantics).
     No materials, no UVs yet (UVs come with U5).
   - `pub fn read(text: &str) -> Result<Vec<ObjObject>, ObjError>`: `v`, `vn` (ignored
     for now), `vt` (ignored), `f` (with `v/vt/vn` forms, negative indices), `o`/`g`
     start a new object; everything else ignored with no error. Points are shared per
     object (re-indexed from 0 per object).
   - Handedness: follow export-settings.md exactly (Flip Z off ⇒ coordinates written as
     they are in our scene). Note in the spec that a default-settings export from the
     reference flips Z, so importing such a file needs a "flip Z" option:
     `ReadOptions { flip_z: bool }` (default false) which negates z and reverses each
     face's winding.
2. **Scene side** (in `forge-io`, depends on `forge-core` + `forge-generators`):
   `export_scene(scene, roots: Option<&[NodeId]>) -> Vec<ObjObject>` collects every node
   with geometry (`forge_generators::evaluate`, world matrix), skipping consumed inputs
   and deformers; `import_into(tx, objects) -> Vec<NodeId>` creates one `forge.polygon`
   per object (identity transform, mesh in the file's coordinates, a smoothing tag).
3. **File menu** in `app.rs`: New, Open…, Save, Save As…, Import OBJ…, Export OBJ…
   (whole scene; "Export Selected as OBJ…" too). Shortcuts: Cmd/Ctrl+O, Cmd/Ctrl+S,
   Cmd/Ctrl+Shift+S. Native extension **`.forge`**. Use the `rfd` crate for dialogs
   (works on Linux via the XDG portal and on macOS); keep the dialog call behind a small
   trait so UI tests can inject a path instead of opening a dialog. The window title shows
   the file name and a `*` when the scene changed since the last save (compare
   `scene.revision()` with the revision at save/load). New/Open on an unsaved scene asks
   Save / Don't Save / Cancel. Errors go to the status bar.

## Hints (planner)
- Parity check for free: for each primitive golden case and the generator cases that
  already pass, build the scene, `export_scene` → `write` → `parse_obj` (the harness's
  parser) → `compare_meshes` against `mesh.obj` with the case's tolerance. That proves
  writer + weld + world transform in one go. Start with `calibration/offset-cube` (it
  has a non-zero position).
- Normals: write one `vn` per corner (dedupe exactly equal normals); check one smoothing
  golden case's normals with `parse_obj_corner_normals` + `max_normal_error`.
- `read(write(x))` round-trips points and faces exactly (weld off).

## Done when
- [ ] New spec `docs/specs/IO.md` §1 OBJ (writer, reader, options, handedness, weld),
      §2 File menu behaviour (own design, cite UI.md); CORE.md unchanged unless needed.
- [ ] Tests: `crates/io/tests/obj.rs` (round-trip, negative indices, `o`/`g` split,
      flip_z winding), `crates/io/tests/export_parity.rs` (golden cases through our
      writer; list exceptions explicitly like the other parity tests), UI test: save →
      load with an injected path, dirty marker clears on save.
- [ ] `sh scripts/gate.sh` prints `GATE: all passed`.
- [ ] `docs/parity/STATUS.md` row "OBJ export/import".
- [ ] Committed on `batch-08` with the trailer below; report written.

Commit trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`

## Out of scope
glTF/FBX/Alembic/USD (P7), materials and UVs, recent-files list, autosave (U7).

## Report (implementer fills in)
- Result: 106/108 primitive and generator golden cases match through `export_scene` ->
  `write` -> harness `parse_obj` (75 primitives, 31 generators; same comparison modes as
  `parity.rs` / `generators_parity.rs`, including the geometric cut/cap cases). The 2 others
  are the known mismatches `loft/mesh-subdivision` and `sweep/helix`, pinned in
  `KNOWN_MISMATCH` and asserted to still differ. `UNSUPPORTED` is empty. `calibration/offset-cube`
  exact (non-zero position, same winding); the 5 smoothing cases match `parse_obj_corner_normals`
  within 1e-5 (via import_into -> smoothing tag -> export_scene -> write).
- Rules derived: the writer needs no special rule beyond export-settings.md (no axis flip, weld
  of bit-equal points, world space). The exporter weld = exact equality was enough: primitives
  pass the strict `compare_meshes` without the harness's tolerant weld (generators use it on both
  sides, as in the existing test).
- Parity-pending: everything about the File menu (IO.md §2), the object layout of the file
  (`o` per object, own `v`/`vn`/`f` sections), reader point-set rule, selection export includes
  descendants.
- Deviations: (1) `ObjObject` has a fourth field `smoothing_angle: Option<f64>` (the writer needs
  the tag angle for corner normals). (2) `read(text, &ReadOptions)` takes the options as second
  argument; `import_into` returns `Result<Vec<NodeId>, EditError>`. (3) Branch: the current cloud
  branch, not `batch-08`. (4) Status messages: only errors are shown (the status bar draws
  everything in the error colour), so no "Exported N objects". (5) Header written is our own
  `# Forge3D OBJ export`; no `usemtl`.
- Files changed: Cargo.lock; crates/io/{Cargo.toml,src/{lib,obj,scene}.rs,tests/{obj,export_parity}.rs};
  crates/ui/{Cargo.toml,src/app.rs,src/app/files.rs,tests/files.rs}; docs/specs/{IO,UI}.md;
  docs/parity/STATUS.md; this brief.
- Open questions for the planner: `rfd` message dialogs (the Save / Don't Save / Cancel prompt) on
  Linux are not served by the XDG file-chooser portal; rfd falls back to zenity/kdialog, which I
  could not test here (no display). File dialogs use the portal. Does the reference export the
  children of a selected group? (we do). Dirty compares revisions, so undo back to the saved state
  still shows `*`.
- Gate output: `GATE: 321 tests passed, 0 failed` ... `cleanroom: ok` / `ok: 202 cases` / `GATE: all passed`
