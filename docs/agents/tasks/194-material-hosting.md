# 194 · Materials and shaders in the document: hosting, links, Generate UVW command

Status: ready · Area: J · Depends on: 123, 125, 127 · Observations: O29, O43 (existing) · Touches: core, material, shaders, app, command, ui, io (native format)

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
- [ ] A script creates a material, puts a Gradient shader on Color, assigns it to a cube, saves,
      reopens, and the tag's link and the shader tree survive (round-trip test).
- [ ] `uv.generate` on a cube with a Cubic tag matches the brief 127 `proj-cubic` case.
- [ ] v11 files still open; the format test covers v12.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md` if parity results change, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
