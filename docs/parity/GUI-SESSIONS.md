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

## Owner notes after first hands-on use (2026-11-26)

Recorded by the planner from the owner's message. These are black-box observations of the
reference app's behaviour, in the owner's own words. They answer parts of G1, G2 and G5 and
**take precedence over our earlier own-design choices** (decision D22).

| # | Observation (owner) | Answers | Replaces |
|---|---|---|---|
| N1 | Viewport navigation: hold **1** + left-drag = pan (move), hold **2** + left-drag = zoom, hold **3** + left-drag = orbit. "One of the best features of the reference app and a main criteria." | G2 | UI.md §6.1 hold keys X / C / V |
| N2 | **E** = move, **R** = rotate, **T** = scale (tool / gizmo) | G2 | W / E / R |
| N3 | Clicking a parameter's keyframe button when a key exists at the current frame removes that key (not only through the context menu) | G5 | add-only click |
| N4 | The clone toolset (Cloner and its family) is reachable from the top level of the UI, not buried in a submenu | G1 | Create menu only |
| N5 | Docking: panels are dockable (drag a panel to another place, split, tab) | G1 | fixed six-area dock |
| N6 | Model / point / edge / polygon mode is set in **one** place, the way the reference does it; our editor has two interfaces for it | G1, G2 | duplicate mode UI |
| N7 | The object manager shows clearly that an object is a child (hierarchy cues) and separates rows more, so the tree is easier to read | G1 | UI.md §13 rows |
| N8 | Buttons are laid out like the reference (menus and layout) and show **icons** for their action | G1 | text buttons |
| N10 | Keyframe drafts (owner, after testing the 224 build): on a property that **has a key track**, with Auto-Key off, changing the value by **any** route (moving the object with the gizmo, the Coordinates panel, the inspector) shows the new value **in the viewport and the inspector** and marks the key button as *modified*, but **does not store it** in the animation. Clicking the key button stores it as a key at the current frame. (Not stated by the owner, parity-pending G5: what happens to an unkeyed draft when the frame changes; our rule is that it is discarded and the animated value returns.) | G5 | inspector-only drafts; gizmo and Coordinates edits not staged |
| N11 | Theme (owner, 2026-11-27, after reviewing the design proposal on branch `design/forge3d-workspace`): use the proposal's **mint** theme for active tools and state, and show **selected objects in orange (amber)**, as an outline or wireframe on the object in the viewport and as the same accent wherever selection appears, so it is clear what is selected throughout the app. The proposal's layout itself is kept as a reference only; the structure stays the reference app's (D22) | D22 palette (ours) | one accent for every state; selection shown as a tint |
| N9 | In the inspector (attribute manager), not every setting is at the same level: the reference divides settings into **tabs** (and groups) | G1 item 3, O64 | one flat list per object |

Still open for the owner's sitting: the exact layout (G1 items 1, 4, 5 and the new G1 items 7–9),
the full key list (G2), and what the shading keys are, now that 1 / 2 / 3 are navigation (G2 new
item).

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

7. **Object manager rows** (N7): how a child is shown (indent per level in pixels, lines or
   brackets joining parent and child, the fold arrow), how rows are separated (lines,
   alternating shades, spacing), what is on a row from left to right (icon, name, tags, the
   two dots, the tick).
8. **Modes** (N6): where model / point / edge / polygon mode is switched (which bar, which
   order), and whether a menu also has it.
9. **Clone toolset at the top level** (N4): where the Cloner, Matrix, Fracture, effectors and
   fields are reached (a menu bar menu, a command bar button with a drop-down, or both), and the
   order of the items.

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


## G9 · Leftovers the scripted observer could not do

Each item names the form row it fills. Record the result in that form, as usual.

1. **O25 Bridge, Close Polygon Hole, Stitch and Sew** (modeling-tools-2.md): run each once in the
   GUI on the form's setup and export the mesh. The scripted tool returned false in four variants.
2. **O26 Loop and Ring selection** (selection-ops.md): select with the interactive Loop and Ring
   tools on the form's cube and save the selection the observer's way.
3. **O30 `spot` and `infinite`** (render-reference.md): build both in the GUI and render to the
   Picture Viewer. A scripted render of the spot comes out black even with the light read back
   pointing down, and a script cannot set the light Type to Infinite (it reads back Square Spot).
3b. **O37 Pose Morph** (pose-morph.md; D20): on the form's setup, set the morph strengths by hand
   in the tag (each case's row: A 0/50/100/150 %, A 100 % + B 50 %, and the absolute mode), and
   after each one let the observer export the mesh by script (it can export; it just can't set the
   strength). About 15 minutes; this checks our own-design Pose Morph (brief 107).
4. **O40 particles** (particles.md): the emitter's particles can't be read by script. In the GUI,
   step to frames 10 and 30 and record the particle count and the positions of the first five
   particles shown by the particle info readout, or say there is no such readout.
5. **O32/O33** `subtract`, `intersect`, `points-source` and `vdb-export` (volumes.md, volumes-2.md):
   these need a dialog or a GUI-only mode.

## G10 · Look, scale and full menus (a Cowork session may do this; owner, 2026-11-26)
Set the window to **1920 x 1080**, OS display scaling 100 % (or note the scaling), the Standard
layout, the default theme. Measure in screen pixels from your own screenshots, then delete them.
1. **Panel geometry:** each panel's position and size (x, y, width, height) and as a percentage
   of the window; the splitter widths; which panels share an area as tabs.
2. **Density:** the menu bar height; the command bar button size and icon size, and the gap
   between groups; the tool palette button size; the object manager row height and indent per
   level; the attribute manager row height, label column width, tab bar height and tab button
   size; the timeline height; the body text size (cap height in pixels) and whether labels are
   bold anywhere.
3. **Structure cues:** separators (lines, gaps, shading), group headers (style, fold arrows),
   how the active tab or tool is shown, how hover is shown, tooltips (delay, what they show).
4. **Menus, complete:** every menu bar menu, every item and **every submenu**, in order, with
   separators (`---`), the shortcut shown beside an item, and `…` items that open a dialog. The
   same for the right-click menus of the object manager, the viewport (no selection, an object,
   points in point mode) and the timeline.
5. **Command bar and tool palette:** each button in order, its tooltip text, whether it has a
   drop-down (and that drop-down's items in order), described in words (never draw or trace an
   icon).
6. **Viewport chrome:** the viewport's own menu bar or header (its menus and items), the axis
   widget, the HUD text, the grid look, how the selected object is highlighted.
7. **Overall feel:** in a few sentences, what makes the layout feel the way it does (dense or
   airy, flat or bevelled, contrast, where the eye goes first). No colour values: we keep our own
   palette (D22).

## Answers
(The owner fills this in, per session, with the date.)

### G10 / G1 partial (owner screenshot, transcribed by the planner, 2026-11-26)
The owner supplied one screenshot of the Standard layout with an empty document. The planner read it
and recorded the structure in words in [../specs/UI.md](../specs/UI.md) §29 (window rows; tool
strip; command bar groups; viewport menu bar and HUD; timeline row; object palette strip with
colour-coded creation groups, including the Cloner at the top level; managers column; attribute
manager tabs as wrapping toggle buttons, two-column groups, step arrows). This answers G1 items 1,
5 and part of 3, and G10 items 1 (proportions only), 3 and 7. The screenshot is not in the
repository. Exact pixel sizes: O66.

### Owner's Cowork session, 2026-11-27 (G10, G1, G2, G4, G5, J)

Source: the owner ran a Cowork session at the reference app on their Mac and supplied its written
answers. The planner transcribed them here, replacing product, vendor and third-party names with
neutral placeholders in brackets. Screenshots stayed on the owner's Mac (`gui-shots/`) and are not in
the repository. Items the session marked open or unverified stay open.

#### G10 (Cowork, 2026-11-27)

#### 1. Window, scale and appearance (A1)

The reference app was foregrounded on macOS. The empty Untitled 1 document showed a single Perspective view using Default Camera, with nothing selected. The display used a charcoal interface with muted light-gray text, violet-blue active controls and colored axis lines. The theme preference was left untouched. The native document window was resized by dragging the right and bottom edges.

| Measurement | Observed |
| --- | --- |
| Window capture, including the title bar | 1920 × 1080 px, verified from the image header |
| Effective capture scaling | 1×. A 1520-point horizontal resize reduced the captured width by 1520 px, and a 274-point vertical resize reduced the captured height by 274 px |
| Capture origin | Top-left of the window. The macOS menu bar sits outside it |
| Native title bar and document/layout row | y 0–28 and 28–52 px |
| Command bar and viewport menu row | y 52–94 and 94–114 px |
| Left tool strip | x 0–43 px from the command bar to the bottom of the viewport |
| Viewport canvas | x 43–1472, y 114–962 px, about 1429 × 848 px |
| Object palette | x 1473–1516 px, approx. 43 px wide |
| Objects/Takes and Attributes/Layers | x 1517–1919 px, approx. 402 px wide. Horizontal split near y 500 px |
| Timeline and status | Timeline x 43–1473, y 963–1055 px. Status bar y 1055–1079 px |
| Common command and palette button sizes | Approx. 27 × 26 px on the top bar and 34 × 34 px on the vertical palettes |

Panel edges and controls below are approximate image-pixel readings. The empty viewport's axis origin is near (757, 538). The empty scene shows a perspective grid, red X, green Y and blue Z axes, orientation widget at upper right, and “Grid Spacing : 50 cm” at lower right. Horizontal tinted margins near the top and bottom correspond to the visible camera frame.

#### 2–8. Window regions (A2–A8)

| Item | Window area | Controls in visual order |
| --- | --- | --- |
| A2 | Top left, y 28–94 | Undo and redo arrows, Untitled 1 document tab with close control, plus for a new tab. Below that, search at far left, a small tray-like control, X/Y/Z axis toggles, and an axes-with-cube control |
| A3 | Command clusters at x 539–1012 and 1350–1470 | Component and model modes, axis/workplane-related controls, magnet and settings, grid controls, target/settings, solo controls, then the render group at far right with clapperboard-style buttons and a circular control |
| A4 | Left strip, controls to about y 605 | Search, selection and selection settings, move/rotate/scale, axes-and-pointer controls, pen tools, brush and freehand tools. Move has a violet-blue background in the baseline |
| A5 | Object palette, x about 1478–1513 | Axes and circle, square outline, cube, T, green circle with handles, green cubes, green ring, violet bent shape, violet axes with cube, magenta field shape, globe, camera, light bulb, and a lower context-sensitive control |
| A6 | Layout row, upper right | Standard, Model, Sculpt, UV Edit, Paint, Groom, Track, Script, Nodes, plus, New Layouts switch. Standard is underlined |
| A7 | Timeline, y 963–1055 | Transport and recording controls above the ruler, current-frame readout, selected-frame highlight, project bounds and preview range. Baseline ruler 0–90 F, numbered every 5 frames at this width |
| A8 | Status bar, y 1055–1079 | Menu glyph at far left and contextual help. With Move active it explains dragging, Shift quantization or adding in point mode, and Ctrl removal |

#### C. macOS menu bar

Each top-level menu was opened live in the empty document. The accessibility tree exposed nested entries, including deep branches, without requiring a separate visible capture for each branch. † marks an entry disabled in that state. The transcript records exposed labels in order. Separator styling, menu checkmarks, and ordinary macOS key equivalents were unavailable in that tree. The two recent-file names are omitted. Individual submenu opening and visual capture were incomplete.

| Menu path | Entries in order |
| --- | --- |
| App menu | About the app · Preferences... · Services · Hide (app) · Hide Others · Show All · Quit and Free License · Quit (app) · Quit and Keep Windows |
| App menu › Services | Activity Monitor · Allocations & Leaks · File Activity · System Trace · Time Profile Active Application · Time Profile App Under Mouse · Time Profile Entire System · Toggle Instruments Recording · Services Settings… |
| File | New Project · Open Project... · Open Recent · Merge Project... · Revert to Saved Project... · Close · Close All Projects · Save Project · Save Project as... · Save Incremental... · Save All Projects · Save Project with Assets... · Save Selected Objects as... † · Save Project for (exchange plug-in)... · Save All Takes with Assets · Save Marked Takes with Assets † · Export... |
| File › Open Recent | 1. Recent project · 2. Recent project |
| File › Export... | 3D Studio (*.3ds) · Alembic (*.abc) · Allplan (*.xml) · Bullet (*.bullet) · COLLADA 1.4 (*.dae) · COLLADA 1.5 (*.dae) · Direct 3D (*.x) · DXF (*.dxf) · FBX (*.fbx) · Forger (*.fpk) · glTF (*.gltf/*.glb) · Illustrator (*.ai) · STL (*.stl) · USD (*.usda/*.usdc) · Volume (*.vdb) · VRML 2 (*.wrl) · Wavefront OBJ (*.obj) |
| Edit | Undo † · Redo † · Undo (Action) † · Cut † · Copy † · Paste † · Delete † · Project Settings... · Project Info... · Scale Project... · Preferences... |
| Create | Null · Spline Primitive · Mesh Primitive · Generator · Deformer · Field · Environment · Camera · Light · Material · XRef · Tag · Bake as Alembic † · Bake as Alembic + Delete † |
| Create › Spline Primitive | Text Spline · Arc · Circle · Helix · n-Side · Rectangle · 4-Side · Cissoid · Cogwheel · Cycloid · Flower · Profile · Star · Formula · Empty Spline |
| Create › Mesh Primitive | Text · Cube · Cylinder · Plane · Disc · Polygon · Sphere · Capsule · Cone · Figure · Landscape · Oil Tank · Pyramid · Platonic · Tube · Torus · Bezier · Empty Polygon |
| Create › Generator | Subdivision Surface · Cloth Surface · Extrude · Lathe · Loft · Sweep · Spline Mask · Boole · Connect · Symmetry · Instance · Array · Atom Array · Remesh · Polygon Reduction · Metaball · LOD · Relief · Vectorizer · Vector Import · Grow Grass † · Python Generator |
| Create › Deformer | Bend · Bulge · Shear · Taper · Twist · FFD · Camera · Correction · Mesh · Explosion · Explosion FX · Melt · Shatter · Jiggle · Squash & Stretch · Collision · Shrink Wrap · Spherify · Delta Mush · Smoothing · Surface · Wrap · Spline · Spline Rail · Spline Wrap · Displacer · Formula · Morph · Point Cache · Wind · Bevel |
| Create › Field | Linear Field · Radial Field · Spherical Field · Box Field · Cylinder Field · Cone Field · Capsule Field · Torus Field · Random Field · Shader Field · Sound Field · Formula Field · Python Field · Group Field |
| Create › Environment | Sky · Floor · Environment · Background · Foreground · Stage |
| Create › Camera | Camera · Target Camera · Stereo Camera · Motion Camera · Camera Morph † · Camera Crane |
| Create › Light | Light · Spot Light · Target Light · Area Light · PBR Light · IES Light... · Infinite Light · Sun Light · Physical Sky |
| Create › Material | New Default Material · Materials · Extensions · Node Material Presets |
| Create › Material › Materials | New PBR Material · New Standard Material · New Node Material · New Uber Material · New Grass Material · New Hair Material · New PyroCluster Material · New PyroCluster VolumeTracer Material · New Shadow Catcher Material · New Sketch Material |
| Create › Material › Extensions | Banji · Banzi · Cheen · Danel · Fog · Mabel · Nukei · Terrain |
| Create › Material › Node Material Presets | Car Paint · Ceramic · Concrete · Cutout · Emission · Gold · Granite · Marble · Metal · Plastic · Rock · Wood |
| Create › XRef | Add XRef... · XRef · XRef Manager · Reload XRefs † · Convert Object Selection to XRef † · Convert Material Selection to XRef † · Simple XRef |
| Create › Tag | << No Object selected >> † |
| Modes | Execution · Coordinates · Points · Edges · Polygons · Model · Object · Animation · Texture · Enable Axis · UV Mode † · Tweak · Modeling Settings · Snap · Workplane · Workplane · Viewport Solo · Viewport Solo Hierarchy · Viewport Solo Automatic |
| Modes › Execution | Use Animation · Use Expressions · Use Generators · Use Deformers · Use Motion System · Use IK · Use Constraints · Use Symmetry · Use Hair |
| Modes › Coordinates | X-Axis / Pitch · Y-Axis / Heading · Z-Axis / Bank · Coordinate System |
| Modes › Snap | Enable Snap · Auto Snap  · 3D Snap · 2D Snap · Tool Specific Snap · Enable Quantizing · Vertex Snap † · Edge Snap † · Polygon Snap † · Spline Snap † · Axis Snap † · Intersection Snap † · Mid Point Snap † · Grid/Workplane Snap † · Grid Point Snap † · Grid Line Snap † · Pixel Snap † · Guide Snap † · Dynamic Guide † · Perpendicular Snap † |
| Modes › Workplane | Locked Workplane · Planar Workplane · Camera Workplane · Axis Workplane · Interactive Workplane · Align Workplane to X · Align Workplane to Y · Align Workplane to Z · Align Workplane to Selection · Align Selection to Workplane † |
| Select | Live Selection · Rectangle Selection · Lasso Selection · Polygon Selection · Selection Filter · Loop Selection U~L † · Ring Selection U~B † · Outline Selection U~Q † · Fill Selection U~F † · Path Selection U~M † · Phong Break Selection U~N † · Mirror Selection... U~V † · Select All † · Deselect All † · Invert U~I † · Select Connected U~W † · Grow Selection... U~Y † · Shrink Selection U~K † · Hide Selected † · Hide Unselected † · Show All † · Invert Visibility † · Convert Selection U~X † · Store Selection † · Convert Vertex Map... · Set Vertex Weight... † |
| Select › Selection Filter | Selector... · Create Selection Object · All · None · Null · Polygon · Spline · Generator · Subdivision Surface · Deformer · Camera · Light · Environment · Particle · Other · Joint |
| Tools | Commander · Move · Rotate · Scale · Place · Dynamic Place · Scatter Pen · Reset Transform † · Freeze Transform † · Unfreeze Transform † · Axis · Arrange † · Center † · Duplicate † · Transfer † · Randomize † · Replace With... † · Project Object... † · Paint Tool · Lighting Tool · Naming Tool · Annotation Tool · Guide · Doodle · Convert |
| Tools › Axis | Axis Center... · Center Axis to · Center Object to · Center Parent to · Center to Parent · View Center · Reset Scale... † |
| Tools › Guide | Measure & Construction · Guide · Guide Tool · Delete Enabled Guides |
| Tools › Doodle | Doodle Paint · Doodle Eraser · Add Doodle Frame · Duplicate Doodle Frame · Clear Doodle Frame · Remove Doodle Frame · Set Doodle Viewport |
| Tools › Convert | Make Editable † · Current State to Object † · Connect Objects † · Connect Objects + Delete † · Polygon Islands to Objects † |
| Spline | Spline Pen · Sketch · Spline Arc Tool · Spline Smooth · Magnet M~I † · Line Cut K~K † · Add · Remove · Clone · Move · Tangents · Segments · Point Order · Boole Commands |
| Spline › Add | Chamfer † · Create Point M~A † · Cross-Section · Round † · Subdivide... U~S † |
| Spline › Remove | Optimize... U~O † · Weld M~Q † |
| Spline › Clone | Create Outline † · Mirror M~H † · Disconnect... U~D † · Split U~P † |
| Spline › Move | Set Point Value M~U † · Straighten † · Project † |
| Spline › Tangents | Hard Interpolation † · Soft Interpolation † · Equal Tangent Length † · Equal Tangent Direction † |
| Spline › Segments | Join Segment † · Break Segment † · Explode Segments † |
| Spline › Point Order | Set First Point † · Reverse Sequence † · Move Down Sequence † · Move Up Sequence † |
| Spline › Boole Commands | Spline Subtract † · Spline Union † · Spline And † · Spline Or † · Spline Intersect † |
| Mesh | Polygon Pen M~E · Bevel M~S † · Extrude M~T † · Inset M~W † · Line Cut K~K † · Volume Mesh... U~H † · Add · Remove · Clone · Cut · Move · Edge to Spline † · N-Gons · Normals · Sculpt · Brushes · UV |
| Mesh › Add | Create Point M~A † · Close Polygon Hole M~D † · Bridge M~B † · Matrix Extrude M~X † · Smooth Shift M~Y † · Subdivide... U~S † · Weight Subdivision Surface M~R † |
| Mesh › Remove | Weld M~Q † · Stitch and Sew M~P † · Collapse U~C † · Melt U~Z † · Dissolve M~N † · Optimize... U~O † · Resolve Non-Manifold † |
| Mesh › Clone | Array † · Clone † · Mirror M~H † · Split U~P † · Disconnect... U~D † |
| Mesh › Cut | Plane Cut K~J † · Loop/Path Cut K~L † · Edge Cut M~F † · Connect Points/Edges M~M † · Triangulate † · Untriangulate U~U † |
| Mesh › Move | Iron M~G † · Brush M~C † · Magnet M~I † · Set Point Value M~U † · Slide M~O † · Spin Edge M~V † · Change Point Order † · Normal Move M~Z † · Normal Scale M~- † · Normal Rotate M~ † |
| Mesh › N-Gons | N-gon Triangulation U~T · Retriangulate N-gons U~G † · Remove N-gons U~E † |
| Mesh › Normals | Align Normals... U~A † · Reverse Normals... U~R † · Break Phong Shading † · Unbreak Phong Shading † · Select Broken Phong Edges † |
| Mesh › Sculpt | Subdivide... † · Increase † · Decrease † · Mask † · Invert Mask † · Clear Mask † · Hide Masked † · Show Masked † · Bake Sculpt Objects † · Sculpt Symmetry † · Project Mesh · Unsubdivide † · Sculpt To Pose Morph † |
| Mesh › Brushes | Draw † · Wax † · Knife † · Inflate † · Amplify † · Repeat † · Grab † · Smooth † · Pinch † · Flatten † · Fill † · Scrape † · Erase † · Select † · Link Size · Link Strength · Link Symmetry · Link Stencil |
| Mesh › UV | New Texture View... · UV Manager... · UV Transform † · Add UV Pins † · Remove UV Pins † · Clear UV Pins † · Set UVW from Projection... † · UV Unwrap... † · UV Weld and Relax... † · UV Weld † · UV Peeler U~J † · Start Interactive Mapping † · Stop Interactive Mapping † · Resize Commands · Mirror U † · Mirror V † · Line Up UV † · UV Straighten... † · UV Rectangularize... † · Align UV Islands † · Unstitch UV † · Boundary to Circle † · Boundary to Quad † · Select All UV Seams † · Transform Tools · Transform Commands |
| Mesh › UV › Resize Commands | Max UV † · Fit UV to Canvas † · Fit Canvas to UV † |
| Mesh › UV › Transform Tools | Magnet · Shear U · Shear V |
| Mesh › UV › Transform Commands | Copy UV † · Paste UV † · Reset UV † · Cycle UV CCW † · Cycle UV CW † · Flip Sequence † · Store UV † · Restore UV † · Remap... † |
| Volume | Clear Cached Volumes · Volume Builder · Volume Mesher · Signed Distance Field Filters · Fog Filters · Vector Filters · Volume Loader |
| Volume › Signed Distance Field Filters | SDF Smooth · SDF Dilate and Erode · SDF Close and Open |
| Volume › Fog Filters | Fog Smooth · Fog Multiply · Fog Invert · Fog Add · Fog Range Map · Fog Curve |
| Volume › Vector Filters | Vector Smooth · Vector Scale · Vector Rotate · Vector Invert · Vector Normalize · Vector Curl |
| (clone toolset menu) | Effector · Cloner · Matrix · Fracture · Voronoi Fracture · MoInstance · Tracer · MoSpline · MoExtrude · PolyFX · Hide Selected † · Swap Cloner/Matrix · (clone) Selection · (clone) Weight Paintbrush · Linear Clone Tool · Radial Clone Tool · Grid Clone Tool |
| (clone toolset menu) › Effector | Plain · Delay · Formula · Inheritance · Push Apart · Python · Random · ReEffector · Shader · Sound · Spline · Step · Target · Time · Volume · Group |
| Character | Manager · Constraints · Character · CMotion · Character Builder · Joint Tool · Joint Align Tool · Create IK Chain † · Bind... † · Weight Tool · Mirror Tool · Commands · Convert · Joint · Skin · Muscle · MSkin · Cluster · Create Cluster † · Add Point Morph † · Add Transform Morph † · Falloff |
| Character › Manager | Pose Library Browser · Weight Manager · VAMP... |
| Character › Constraints | Add Aim Constraint † · Add Axis Constraint † · Add Clamp Constraint † · Add Mirror Constraint † · Add Origin Constraint † · Add Parent Constraint † · Add Plane Constraint † · Add Point Constraint † · Add Position Constraint † · Add Transform Constraint † · Add Rotation Constraint † · Add Scale Constraint † · Add Spline Constraint † · Add Spring Constraint † · Add Surface Constraint † · Add Up Vector Constraint † · Add Bounds Constraint † |
| Character › Character Builder | Save Character Template † · Save Character Template As... · Load Character Template... · Reload Character Templates · Copy Component † · Cut Component † · Paste Component † · Paste Mirror Component † · Paste to Component † · Mirror to Component † · Delete Component † |
| Character › Commands | Align † · Copy Chain † · Mirror Chain † · Map Chain † · Set Preferred † · To Preferred † · Reset Pose † · Set Pose † |
| Character › Convert | Spline to Joints † · Joints to Spline † · Convert to Nulls † · Convert to Joints † · Selection to Joints † · Selection to Nulls † · Convert Joints to Polygon Object † |
| Animate | Play Mode · Playback · Record · Frame Rate · Preview Range · Show Tracks · Make Preview... · Cappucino... · Add Relative Animation Layer † · Add Absolute Animation Layer † · Add Motion Clip... † · Add Empty Motion Layer † · Pivot · Auto Redraw... |
| Animate › Play Mode | Preview Range · Simple · Cycle · Ping-Pong |
| Animate › Playback | Play Forwards · Play Backwards · Stop · Play Sound · Go to Start · Go to End · Go to Frame... · Go to Next Frame · Go to Previous Frame · Go to Next Key · Go to Previous Key |
| Animate › Record | Record Active Objects † · Record Animated † · Record Hierarchy † · Autokeying · Keyframe · Tween Tool · Position · Scale · Rotation · Point Level Animation · Parameter · Link XYZ Sub-channels · Delete Keys † |
| Animate › Record › Keyframe | Active Objects · Restrict Editor Selection |
| Animate › Frame Rate | All Frames · Project · 1 · 5 · 10 · 15 · 18 · 24 · 25 · 30 · 50 · 60 · 100 · 250 · 500 |
| Animate › Preview Range | Set Range to Markers · Go to Next Range · Go to Previous Range · Set Min Preview Range · Set Max Preview Range |
| Animate › Show Tracks | Show Position F-Curves † · Show Scale F-Curves † · Show Rotation F-Curves † |
| Simulate | Cloth Cache Tool · Dynamics · Particles · Thinking Particles · Forces · Hair Objects · Hair Mode · Hair Edit · Hair Selection · Hair Tools · Hair Options |
| Simulate › Dynamics | Connector · Spring · Force · Motor |
| Simulate › Particles | Emitter · Bake Particles... † |
| Simulate › Thinking Particles | TP Geometry · TP Settings... · TP ParticleDraw † |
| Simulate › Forces | Attractor · Deflector · Destructor · Field Force · Friction · Gravity · Rotation · Turbulence · Wind |
| Simulate › Hair Objects | Add Hair · Feathers · Fur |
| Simulate › Hair Mode | Tips · Roots · Points · Guides · Vertices · Next Vertex † · Previous Vertex † |
| Simulate › Hair Edit | Cut Guides † · Copy Guides † · Paste Guides † · Paste to New † · Delete Guides † · Convert to Spline † · Convert from Spline † · Connect Guides † · Split Guides † · Even Segments † · Set Initial State † · Hairs to Guides † · Set Roots · Unroot Guides † |
| Simulate › Hair Selection | Live Selection · Rectangle Selection · Lasso Selection · Polygon Selection · Selection · Set Selection † · Select All † · Deselect All † · Invert † · Grow Selection · Select Hidden † · Soft Selection · Soft Selection Manager · Convert Selected † · Hide † · Hide Locked † · Show All † · Lock Selected † · Unlock All † · Unlock Selected † |
| Simulate › Hair Tools | Move · Rotate · Scale · Brush · Comb · Clump · Curl · Cut · Push · Straighten · Add Guides · Mirror |
| Simulate › Hair Options | Symmetry · Symmetry Manager · Dynamics Interaction |
| Tracker | Motion Tracker · Object Tracker · 2D Tracks · Motion Tracker Graph View · Motion Tracker Track View · Full Solve · Lock View on Tracks † · Auto Update Tracks † · Create Mask † · Create User Tracks † · Select · Frame All Tracks · Frame Selected Tracks † · Nudge Track · Track Bidirectional † · Track Forward † · Track Backward † · Insert Key at Track Position † · Remove Key † · Trim Track † · Untrim Track † · Constraints |
| Tracker › Select | Select All Auto Tracks † · Select All User Tracks † · Select Reconstructed Tracks † · Invert Selection † · Hide Selected † · Hide Unselected † · Show All † · Invert Visibility † · Delete Selected † · Restore Deleted † |
| Tracker › Nudge Track | Nudge Track Up † · Nudge Track Down † · Nudge Track Left † · Nudge Track Right † |
| Tracker › Constraints | Create Position Constraint † · Create Vector Constraint † · Create Planar Constraint † |
| Render | Render View · Render Region · Render Selection † · Render to Picture Viewer · Team Render to Picture Viewer... † · Takes · Add to Render Queue... · Render Queue... · Interactive Render Region · Edit Render Settings... · My Render Setting · Team Render Machines... · Share Machine over Network † · Node Spaces · Lens Distortion · Flush Illumination Caches |
| Render › Takes | Render All Takes to PV · Render Marked Takes to PV † · Team Render All Takes to PV † · Team Render Marked Takes to PV † |
| Render › Node Spaces | Current (Standard/Physical) · Standard/Physical |
| Extensions | Console · Data Inspector... · Script Log... · Script Manager... · Tools · User Scripts · (compositing app) Live Link † · (CAD app) Scene Combine... † · Combine Project... † · Get (external renderer) † · (vendor motion-capture app) · (sculpting app) bridge · USD Bridge · Socket Server Plugin |
| Extensions › Tools | Execute Last Plugin · Reload Python Plugins · Source Protector... · Plugin Manager... · Export Licensing Overview... |
| Extensions › User Scripts | Run Script... · Script Folder... |
| Extensions › (sculpting app) bridge | Install · Importer · Exporter · Assign Polygroup |
| Extensions › USD Bridge | Update USD Changes † · Close USD Bridge † |
| Window | Customization · Minimize · Full-Screen Mode · Full-Screen (Group) Mode · Asset Browser... · Object Manager... · Scene Manager... · Node Editor... · Material Manager... · Material Editor... · Timeline (Dope Sheet)... · Timeline (F-Curve)... · Attribute Manager... · Coordinate Manager... · Take Manager... · Layer Manager... · Sculpt Layers... · Structure Manager... · Project Asset Inspector... · Picture Viewer... · Projection Man... · New View Panel... · New Texture View... · Color Settings · Layer Manager · Untitled 1 |
| Window › Customization | Command Manager... · Menus · Menu Manager... · Layouts · Load Layout ... · Save as Startup Layout · Save Layout as... · Lock Layout · New Group Window... · New Palette... · Load Palette... · Customize Palettes... · Save as Default Scene |
| Window › Customization › Menus | (app) Menu · (paint module) Menu · Model Menu · Sculpt Menu · User Menu 1 · User Menu 2 · User Menu 3 · User Menu 4 · User Menu 5 |
| Window › Customization › Layouts | Standard · Model · Sculpt · UV Edit · Paint · Groom · Track · Script · Nodes |
| Help | searchFieldAction: · search text field (settable) · table search result · Show Help... · Vendor Support... · Online Learning... · Quick Start Dialog... · Manual Installation... · License Manager... · Sign-Out · Legal Information... · About... |

#### B, D, E and screenshot coverage

A1–A8 were observed in full-window views. Corner triangles mark grouped buttons. The in-window menu headings are recorded in G1. The capture surface omitted the transient popups when opening a viewport menu and right-clicking a scene object. The nested visual menu captures, bar drop-down contents, panel menu contents, context menus, per-item close-ups and local screenshot batch remain incomplete. The session could observe full-window images in memory, but writes to the requested Desktop folder were denied.

#### G1 (Cowork, 2026-11-27)

#### 1. Standard layout

| Area | Panels and controls in order |
| --- | --- |
| Top | Native title bar, document tabs and layout switcher, command bar, viewport menu strip |
| Left column | Search and vertical selection, transform and modeling tools |
| Centre | Perspective viewport with menu and camera navigation controls. Viewport menu labels are menu glyph, View, Cameras, Display, Options, Filter, Panel |
| Between viewport and managers | Vertical object creation palette |
| Right top | Objects and Takes. The Objects header is menu glyph, File, Edit, View, Object, Tags, Bookmarks, then search and utility buttons |
| Right bottom | Attributes and Layers. The Attributes header is menu glyph, Mode, Edit, User Data, then navigation, search and utility buttons |
| Bottom | Transport and recording controls, current-frame field, frame ruler, range row, then status bar |

#### 2. Object Manager columns and visibility

The observed row order is tree disclosure and connector gutter, colored object-type icon, object name, a stacked layer glyph, a vertical pair of visibility indicators, a type-dependent enable/activation cell, and tags. Selected names are amber. The visibility indicators are the editor (upper) and render (lower) overrides. The check controls evaluation on objects that expose it. Clicking the Light check changed it to a red X and restored default viewport lighting. Camera rows instead have an active-camera target and Null rows have no evaluation check. Two successive clicks on the Cube’s upper visibility dot first kept the Cube visible and then hid it from the viewport, leaving the separate Sphere visible. A third click restored the default state and made the Cube visible again. The lower dot’s rendered result and the context list remain open.

#### 3. Attribute Manager tabs

| Selection | Tabs in reading order |
| --- | --- |
| Cube | Basic, Coord., Object, Phong |
| Bend | Basic, Coord., Object, Fields |
| Cloner | Basic, Coord., Object, Transform, Effectors |
| Camera | Basic, Coord., Object, Physical, Details, Stereoscopic, Composition, Spherical |
| Default material, named Mat | Basic, Color, Reflectance, Illumination, Viewport, Assign |
| Light | Basic, Coord., General, Details, Visibility, Shadow, Photometric, Caustics, Noise, Lens, Project |

Shift-clicking Light’s Details with General active showed both tab bodies in one vertical scroll area. General appeared first, with Details below it.

#### 4. Menus

App menu, File, Edit, Create, Modes, Select, Tools, Spline, Mesh, Volume, (clone toolset menu), Character, Animate, Simulate, Tracker, Render, Extensions, Window, Help. See the G10 table for the Create and Mesh trees and their descendants. Create › Tag depends on selection. The empty document exposed only a disabled “No Object selected” entry there.

#### 5. Command bar and tool palette

The top bar starts with the axis-related group, then the central mode group and neighboring axis, snap and workplane-related clusters, followed by the render cluster at the right. The left strip places selection above Move, Rotate and Scale, with spline/modeling tools and brushes below. G10 A2–A5 gives the observed placements and describes the glyphs.

#### 6. Content browser and recent files

Window › Asset Browser… opened a separate floating window about 304 × 298 px. Its header showed a menu glyph, Create, Edit and View, a search glyph and a utility control. The bottom showed list/grid toggles and a thumbnail-size slider. The body remained blank after its loading indicator disappeared, so the category hierarchy remains open. File › Open Recent showed two populated entries. Capacity, reordering and saved-preset external references remain unverified.

#### 7. Object Manager rows (F)

| Measurement | Observed |
| --- | --- |
| Row pitch | 20 px. Consecutive initial row centers were y 112, 132, 152 and 172 px |
| Child indentation | Approx. 12 px between the left edge of Bend’s icon and its Linear Field child icon |
| Disclosure and connectors | Small boxed minus in the gutter for the expanded Bend, with a thin right-angle connector to its child |
| Example type glyphs | Cube is a shaded cube, Cloner a green ring of pieces, Bend a violet bent outline, Linear Field a magenta field shape |
| Right-side glyphs | Two small gray dots stacked vertically, then an evaluation check where applicable. Cube and Sphere showed small violet Phong tag glyphs further right |

The exact test tree was observed with Cloner selected. Cube contained Bend, Cloner contained Sphere, and Null contained Child A and a collapsed Child B with its own child. An expanded branch shows a boxed minus, and a collapsed branch shows a boxed plus. The scene was set up in a separate untitled test document. The available drag control completed the gesture before a capture could be taken, so both in-progress drop markers remain open.

#### 8. Modes and animation state (H)

Points was the leftmost button in the centered mode row, near (552, 74). Activating it moved the violet-blue selection from Model near (637, 74) to Points and changed which modeling buttons appeared in the left strip. The parameter key and frame/selection tests are recorded under G4 item 2 and G5 item 1.

#### 9. Remaining prompt

The supplied session file ends its G1 questions at item 6. The user’s F and H requests identify items 7 and 8. No item 9 question was provided.

#### G2 (Cowork, 2026-11-27)

#### 1. Shortcuts

Shift+F12 opened Command Manager. The window provides Name Filter and Shortcut Filter, and shows Name, Information, Shortcut and Plugin columns. The table below uses its main-app or Model entries where present, with sequential shortcuts also observed in the native menus. “None” denotes a blank matching shortcut cell. M then a letter means two keypresses in sequence.

| Requested action | Shortcut | Context |
| --- | --- | --- |
| Move / Rotate / Scale | E / R / T | Model tools |
| Live Selection / Rectangle Selection | 9 / 0 | Model tools. A separate Rectangle Selection Hotkey row showed MX2 |
| Points / Edges / Polygons / Model | None / none / none / none | Mode command rows. Toggle Modeling Mode separately showed Return |
| Undo / Redo | Cmd+Z / Cmd+Y | Undo (Action) is Shift+Z. Undo View and Redo View are Cmd+Shift+Z and Cmd+Shift+Y |
| Frame all geometry | H or Alt+H | Frame Geometry. Generic Frame All row was blank, with H shown in several editor-specific rows |
| Frame selected | S or Alt+S for elements. O or Alt+O for objects | Distinct viewport commands |
| Toggle active view | Middle mouse button or Page Up | Command Manager’s Toggle Active View |
| All Views | F5 | Verified opening four panes |
| Perspective / Top / Right / Front | F1 / F2 / F3 / F4 | View 1 / XZ / YZ / XY. Four-view positions are upper left, upper right, lower left, lower right |
| Render View / Render to Picture Viewer | Cmd+R / Shift+R | Team Render to Picture Viewer separately showed Cmd+Shift+R |
| Play forwards / backwards / stop | F8 / F6 / F7 | Space also plays forward in Timeline and Picture Viewer. Space in the modeling context toggles selection tools |
| Record / Autokey | F9 / Cmd+F9 | Record Active Objects and Autokeying. Record Animated is Alt+F9 |
| Next / previous frame | G / F | Go to Next Frame and Go to Previous Frame |
| Next / previous key | Cmd+G / Cmd+F | Go to Next Key and Go to Previous Key |
| Knife / Loop Cut | K then K / K then L | Line Cut and Loop/Path Cut. Plane Cut is K then J |
| Extrude / Inset / Bevel / Bridge | M then T / M then W / M then S / M then B | Mesh |
| Subdivide / Optimize | U then S / U then O | Mesh Add and Remove |
| Select All / Deselect All | Cmd+A / Cmd+Shift+A | Standard selection |
| Invert / Grow / Shrink | U then I / U then Y / U then K | Select menu |
| Delete | Backspace or Delete | Command Manager showed BS, Del |
| Group / Ungroup | Alt+G / Shift+G | Group Objects and Ungroup Objects |
| Make Editable / Current State to Object | C / none | Tools › Convert |
| Copy / Paste | Cmd+C / Cmd+V | Paste also displayed Cmd+Shift+V |
| Duplicate | None | Model Duplicate command |
| Hide / unhide | None / none | Hide Selected and Show All in the component visibility commands |

#### 2. Mouse navigation

| Input | Live evidence | Direction / status |
| --- | --- | --- |
| 1 + drag | Command Manager lists Move Camera | Pan direction pending a combined key-and-drag test |
| 2 + drag | Command Manager lists Scale Camera | Dolly direction pending a combined key-and-drag test |
| 3 + drag | Command Manager lists Orbit Camera | Orbit direction and pivot pending a combined key-and-drag test |
| Scroll wheel | Verified in Perspective | Scroll up zoomed in. Scroll down reversed the change |
| Alt + left / middle / right drag | In-window combined gestures unavailable through the session’s drag control | Mapping and drag direction unverified |

The targeted Command Manager filters cover G2’s named actions. An exhaustive scroll capture and the five requested standalone tooltip captures remain open.

#### G4 (Cowork, 2026-11-27)

#### 1. Number fields

Cube Coord. P.X is a dark numeric box with left/right chevrons and an inline cm unit. Double-clicking the value allowed direct text entry and Return committed it. On the Cube’s Object tab, segment controls pair the numeric box with a slider. On Size X, the right chevron changed 200 cm to 201 cm and the left chevron returned it to 200 cm. A rightward drag starting on the chevron registered a single 1 cm increment. Continuous scrubbing and fine/coarse modifiers remain open.

#### 2. Parameter key indicators (H2–H5)

| State | Indicator | Verified result |
| --- | --- | --- |
| Unanimated | Hollow gray diamond | Clicking it on Cube Coord. P.X at frame 0 created a key at 0 cm |
| Key at current frame | Solid red diamond | Returning to frame 0 displayed the red diamond with 0 cm |
| Edited at the keyed frame without recording | Solid amber/orange diamond | Changing P.X to 50 cm displayed the edited value. Selecting Cloner and reselecting Cube at frame 0 retained 50 cm and the amber diamond |
| Animated with no key at the current frame | Hollow red diamond | Scrubbing to frame 30 reevaluated X to 0 cm, discarding the unrecorded edit |

#### 3. Field list

Bend › Fields shows an enable checkbox followed by Name, Blending, Opacity. The empty list says “Double click to create a New Field.” Buttons beneath include Linear Field, Solid, Clamp, and small utility controls including a folder-plus glyph. Clicking Linear Field created a scene child under Bend and a checked layer with Normal blend and 100% opacity. Selecting the layer exposed Blending, Field and Remapping tabs below. Reordering and folder behavior remain open.

#### 4. Link fields

Camera › Object exposes Focus Object as an empty link field. Bend’s Camera Deformer variant also displayed an empty Camera link field. Drop and clear behavior remain unverified.

#### G. Supplemental inspector observations

| Selection | Tabs in reading order |
| --- | --- |
| Sphere | Basic, Coord., Object, Phong |
| Circle spline | Basic, Coord., Object |
| Text | Basic, Coord., Object, Caps, Selections, All, Lines, Words, Letters, Phong |
| Extrude | Basic, Coord., Object, Caps, Selections, Phong |
| Subdivision Surface | Basic, Coord., Object |
| Matrix | Basic, Coord., Object, Transform, Effectors |
| Fracture | Basic, Coord., Object, Effectors |
| Random effector | Basic, Coord., Effector, Parameter, Deformer, Fields |
| Plain effector | Basic, Coord., Effector, Parameter, Deformer, Fields |
| Linear Field | Basic, Coord., Field, Remapping, Color Remap, Direction |
| Phong tag | Basic, Tag |

G1 item 3 contains the other seven selections. All five Cloner tabs and all 11 Light tabs were opened. Widget examples included the Cube numeric field and segment slider, the Light Type drop-down and checkbox grid, the Light Color swatch and its H/S/V controls, the Camera Focus Object link, and the Bend Field list. Linear Field Remapping showed a filled rising response plot. A separate gradient editor, editable spline-curve controls and the close-up batch remain unverified.

#### G5 (Cowork, 2026-11-27)

#### 1. Timeline strip

Clicking the ruler scrubbed from frame 0 to frame 30. The current-frame field and blue playhead updated together. The frame-0 animation key showed a small light-gray mark on the lower strip. The empty document starts at 0 F with a 0–90 F project and preview range and 30 fps. The bottom row shows start/end project values and the preview interval.

#### 2. Dope Sheet

Window › Timeline (Dope Sheet)… opened a floating Timeline window. The Dope Sheet button was active and the left side listed Summary followed by an expandable Cube row. At frame 0 that row and Summary showed narrow orange key marks. The top has a frame ruler and transport/edit tools. The footer showed Current Frame 0, Preview 0→90, Selection 0→0. Key scaling, motion and ripple were not exercised.

#### 3. F-Curve

Window › Timeline (F-Curve)… opened the same style of floating window with the F-Curve Mode button active. The Cube remained in the left tree and the right side became a value grid with a zero line. Tangent behavior, weighted tangents, context presets and the velocity view remain open.

#### 4. Range, markers and playback

Animate › Preview Range lists Set Range to Markers, Go to Next Range, Go to Previous Range, Set Min Preview Range and Set Max Preview Range. Animate › Play Mode lists Preview Range, Simple, Cycle and Ping-Pong. The separate Timeline window exposes a Marker menu. Motion paths and ghosting were not tested.

#### J (Cowork, 2026-11-27)

| Item | Window | Observed |
| --- | --- | --- |
| J1 | Preferences, approx. 731 × 612 px | Opens to Interface. The category list reads Interface, Input Devices, Navigation, Viewport Hardware, Viewport Display, Files, Plugins, Units, Memory, Communication, Node Editor, Renderer, (paint module), Asset Browser, Hair, Material, Modeling/Sculpting, Projection Man, Sketch and Toon, Takes, Timeline/Spline Gadget, Extensions, Import/Export, Scheme Colors. Interface shows language, fonts, menu controls, help and insertion settings. No values were changed |
| J2 | Render Settings, approx. 863 × 674 px | Renderer chooser and category tree on the left. Output is active on the right. The observed preset is 1280 × 720, 30 fps, Current Frame, square pixels |
| J3 | Picture Viewer, approx. 1396 × 970 px | Shift+R rendered frame 0 and added a history row. The large image area is on the left. The right rail shows Navigator/Histogram, a zoom control, and History, Info, Layer, Filter and Stereo. A and B compare controls are on the top toolbar |
| J4 | Asset Browser, approx. 304 × 298 px | Separate window via Window › Asset Browser. See G1 item 6 for the controls and observed loading behavior |

The reference app was left open on a new empty Standard-layout document at 1920 × 1080. Two unsaved test documents were left open in adjacent tabs.
