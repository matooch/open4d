# Tool and UI defaults — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO

Everything our specs mark **parity-pending** that isn't covered by a mesh export.
Record *behaviour* (what happens), not look: our colours, icons and layout stay our own.

## Viewport navigation (VIEWPORT.md §1, UI.md §6)

| Action | Mouse/keys in the reference app |
|---|---|
| Orbit | TODO |
| Pan | TODO |
| Zoom/dolly | TODO (and which wheel direction zooms in) |
| Frame selected / frame all | TODO |
| Switch to an axis view (top/front/…) | TODO |

Default camera: perspective field of view (if shown): TODO

## Selection (GEOMETRY.md §4, VIEWPORT.md §2)

On a default plane with segments (5, 5), editable, polygon mode:

| Test | Result |
|---|---|
| Select one inner polygon, grow selection once — how many polygons selected? Do corner-touching polygons get added? | TODO |
| Shrink that selection once — result? | TODO |
| Rectangle selection: default selects only visible elements? Option label? | TODO |
| Rectangle selection of polygons: needs the whole polygon inside or any part? | TODO |

## Shortcuts (UI.md §5)

| Action | Shortcut |
|---|---|
| Undo / redo | TODO |
| Delete | TODO |
| Rename | TODO |
| Shading modes | TODO |

## Timeline (UI.md §7)

| Setting | Default |
|---|---|
| Start / end frame | TODO |
| Frame rate | TODO |

## Units display (UI.md §4)

| Question | Answer |
|---|---|
| Angles shown in degrees? | TODO |
| Percent fields: 0–100 with "%"? | TODO |
| Length unit suffix shown in fields | TODO |

## Splines (GEOMETRY.md §5)

| Question | Answer |
|---|---|
| Interpolation types offered (labels, in order) | TODO |
| Default type | TODO |
| Intermediate-point options and default | TODO |
