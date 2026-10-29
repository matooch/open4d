# 10 · Timeline: current time, record, autokey, playback

Status: done · Branch: the planner's session branch · Planner: main session · Implementer: Claude Sonnet 5.5

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
- Result: no golden data (own design). 22 new tests in `crates/ui/tests/animate.rs`, 2 in
  `crates/core/tests/file.rs`, one rewritten in `crates/ui/tests/models.rs`; all pass.
  Gate: `GATE: all passed`.
- Choices:
  - Scene settings: `SceneSettings { fps, start, end }` in core (field of `Scene`,
    `Scene::settings()`, undoable `Tx::set_settings`), defaults 30, 0, 90. Chosen over a
    root-less node because a fake node would show in the tree, exports and `len()`.
    **File version 4 -> 5** (`settings:` top-level field; old files load with defaults;
    invalid saved settings reset with a warning). Existing version assertions in
    `core/tests/{file,tracks}.rs` moved 4 -> 5 and the "future version" cases 5 -> 6.
  - Current frame and playing flag are editor state (not saved).
  - Evaluated scene: `EditorState::evaluated()` (`Arc<Scene>`, cached per revision, node
    count, frame, fps in `animate::Evaluated`). Viewport, Attributes, Coordinates,
    picking, gizmo and component pivots read it; drags start from it.
  - Make Editable and Current State to Object copy the source's `transform.*` tracks
    (done in `spawn_like`); other (generator) tracks are dropped. The mesh is baked from
    stored parameter values, not the values at the current frame (parity-pending).
- Rules (ANIM.md section 2): record = 9 tracks in the order of keyframes.md section 3,
  Spline keys, an existing key on the frame keeps its interp; autokey keys all three Vec3
  components; rule 5 keys only changed animated components (tolerance 1e-9), components
  without a track change the stored value; gizmo/Coordinates edits skip unchanged Vec3
  params, so a move with autokey keys Position only; playback anchors to the clock
  (`frame = start + (anchor - start + floor(dt*fps)) mod (end-start+1)`).
- Shortcuts: K, Shift+K, Space (only while the pointer is over the viewport), `,` `.`,
  Shift+`,` Shift+`.`. No clash with Ctrl/Cmd+O/S/A/Z/Y, Delete, 1-5, W/E/R/Q, F2.
- Parity-pending: everything above except "one record = 9 tracks, Spline" (autokey, rule
  5, shortcuts, default range, panel layout, marker look).
- Deviations: (1) Timeline panel has a custom scrub strip instead of an `egui::Slider`
  (needed to draw key ticks on it); it is labelled "Frame strip". (2) `apply_drag`,
  `set_coord`, `apply_edit` keep their signatures; the animated versions are
  `apply_drag_anim`, `set_coord_anim`, `apply_edit_anim`. (3) `Timeline` lost `start`,
  `end`, `fps` and `tick()`; the old placeholder test was replaced. (4) A component-mode
  drag (`ComponentDrag::apply`) still reads the stored world matrix; only the pivot and
  picking use the evaluated scene.
- Not verified without a display: how the panel, markers and strip look, scrubbing by
  mouse, real-time playback pacing, Space while hovering the viewport (the hover state is
  set only when the viewport draws).
- Open questions: O10 (autokey and editing an animated parameter in the reference).
- Gate output (last lines):
  ```
  note: make-editable/spline has no mesh or samples file (tree or form only)
  GATE: all passed
  ```
