# Forge3D workspace redesign

**Proposal for Open4D · 27 November 2026**  
**Review baseline** `matooch/open4d` at `c53eef4` (PR #64)

## Goal

Make the first session feel like a desktop 3D editor. A user should be able to create a primitive or Cloner, see where it sits in the scene, change its properties, and set or inspect a key without losing the viewport. Cinema 4D is the primary reference for the panel hierarchy and persistent transport. Blender informs the use of a viewport header for context, a scan-friendly scene tree, and an obvious way to switch animation editors. Forge3D’s visual language uses graphite surfaces, mint tool and time state, amber selection, and its own icon art.

The supplied Forge3D screenshot shows the workflow cost clearly. The 150 pt text tool area takes space even in Object mode, the upper strip mixes primitive creation, display and animation controls, and the orange fill competes with selection. The reference’s compact tools, separated managers, neutral shaded objects and consistent frame controls address those issues. The empty scene, Cloner, field inspector, and 0→30 frame observations also show why the redesign has to cover more than a selected primitive.

## Standard workspace

| Area | Proposed design and behavior |
| --- | --- |
| Application header | Keep the existing menu bar and document title/dirty indicator. Put workspace switching in the next row, with Standard, Modeling and Animate visible and the remaining registered workspaces in overflow when needed. Give Primitives and Cloner persistent positions at the trailing edge of this row. Cloner is one action, with a separate category menu for related tools. Use the registry for creation and keep menu routes available. |
| Viewport header | Put camera/view, Object/Point/Edge/Polygon, display and overlays in this header. Show the mode in one place. Keep camera gestures local to the viewport. Group rendering with the existing Render/Post controls. Collapse secondary items into an overflow menu as the region contracts. |
| Tool rail | Use a 44 pt rail beside the viewport. Show Select, Move, Rotate and Scale, then applicable mesh tools. Give each icon an accessible name and a tooltip with its configured shortcut. Use the existing Forge icon atlas, adding original glyphs where coverage is missing. Preserve E/R/T and hold 1/2/3 + drag when using default bindings. |
| Viewport | Use neutral shaded geometry with a thin amber silhouette for selected objects and a colored transform gizmo. Use the same selection accent in the scene tree. Fade minor gridlines with distance and depth-test the grid against opaque geometry. Keep component highlights distinct from object outlines. Preserve visibility, picking, display modes and user materials. |
| Objects | Use a 300–340 pt right dock at normal desktop widths. Allocate about 40–45% of its starting height to Objects. Use 26 pt rows with disclosure arrows, 14 pt indentation, a type glyph and stable columns for tags, editor visibility, render visibility and enabled state. Preserve current drag, add-to-selection, rename and tri-state visibility behavior. Search keeps ancestors visible when a descendant matches. |
| Attributes | Place below Objects with its own scroll region. Header shows selected name and type. Put Transform and descriptor-backed tabs directly below it. Primitive example: Transform / Object, with Basic available for common fields. Cloner example: Transform / Object and any groups supplied by its descriptors. Render field remap, sliders and color controls when their descriptors support them. Keep tab order stable for each type. Wrap tabs at normal width and use overflow at tighter widths. |
| Transport | Dock beneath the viewport column so the right-hand managers remain visible. Keep play/pause, step, current frame, key and auto-key in one row, then the ruler and range. Use overflow for markers, FPS and playback options, preserving their existing entry points. Show active auto-key with an explicit filled record state and text or accessible label. |
| Empty selection | Keep the empty grid and a small creation affordance in Objects. Show the project’s time settings in Attributes, backed by existing commands. Selecting an item or clearing selection must never change the camera or frame. |

### Sizing and states

Target 1440 × 900 logical points for review, with a 1280 × 800 baseline and a 1024 × 720 compact check. Start with a 32 pt menu row, 38 pt workspace row, 34 pt viewport header, 88 pt Standard transport and a slim native status line. At compact widths, shrink the inspector toward 250 pt, use overflow for workspace names and tool groups, and stack XYZ fields if their value areas would fall below 52 pt. For Animate, reserve roughly 250–320 pt for the editor under the viewport. Keep dividers resizable and preserve saved layouts. A reset selects the new defaults. Arbitrary nested docking belongs to brief 226; the first visual pass can use the current named areas.

| Dark token | Proposed value | Use |
| --- | --- | --- |
| Window / panel | `#171B21` / `#20262E` | Shell and manager surfaces |
| Viewport / input | `#252C35` / `#181D24` | Canvas backdrop and recessed controls |
| Divider | `#343D49` | Panel edges and quiet grid accents |
| Primary / secondary text | `#E8EDF2` / `#A7B2C0` | Controls, values, units and secondary labels |
| Active tool / frame | `#73D6BC` | Tool glyph and checked state, active workspace, playhead |
| Selection | `#F3B36B` | Object outline, selected row accent and selected key |
| Recording / error | `#F17A80` | Auto-key and errors, paired with label or shape |

Use the bundled UI font at 13 pt for controls, 12 pt for secondary metadata, and tabular figures for coordinates and frames. Use 16–18 pt icons with 26–32 pt pointer targets and a visible keyboard focus ring. Verify text contrast at 4.5:1 in dark, light and high-contrast themes. Keep toggle, selection and focus states legible in grayscale through shape, outline or text.

## Animation workspace

Use **Dope Sheet** and **F-Curve** (the existing Value Graph) as adjacent editor choices in the Animate dock. Retain the existing Speed Graph through the graph controls. Each editor has a track list on the left and a time ruler above its canvas. The Dope Sheet shows object summaries and per-property diamonds. F-Curve shows units, a zero line, axis-labeled curves and handles on the selected key. Preserve the scene frame and selected objects across switches. Keep editor zoom/filter and track visibility as view state. The ruler and transport reflect the same scene frame; use shared frame-to-x logic wherever the editors share the same time domain and scale.

For a keyed attribute, distinguish an empty diamond (no track), hollow diamond with a track marker (track exists at other frames), and filled diamond (key at the playhead). Retain the merged click-to-add/remove behavior. Scrubbing evaluates the scene and refreshes the key indicator and property values while leaving undo history intact. A key edit or retime remains undoable through the existing command routes.

## Implementation sequence

| Pass | Reuse in the Rust/egui editor | Existing briefs |
| --- | --- | --- |
| Workspace chrome and creation | Update `crates/ui/src/layout.rs`, `app.rs`, `theme.rs`, `icons.rs` and `icon_table.rs`. Retain registry command IDs and current key bindings. Move primitive actions into a group and put Cloner directly in the header. Split theme tokens for active tool, selection and time instead of using `accent` for every state. | 227, 228; preserve merged 223/225 |
| Managers and layouts | Use `object_manager.rs` and `tree.rs` for hierarchy and behavior. Build Attributes tabs using `attributes.rs`’s `AttributeView::tabs` / `groups_of` and `widgets`; preserve stable IDs, mixed-value edits, unit conversion and undo. Use `workspaces.rs` / `dock.rs` for layouts. Add migration only if the serialized layout schema changes. | 226, 231; preserve merged 229; verify remaining descriptors against O64 |
| Viewport readability | `viewport_panel.rs` currently supplies a selected-object tint. Prototype the silhouette in the viewport path under `crates/viewport`; profile the extra pass, and verify grid occlusion, instanced selection and picking under each shipped display mode before changing the default. | 227 + viewport follow-up |
| Animation presentation | Use `timeline.rs`, `dopesheet.rs`, `fcurve.rs` and `key_indicator.rs`. Route scrub and edits through the existing scene-timeline and `key.*` commands. Keep inspector → transport → graph updates coherent on frame change. | 227, G5; preserve merged 224 and S7 |

Treat this as a proposed revision to the layout and palette in `UI.md` §29. The decision under review is a Forge3D header with creation actions and one tool rail. The reference’s workflows and manager placement inform the proposal. The compact review prototype uses sample scene data and demonstrates selection, inspector tabs, viewport modes, frame scrubbing and animation-editor switching. The native renderer, full docking and production commands are implementation work.

## Open the prototype

Download [the standalone prototype](../prototypes/forge3d-workspace.html) and open it in a browser. It runs offline. Select **Orb**, open **Animate**, and scrub to frame 15. Position X shows −24 cm, midway between the keys at frames 0 and 30. Switch between **Dope Sheet** and **F-Curve** to inspect that same frame. Use **Empty scene** to review the initial project state.

## Acceptance checks

1. Open a blank scene at 1280 × 800. The grid, creation actions, Objects, project time and transport are visible. Add a Sphere from Primitives and a Cloner via its direct action. Both are discoverable by keyboard and command search.
2. Select a Sphere from the viewport and then from Objects. The selection, gizmo and Attributes agree. Expand a parent, filter to a child, and clear the filter. Hierarchy, visibility and enabled states remain correct. Reparent, undo and verify the tree returns to its previous order.
3. At frames 0 and 30, key Position X. Scrub 0 → 15 → 30 and verify interpolated values against scene evaluation. Remove the key at 30 by clicking its diamond, then undo. Verify the strip, Dope Sheet, F-Curve and inspector reflect the same frame and key state.
4. Switch Standard → Animate → Standard while a child is selected. Keep the selected child, frame and viewport camera. Resize the manager split, restart, and verify the saved layout loads. Reset and verify the proposed defaults.
5. Run the existing layout, workspace, icons, accessibility, object manager, attribute manager, animation and key-edit tests. Manually check at 1024 × 720 and 1280 × 800 at 100% and 150% UI scale, plus native Mac navigation and a GPU viewport check for grid depth, silhouette and frame time.

## Sources

- Owner-supplied Forge3D screen and black-box stills 070/073 (frame and key), 081/082 (animation editors), 093 (Cloner), 124 (empty project), and 123 (field/inspector). The proposed viewport uses an original sample scene.
- [UI spec, including D22 §29](https://github.com/matooch/open4d/blob/c53eef4fdae3130f8396fb57fdd63b685c80d9ba/docs/specs/UI.md) · [G1/G2/G5 observations and owner notes](https://github.com/matooch/open4d/blob/c53eef4fdae3130f8396fb57fdd63b685c80d9ba/docs/parity/GUI-SESSIONS.md) · [Project progress](https://github.com/matooch/open4d/blob/c53eef4fdae3130f8396fb57fdd63b685c80d9ba/docs/PROGRESS.md)
- [Merged UI round](https://github.com/matooch/open4d/pull/64) · [Forge UI source](https://github.com/matooch/open4d/tree/c53eef4fdae3130f8396fb57fdd63b685c80d9ba/crates/ui/src) · [Clean-room notes](https://github.com/matooch/open4d/blob/c53eef4fdae3130f8396fb57fdd63b685c80d9ba/docs/CLEANROOM.md)
