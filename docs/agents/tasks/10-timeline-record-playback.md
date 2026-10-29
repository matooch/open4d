# 10 · Timeline: current time, record, autokey, playback

Status: ready (after 09) · Branch: the planner's session branch · Planner: main session · Implementer: Claude Sonnet 5.5

## Goal
You can animate in the editor:
- set the current frame;
- record position, rotation and scale keys for the selected objects;
- turn on autokey so edits write keys;
- see key markers on the timeline;
- jump between keys;
- play the animation at the scene's fps.

The viewport, the Attributes panel and the Coordinates panel all show the scene at the
current frame.

## Inputs
- Brief 09 (`Track`, `Key`, `Tx::set_key`, `forge_anim::scene_at`, ANIM.md §1).
- `docs/parity/keyframes.md` §3: one record makes **9 tracks per object** (Position
  X/Y/Z, Scale X/Y/Z, Rotation H/P/B), and a recorded key's interpolation is Spline.
- `crates/ui/src/timeline.rs` (placeholder), `crates/ui/src/{app,state,attributes,coordinates,viewport_panel,commands}.rs`,
  `docs/specs/UI.md` §7.

## Design decisions (planner)
1. **Time:**
   - `EditorState.timeline.current` (frame, i64) is the single source of the current time.
   - `fps` and the range move into the **scene** as document settings, so they save:
     store them as params on a root-less document node, or as a small `SceneSettings`
     in core with file support. Choose one and say why in the report.
   - The evaluated scene for display is `scene_at(doc.scene(), current / fps)`. Cache it
     per (scene revision, frame).
2. **Display:**
   - The viewport, the Attributes panel and the Coordinates panel read the evaluated scene.
   - Animated params show a filled key marker in the Attributes panel when a key sits on
     the current frame, and a hollow one when they're animated but have no key here.
   - The markers are drawn by us, not copied art.
3. **Record:** `Command::RecordKeys` writes keys at the current frame for every selected
   object's 9 transform tracks (O2). It is one undo step, and the interpolation is Spline.
4. **Autokey:** a toggle. While it is on, any edit to an animatable Float or Vec3
   parameter writes keys at the current frame for the edited value: all three components
   for a Vec3, as record does. It is one undo step per edit or drag.
5. **Editing an animated parameter with autokey off:** the edit **updates or creates the
   key at the current frame** for that track. This is our own design, parity-pending
   until O10 observes the reference. It avoids the value snapping back on the next frame
   change.
6. **Timeline panel:**
   - Play / Stop, start, end and fps fields, a frame slider, and the current frame field.
   - Previous / next key buttons for the selected objects.
   - Key markers (ticks) for the selected objects' keys.
   - Delete Keys at the current frame (selected objects).
   - Playback advances at the scene fps using the frame clock (`ctx.input(|i| i.time)`),
     drops frames rather than slowing down, and loops at the end.
7. **Shortcuts**, own design and parity-pending:
   - Record: K
   - Autokey toggle: Shift+K
   - Play / Stop: Space when the viewport has focus
   - Previous / next frame: , and .
   - Previous / next key: Shift+, and Shift+.

   Check for clashes with the existing shortcuts and report them.

## Hints (planner)
- Put the pure logic in `crates/ui/src/animate.rs`, so tests need no GPU: frame ↔ time,
  record, autokey writes, key navigation and the playback stepper. Pass time in as an
  argument to the stepper.
- The existing gizmo drags write `transform.*` through `write_transform`. Autokey and
  rule 5 hook in there and in the Attributes panel's commit path.
- Make Editable and Current State to Object (brief 03) copy params. Decide whether tracks
  are copied too, say what you chose in the report, and add a test for it.

## Done when
- [ ] Spec: `docs/specs/ANIM.md` §2 (timeline, record, autokey, rule 5, shortcuts,
      parity-pending markers) and `UI.md` §7 replaced.
- [ ] Tests:
  - `crates/ui/tests/animate.rs`: record makes 9 tracks per object in one undo step;
    autokey on and off; rule 5; prev/next key; Delete Keys; the playback stepper (fps 30,
    a 1 s step advances 30 frames and loops); the evaluated scene at frame 15 moves the
    viewport mesh;
  - an egui_kittest test that clicks Record and checks the key marker;
  - save/load keeps tracks, fps and range.
- [ ] `sh scripts/gate.sh` prints `GATE: all passed`. `docs/parity/STATUS.md` has a row
      "Timeline + record".
- [ ] Committed with the trailer below; report written.

Commit trailer:
```
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01AF8235TxjbnG4zHfXN22Nb
```

## Out of scope
F-curve editor, dope sheet, tangent editing, animated non-transform rendering
optimisations, and sound.

## Report (implementer fills in)
