# GUI sessions (owner at the reference app)

Some behaviour only shows in the GUI: layout, mouse tools, drag behaviour, shortcuts,
editor look. These sittings need a person at the reference app. Each takes 20–40 minutes.

**Rules** ([../CLEANROOM.md](../CLEANROOM.md)):
- Write down what you **see and do**, in your own words.
- **Never** commit screenshots, icon images, layout files or preset files.
- You may take screenshots for yourself while writing, then delete them.

Write the answers into this file under each session, commit on `observations`, and push.
The reviewer turns them into specs.

You can also do a sitting **with an observer session's help**:
1. You perform the steps.
2. The observer, using `viewport_screenshot` through the bridge, describes what changed.
3. You confirm the description.

## G1 · Layout and managers
1. Reset to the Standard layout. List the panels in each area of the window (top, left
   column, centre, right top, right bottom, bottom strip), left to right and top to bottom.
2. **Object manager:**
   - the columns from left to right;
   - what the two dots and the tick do;
   - what right-click on an object offers (top-level menu items only).
3. **Attribute manager:** for a Cube, a Bend, a Cloner, a Camera and a material, list the
   tab names in order.
4. **Menus:** the top menu bar names in order; the Create menu's submenus; the Mesh
   menu's submenus.
5. **Command bar and tool palette:** the order of the button groups (describe the icons
   in words only).
6. **Content browser and recent files** (brief 154, parity-pending):
   - where the content browser opens by default and its tabs or top-level folders;
   - how many files File > Open Recent keeps, and whether opening one moves it to the top;
   - whether a saved preset of a selection keeps links to objects outside the selection.

## G2 · Shortcuts and navigation
1. For each of these, the key it uses (write "none" if none):
   - move, rotate, scale, live selection, rectangle selection;
   - points / edges / polygons / model mode;
   - undo, redo;
   - frame all, frame selected;
   - the 4-view toggle; each view;
   - render view, render to picture viewer;
   - play, record, autokey;
   - next/previous frame and key;
   - knife, loop cut, extrude, inset, bevel, bridge;
   - subdivide, optimize, select all, deselect, invert, grow, shrink;
   - delete, group, ungroup;
   - make editable, current state to object;
   - copy, paste, duplicate, hide, unhide.
2. Mouse navigation: what Alt+left/middle/right drag do; what the 1/2/3 keys plus drag
   do; what the scroll wheel does; the direction of each.

## G3 · Interactive modeling and selection
1. Loop/Path Cut: how you place a cut, how many cuts, how the offset is set, what the
   HUD shows.
2. Knife modes Line, Plane, Loop: the steps for each.
3. Bridge two border loops; Close Polygon Hole; Stitch and Sew: the steps for each.
4. Selection:
   - click, Shift+click, Ctrl/Cmd+click;
   - rectangle select and its "only visible" option;
   - loop select (double-click?) and ring select: the steps;
   - Grow and Shrink with the …-dialog: which settings it shows.
5. When switching points ↔ polygons mode, is the selection converted?
6. Soft selection: where it lives and its modes.
7. **Cuts to export** (brief 024; the script couldn't run them). Do each in the GUI on a
   default cube made editable (segments 1/1/1), then have the observer export the mesh
   through the bridge to `tests/golden/modeling-tools/<case>/mesh.obj` with a `case.toml`
   naming the steps:
   - `loopcut-mid`: Loop/Path Cut across the 4 side faces, offset 50 %;
   - `loopcut-quarter`: the same at 25 % (note which end the 25 % is measured from);
   - `loopcut-two`: two cuts at once, if the tool's count setting allows it;
   - `knife-line`: Knife Line from the middle of the top face's left edge to the middle
     of its right edge, seen from the Top view; note whether a cut that ends inside a
     polygon is kept or dropped;
   - `slide-loop` (2/2/2 cube): slide the middle loop 50 % up; note the range and the
     sign of the value in the HUD;
   - `bridge-two` (1/1/1 cube with top and bottom polygons deleted): Bridge the two open
     borders; note how the tool picks which points pair up;
   - `close-hole` (1/1/1 cube with the top polygon deleted): Close Polygon Hole on it;
   - `stitch` (2/1/1 cube, polygon 6 disconnected and moved up 20): Stitch and Sew the
     seam back; note the merge position and tolerance;
   - `pen-quad`: Polygon Pen, click four points of a new quad above the cube; note the
     snap radius and how to finish the polygon.
8. Extrude drag: how far the offset changes per screen pixel at the default camera
   (drag 100 px and read the offset).

9. **Spline point and handle dragging** (brief 190, parity-pending): on an editable Bezier
   spline in Points mode, drag one tangent handle. Does the opposite handle follow (mirrored),
   and which key breaks the link? Does dragging a point move its handles with it?

## G4 · Attribute manager widgets and field lists
1. Number fields: drag to scrub? With which modifier for fine/coarse steps? What arrow
   clicks do.
2. Keyframe dots next to parameters: their states and colours (in words), and what
   clicking them does.
3. Field list:
   - how to add a field;
   - the columns (name, blend, strength, …);
   - drag reordering;
   - folders.
4. Link fields: drag and drop; clear.

## G5 · Timeline, dope sheet and F-curves
1. Timeline strip at the bottom: what it shows, how to scrub, what keys look like.
2. Dope sheet: how to open it; the track tree; moving and scaling keys; ripple.
3. F-curve mode:
   - how handles look and behave;
   - breaking tangents;
   - weighted tangents;
   - the right-click key presets (ease in/out names);
   - the speed/velocity curve view, if present, and how it is edited.
4. Preview range, markers and playback modes: where each is set.

- **Motion path and onion skin** (brief 095, parity-pending): with an object animated along
  x over frames 0–30, turn on its motion path and ghosting. Record the path colours (past and
  future), key marker shape and size, default frame range and ghost count, ghost spacing and
  fade, and whether unselected animated objects show paths too.

## G6 · Rigging tools
1. Joint tool: click-to-create a chain; how the tool aligns joints.
2. Weight painting: brush modes, strength, falloff; how the weights are shown in the
   viewport (in words).
3. Weight manager: its list and buttons.
4. Node expression editor (classic): how to add nodes, connect ports, and what the node
   look is (in words).

## G7 · Materials and UV
1. Material manager: how materials are shown; drag onto an object.
2. Material editor: the channel list on the left; the preview.
3. Node material editor: the look of nodes and ports, and how to add nodes.
4. UV editing layout: the panels in it; how to relax and pack.

## G8 · Viewport and picture viewer
1. Display modes menu: the names of each shading and line mode.
2. 4-view layout: which views and in which corners.
3. Picture viewer: the history list, A/B compare, channels.
4. Safe frames and the HUD: what they show by default.

- **Null display** (brief 046): for each Null Display shape, the drawn shape at Radius 100 and
  Aspect Ratio 1, then one shape at Radius 50 / Aspect 2, and each Orientation option
  (screenshot plus what the shape looks like in words).


## Answers
(The owner fills this in, per session, with the date.)
