# Window look and scale from screen captures — observation (O66)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO

Answers GUI session G10 items 1–3, 5–7 and G1 items 1, 5, 7. No one operates the GUI (owner
decision in r25-drift.md). Set the state **through the scripting API only**: load the Standard
layout with its command, open a new document, create the objects named below and select them.
Then capture the screen with `screencapture -x <file>` (the OS command; it doesn't touch the app)
into /Users/bigman/forge-observer/gui-shots/. **Never** commit a capture. Never trace or redraw an
icon. Delete the captures at the end. If the main window's size can't be set through the API,
record the size it has and the display scaling (pixel size of the capture against the screen's
point size).

## Captures
| # | State set through the API | Measured |
|---|---|---|
| 1 | Standard layout, empty document | panel rectangles (x, y, w, h and % of window), splitters, menu bar height, command bar button and icon size, group gaps, tool palette button size, timeline height, body text cap height |
| 2 | A Cube with a Bend child, a Cloner with a Sphere child, a Null with two children, the Cloner selected | object manager row height, indent per level, connector lines and fold arrows (described), row separators, what each row shows left to right; attribute manager tab bar height, tab button size, row height, label column width, group headers |
| 3 | Same, point mode on (through the API) with the Cube made editable | how the mode is shown and where |
| 4 | Same, a Light selected | attribute manager tabs for a light |

## Results
| # | Item | Value | Notes |
|---|---|---|---|
| TODO | | | |

## Overall feel (G10 item 7)
A few sentences: dense or airy, flat or bevelled, contrast, where the eye goes first. No colour
values (we keep our own palette, D22).
