# Make Editable and Current State to Object — observation (O3)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted or GUI ([agents/OBSERVER.md](../agents/OBSERVER.md)); level A for the
meshes, level C for the tree. Unblocks: the parity-pending rules of brief 03.

For each case, record the **object tree after the command** (names, types as shown,
order, tags, children) and export the result as OBJ with the usual settings to
`tests/golden/make-editable/<case>/mesh.obj` (for a tree with several polygon objects,
export the whole scene and note which object is which group).

## Cases

| Case | Scene before | Command | Tree after (names / types / tags / children) | Notes |
|---|---|---|---|---|
| cube | default cube with its smoothing tag | Make Editable | | name kept? tags kept? new selection tags? |
| cube-deformed | cube (segments 4/8/4, size_y 400) > Bend strength 90 | Make Editable on the cube | | is the bend applied or kept as a child? |
| cube-deformed-csto | same | Current State to Object on the cube | | where is the new object put, what is it named? |
| subdivision | Subdivision Surface > default cube | Make Editable on the generator | | one object or a hierarchy? what happens to the input? |
| subdivision-csto | same | Current State to Object | | |
| extrude | Extrude > Rectangle spline (defaults) | Make Editable | | caps as separate objects? |
| array | Array > default cube | Make Editable | | |
| sphere-moved | sphere at position (10, 20, 30), rotation H 45° | Make Editable | | transform kept on the object or baked into the points? |
| spline | Circle spline | Make Editable | | (tree only; splines don't export) |

## Behaviour notes
Undo: is each command one undo step? Selection after the command? Shortcut keys and menu
paths as shown: TODO
