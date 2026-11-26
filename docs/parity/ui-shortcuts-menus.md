# Shortcuts, commands and menus through the scripting API — observation (O65)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO

Answers GUI sessions G2 (every shortcut) and G1 item 4 (menus) without operating the GUI (owner
decision in r25-drift.md). Everything comes from the scripting API of the running app. Never read
resource, menu or preference files from disk, and never send mouse or key events.

## 1. Shortcuts
List every shortcut the API reports (the app's shortcut table): key with modifiers, the command
id, the command's name as the app returns it, and the manager or context it applies to (global,
viewport, object manager, timeline …) if the API gives it. Sort by context, then key.

| Context | Key | Command name | Command id | Notes |
|---|---|---|---|---|
| TODO | | | | |

## 2. Menus
Walk the main menu structure the API returns: every menu bar menu, every item and every submenu
in order, with separators as `---`, the command name, and its shortcut from §1. Indent submenus
with `>`.

| Menu | Path | Item | Command id | Shortcut |
|---|---|---|---|---|
| TODO | | | | |

## 3. Command bar and tool palette
If the API exposes the default layout's command palettes (their command ids in order), list them
per palette with each command's name and whether it is a drop-down group. If not, write "not
observed: <reason>".

## 4. Mouse navigation
The API cannot drag the mouse; G2 item 2 stays for the owner notes (N1).
