# 234 · Forge theme: mint state accent, amber selection everywhere

Status: done (parity-pending: the owner's check of the look in a Mac build) · Area: L, K · Depends on: 233 · Observations: owner note N11 (GUI-SESSIONS.md); design reference: branch `design/forge3d-workspace`, `docs/specs/WORKSPACE-DESIGN.md` (palette and viewport sections only) · Touches: ui, viewport

## Goal
Owner note N11: adopt the proposal's palette. **Mint** marks active tools, modes, toggles, the active
workspace tab and the playhead. **Amber** marks selection everywhere: selected objects get an amber
outline/wireframe in the viewport, and selected rows, keys and so on use the same amber, so there is a
clear hierarchy of what is selected. **Red** marks recording and errors. The layout stays the
reference's (D22, UI.md §29, brief 233); only colour roles and the selection display change.

## Steps
1. **Spec (UI.md §24 themes, §29.2):** replace the single `accent` with role tokens.
   - **Dark theme values**, from the proposal:
     - window `#171B21`, panel `#20262E`
     - viewport `#252C35`, input `#181D24`
     - divider `#343D49`
     - text `#E8EDF2` / `#A7B2C0`
     - active/state (mint) `#73D6BC`
     - selection (amber) `#F3B36B`
     - recording/error `#F17A80`
   - **Light and high-contrast:** derive each role so text reaches 4.5:1 and states read in greyscale
     (shape, outline or text as well as colour).
   - **Key diamond (232/233):** key-here, key-elsewhere and no-track use the time/state colours. The *modified*
     state keeps its own distinct hue (not amber, so it doesn't read as "selected"; state the choice).
2. **Role audit:** every use of the old accent in `crates/ui` and `crates/viewport` maps to one role
   (tool/state, selection, time, recording, focus). A test lists widgets per role.
3. **Viewport selection display (VIEWPORT.md):**
   - **Look:** selected objects get an amber silhouette outline (screen-space, a few px, drawn over
     shaded modes) and, in wireframe or "lines" display, amber edges. The active object is brighter or
     thicker than the rest of the selection. Component selection (points, edges, polygons) stays distinct.
   - **Coverage:** it must work with instancing (clones), picking, every display mode and the real-time Render mode.
   - **Cost:** profile the extra pass and record it in PERFORMANCE.md.
4. **Same amber elsewhere:**
   - object manager: selected names and rows (233's rows);
   - selected keys in the timeline, dope sheet and F-curve;
   - the selected gizmo axis keeps its own axis colours.
5. **Grid:** fade minor lines with distance and depth-test the grid against opaque geometry (from
   the proposal; VIEWPORT.md).
6. **Tests:**
   - token contrast per theme;
   - every accent use has a role;
   - a viewport readback where the selected object's outline pixels are amber and an unselected
     object's are not, on lavapipe;
   - instanced selection;
   - grid occlusion.

## Done when
- [x] Mint/amber/red roles in all three themes, with no plain `accent` left; amber selection outline in the viewport and amber selection in the managers and editors.
- [ ] The owner checks the look in a Mac build (Report).
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)

- **Spec first:** UI.md §8 and §24.1 (role tokens, the three token tables, contrast targets, greyscale rule, the role audit), §12 (key diamond: `key` is the time colour, `modified` a violet), §13 (selected rows and names in the selection colour, a 2 px left bar), §29.2 (roles in the look); VIEWPORT.md §6 (palette) and new §16 (look, passes, cost, grid, tests), ANIM.md (key colours), EDITABLE.md, PERFORMANCE.md §7. Cited as owner note N11 and the design reference's palette and viewport rows (colours ours, D22; layout untouched).
- **Tokens:** `Theme` now has `window panel viewport input faint divider text weak_text state state_text time selection selection_text record error component_selection focus key modified`; no `accent`/`accent_text` (a test scans `crates/ui/src` and `crates/viewport/src`). `time == state == key`, `record == error` in every theme (tested). `modified` is a violet (`#B79CF2` dark, `#6B3FC4` light, `#D2B0FF` high contrast), not amber: amber is selection, and the test requires 30 degrees from `key`, `selection` and `record`.
- **Role audit:** `theme::role_audit()` lists the widgets per role with their greyscale cue; the old `accent` uses map as: object manager rows and names, dope sheet / F-curve selected keys and box select, node canvas selected nodes and box select, UV points -> `selection`; timeline playhead, key ticks, preview band and handles (the playhead was `component_selection`), dope sheet and F-curve current-frame line -> `time`; console results, HUD guide, history "current", palette shortcut hints, the active pane outline of the four-view layout (was the component cyan), hyperlinks, egui checked toggles and tabs -> `state`; Auto-Key label while on and UV seams -> `record`. The weight manager's joint rows use `Theme::selection_scope`.
- **Viewport:** selected objects get an amber outline (active: brighter and 3 pt instead of 2 pt) by a mask pass plus a scissored dilation pass (VIEWPORT.md §16.2), in every filled display mode, with instanced clones (a ring per clone), MSAA and open surfaces; Wire and the wire modes draw the selected object's edges amber, Bounding Box its box; the component target gets none; the fill keeps the object colour with a 14 % warm tint. Render mode: the panel draws the outline from a transparent overlay (`overlay_only`, depth-only occluders) over the render engine's image; render-rt is untouched. Screenshots (`viewport.screenshot`) use the same passes. The palette comes from the theme (`viewport_panel::palette_from_theme`). A first design (inverted hull) was dropped: it leaked over interior edges of boxes and failed on planes; the test that caught it stayed.
- **Grid:** minor lines fade with distance from the centre (16 segments, smoothstep, `Palette.grid_fade`, on in the editor, off in the renderer default so the baselines are unchanged); the grid was already depth-tested (`LessEqual`, no write) and is now tested by readback.
- **Cost (lavapipe, release, 640 x 360, with readback):** 400 objects 17-19 ms, one selected the same, all selected 23-27 ms (+30-40 %); 10 000 clones 32 ms, all selected 57 ms (+77 %). PERFORMANCE.md §7. Not measured on a hardware adapter.
- **Tests changed (old colours):** accessibility.rs `every_theme_meets_the_contrast_targets` and models.rs `theme_meets_contrast_targets` (accent text -> state text and selection text), object_manager.rs `selected_names_are_drawn_in_the_selection_colour` (was `..._in_the_accent`; adds the row bar). **Added:** theme_roles.rs (9: dark tokens, role equalities, contrast per role and surface, hue gaps, light and high-contrast keep the dark hues, visuals, selection scope, audit, no `accent`), selection_look.rs (4: palette from theme, active outline, tint, the editor's render path outlines only the selected object), viewport selection_display.rs (13, lavapipe: ring pixels exactly amber and only beside the selected box, active brighter and wider, occlusion, Wire, all nine modes, MSAA, instanced, open surface from both sides, overlay target, overlay occlusion, picking, grid fade, grid occlusion), shader_translate.rs (2: `render.wgsl` and `outline.wgsl` through naga to SPIR-V, MSL, HLSL; it caught a reserved word before any GPU did). Each new or changed file ran 5 times in a row without a failure.
- **Not done / for the owner:** the look in a Mac build (Metal) is unchecked; the Render-mode overlay has no automated readback (the headless harness has no `RenderState`), only the renderer-level overlay tests.
