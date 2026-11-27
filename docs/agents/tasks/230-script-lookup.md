# 230 · Script Manager: scene lookup and context search

Status: ready · Area: L · Depends on: 160, 215 · Observations: owner notes (GUI-SESSIONS.md), G1/G2/G5 · Touches: ui, script, luau

## Goal
In the Script Manager, find values and things in the scene while writing a script, and search the API in context (owner request).

Sources: owner notes N1–N8 and GUI sessions G1/G2/G5 in [../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md), decision D22. Write the spec change in [../../specs/UI.md](../../specs/UI.md) first, citing the note (for example "owner note N1"). Where the exact layout is still open (G1 not yet answered), build it so the order and placement are data (a table in code or a TOML file), mark the spec rule `parity-pending G1`, and say so in the Report. Clean room: no screenshots, icons or layout files from the reference app.

## Steps
1. **Scene lookup panel** beside the editor: search objects by name or path, then insert a handle (`find("…")` in Luau, `scene().find("…")` in Rhai) or copy its path. Pick an object to list its params (id, label, type, current evaluated value) and insert a read or write call for one with a click.
2. **Context search / completion:** typing `.` or `:` after a known handle, or Ctrl+Space, lists the members that fit (API functions, param ids of that object's type, command ids for `run(`), with signature and one-line doc. Built from the command registry, the binding tables (SCRIPTING.md §3, BEHAVIOURS.md §4.3) and the node type param descriptors, never a hand-kept list.
3. **Pick from the viewport or object manager:** Alt-drag an object into the editor inserts its handle.
4. Same for the Console REPL.
5. Tests: completion lists for a cube handle include `radius`-style param ids of that type; the command list matches the registry; lookup by partial name; insertion text per language.

## Done when
- [ ] Lookup, completion and drag-to-insert work for Rhai and Luau; the tests pass.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
