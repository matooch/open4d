# 224 · Keyframe button toggles (click removes an existing key)

Status: ready · Area: L · Depends on: 090, 150 · Observations: owner notes (GUI-SESSIONS.md), G1/G2/G5 · Touches: ui, anim, command

## Goal
Clicking a parameter's keyframe button when a key exists at the current frame removes it. When there is no key, the click adds one (owner note N3).

Sources: owner notes N1–N8 and GUI sessions G1/G2/G5 in [../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md), decision D22. Write the spec change in [../../specs/UI.md](../../specs/UI.md) first, citing the note (for example "owner note N1"). Where the exact layout is still open (G1 not yet answered), build it so the order and placement are data (a table in code or a TOML file), mark the spec rule `parity-pending G1`, and say so in the Report. Clean room: no screenshots, icons or layout files from the reference app.

## Steps
1. UI.md (attribute manager §12) and ANIM.md: the button's three states (no track; a track but no key at this frame; a key at this frame) and what a click does in each. The context menu stays.
2. Remove the key through the existing `key.delete` command, as one undo step. Removing the last key keeps an empty track or deletes it: decide, mark it `parity-pending G5`, and state the rule.
3. The same toggle applies to the timeline's "key all" button for the selection, if it exists.
4. Tests: add, then click again to remove; undo restores the key; a click on a frame between keys adds a key; the button state updates in the same frame.

## Done when
- [ ] Click toggles add/remove at the current frame, with one undo step each (egui_kittest test).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
