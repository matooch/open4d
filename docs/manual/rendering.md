# Rendering

Forge renders with a CPU path tracer. It uses the scene as it is at the current frame:
geometry, instances, shader node trees, lights, the camera and the scene's stored render
settings. The same snapshot model feeds the CLI, the Picture Viewer and the interactive
preview.

## Cameras

Create a camera with `object.create` (`forge.camera`). The first camera in tree order is
the render camera; use **Look Through Camera** to view the scene through a selected one.
A camera looks along its local +Z.

The Object tab holds the projection (perspective or the axis projections), the sensor
size and the horizontal field of view. The focal length follows from the sensor and the
field of view, so changing either changes the lens. Film offset shifts the frame without
moving the camera.

The Physical tab has the f-stop. A small f-stop opens the thin lens: enable **Depth of
Field** in the render settings (or pass `--dof` to the CLI) and objects away from the
focus distance become blurred. The focus distance is the Object tab's **Focus Distance**;
use a focus object to track a moving subject.

## Lights

A light object (`forge.light`) has a type: omni, spot, infinite, area, square spot,
parallel, parallel spot, square parallel spot or IES. Area lights take a shape (disc,
rectangle, line, sphere, object) and sizes. The light shines along its local +Z.

The General tab sets the colour and intensity, whether the light is visible, and which
channels it contributes to. The Details tab sets the cone angles, falloff and the
area/parallel options. The Shadow tab sets the shadow mode (none, soft shadow maps,
raytraced hard or area) and its density and colour.

A scene with any light object uses those lights and switches the default head light off.
Without lights, the renderer adds a head light unless **Default Light** is turned off.

## Materials and shaders

A Standard material (`material.create`) has a texture slot for each channel: colour,
luminance, transparency, specular colour and mask, bump, normal, alpha and displacement.
A slot accepts a shader node tree built from `forge.shader.*` types, for example colour,
gradient, noise, fresnel, falloff, layer, colorizer, bitmap, checkerboard and tiles.
Child shaders are inputs of their parent; `shader.set` adds one to a parent.

The node-graph material (`forge.material.node`) is a different kind: it exposes typed
Surface inputs (base colour, roughness, metallic, textures and so on) and can host colour,
scalar, texture and mix nodes. Use the shader tree when you need noise, fresnel or other
procedural looks.

Assign a material to objects with **Assign Material** (`material.assign`, `nodes`). The
assignment is a texture tag; changing the material updates every object that uses it.

```sh
forge cmd scene.forge material.create --save
# -> {"material": 2}
forge cmd scene.forge shader.set --save \
  --args '{"type":"forge.shader.noise","material":2,"channel":"color","params":{"space":"object","global_scale":0.05}}'
forge cmd scene.forge shader.set --save \
  --args '{"type":"forge.shader.fresnel","material":2,"channel":"bump","params":{"ior":1.5}}'
forge cmd scene.forge material.assign --save \
  --args '{"material":2,"nodes":[1]}'
```

## Render settings

The document can store its own render settings with `render.settings`:

```sh
forge cmd scene.forge render.settings --save \
  --args '{"width":1280,"height":720,"samples":64,"max_depth":8,"seed":1,"filter":"cubic","filter_width":2.0,"depth_of_field":false}'
forge cmd scene.forge render.settings --save --args '{"reset":true}'
```

Width and height are pixels, samples are per pixel, and max depth counts path vertices.
The filter is box, triangle, gaussian or cubic with a width in pixels. Depth of Field
traces camera rays through the camera's thin lens. A scene without stored settings uses
the caller's defaults (the CLI's are 320×240, 16 samples and depth 5).

The Picture Viewer and the interactive preview use the same settings. The viewer's
options can add render defaults from the user Preferences for scenes that do not store
their own.

## Passes and formats

`--aov` adds passes: `depth`, `normal`, `albedo`, `position`, `object_id`, `material_id`,
`shadow`, `diffuse`, `specular`, `emission`, or `all`. Data passes are written in linear
space with an opaque alpha; colour passes use the display transform.

Output formats are PNG, TIFF, JPEG, HDR and EXR, at 8, 16 or 32 bits. Separate files name
each pass with the `{pass}` token. `--multipart` writes one EXR with a layer per pass.

## Rendering from the CLI

```sh
forge render scene.forge --out shots/look.png \
  --width 960 --height 720 --samples 64 --max-depth 8 --seed 7 \
  --dof --aov depth,normal,albedo,object_id
forge render scene.forge --out shots/look-{pass}.exr --multipart --aov all
forge render scene.forge --out shots/anim-{frame:4}.png --start 0 --end 90 --step 1
```

`--frame` renders one frame; `--start` and `--end` render a sequence (add `--step`).
Command-line flags override the scene's stored settings. The command prints one line per
written file and reports warnings for options a renderer had to approximate.

## Determinism and comparison

Sampling uses a fixed per-pixel seed, and the CPU tracer uses `f64` for shading, so the
same scene and settings produce the same image on every run. The S8 acceptance test
(`s8_lookdev_area_light_camera_dof_material_and_aovs`) renders an area-lit scene through a
physical camera with depth of field, a noise/fresnel shader tree and all passes, and
compares the beauty with `tests/acceptance/s8-reference.png`. Every CI operating system
runs the same test; each colour channel must stay within 1 dB of the reference and PSNR
must be at least 30 dB.

## The Picture Viewer and IPR

**Picture Viewer** is a centre dock panel. It queues still renders, keeps a bounded
history with A/B comparison, shows data passes through display mappings, adjusts exposure
on colour passes and saves the displayed view as PNG. **All passes** captures every AOV.

**Interactive Render** is a viewport-area tab. It renders the current frame at up to a
512-pixel edge and accumulates tiles, restarts when the scene changes, and pauses or
stops without touching the scene or the viewer history.
