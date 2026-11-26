# 223 · Viewport navigation 1 / 2 / 3 and tool keys E / R / T

Status: ready · Area: L · Depends on: 153, 152 · Observations: owner notes (GUI-SESSIONS.md), G1/G2/G5 · Touches: ui, viewport

## Goal
Hold **1** + left-drag = pan, **2** + left-drag = zoom, **3** + left-drag = orbit (owner note N1), and **E / R / T** = move / rotate / scale (N2), replacing the X / C / V hold keys and W / E / R.

Sources: owner notes N1–N8 and GUI sessions G1/G2/G5 in [../../parity/GUI-SESSIONS.md](../../parity/GUI-SESSIONS.md), decision D22. Write the spec change in [../../specs/UI.md](../../specs/UI.md) first, citing the note (for example "owner note N1"). Where the exact layout is still open (G1 not yet answered), build it so the order and placement are data (a table in code or a TOML file), mark the spec rule `parity-pending G1`, and say so in the Report. Clean room: no screenshots, icons or layout files from the reference app.

## Steps
1. UI.md §6.1: replace the hold keys with 1 / 2 / 3. Keep Alt + mouse navigation as it is. The digits are plain hold modifiers while the pointer is over the viewport. A tap with no drag does nothing (state the rule).
2. Shading keys: they can no longer be on 1–5. Move them to a free binding and mark it `parity-pending G2` (the new G2 item asks what the reference uses). Update the key map defaults (UI.md §14.2) and migrate user override files that bound digits to shading: warn once, and keep the user's other bindings.
3. Gizmo/tool keys E / R / T. Check what E and R did before and move those bindings (key map conflict rules §14.4). W is free unless G2 says otherwise.
4. Tests: every gesture in `nav_action`; the digit is held only while the pointer is over the viewport; no selection or gizmo while held; the key map has no conflicts; old override files migrate.

## Done when
- [ ] 1 / 2 / 3 + left-drag pan, zoom and orbit, and E / R / T switch the tool, in the editor (tests in `crates/ui/tests/navigation.rs` and the key map tests).
- [ ] The owner confirms the feel on their Mac (Report: what they said).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
