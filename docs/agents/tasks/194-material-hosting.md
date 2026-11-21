# 194 · Materials and shaders in the document: hosting, links, Generate UVW command

Status: done · Area: J · Depends on: 123, 125, 127 · Observations: O29, O43 (existing) · Touches: core, material, shaders, app, command, ui, io (native format)

## Goal
Brief 123 built the Standard material and brief 125/126 the shaders as libraries, but the document
can't hold them: `Document::create` refuses `Category::Material` (CORE.md section 2), so the
texture tag's `material` link can't be set, texture slots hold shader names as strings, and there
is no Generate UVW command (brief 127 Report). Give materials and shaders a home in the document.

## Design decisions (planner)
- **Document material list.** A document owns an ordered list of materials (the Material manager's
  list), separate from the object hierarchy. Materials are nodes with the existing param
  descriptors, so undo, animation and serialization work unchanged. Spec it in CORE.md section 2
  first (where non-hierarchy nodes live, how links address them).
- **Shaders** are child nodes of the material channel that uses them (a shader tree per slot), so
  the `<channel>.texture` param becomes a link to that shader node. Keep reading the old string
  form for files written before this brief.
- **Links.** The texture tag's `material` link resolves to a list entry; deleting a material clears
  the links (one undo step).
- **Commands:** `material.create`, `material.delete`, `material.assign` (creates the texture tag),
  `shader.set` (put a shader on a channel), `uv.generate` (Generate UVW from the tag's projection,
  per brief 127). Regenerate the command reference.
- **Native format:** bump to v12 for the material list; write a migration test from v11.
- **UI:** the existing Materials panel lists document materials; drag onto an object assigns.
  Level C, parity-pending G6/G7 (no GUI answers yet).

## Done when
- [x] A script creates a material, puts a Gradient shader on Color, assigns it to a cube, saves,
      reopens, and the tag's link and the shader tree survive (round-trip test).
- [x] `uv.generate` on a cube with a Cubic tag matches the brief 127 `proj-cubic` case.
- [x] v11 files still open; the format test covers v12.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: materials and shaders live in the document. Tests: `crates/core/tests/materials.rs` (list order, undo, placement errors, link clearing on delete in one step, deep revision), `crates/core/tests/native_format.rs` (v12 fixtures byte-stable, v11 fixtures migrate and resave, a v11 reader refuses v12), `crates/shaders/tests/hosting.rs` (4), `crates/app/tests/material_commands.rs` (7: the script/save/reopen round trip of the first done-when item, delete + undo, `shader.set` replace/inputs/errors, assign errors, `uv.generate` errors and reuse, and the `proj-*` cases), `crates/ui/tests/material_manager.rs` (3). `uv.generate` against the brief 127 cases: 6/6 exact projections within 1e-5, `proj-frontal` still `KNOWN_MISMATCH` (3.6e-4, UV.md U4).
- Rules (own design, level C, parity-pending): materials in `Scene::materials()`, shaders as children of a material or shader; the eleven `<channel>.texture` slots are `Link` params accepting Shader (before, strings; no file could hold a material, so no data migration; `Material::from_values` still takes the string form); `Material::from_node` names a linked shader `#<id>`, `ShaderSet::from_material` builds the shaders under the same names; deleting a material or shader clears links into the removed subtree inside the same transaction; a new `forge.uvw` tag (FloatList, u/v per corner) stores Generate UVW.
- Deviations: the done-when line says a Cubic tag on a cube; the golden `proj-cubic` case is a sphere, so the command test runs on that sphere. `uv.generate` takes an optional `camera` argument because the document has no camera node. The format bump to v12 is shared with brief 027 (CORE.md 6.11; committed with it). The UI is a minimal Materials panel (New, Assign, Delete) without drag-and-drop onto objects.
- Files: crates/core (scene.rs, document.rs, file.rs), crates/material (params.rs, material.rs, lib.rs), crates/shaders (host.rs), crates/app (material_commands.rs, lib.rs), crates/ui (material_manager.rs, dock.rs, app.rs, icon_table.rs), docs/specs (CORE, MATERIAL, SHADERS, UV, COMMANDS), docs/manual/reference.
- Open: drag-and-drop assignment, a shader tree view and a camera node for Frontal (G6/G7 and a camera brief); `forge.uvw` is not yet read by the renderer and the viewport.
