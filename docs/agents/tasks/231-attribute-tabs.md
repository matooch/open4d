# 231 · Attribute manager: settings divided into tabs and groups, like the reference

Status: waiting (O64) · Area: L · Depends on: 150, O64 · Observations: owner note N9 (GUI-SESSIONS.md), O64 [attribute-tabs.md](../../parity/attribute-tabs.md) · Touches: core, generators, modifiers, clone-engine, fields, rig, dynamics, material, volume, ui

## Goal
Not every setting sits at the same level. Each object's settings are divided into tabs (and
groups inside tabs) the way the reference does it (owner note N9, D22). The attribute manager
already draws tabs from `UiHint.group = "Tab/Group"` (UI.md §12), but almost every node type puts
all its params in one group, so the user sees one flat list.

## Steps
1. **Spec:** UI.md §12, with the tab rules from O64's behaviour notes: shared tabs (transform /
   coordinates, display, basic), their order, and whether the tab bar is always shown (today
   only with two or more tabs). Add a short table per type to the type's own spec (or one table in
   UI.md §12.1), citing O64.
2. **Data, not layout code:** set `.group("Tab/Group")` on every param descriptor of every
   registered type from O64's tables. Common params (`transform.*`, `object.*`) get their tabs
   once, in the shared builder. Types O64 didn't observe keep a sensible split marked
   `parity-pending O64` (state it in the Report).
3. **Look:** a tab bar like the reference's (O64 behaviour notes; G1 for the look if answered):
   tabs as a row of toggle buttons; Shift-click shows several tabs stacked (if O64 or G1 says the
   reference does it; otherwise one at a time). Groups are collapsible headers inside the tab.
   The shown tab is remembered per object type.
4. **Plugins and scripts:** plugin params (PLUGINS.md) and behaviour exposed params
   (BEHAVIOURS.md §3.1) accept an optional `group = "Tab/Group"`.
5. **Tests:** every registered type's params all have a tab (a registry test that fails listing
   offenders); per-type tab lists equal the O64 tables for the observed types; file round trip
   unchanged (groups are UI only); kittest: switching tabs shows only that tab's rows.

## Done when
- [ ] Every type's settings are split into tabs/groups; the observed types match O64.
- [ ] The owner checks a Cube, a Cloner and a Light against the reference (Report).
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
