# The editor window

This chapter describes the editor window: its panels, the managers, the keyboard and the
mouse. The arrangement, the button order and the default keys are our own design; they may
change in a later release, and the key map can be edited (see below).

## Layout

From top to bottom the window has the menu bar, the **command bar**, the working area and
the status bar. The working area holds the **tool palette** on the left, the **viewport** in
the centre, the **Object manager** above the **Attribute manager** on the right, and a strip
along the bottom with the **Timeline** and a **Coordinates** tab.

- *Window > Reset layout* restores this arrangement.
- *Window* also opens and closes the optional panels (content browser, layer manager,
  console, undo history) and moves a panel to another area.
- A **workspace** is a named layout. The *Workspace* menu switches between the built-in
  ones, and *Save layout as...* in the same menu stores your own. `ui.workspace`, `ui.layout_save`,
  `ui.layout_load` and `ui.layout_delete` do the same from a script.

Every button in the command bar and the tool palette runs one registry command with fixed
arguments. Nothing happens on a click that a script could not do with the same command.

## Object manager

The Object manager shows the scene tree. Each row has an expand arrow, the name (rename with
a double click or F2), two **visibility dots** (editor and render), an **enable tick** and one
label per tag.

- A dot cycles grey (follow the parent), red (off) and green (forced on) on every click. The
  tick switches the object off for evaluation.
- Click selects one row, Ctrl/Cmd-click toggles it, Shift-click selects the range.
- Drag a row onto the middle of another row to make it a child, or onto the top or bottom
  quarter to place it before or after. The cursor shows an insertion line or a highlight.
  Dragging a row onto a link field in the Attribute manager sets the link.
- The search field above the rows filters by name, type or tag and keeps the parents of every
  match visible.
- Right-click offers Rename, Group Objects, Make Editable, Hide/Show, Disable/Enable, Move to
  Root and Delete.

Each of these gestures is one undo step.

## Attribute manager

The Attribute manager lists the parameters of the selected object. Every row comes from the
parameter's descriptor, so a plug-in's parameters look like the built-in ones.

- Parameters are grouped; when there are several tabs a tab bar appears.
- Number fields can be dragged: hold Shift for ten times finer steps and Ctrl for ten times
  coarser. Click a field to type a value. Up and Down add one step.
- Angles are shown in degrees and percentages as percentages; the file stores radians and
  fractions.
- With several objects selected the panel edits all of them. Where their values differ the
  field shows "mixed"; typing a value sets it on every object in one undo step, and editing
  one component of a vector keeps the other components of each object.
- While a modelling tool is active (see [Editing meshes](editing-meshes.md)) the panel shows
  the tool's settings with **Reset Values**, **Apply** and **Done**.

## Coordinates manager

The Coordinates tab edits position, rotation (H, P, B in degrees) and scale of the selection.
Switch between **Local** and **World** space, and between **Scale** and **Size** (the size of
the bounding box times the scale). With **Manual** checked, typed values only fill a draft;
**Apply** writes it as one undo step and **Reset** drops it. In a component mode the tab shows
the pivot, rotation and size of the selected points, edges or polygons instead.

## Keyboard

Every shortcut runs a registry command, so a key press is exactly as undoable as the same
menu item. The defaults:

| Keys | Action |
|---|---|
| Ctrl/Cmd+O, S, Shift+S | Open, Save, Save As |
| Ctrl/Cmd+Z, Shift+Z (or Y) | Undo, Redo |
| Delete, F2 | Delete, Rename |
| Ctrl/Cmd+A, Shift+A, I | Select all, none, invert |
| Alt+1 to Alt+4 | Model, Points, Edges, Polygons mode |
| Ctrl/Cmd+Shift+E | Make Editable |
| Ctrl/Cmd+Shift+P | Command palette |
| 1 to 9 | Shading modes |
| W, E, R, Q | Move, rotate, scale gizmo; local/world |
| H, S | Frame all, frame selected |
| Ctrl/Cmd+1 to 7 | Perspective, front, right, top, back, left, bottom view |
| K, Space, comma, period | Record key, play/stop, previous/next frame |

The **command palette** searches every command by name or chord and runs it; it also lists
presets such as "shading wire". **Window > Key map** edits the bindings: pick a chord, choose
a command, and your changes are stored in the settings file next to your other preferences.
A removed or changed default can always be restored from the same window.

## Viewport navigation

| Gesture | Effect |
|---|---|
| Mouse wheel | Zoom towards the pointer |
| Alt + left drag, or hold V and drag | Orbit |
| Middle drag, or hold X and drag | Pan |
| Alt + right drag, or hold C and drag | Dolly |

While Alt or one of these hold keys is down, the viewport does not select or move anything.
*View > Frame all / Frame selected* and the view shortcuts above fit the camera to the scene.
`view.layout` toggles between one and four panes, and `view.maximize` enlarges the active
pane.

## Themes, scale and accessibility

*View > Theme* offers a dark, a light and a high-contrast theme, and *View > UI scale* sets the size of the interface from 50 % to 300 %. Text input works with input methods
for languages that need them, every control has an accessible label, and the interface can be
used with the keyboard alone (Tab and Shift+Tab move between controls).
