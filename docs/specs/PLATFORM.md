# PLATFORM — per-OS paths, fonts, file dialogs, modifiers (brief 175)

Sources: public documentation of the `directories` crate (XDG Base Directory spec on Linux,
Known Folder API on Windows, Apple File System Programming Guide on macOS), the `rfd` crate
docs, the egui docs (`FontDefinitions`, `MissingGlyphPolicy`), the licence texts shipped with
the fonts. No reference-app observation is involved.

## 1. Where things live

One crate, `forge-platform` (`crates/platform`), owns every per-user directory. `forge-rpc`,
the script panel and the key map ask it; none of them builds a path from `$HOME` itself.
It is a separate crate (not a `forge-core` module) so the pure scene crate stays free of OS
and filesystem concerns, and `forge-rpc` can use it without depending on the UI.

The application directory name is `forge3d`.

| | config (`settings.json`) | data | cache |
|---|---|---|---|
| Linux | `$XDG_CONFIG_HOME/forge3d`, else `~/.config/forge3d` | `$XDG_DATA_HOME/forge3d`, else `~/.local/share/forge3d` | `$XDG_CACHE_HOME/forge3d`, else `~/.cache/forge3d` |
| macOS | `~/Library/Application Support/forge3d` | same as config | `~/Library/Caches/forge3d` |
| Windows | `%APPDATA%\forge3d` | `%APPDATA%\forge3d` | `%LOCALAPPDATA%\forge3d\cache` |

An `XDG_*` variable counts only when it is non-empty and an absolute path (XDG spec); it
applies on Linux only. The base folders come from `directories::BaseDirs`; the rules above
live in the pure function `resolve(os, env, base)` so every OS branch is tested on every
host.

## 2. The settings file

`settings_path()`:
1. `$FORGE_SETTINGS` when set (any OS, even without a home folder);
2. else `<config>/settings.json` from §1;
3. `None` only when no home folder can be determined.

Precedence: `FORGE_SETTINGS` > `XDG_CONFIG_HOME` (Linux) > the OS default.

## 3. Migration

Before 175 every OS used `$XDG_CONFIG_HOME/forge3d/settings.json`, else
`~/.config/forge3d/settings.json` (macOS and Windows with `HOME` set). `settings_path()`
silently migrates: when the new path (rule 2) differs from the legacy path, the new file does
not exist and the legacy file does, the legacy file is copied to the new path (parents are
created). The legacy file is left in place so an older build keeps working. Errors are
ignored (the legacy file is simply not used). On Linux both paths are equal: nothing to do.
`FORGE_SETTINGS` disables migration.

## 4. Fonts

egui's bundled fonts lack most symbols; with `MissingGlyphPolicy::Panic` (tests) egui panics
and in the app it draws tofu. `forge-ui` registers `ForgeSymbols.ttf`
(`crates/ui/assets/fonts/`), a subset of DejaVu Sans (Latin-1, punctuation, arrows, maths,
technical, geometric shapes, miscellaneous symbols, dingbats; 171 KB), as the last fallback of
the Proportional and Monospace families (`theme::install_fonts`). Licence: Bitstream Vera /
DejaVu (permissive, redistribution embedded in a binary allowed provided the notice is kept;
the notice is `LICENSE-DejaVu.txt` next to the font). It is a font file, not a crate, so
`deny.toml` is unaffected; it is the same class of licence as the egui fonts already accepted
there. The UI button glyphs stay short ASCII tags (UI.md §5).

## 5. File dialogs

`forge_ui::Dialogs` (`pick_open`, `pick_save`, `confirm_unsaved`) is the seam; `NativeDialogs`
implements it with `rfd` (XDG desktop portal on Linux, the system dialogs on macOS and
Windows). Tests inject a scripted implementation with `Editor::set_dialogs`.

## 6. Modifiers

Shortcuts use egui's `COMMAND` modifier (Cmd on macOS, Ctrl elsewhere); the key map (UI.md,
brief 152) stores chords with `command` and displays them per OS.
