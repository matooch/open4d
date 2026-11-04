# Forge3D

Forge3D is a native 3D modelling and animation application written in Rust. It runs on
Linux, macOS and Windows and is built around one idea: **everything you can do is a
command**.

- Menus, buttons, the command line, scripts, the JSON-RPC server and the MCP server all run
  the same registry of commands, so they behave identically and share one undo history.
- Every object type is registered by a plugin and describes its parameters as data. The
  attribute panel, undo, animation and the file format all derive from that one description.
- Evaluation is deterministic: the same scene and frame give the same geometry every time.

## How this manual is organised

The chapters in the first half explain concepts and workflows. The chapters under
*Reference* are generated from the code (`forge docs`) and checked by the tests, so they are
always exact: every command with its arguments, every CLI option, RPC method, MCP tool and
script function.

Where a chapter describes behaviour that is still being tuned, it says so.

## Conventions

- Distances are in scene units, angles in degrees and percentages as shown in the editor.
- Node ids are integers; frames are integers.
- Command ids look like `object.create`. Node type ids look like `forge.cube`.
