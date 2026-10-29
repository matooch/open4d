# Render references — observation (O30)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted, with `render_frame` or the scripting render call, using the **Standard**
renderer. These are renders of our own simple test scenes, not of the app's UI.

Folder `tests/golden/render/<case>/` holds:
- `render.png`: 320 × 240, PNG, 8-bit RGB;
- `mesh.obj`: the whole scene's geometry, with the usual settings;
- `case.toml`: `feature = "render"`, `observation = "docs/parity/render-reference.md"`.
  `[params]` holds every object, light, camera and render setting you changed, under our
  keys, plus the camera's position, rotation and focal length.

Unblocks: U6 (CPU renderer); compared by PSNR, level C.

## Fixed setup for every case

- New document.
- A camera made through its menu command, at (0, 200, −600), rotation P −15°, focal
  length 36 mm, set as the render camera.
- Render settings:
  - output 320 × 240;
  - anti-aliasing at its default, recorded;
  - everything else at the new document's defaults, recorded.

## Cases

| Case | Scene | Lights |
|---|---|---|
| cube-default-light | default cube | none: the default light |
| sphere-default-light | default sphere | none |
| cube-plane | default cube at y 100 on a default plane | none |
| omni | the same | one Omni light at (300, 400, −300), defaults |
| omni-shadow | the same | the same, Shadow Raytraced (Hard) |
| spot | the same | one Spot light at (0, 500, 0) pointing down (rotation P −90°) |
| infinite | the same | one Infinite light, rotation P −45°, H 30° |
| area-soft | the same | one Area light at (300, 400, −300), shadow Area |
| material-red | default sphere | none. A Standard material with Color (1, 0, 0) and the reflectance at its default |
| material-matte | default sphere | none. A Standard material with Color (0.8, 0.8, 0.8) and the reflectance layer removed or set to 0 |

Record the render time and anything that might vary between runs: noise, sampling, seed.
If the renderer is not deterministic, render twice and say whether the two images match.

## Notes
TODO
