# Rhai plugin packages (`crates/plugin`, package `forge-plugin`)

Source: our own design (docs/roadmap/areas/N-platform.md "Plugins", brief 184). No reference
observation is involved. Rhai is documented at <https://rhai.rs/book/>. Native plugin ABI is
v2 and out of scope.

A **plugin package** is a folder with a manifest and one Rhai entry script. It registers
**node types** (generators whose geometry comes from a Rhai function) and **commands**
(Rhai functions that appear in the command registry, so CLI, RPC, MCP, menus and scripts
all see them). Everything runs inside the sandbox of SCRIPTING.md §5.

## 1. Folder layout

```
my-plugin/
  plugin.toml     the manifest
  main.rhai       the entry script (the manifest's `entry`)
```

`discover(root)` loads every direct subfolder of `root` that has a `plugin.toml`, sorted by
folder name; a folder that fails to load is reported, not fatal to the others.

## 2. Manifest (`plugin.toml`)

| Key | Type | Rule |
|---|---|---|
| `name` | string | Required. `[a-z][a-z0-9_]*`. It is the plugin's **namespace**: every node type id and command id must start with `<name>.`. |
| `version` | string | Required. `MAJOR.MINOR.PATCH`, digits only. |
| `entry` | string | Required. Relative path of the entry script inside the folder: no `..`, not absolute, at most 1 MiB. |
| `permissions` | array of strings | Optional, default none. Known: `commands` (plugin commands may run registry commands). Unknown names fail the load. |
| `[[node_types]]` | tables | `id`, `label`, `build` (function name), optional `category` (`generator`, the default, or `object`), optional `in_create_menu` (default true), `[[node_types.params]]`. |
| `[[commands]]` | tables | `id`, `label`, `function` (function name). |

Param tables (`[[node_types.params]]`): `id`, optional `label` (default: the id), `kind`
(`bool`, `int`, `float`, `vec3`, `color`, `string`, `enum`), `default`, optional `min` /
`max` (int, float, vec3), optional `unit` (`none`, `distance`, `angle`, `percent`, `time`),
`options` (enum only: array of `{id, label}`). Defaults, `min` and `max` of an `angle`
param are written in degrees in the manifest and stored in radians (CORE.md §1); a
`percent` param is written as a percentage and stored as a fraction. Missing `default`:
`false`, `0`, `0.0`, `[0,0,0]`, `""`, the first option. A default outside `min..max`, an
enum default that is not an option, a duplicate param id, or a non-finite number fails the
load. These descriptors are the only definition of the parameters: the UI, undo, animation
and the file format derive from them.

## 3. Entry script

Only the **function definitions** of the entry script are used; its top-level statements
never run. Each function named by the manifest must exist with **exactly one parameter**
(checked at load, together with the syntax). The entry script cannot import anything
(`no_module`).

### 3.1 Node type build function

`fn build(params)` is called with a map of the node's parameters in display units (degrees,
percent as a number, `vec3` and `color` as `[x, y, z]` arrays, enums as the option id).
Params the node does not hold take the descriptor default. It must return
`#{ points: [[x, y, z], ...], polys: [[i, ...], ...] }` (arrays of numbers; `vec3` values
are accepted as points). The result must pass `MeshData::validate` (finite coordinates,
polygons of at least three distinct in-range indices) and is capped at 1,000,000 points and
1,000,000 polygons. Anything else is a build error naming the node type.

The build function runs in a **pure sandbox**: no scene, no commands (`run`, `create`,
`scene()`, ... are undefined), no clock. It has the maths types and seeded random numbers
(SCRIPTING.md §3), so equal params give an identical mesh. It uses the default
`ScriptOptions` limits.

`RhaiPlugin::evaluate(scene, id)` returns `None` if the node's type is not one of the plugin's,
else the mesh (`PolyMesh`) or the error.

### 3.2 Command function

`fn name(args)` is called with the command's JSON arguments object converted to a map (`#{}`
when the command is run without arguments). Its return value becomes the command result
(JSON). The command's argument schema is an open object (`additionalProperties`); its
sample arguments are `{}`. It runs with the SCRIPTING.md semantics: one `Run Script`-style
undo step on success, every edit rolled back on failure, `BLOCKED_COMMANDS` stay blocked.

A plugin command is `undoable` only if the manifest grants `commands`. **Without the
`commands` permission the command may run no registry command**: the run gets a command
budget of zero, so `run`, `create`, `set_param`, ... fail with the command-limit error.
With it, the budget is the default `max_commands`.

## 4. Registration

`RhaiPlugin::load(dir)` reads and validates everything; nothing is registered until
`registry.install(&plugin)` (node types, atomic as for any `forge_core::Plugin`) and
`plugin.register_commands(&mut commands)` (`forge_command::CommandPlugin`). The plugin id
is `plugin.<name>`. A clash with an existing node type or command id is an error, and
because ids must start with `<name>.`, two plugins clash only if they share a name.
`install_all(root, &mut registry, &mut commands)` does this for `discover(root)` and returns the
loaded plugins plus the failures (a failing or clashing plugin is skipped, the others still
install). A plugin command needs the shared command registry to run registry commands:
after wrapping the registry in an `Arc`, call `plugin.bind(&commands)` (only a weak
reference is kept); an unbound plugin command fails with "not bound".

## 5. Errors (`PluginError`)

`Io`, `Manifest` (TOML or a rule above, with the key), `Script` (syntax error or a missing
function, with the line), `Build` (a build function failed or returned a bad mesh).

## 6. Not in this version

- The editor/CLI do not install `plugins/` folders automatically yet; hosts call
  `install_all`. Scene evaluation (`forge_generators::evaluate`) does not call
  `RhaiPlugin::evaluate` yet, so plugin node types are created and edited like any node but
  their mesh is produced by hosts that call it (follow-up: evaluation hook).
- No native (dynamic library) plugins; no `import` between scripts; no permissions beyond
  `commands`.

## 7. Example

`examples/plugins/ring-kit/` registers node type `ring_kit.ring` (params `radius`, `tube`,
`segments`, `sides`) and command `ring_kit.add_ring` (creates a ring node and renames it).
Tests: `crates/plugin/tests/plugin.rs`.
