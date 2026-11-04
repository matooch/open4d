# Getting started

## Start the editor

Run the editor binary (`forge-studio`). The default layout, top to bottom, is:

- the **menu bar** (File, Edit, Select, Mesh, Create, Animate, View, Window);
- the **command bar** with buttons for undo and redo, creating common objects, shading and
  playback;
- the **tool palette** on the left (edit mode, selection helpers, modelling tools);
- the **viewport** in the centre;
- the **Object manager** above the **Attribute manager** on the right;
- the **timeline** along the bottom, with a Coordinates tab, and a status bar.

*Window > Reset layout* restores this arrangement.

## Your first scene

1. Click the cube button in the command bar (or run *Create* and pick a cube).
2. Select it in the Object manager; its parameters appear in the Attribute manager.
3. Change the size, then press the undo button to see it revert. Every edit is one undo step.
4. Choose *File > Save* (Ctrl/Cmd+S). Scenes are `.forge` files.

## The same thing from a terminal

```sh
forge new scene.forge
forge cmd scene.forge object.create --args '{"type":"forge.cube"}'
forge info scene.forge
forge export scene.forge --out scene.obj --format obj
```

`forge info` prints the node tree. See [Automation](automation.md) and the
[command line reference](reference/cli.md) for everything else.
