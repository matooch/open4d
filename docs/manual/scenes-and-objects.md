# Scenes and objects

A scene is a tree of **nodes**. A node has a type (for example `forge.cube`), a name, a
transform (position, rotation, scale), parameters, optional tracks (animation) and children.

## Selecting

Click selects one object. Ctrl-click toggles an object. Shift-click selects the range of
visible rows between the anchor and the clicked row. The active object is the last one you
selected.

## Organising

- `object.group` wraps the selection in a null object.
- `object.reparent` moves a node under another. Children inherit their parent's transform.
- `object.visibility` and `object.enable` switch an object off in the viewport or in
  evaluation.
- `object.duplicate`, `object.arrange`, `object.randomize` and `object.transfer` copy and
  distribute objects; see the [command reference](reference/commands.md).

## Parameters

Every parameter is declared once with an id, a type, a range, a default and a unit. The
Attribute manager lists them, scripts and RPC set them by id (`param.set`), animation
tracks target them by id, and the file stores them by id. Angles are shown in degrees but
stored in radians; percentages are stored as fractions.

## Undo and redo

Every undoable command is one step. A script run is a single step, however many commands it
ran. *Edit > History* lets you jump to any earlier state.

## Viewport

The `view.*` commands control shading, layout, lighting, shadows, anti-aliasing, the HUD and
the gizmo mode and space. They are not part of the undo history.
