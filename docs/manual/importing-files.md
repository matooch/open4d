# Importing files from other apps

Forge3D reads FBX, glTF (`.glb`, `.gltf`), Alembic (`.abc`), STL, PLY, OBJ and ASCII USD. In
the editor use *File > Import*; on the command line `forge convert` turns any of them into a
native scene, so a script, a CI job or an agent can do the same without a window.

```sh
# import: file -> native scene (the folder is created)
forge convert model.fbx --out work --format forge

# look at what arrived: the node tree with names, types, transforms, cameras, lights
forge info work/model.forge

# evaluate a frame; the report lists each object's point count and bounding box,
# --export also writes that frame as OBJ
forge eval work/model.forge --frame 15 --export frame15.obj

# convert on to another format at a frame (obj, stl, ply, gltf, usda, fbx, abc)
forge export work/model.forge --format gltf --frame 15 --out frame15.glb
```

Every command takes `--json` for machine-readable output. The importer's options are passed
with `--option key=value`; `forge commands` lists them (`io.formats`).

## Units and axes

Scenes use centimetres, left-handed, Y up.

- **FBX** and **Alembic** carry their own unit and axes; Forge3D converts them. A file in
  metres arrives 100 times larger in scene units, so it looks the same.
- **glTF** is defined in metres, so the importer multiplies by 100 (`scale` defaults to
  100). Some exporters write centimetre values into their glTF anyway; if the model arrives
  100 times too large, import with `--option scale=1`.
- **STL** and **PLY** have no unit: `scale` multiplies the coordinates as they are.

## What each format carries

| | Hierarchy and names | Animation | Skin | Cameras | Lights | Materials |
|---|---|---|---|---|---|---|
| FBX | yes | transform keys | joints, weights, bind pose | yes | yes (omni, spot, infinite, area) | name and diffuse colour |
| glTF | yes | transform keys (if the exporter wrote them) | joints, weights | yes | no | colour, metallic, roughness |
| Alembic | yes (transform and shape objects) | transforms and per-frame points | no (the deformed points are baked) | yes | no | no |
| STL | no: one object, triangles in world space | no | no | no | no | no |

Anything a file does not carry simply does not appear: importing the STL of a scene with
joints gives one polygon object, and an FBX of a spot light gives a light, but a glTF of the
same scene gives none. An STL without triangles is refused with `no polygons found`.

STL and glTF hold triangles only, so quads and n-gons arrive as triangles. Check geometry
by evaluating and exporting, then comparing the OBJ with your source:

```sh
forge convert source.abc --out work --format forge
forge eval work/source.forge --frame 0 --export check.obj
```

A skinned FBX or glTF is posed by its joints: key a joint with `forge cmd work/arm.forge
key.set_parameter ...` (see [Automation](automation.md)) and `forge eval --frame N` reports
the deformed bounds. glTF skins are evaluated with normalised weights, so a file whose weights
sum to slightly less than one can differ from its source application by a few thousandths of a
millimetre.
