# L · UI and UX: layout, managers, preferences

## Scope (v1)

- [ ] **Familiar default layout** (the reference's "Standard" layout arrangement, own
  visuals):
  - top: menu bar and a command bar (undo/redo, mode buttons, render buttons, primitive
    and generator palettes);
  - left: the mode/tool palette (points/edges/polygons/model/axis, snapping, workplane);
  - centre: the viewport(s);
  - right top: Object manager; right bottom: Attribute manager;
  - bottom: timeline strip, Material manager (left) and Coordinates manager (centre).
- [ ] **Icons:** our own icon set, vector-drawn in code or as SVG we author, for every
  command, object type and tag. **Never** traced from the reference (CLEANROOM.md).
- [ ] **Object manager:**
  - columns for name, visibility dots (editor/render), enable tick, and tags;
  - drag and drop to reparent and reorder; multi-select; search/filter;
  - layers colour; context menus.
- [ ] **Attribute manager:**
  - tabs (Basic, Coord., Object, Phong …) and collapsible groups;
  - every widget type (float with drag-scrub and units, vector, enum, link with drop,
    spline/curve editor, gradient editor, colour picker, field list, text);
  - multi-object edit (mixed values); keyframe dots; lock/unlock; history (back/forward).
- [ ] **Coordinates manager:** object/world, size/size+/scale modes, apply.
- [ ] **Timeline manager, dope sheet and F-curve** (from G). **Material manager** (from J).
- [ ] **Shortcuts:**
  - the reference's well-known default keys, from the public manual and G2;
  - a customisation editor;
  - viewport navigation (Alt+LMB/MMB/RMB orbit/pan/dolly; 1/2/3 key drags; F1–F5 views;
    H frame all; S frame selected; O frame object).
- [ ] **Command palette** (search every registry command); undo history panel.
- [ ] **Preferences:** units, autosave interval, theme (dark default), UI scale,
  GPU/backend choice, input devices, default layout, file paths.
- [ ] **Layouts:** save/load; Standard, Model, Animate, UV Edit, Rigging, Script
  workspaces.
- [ ] **Content browser:** our own presets library (objects, materials) and recent files.
- [ ] **Layer manager**, **Console panel** (log and Rhai REPL), and **HUD overlays**.
- [ ] **HiDPI, IME, keyboard-only navigation, high-contrast theme** (basic accessibility).

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O10, G1 | GUI-SESSIONS §G1: layout arrangement, manager columns, tab names per object type, menu structure | gui-only |
| G2 | GUI-SESSIONS §G2: shortcut list for the 80 most-used commands; mouse navigation | gui-only |
| G4 | GUI-SESSIONS §G4: Attribute manager widgets (scrub behaviour, field list, keyframe dots, link drop) | gui-only |
| O29 | preferences and document defaults | todo |

## Implementation design

- **Dock:** stays on `egui_dock` (already in place). A `Layout` is serialisable state.
  The default layout is defined in code to match G1.
- **Icons:** our own icon set (`crates/ui/icons/*.svg`), authored by us, rendered through
  `resvg` into a texture atlas at startup for the current UI scale.
- **Widgets:** `crates/ui/src/widgets/` holds one module per widget type. Each widget is
  driven by `ParamDesc` (DECISIONS D3), so new node types get their UI for free.
- **Shortcuts:** a `KeyMap` resource maps key chords to command ids. Defaults ship as a
  TOML file we write. User overrides go in the settings directory (N/175).
- **Tests:** egui_kittest for every manager's core interactions; snapshot tests of the
  layout (rendered offscreen with lavapipe, compared to our own baseline images, never
  against the reference).

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 147 | Familiar default layout: command bar, tool palette, manager arrangement | 011 | G1 |
| 148 | Icon set and icon atlas | 147 | G1 (layout only, no tracing) |
| 149 | Object manager parity: columns, dots, tags, drag/drop, search, context menus | 147 | G1 |
| 150 | Attribute manager parity: tabs, widgets, multi-edit, keyframe dots, history | 147 | G4 |
| 151 | Coordinates manager modes | 150 | G1 |
| 152 | Key map: defaults, customisation editor | 011 | G2 |
| 153 | Viewport navigation parity (keys, mouse, frame commands, views) | 152 | G2 |
| 154 | Content browser (own presets) and recent files | 163 | – |
| 155 | Preferences window and settings storage | 175 | O29 |
| 156 | Layout save/load and workspaces | 147 | G1 |
| 157 | HUD and viewport overlays | 143 | G8 |
| 158 | Layer manager | 149 | G1 |
| 159 | Command palette and undo history panel | 011 | – |
| 160 | Console panel (log + Rhai REPL) | 016 | – |
| 161 | HiDPI, IME, themes, keyboard navigation | 147 | – |
| 162 | S3 acceptance test and UI docs | 147–153 | – |

## Done when
A user of the reference app finds every manager where they expect it. The default
shortcuts work. S3 passes on all three OS.
