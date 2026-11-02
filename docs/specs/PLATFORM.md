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

## 7. Autosave and recovery (brief 179)

Sources: this repo's CORE.md §6 (the native format), §1 above (the data directory), the Rust
standard library docs (`File::try_lock`, `fs::rename`). No reference-app observation is
involved; the policy is ours.

The pure parts (settings, naming, schedule, pruning, session markers, atomic write, the
background writer) are in `forge_platform::autosave`; they take paths and a clock as
arguments and never read the process environment, so every rule is tested with temp
directories and a manual clock. `forge-ui` (`autosave.rs`) joins them to the document.

### 7.1 Settings

The shared settings file (§2) may hold an `autosave` object; every field is optional, a wrong
type or out-of-range value falls back to the default (clamped where noted). Read once at start.

| key | default | meaning |
|---|---|---|
| `autosave.enabled` | `true` | `false` turns periodic autosave off (recovery of an earlier crash still works) |
| `autosave.interval_seconds` | `300` | seconds between autosaves; clamped to 1 .. 86400 |
| `autosave.max_files` | `10` | at most this many autosave files are kept in the directory (1 .. 1000) |

### 7.2 Where

`<data>/autosave/` (§1) unless `FORGE_AUTOSAVE_DIR` names another directory. In it:

- `<key>.forge.autosave`: the scene exactly as `file::save_with` writes it (text, plus chunks);
- `<key>.forge.autosave.json`: metadata `{"session", "original", "saved_at"}` (`original` is the
  document's file path or `null`, `saved_at` seconds since the Unix epoch);
- `sessions/<session id>.lock`: the session marker (§7.5).

Key (the document's stable name):
- never saved: `untitled-<session id>`; the session id is chosen once per process and is
  stable for the whole run, so every autosave of the same untitled document overwrites one file;
- saved or opened from a path: `<stem>-<hash>` where `stem` is the file stem reduced to
  `[A-Za-z0-9._-]` (other characters become `_`, at most 40 characters, empty gives `scene`)
  and `hash` is the 16-digit hex FNV-1a 64 of the absolute path's UTF-8 text. Two files with
  the same name in different folders never collide, and the same file always maps to one key.

Session id: `<unix seconds>-<pid>` in hex-free decimal, e.g. `1730000000-4242`.

### 7.3 When

`Schedule::due(now, revision, saved_revision)` is true when all hold:
1. autosave is enabled;
2. at least `interval_seconds` passed since `last` (the start of the session, the previous
   autosave attempt, or the last explicit reset: a failed write also waits a full interval);
3. the document changed since the last save: `revision != saved_revision` (dirty);
4. it changed since the last autosave: `revision != last_autosaved_revision`.

After a save, new, open or recover the schedule is reset (`last` = now, last autosaved
revision forgotten). The editor checks on every frame and asks egui for a repaint at least
every 10 seconds (and at the interval) while autosave is enabled, so an idle window still
autosaves.

### 7.4 How (threading and atomicity)

On the UI thread the editor serialises the scene (`save_with`, with `base_dir` = the folder
of the document's path, if it has one; the same rule as `file.save`) because only it may read
the document. The bytes go to one background writer thread over a channel; the UI never waits
for the disk. The writer writes the metadata file, then the data file, each as: write
`<name>.tmp` in the same directory, `sync_all`, `rename` over the target (atomic on every
supported OS); after a write it prunes. Jobs run in the order sent, so an explicit save's
"remove" is never overtaken by an earlier write. `flush()` waits for the queue to drain (tests,
`file.autosave_now` with `wait`, and the clean exit).

Pruning: after each write keep the `max_files` newest entries (by `saved_at`, then file name);
the entry just written is never removed. Leftover `*.tmp` files older than any entry are
removed at start.

### 7.5 Crash detection

Each process creates `sessions/<session id>.lock` at start and holds an exclusive advisory
lock on it (`File::try_lock`) until it exits. The OS releases the lock when the process dies,
however it dies. On start, before creating its own marker, a process scans `sessions/`: a
marker whose lock it can take belongs to a **crashed** session (clean exits delete their
marker); a marker whose lock is held belongs to a **running** session and is left alone.

A **recoverable** entry is an autosave whose metadata `session` has a crashed marker. Entries
of running sessions, or of sessions without a marker, are not offered (the latter are left over
from a clean exit that could not delete them; they only count towards pruning). Stale markers
are deleted once no recoverable entry of their session remains (all recovered or discarded);
until then they stay, so a crash while the dialog is open offers the same entries again.

### 7.6 Clean exit and explicit save

- A successful save (`file.save`, `file.save_as`, the dialogs), `file.new` and `file.open`
  remove the autosave of the document that was replaced or saved (under its old key and its new
  key) and reset the schedule.
- The window's clean exit (`App::on_exit`) flushes the writer, removes this session's
  autosave files and deletes its marker. Closing a dirty document first asks "save changes?" (UI.md §15.1), so work is
  discarded only when the user chose Don't Save.

### 7.7 Recovery

On start `Autosave::start` lists the recoverable entries. If there are any, the editor shows a
window "Recover unsaved work" (UI.md §15) listing each with its name (original path or
"Untitled"), the time it was saved (UTC `YYYY-MM-DD HH:MM`) and its size, with **Recover** and
**Discard** per entry and **Discard all**.

- Recover loads the autosave (`load_with`, `base_dir` = the original path's folder) and
  replaces the document like `file.open` would, but: the document keeps the original path
  (`None` for an untitled one), is marked dirty (so Save writes the original path and the
  title shows `*`), and is autosaved again at once under its new key, and then the old entry
  is removed. If the current document is dirty the usual "save changes?" prompt comes first.
  A file that fails to load keeps its entry and shows `Recover failed: ...` in the status bar.
- Discard removes the entry's two files.

### 7.8 Commands

`file.autosave_now` (`wait?`: bool, default false): serialises the document and queues an
autosave regardless of the schedule, unless autosave has no directory (headless, or no home
folder: `Failed("needs the editor window")` headless, `Failed("autosave unavailable: ...")`).
Returns `{"path", "queued": true}` (`wait` flushes first). It is not undoable and not a
document change.

## 8. Logging and crash reports (brief 178)

Sources: the public docs of the `tracing` and `tracing-subscriber` crates, the Rust standard
library docs (`std::panic::set_hook`, `std::backtrace::Backtrace`), the XDG Base Directory
spec for the data folder (§1). No reference-app observation is involved; the policy is ours.

The pure parts (level parsing, the rotating file, the in-memory tail, the report text, the
"unseen report" bookkeeping) are in `forge_platform::logging` and `forge_platform::crash`; they
take paths, strings and numbers as arguments. `forge_app::diagnostics::install` joins them to
the process (log setup, panic hook, command history) and is called first thing by
`forge-studio`, `forge-rpc-serve` and `forge-mcp`.

### 8.1 Where

- Logs: `<data>/logs/` (§1) unless `FORGE_LOG_DIR` names another directory. Current file
  `forge.log`; rotated files `forge.log.1` (newest) ... `forge.log.<max_files>`.
- Crash reports: `<data>/crashes/` unless `FORGE_CRASH_DIR` names another directory.

Without a home folder and without the variables, no file is written (the in-memory tail and,
in debug builds, stderr still work).

### 8.2 Logging

`tracing` events (`tracing::info!` etc.) with the `tracing-subscriber` `fmt` layer, no ANSI
colours, one line per event: `<time> <LEVEL> <target>: <message>`.

Level: the first of
1. `FORGE_LOG` (environment),
2. `log.level` in the settings file (§2),
3. `info`.

The value is a `tracing-subscriber` target filter: `debug`, or `warn,forge_ui=debug`. A value
that does not parse falls back to the next source (an invalid `FORGE_LOG` is reported once as a
warning in the log).

Settings object `log` (all optional; wrong type or out-of-range values fall back to defaults,
clamped where noted):

| key | default | meaning |
|---|---|---|
| `log.level` | `"info"` | filter as above |
| `log.max_bytes` | `1048576` | rotate when the next line would pass this size; clamped 1024 .. 1073741824 |
| `log.max_files` | `5` | rotated files kept; clamped 1 .. 100 |

Rotation (`RotatingFile`): a line is never split. Before a write that would make the current
file larger than `max_bytes` (and the file is not empty) `forge.log.<max_files>` is deleted,
`forge.log.<i>` becomes `forge.log.<i+1>`, `forge.log` becomes `forge.log.1`, and a fresh
`forge.log` is opened. The file is opened in append mode so a restart continues it.

Sinks: the file; the in-memory tail (the last 200 lines, used by the crash report); and stderr
in debug builds (`cfg!(debug_assertions)`) or when `FORGE_LOG_STDERR=1`. **Nothing is ever
written to stdout**: stdout belongs to the CLI output contract (CLI.md) and to the MCP
protocol (RPC.md §8). `forge-cli` (`forge ...`) does not install logging: its `error:` /
`warning:` lines on stderr are its documented user output, not diagnostics.

Never logged: the RPC token. Events carry ids, paths and counts only.

Converted from `eprintln!`: the editor's RPC start notices and screenshot failure, the RPC
server's start and stop, command failures (debug level for each command run, warn level for a
failed one). The startup banner `rpc: listening on ...` stays on stderr too (automation.md
documents it).

### 8.3 Command history

`forge_command::history` keeps the last 50 command runs in a process-wide ring buffer:
sequence number, command id and the arguments as compact JSON (cut at 2000 characters). It is
filled in `CommandRegistry::run`, the one funnel for the editor, RPC, scripts and the CLI. The
arguments are scene data and are kept as is, except that any object key whose name contains
`token` (any case) has its value replaced by `"<redacted>"`.

### 8.4 Crash report

`install_hook` replaces the panic hook: it writes the report, logs it at error level, then
calls the previous hook (so the default message and exit behaviour are unchanged). A panic
inside the hook is not reported again (re-entrancy guard). The report is plain UTF-8 text,
file name `crash-<session>-<n>.txt` where `<session>` is the process session id
(`<unix seconds>-<pid>`, §7.2; the same id names the autosave session marker) and `<n>` is
1, 2, ... if one process panics more than once.

```
Forge3D crash report
Time: <YYYY-MM-DD HH:MM:SS> UTC (unix <n>)
Application: <name> <version>
Session: <session id>
OS: <os> <arch>
GPU: <adapter name, backend, driver> | unknown
Thread: <name or unnamed>
Panic: <message>
Location: <file:line:column> | unknown

Backtrace:
<std::backtrace::Backtrace::force_capture text>

Last <k> commands (oldest first):
<seq> <id> <args json>

Last <k> log lines (oldest first):
<line>
```

The GPU line is set by the editor from the wgpu adapter once the window exists
(`crash::set_gpu_info`). Every configured secret (the RPC token) is replaced by `<redacted>`
everywhere in the text. Writing a report never fails the panic: errors are ignored.

### 8.5 Next start

`crash::unseen(dir)` lists the reports in the crash folder that are not named in
`<dir>/.seen`. The editor shows a window "Forge3D crashed last time" (UI.md §15.2) with the
report names, **Open folder** and **Dismiss**; both mark the listed reports as seen (they are
not deleted). If the autosave recovery window (§7.7) has entries, the notice says so.
