# Tutorial: model a chair

This walk-through builds a simple chair with a single central leg: half a seat is modelled
once, mirrored, and smoothed with a subdivision surface. It uses only the editor window. The
same steps run headless in the acceptance test `crates/ui/tests/s3_chair.rs`, which also checks
that the chair survives a save and a reopen and exports as an OBJ file.

Distances are in the scene's units; the default cube is 200 wide.

## 1. A primitive

1. Choose *Create > Generator > Cube*. The cube is selected and its parameters appear in the
   Attribute manager.
2. Set *Size X* to 200, *Size Y* to 40 and *Size Z* to 200, and set the cube's position X to
   100 in the Coordinates tab. This is half of the seat: its inner face lies on the plane
   x = 0, where the mirror will go.

The title bar gets a `*`: the scene has unsaved changes.

## 2. Make it editable

Choose *Edit > Make Editable* (Ctrl/Cmd+Shift+E). A cube is a parametric generator; making it
editable turns it into a polygon object with 8 points and 6 polygons that you can reshape.

## 3. The leg: inset, then extrude

1. Switch to polygon mode (Alt+4) and select the bottom face of the seat.
2. Choose *Mesh > Extrude Inner*. The tool's settings appear in the Attribute manager. Set
   *Offset* to 60 and press **Apply**. The face gets a smaller face inside it.
3. Choose *Mesh > Extrude*, set *Offset* to 180 and press **Apply**. The new inner face is
   pulled down and forms the leg.
4. Press **Done**.

Each **Apply** is one undo step; Ctrl/Cmd+Z takes it back.

## 4. Bevel the seat edge

Switch to edge mode (Alt+3), select the outer top edge of the seat, choose *Mesh > Bevel* and
apply an offset of 8. The sharp edge is replaced by a narrow strip.

## 5. Loop cut

Choose *Mesh > Loop/Path Cut*, move the pointer over an edge to preview the cut, and click.
A ring of new edges is inserted across the polygons around that edge. Undo and redo with
Ctrl/Cmd+Z and Ctrl/Cmd+Shift+Z.

## 6. Symmetry

Return to object mode (Alt+1). Create a **Symmetry** object (*Create > Generator > Symmetry*)
and drag your polygon object onto it in the Object manager so that it becomes its child. The
Symmetry generator mirrors its child about the plane x = 0 (the default *ZY* plane) and welds
the points on the plane, so the chair is now complete and closed.

Edits to the child still update the result. For editing the points of one half and seeing the
other half follow, see `edit.symmetry` in the [command reference](reference/commands.md).

## 7. Subdivision surface

Create a **Subdivision** object and drag the Symmetry object onto it. The result is the
smoothed chair; *Viewport level* and *Render level* in the Attribute manager set how often the
surface is subdivided.

Note: in a small window the *Create > Generator* list is longer than the window; scroll it
with the mouse wheel to reach Symmetry and Subdivision at the bottom.

## 8. Save, reopen, export

- *File > Save* (Ctrl/Cmd+S) writes a `.forge` file. *File > Open* (Ctrl/Cmd+O) reads it back;
  the chair evaluates exactly as before.
- *File > Export OBJ...* asks for a file name, then shows the export options; the file is
  written when you press **Export**. The OBJ holds the evaluated surface, one object with one
  face per polygon of the smoothed chair. See [Files and recovery](files-and-recovery.md).

## From a script

Every step above is a command, so the whole tutorial can be run with `forge cmd` or from a
script; see [Automation](automation.md).
