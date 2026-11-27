# 232 · Keyframe drafts from every edit route, shown in the viewport

Status: done (parity-pending: G5 discard rules and the multi-object/Record choices) · Area: L, G · Depends on: 224, 097, 151 · Observations: owner note N10 (GUI-SESSIONS.md) · Touches: ui, app, anim, viewport

## Goal
On an animated property, with Auto-Key off, any edit (gizmo drag in the viewport, the Coordinates
panel, the inspector) changes the object **visibly** in the viewport and the inspector and marks
that property's key button as *modified*, but nothing is written to the animation until the user
clicks the key button (owner note N10). Today drafts exist only in the inspector (UI.md §12 "With
Auto-Key off …"), the viewport does not show them, and the gizmo and Coordinates paths write
straight through.

## Steps
1. **Spec:** UI.md §12 and ANIM.md, citing N10:
   - **One draft layer** per document view (`(node, param) → value`), sitting on top of the evaluated
     animation for display, the gizmo and the inspector. It is never saved, rendered or exported.
   - **When an edit becomes a draft:** a property with ≥ 1 key track, Auto-Key off. A property with no
     track still edits the document directly, as today. With Auto-Key on, the edit keys immediately,
     as today.
   - **Committing:** clicking the key button commits that row's draft as a key at the current frame
     (one undo step; vector rows commit all three components). "Key all" / Record commits every draft
     of the selection.
   - **Discarding** (parity-pending G5): changing the frame, undo, a document edit to the same param,
     or Escape in the viewport. The animated value returns.
   - **Display:** the key button shows the *modified* state (§12 indicator). The viewport shows the
     drafted pose. The object manager or HUD may show a small "modified" hint *(open)*.
2. Route the gizmo (MANIPULATE.md) and the Coordinates manager (§20) through the draft layer for
   tracked properties. Multi-selection drafts per object.
3. **Evaluation:** `App::evaluated` (or the viewport's scene build) applies the draft layer after
   animation. Behaviours and expressions see the drafted value while the user looks at it *(open:
   state it)*. Rendering and export ignore drafts.
4. **Tests:**
   - A gizmo move on a keyed object leaves the track unchanged, moves the object in the evaluated
     view, and marks the key button modified.
   - Clicking the key writes the key; undo removes it.
   - A frame change discards the draft.
   - A property without tracks still edits the document.
   - Auto-Key on still keys immediately.
   - The Coordinates panel takes the same path.
   - Render, export and save ignore drafts.

## Done when
- [x] Move a keyed object with the gizmo: it moves, the key button shows modified, the track is unchanged; click the key and the key is stored (egui_kittest and app tests).
- [ ] The owner confirms it matches the reference (Report).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

- Spec: UI.md §12.1 (new; replaces the inspector-only draft rule), ANIM.md §2.4 note and §2.4b, MANIPULATE.md §3, UI.md §20.2 note; all cite owner note N10.
- Rules (ours where N10 is silent, `parity-pending G5`): one draft layer `(node, param) -> whole value` in `forge_app::drafts::DraftLayer` (held by `App`); a draft is made when Auto-Key is off and the node has a track on the parameter (any component), multi-object edits decide per object (animated ones draft, the rest are written in one step and the drafts survive it), a partly animated Vec3 drafts whole. Key button (and the context-menu "Add / Update Key") commits every selected object's draft of that row through `key.set_parameter` in one undo step, other rows' drafts stay. Record / K keys drafted Position, Rotation, Scale and the selection's other drafts in one step "Record"; a selection with a draft never takes Record's toggle-off branch. Discard (lazy, by scope): frame change, any document change (edit, undo, redo, load; node count included because root add/remove does not raise the scene revision), selection change, Auto-Key toggle, Escape over the viewport; dragging back to the animated value removes the draft. Behaviours and expressions see the drafted value (the layer replaces the parameter's tracks in the displayed scene). No extra "modified" hint beyond the key button. Scripted `param.edit` / `transform.edit` / `transform.set` keep ANIM.md rule 5 (write the key); only the three interactive routes draft.
- Per route: gizmo `apply_drag_edit` -> `apply_transform_edits`; Coordinates `coord_run_drafts` (`forge_app::coords::edit_results`, the maths of `transform.edit`, then `apply_transform_edits`; untracked selections still run the command); inspector `attribute_row` stages per selected object, untracked ones still go through `param.edit`.
- Display: `App::evaluated()` and `scheduled_draw()` apply the layer (via `Scene::with_drafts`); `App::eval`, `scheduled`, `eval_mesh`, export, render and save do not.
- Tests: crates/ui/tests/keyframe_drafts.rs (12 tests, 5 runs green); existing attribute_manager, animate, coordinates_manager pass unchanged (no existing test asserted an interactive write-through; the library-level rule 5 tests remain valid because `apply_drag_anim` and `transform.edit` keep it).
- Not verified: a real gizmo drag in a window (the test calls `apply_drag_edit`, which the editor's drag handler calls); the owner's feel check is pending.
