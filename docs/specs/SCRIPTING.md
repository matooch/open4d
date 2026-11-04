# Scripting (`crates/script`, package `forge-script`)

Source: our own design (docs/roadmap/areas/A-automation.md, DECISIONS D5); no reference
observation is involved. Rhai is the embedded language (crate `rhai`, pinned to `=1.27.0`,
licence MIT OR Apache-2.0; its dependency `smartstring` is MPL-2.0, file-level copyleft,
unmodified). Rhai documentation: <https://rhai.rs/book/>.

A script is a sequence of **registry commands** (COMMANDS.md) plus read-only scene queries.
There is no second API: every edit goes through `CommandRegistry::run` with the host's
`CommandContext`, so a script can do exactly what the menus, the CLI and RPC can do.

## 1. Entry point

```rust
run_script(ctx: &mut dyn CommandContext, commands: &CommandRegistry, source: &str,
           opts: &ScriptOptions) -> Result<ScriptReport, ScriptFailure>
```

- `ScriptOptions` (all limits in §5): `max_operations`, `max_commands`, `timeout`,
  `max_call_levels`.
- `ScriptReport { output: Vec<String>, value: serde_json::Value, commands: usize,
  operations: u64 }`: lines written by `print`/`debug` (without the trailing newline), the
  value of the script's last expression as JSON (`null` for `()`), the number of commands
  run and Rhai operations used.
- `ScriptFailure { kind, message, line: Option<usize>, output: Vec<String> }`;
  `kind` is `Syntax`, `Runtime` or `Limit`. `Display` is `message`, prefixed `line N: `
  when known. `output` is what was printed before the failure.

## 2. Undo

- **One script run is one undo step** labelled `Run Script`, however many commands it ran
  (`Document::group_undo_since`, CORE.md §4). A run that changes nothing leaves no step.
- **A failed run changes nothing**: when the script fails (syntax is checked before any
  command; runtime error; limit hit), every edit it made is rolled back
  (`Document::rollback_to`) and no redo entries are left. View state a command changed
  (current frame, selection, edit mode, autokey) is not rolled back, except that
  selected nodes that no longer exist are dropped. A command error caught by `try`/`catch`
  does not fail the run; that command changed nothing.
- `edit.undo` and `edit.redo` are not available to scripts (they would break the single
  step). `file.new` and `file.open` replace the document, so they are not available either.

## 3. Binding reference

Units are the commands' units: **degrees**, percent as shown, scene units for distances
(COMMANDS.md §4). Node ids are integers. Rhai integers are `i64`, floats `f64`.

### Commands

| Function | Meaning |
|---|---|
| `run(id)` / `run(id, #{...})` | Runs command `id` with the map as arguments. Returns the command's result converted to a Rhai value (map, array, string, number, bool or `()`). Throws a runtime error (message `id: <CommandError>`) if the command fails; catch it with `try { } catch (e) { }`. |
| `commands()` | Array of all command ids the host registered. |
| `create(type)` | `run("object.create", #{type: type})`, returns the new node id. |
| `create(type, name)` | The same, then renames the node; still returns the id. |
| `set_param(node, name, value)` | `param.set`. `value` may be a number, bool, string, `vec3`, array or `()`. |
| `set_transform(node, #{pos, rot, scale})` | `transform.set`; the map keys are optional, values are `vec3` or `[x, y, z]`. |

`run`, `create` (two commands when it renames), `set_param`, `set_transform` and `frame(n)` count against `max_commands`, one per command run. Animation keys are written with `run("key.set", #{node, track, frame, value})` (COMMANDS.md §6).

Commands that are not available in scripts (error, nothing runs): `edit.undo`,
`edit.redo`, `file.new`, `file.open`, `file.save`, `file.save_as`, `file.import_obj`.
`file.export_*` is allowed: the script names the path explicitly, which is the only
file-system access a script has (the host decides what else, see §5).

### Scene queries (read-only)

`scene()` returns a handle to the live document (queries see edits made earlier in the
same script).

| Method | Result |
|---|---|
| `scene().nodes()` | All non-tag node ids in tree order. |
| `scene().roots()` | Root node ids. |
| `scene().find(name)` | Id of the first node (tree order) named `name`, or `()`. |
| `scene().find_all(name)` | Array of ids. |
| `scene().node(id)` | A node handle; throws for an unknown id. |
| `scene().selection()` | Array of selected node ids. |
| `scene().len()` | Number of nodes, tags included. |
| `scene().fps` / `.start` / `.end` | Project settings. |
| `node.id`, `node.name`, `node.type` | Properties (`type` is e.g. `forge.cube`). |
| `node.parent()` | Parent id or `()`. |
| `node.children()` | Array of child ids. |
| `node.param(name)` | Current value in display units (the `query::display_value` mapping: angles in degrees, vec3 as `vec3`, enum as string, link as id or `()`); throws for an unknown parameter. |
| `node.params()` | Map of every parameter. |

Queries read the stored scene (not the animated one at the current frame).

### Time and maths

| Function | Meaning |
|---|---|
| `frame()` | The current frame. |
| `frame(n)` | `timeline.set_frame`; returns the (clamped) frame. |
| `deg(x)` | Degrees to radians (`x * PI / 180`), for scripts that do their own trigonometry. Commands themselves take degrees. |
| `to_deg(x)` | Radians to degrees. |
| `vec3(x, y, z)` | A vector type with fields `x`, `y`, `z`; operators `+ - * /` (vector with vector, vector with number), unary `-`; methods `len()`, `dot(v)`, `cross(v)`, `normalized()`, `to_array()`. Converts to `[x, y, z]` in command arguments. `vec3` values can be compared with `==`. |
| `rand(seed)` | A seeded generator: `r.float()` in `[0, 1)`, `r.range(a, b)` float in `[a, b)`, `r.int(a, b)` integer in `[a, b]` (a <= b, else error). Deterministic: the same seed gives the same sequence on every platform (SplitMix64: state += 0x9E3779B97F4A7C15, then the standard mixing; `float` is the top 53 bits / 2^53). Rhai's own random functions are not available. |

Standard Rhai (loops, maps, arrays, closures, string functions, `PI()`, `sin` ...) is
available. `print(x)` and `debug(x)` append a line to the report's `output`.

The binding reference above is mirrored as data in `crates/script/src/api.rs` (`API`), which
the generated `docs/manual/reference/scripting.md` renders. A unit test compares it with the
functions the engine registers (name and parameter count, read through Rhai's `metadata`
feature, enabled for tests only), so adding, removing or re-shaping a binding without
updating the table fails the build. `forge_script::BLOCKED_COMMANDS` is the list of commands
refused in scripts.

## 4. Language surface removed

`import`/`export` and modules (feature `no_module`), time functions (`no_time`: `timestamp`),
and `eval` (disabled symbol). Scripts cannot read files, write files, open sockets or read
the environment; the only host effect is through commands.

## 5. Sandbox limits

Defaults (`ScriptOptions::default()`); a script that exceeds one stops with a `Limit`
failure and is rolled back (§2):

| Limit | Default | Error text |
|---|---|---|
| operations | 5 000 000 | `script exceeded the operation limit (5000000)` |
| wall-clock timeout | 10 s (checked every 1024 operations) | `script exceeded the time limit (10s)` |
| commands per run | 10 000 | `script exceeded the command limit (10000)` |
| call depth (recursion) | 48 | `script exceeded the call depth limit (48)` |
| expression depth | 64 (functions 32) | syntax error |
| string size | 1 MiB; array and map length 100 000 | `script data too large` |

Output lines are capped at 10 000 (later lines are dropped silently).

## 6. Hosts

- **CLI** `forge run <script.rhai> [--scene in.forge] [--save out.forge]` (CLI.md §run).
- **Editor**: the Script Manager panel (UI.md §10) runs on the editor's own context, so
  the run is one undo step in the editor's history.
- **Editor REPL**: the Console panel (UI.md §19) runs one entry at a time in the same sandbox
  (default limits, no state between entries); each entry is one undo step like a panel run.
- RPC and MCP do not run scripts (A-automation: `script.run` only with
  `rpc.allow_scripts`, a later setting; not part of brief 016).

## 7. Examples (`examples/scripts/`)

| File | Builds | Checked by the test |
|---|---|---|
| `cube_grid.rhai` | a 3x3x3 grid of cubes under a null `Grid` | 27 cube children at the right positions, one undo step |
| `animated_sphere.rhai` | a sphere with position x keys at frames 0, 15, 30 (`key.set`) | evaluated x at frames 0, 15, 30 |
| `random_cubes.rhai` | cubes at seeded random positions | positions identical between two runs |
| `query_scene.rhai` | prints a tree using the queries | output lines |

## 8. Tests

`crates/script/tests/script.rs` (headless `App`):

- every example runs and the resulting scene is checked (§7);
- binding behaviour: `run` result conversion, `create`, `set_param`, `set_transform`,
  `try/catch` of a failing command, scene queries, `vec3` maths, `rand` determinism
  (a fixed first value);
- undo: a 3-command script is one step, undone in one `undo`; a failing script leaves
  the document and the undo stack as before;
- sandbox: `loop {}` stops with the operation limit; deep recursion with the call depth
  limit; a `while` with `run` hits the command limit; `import "x"`, `eval`,
  `timestamp()` are rejected; `file.open` and `edit.undo` are refused; a syntax error
  changes nothing and reports a line.

CLI tests (`apps/forge/tests/cli.rs`): `forge run` with and without `--scene`/`--save`,
`--json` output, script errors and exit codes. UI test: the Script Manager panel runs a
script and shows its output.

## Plugin entry points

`run_function(ctx, commands, source, function, arg, opts)` runs only the function
definitions of `source` and calls `function(arg)` with the same undo, rollback and limits as
`run_script`. `eval_function(source, function, arg, opts)` does the same with no context and
no command bindings (maths and random numbers only). `script_functions(source)` lists the
defined functions with their arity. Plugin packages use them (PLUGINS.md).
