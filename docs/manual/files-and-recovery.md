# Files and recovery

## The native format

Scenes are saved as `.forge` files. *File > Open*, *Save* and *Save As* behave as in most
editors. A scene with unsaved changes shows `*` after its name in the title; New and Open
then ask whether to save.

## OBJ

*Import OBJ* adds the polygons of a Wavefront OBJ file as new objects in one undo step.
*Export OBJ* writes the whole scene; *Export Selected as OBJ* writes the selection.

## Autosave

Forge3D writes an autosave copy periodically (every five minutes by default, keeping the
ten newest). After a crash it offers to recover the newest one. Settings live in the
shared settings file:

| Key | Default | Meaning |
|---|---|---|
| `autosave.enabled` | `true` | Turn periodic autosave off or on |
| `autosave.interval_seconds` | `300` | Seconds between autosaves |
| `autosave.max_files` | `10` | How many autosave files to keep |

## Where things are stored

| | Settings | Data |
|---|---|---|
| Linux | `~/.config/forge3d` | `~/.local/share/forge3d` |
| macOS | `~/Library/Application Support/forge3d` | same |
| Windows | `%APPDATA%\forge3d` | same |

On Linux the `XDG_*` variables are honoured when they hold absolute paths.
