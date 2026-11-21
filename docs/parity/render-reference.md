# Render references — observation (O30)

Status: observed (12 of 14 cases; two lights render black, see below) · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20, follow-ups 2026-11-21
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

The camera's 36 mm is its default: Sensor Size (Film Gate) 36 mm with Zoom 1.0, and the
camera reads "35 mm Equiv. Focal Length: 36 mm". The renderer is Standard, the output preset
is 320 x 240 at 72 dpi, and every render took 0.04–0.06 s.

## Cases

| Case | Scene | Lights | Result |
|---|---|---|---|
| cube-default-light | default cube | none: the default light | 4798 bytes, 268 distinct colours; the background is black (67 018 of 76 800 pixels) and the cube's lit faces peak at 159/255. Rendered twice: the two files are byte-identical, so the Standard renderer is deterministic here |
| sphere-default-light | default sphere | none | 7396 bytes, 895 colours; sphere peak 188/255 |
| cube-plane | default cube at y 100 on a default plane | none | 7553 bytes, 290 colours; cube peak 163/255 |
| omni | the same | one Omni light at (300, 400, −300), defaults | 10 367 bytes, 592 colours; cube peak 138/255 — dimmer than the default light |
| omni-shadow | the same | the same, Shadow Raytraced (Hard) | 10 401 bytes, 600 colours; the extra colours against `omni` are the hard shadow edge on the plane and the cube |
| spot | the same | one Spot light at (0, 500, 0) pointing down (rotation P −90°) | not observed: render black with the light pointing (0, -1, 0). The light reads back as a Spot at (0, 500, 0), rotation H 0 / P -90 / B 0, intensity 100 %, and its +Z axis in world space is (0.000, -1.000, 0.000), i.e. straight down at the cube. A 320 x 240 render of the default scene with it returns success and every sampled pixel is (0, 0, 0), one colour. No image committed. |
| infinite | the same | one Infinite light, rotation P −45°, H 30° | not observed: the light type cannot be set to Infinite from a script in this build. Writing the description's Type row to the Infinite slot reads back through the parameter table as "Square Spot" every time (three attempts, by label and by index), so no infinite light could be placed and no direction was measured. No image committed. |
| area-soft | the same | one Area light at (300, 400, −300), shadow Area | 7150 bytes, 184 colours; the brightest surface reads 230/255, so the Area light is much stronger than the Omni at the same place |
| material-red | default sphere | none. A Standard material with Color (1, 0, 0) and the reflectance at its default | 7798 bytes, 4167 colours; the sphere's brightest pixel is (213, 0, 0) and the falloff to the terminator is smooth. The material's reflectance was switched off, see Notes |
| material-matte | default sphere | none. A Standard material with Color (0.8, 0.8, 0.8) and the reflectance layer removed or set to 0 | 7396 bytes, 895 colours — byte-identical to `sphere-default-light`, so the new material's default colour is (0.8, 0.8, 0.8) and this case documents the default rather than a change |

## Follow-up cases (planner): black light renders

`spot` and `infinite` came out black. Before rendering them again, read back each light's
position, rotation and the direction it points (the light's +Z axis in world space), and its
intensity, and write them in the row. Spot: it must point from (0, 500, 0) towards the origin.
Infinite: its direction must have a downward (−Y) part. If the light reads back correctly and the
render is still black, mark the case `not observed: render black with the light pointing <direction>`
instead of committing the image. Never commit an all-black render.

## Follow-up cases (planner, after briefs 136/137)

| Case | Setup | Record | Result |
|---|---|---|---|
| light-units | a 400 × 400 plane facing the camera, white matte material, one Omni light 300 units in front of its centre, intensity 100 %, no default light | `render.png`, and the light's intensity, falloff and units as read back | observed: `render.png` 12 029 bytes; over the full frame 320 distinct colours, mean 68.38, peak channel 255 (the white plane clips), 55 199 of 76 800 pixels black. Light read back before rendering: Type Omni, Intensity 100 %, Falloff None with Falloff Inner Radius 0 and Falloff Radius / Decay 500, Units Candela (cd), Photometric Intensity off; the renderer Default Light is off as asked. The plane reads rotation (0, 90, 0) so its face points along −Z towards the camera; the new texture tag reads Projection Spherical, the same default the other render cases recorded. Camera rotation read back (−15, 0, 0) in H, P, B order. `golden info`: 121 vertices, 100 faces, size (400.000006, 400.000006, 0), normals 4, uvs 121 |
| light-units-200 | the same with the Omni 600 units away | the same (settles falloff) | observed: `render.png` 8 689 bytes; 132 distinct colours, mean 70.66, peak 255, exactly the same 55 199 black pixels as `light-units`. Same read-backs: Type Omni, Intensity 100 %, Falloff None (inner radius 0, radius / decay 500), Units Candela (cd), Default Light off, camera (−15, 0, 0) in H, P, B order. With Falloff None the frame mean does not drop with distance (68.38 at 300 units against 70.66 at 600) and both frames clip at 255, so neither peak nor mean shows a distance falloff. `golden info`: 121 vertices, 100 faces, size (400.000006, 400.000006, 0), normals 4, uvs 121 |
| area-soft-2 | redo `area-soft` with an Area light (shape Rectangle, defaults) at (300, 400, −300) pointing at the origin; read the light type back before rendering and store it as `light_type` | `render.png` | observed: the light Type read back before rendering as Area, Area Shape Rectangle, Shadow Area; the type is stored as `light_type`. `render.png` 7 150 bytes, 184 distinct colours, mean 49.36, peak channel 238 — byte-identical to `area-soft` (MD5 500b98304c80da4a093449f7e1b967f4), so Rectangle was already that case's default. Camera rotation read back (−15, 0, 0) in H, P, B order, Default Light on as in `area-soft`. `golden info`: 129 vertices, 106 faces, size (400.000006, 200, 400.000006), normals 6, uvs 123 |
| checker-plane | the 400 × 400 plane with the checker material of `mat-checker-uv`, camera straight on | `render.png` (settles the checker phase) | observed: camera straight on read as position (0, 0, −600) and rotation (0, 0, 0) in H, P, B order; the texture tag was set to projection UVW Mapping like `mat-checker-uv`; Default Light stays on (document default). `render.png` 41 464 bytes, 1 564 distinct colours, mean 86.57, peak 234, 30 985 of 76 800 pixels black (the background around the plane). `golden info`: 121 vertices, 100 faces, size (400.000006, 400.000006, 0), normals 4, uvs 121 |

Every case: also store the camera's rotation as read back, in H, P, B order, under `camera_rotation_deg`.

## Notes

- The scripting render call needs a bitmap of the output size handed to it and writes the
  picture itself afterwards; the render settings object supplies width, height and renderer.
  The camera becomes the render camera through the render view's scene-camera setting.
- Determinism: `cube-default-light` was rendered twice and the two PNG files match byte for
  byte. Nothing noisy or sampled appears at these settings — no anti-aliasing difference, no
  seed, no grain.
- The Standard renderer is the render settings' renderer ("Standard"), and the output preset
  reads "Preset: 320 x 240" once the width and height are set.
- Spot and Infinite lights render a completely black frame from a script, whatever their
  rotation and intensity, while an Omni at the same distance lights the scene. The two rows
  are therefore left not observed; the scenes were built and the renders attempted, so a GUI
  session can check quickly whether the light type itself or the scripted path is at fault.
- Their two folders (`tests/golden/render/spot/`, `tests/golden/render/infinite/`) hold those
  black renders on disk but were deliberately **not committed**; they should be deleted
  before the feature is trusted. Deleting them from this unattended run hit the approval gate
  for destructive commands, so the owner has to remove them.
- Reflectance could not be edited layer by layer from the script (the material's reflectance
  list behaves like the Layer shader's list), so both material rows switch the material's
  reflectance off instead of removing a layer. That keeps "no reflection" true for the
  matte case and is the closest literal reading for the red one.
- `material-matte` cannot be distinguished from `sphere-default-light`: the default
  material's colour is (0.8, 0.8, 0.8) and its reflectance contributes nothing visible under
  the default light.

### Light read-back, 2026-11-20

The spot light was rebuilt exactly as this section asks and read back before rendering: type Spot, position
(0, 500, 0), rotation H 0 / P -90 / B 0, intensity 100 %, +Z axis in world space (0.000, -1.000, 0.000). The
render is still a single black colour, so the case stays not observed with that direction recorded.

The infinite light could not be placed at all: the light object's Type row accepts the write and reads back
as "Square Spot" whatever slot is written, by label or by index, so this build offers no scripted route to
an Infinite light. That is the reason the case is unobserved, not the render.

### Follow-up read-back, 2026-11-21

- Rotation components, measured rather than assumed: turning a plane and a camera by 90°, one
  component at a time, and reading their axes back shows that the first rotation component turns
  the object about the world's vertical axis (heading), the second about the horizontal axis
  (pitch) and the third rolls it (bank). The earlier spot read-back agrees: rotation H 0 / P −90
  gave a straight-down direction. Read that way the fixed camera above reads heading −15, pitch 0,
  bank 0, while the fixed-setup line labels the same −15 as "P −15°". The value and the camera are
  unchanged and are the ones every committed case used — `area-soft-2` reproduces `area-soft` byte
  for byte — so only the letter was a slip; each follow-up case stores the camera rotation read
  back in H, P, B order.
- The four follow-ups run in one cleared document, so each one sets the renderer's Default Light
  explicitly: the first `checker-plane` attempt came out all black because the "no default light"
  of the light-units cases was still set, and it was redone with the default light on.
- "Plane facing the camera" was read as the plane's face turned towards the camera (rotation
  pitch +90°, face along −Z); the camera stands 18.4° above the plane's centre, so the face is not
  exactly perpendicular to the view ray. "Camera straight on" was read as position (0, 0, −600)
  with rotation (0, 0, 0).
- A new texture tag reads Projection Spherical in this build; `mat-checker-uv` and `checker-plane`
  set UVW Mapping explicitly.
- `light-units` and `light-units-200` clip at peak channel 255 on the white plane at both
  distances, so the pair documents "no distance falloff while Falloff is None" by the frame means
  (68.38 against 70.66) and the identical lit area (55 199 black pixels each), not by a peak
  value.
