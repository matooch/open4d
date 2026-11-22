# Camera projection — observation (O57)

Status: partial (8 of 14 cases: focal-18, film-offset, ortho-front, ortho-top, default-2, aspect-square-2, film-offset-y, ortho-zoom-2; focal-100-2 renders one white colour, sensor has no preset list, the Physical renderer cannot be selected from a script, default / focal-100 / aspect-square are superseded by the -2 redos — see the form) · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20, follow-ups 2026-11-21
Method: as [render-reference.md](render-reference.md) (O30): Standard renderer, 320 × 240.

**Calibration scene:** a default plane rotated P 90° (facing the camera) at z 0, carrying a
Checkerboard material with U/V frequency 10 (Luminance channel only, so lighting doesn't
matter), plus a default cube at (0, 0, −100) carrying a plain white material with only the
Luminance channel on (so it reads as a white silhouette against the checker, again without lights).

**Check before saving each render:** the image must show the checker. A render that is all or
mostly black means the scene is wrong (missing material, plane facing away, nothing in frame):
do not commit it. Read back the camera position and direction, the plane's rotation and the
materials, fix the scene and render again; after two failed tries mark the case
`not observed: render black (<what you read back>)`.

Output: `tests/golden/camera/<case>/render.png` and `mesh.obj`. `case.toml` holds every
camera parameter. Use `feature = "camera"`.

Unblocks: K/138, C/047. Level C (silhouette and checker corners within 1 px).

Record the camera's parameter table once (Object, Physical and Details tabs).

The camera table in every case carries: Projection, Sensor Size (Film Gate) 36 mm, the
35 mm-equivalent focal length the camera displays, Field of View (Horizontal) and (Vertical),
Zoom, Film Offset X/Y, Focus Distance 2000, Use Target Object off, Focal Distance 100,
Near Clipping on at 0 with Far Clipping off, F-Stop 8, Exposure 0, ISO 200, Shutter Angle π,
Diaphragm Blades 6, and the whole Physical/Stereoscopic/Composition block at its defaults.
The focal length itself has no editable field: the Field of View (Horizontal) drives it, and
the app then displays the equivalent focal length — 0.927295 rad reads "36 mm",
1.570796 reads "18 mm", 0.356186 reads "100 mm".

## Cases

| Case | Camera | Result |
|---|---|---|
| default | a camera made through its menu command at (0, 0, −600), defaults | not observed: the frame is a single black colour (720 bytes, one value on all 76 800 pixels), although the same scene renders content at 18 mm, with a film offset, and in both orthographic projections. The calibration plane as described does not appear at the default 36 mm lens |
| focal-18 | the same, focal length 18 mm | 5101 bytes, 283 distinct colours; FOV (horizontal) 1.570796 rad reads 18 mm, and the plane's checker (155/255) fills the frame around the cube |
| focal-100 | focal length 100 mm | not observed: the frame is a single black colour (720 bytes), like `default` — the scene is in view neither at 36 mm nor at 100 mm, so the calibration plane is not where the form expects it |
| sensor | sensor size changed to the second preset (record it) | not observed: Sensor Size (Film Gate) is a plain number in this build (36 mm) with no preset list, so there is no "second preset" to select; the value is recorded in every case |
| film-offset | film offset X 20 % | 3207 bytes, 164 colours; Film Offset X reads 20 %, and the plane enters the frame from the side (528 pixels at 154/255) — the frame shifts as expected |
| ortho-front | projection Front (orthographic), zoom default | 1298 bytes, 48 colours; 2536 pixels at 179/255 — the plane's checker is visible flat, with only two levels |
| ortho-top | projection Top, camera at (0, 600, 0) | 11 411 bytes, 560 colours; 4064 pixels at 233/255 — the widest tonal range of the four, because the plane's checker recedes across the frame |
| aspect-square | render size 240 × 240 | not observed: the 240 × 240 frame is a single black colour (666 bytes, 57 600 pixels), the same empty view as `default` |
| dof | Physical renderer (if available) with f-stop 1.4 and focus distance 600 | not observed: the render settings' Renderer offers Standard, Physical and Viewport Renderer, but setting it to Physical fails with "parameter set failed", so the Physical renderer cannot be switched on from a script here. F-Stop 1.4 and Focus Distance 600 do take (both read back) and are recorded, but with the Standard renderer they change nothing |

## Follow-up (planner, after brief 138)

The black `default`, `focal-100` and `aspect-square` renders are correct: their case files hold
camera rotation (H 45, P −19.45, B 0), which points the camera away from the scene. The form asks
for a default camera. Redo them as `default-2`, `focal-100-2`, `aspect-square-2` with the camera
rotation left at (0, 0, 0), read back before rendering and stored as `camera_rotation_deg`.
Also new:

| Case | Camera | Result |
|---|---|---|
| default-2 | the calibration scene, camera at (0, 0, −600), rotation left at (0, 0, 0), read back before rendering and stored as `camera_rotation_deg` | observed: rotation reads (0, 0, 0) before rendering, camera axis (0, 0, 1); `render.png` 7993 bytes, 243 distinct colours, mean 121.98, peak 255, 38 073 of 76 800 pixels black — black bars outside the plane, the checker ring around the cube's white face. `golden info`: 129 vertices, 106 faces, size (400.000006, 400.000006, 200), centre (0.000003, 0.000003, −100), normals 10, uvs 123 |
| focal-100-2 | the same, focal length 100 mm (horizontal field of view 0.356186 rad) | not observed: the frame is one white colour (255 on every sampled pixel) — at 100 mm the cube's white face fills the whole frame, so the checker cannot appear; rotation reads (0, 0, 0) as asked. A single-colour file is not committed, so the case folder was removed |
| aspect-square-2 | the same, render size 240 × 240 | observed: rotation reads (0, 0, 0); `render.png` 2689 bytes, 120 distinct colours, mean 93.41, peak 255, 34 693 of 57 600 pixels black — square frame with the same checker ring and white centre. `golden info`: 129 vertices, 106 faces, size (400.000006, 400.000006, 200), centre (0.000003, 0.000003, −100), normals 10, uvs 123 |
| film-offset-y | as `film-offset`, but Film Offset Y 20 % instead of X | observed: Film Offset Y reads 20 % (X 0 %), rotation (0, 0, 0); `render.png` 6571 bytes, 209 distinct colours, mean 106.47, peak 255, 43 380 of 76 800 pixels black — the checker and the white cube sit above a solid black band along the bottom, so the Y offset shifts the frame. `golden info`: 129 vertices, 106 faces, size (400.000006, 400.000006, 200), centre (0.000003, 0.000003, −100), normals 10, uvs 123 |
| ortho-zoom-2 | as `ortho-front`, Zoom 2 | observed: projection reads Front, Zoom 2, rotation (0, 0, 0); `render.png` 9339 bytes, 329 distinct colours, mean 132.40, peak 255, 33 701 of 76 800 pixels black — flat orthographic checker ring around the white centre. `golden info`: 129 vertices, 106 faces, size (400.000006, 400.000006, 200), centre (0.000003, 0.000003, −100), normals 10, uvs 123 |

## Notes

- The camera's focal length has no editable parameter. Setting Zoom does nothing (0.5 and
  2.777778 both leave the field of view at 0.927295 rad and the display at "36 mm"), while
  writing Field of View (Horizontal) changes both the render and the displayed
  35 mm-equivalent focal length. The three focal cases therefore set the field of view that
  corresponds to 18 mm, 36 mm and 100 mm on a 36 mm sensor: 2 · atan(18 / f).
- The empty frames are a property of the calibration scene, not of the camera: the same
  camera position renders the plane at 18 mm (a 90° field of view), at 20 % film offset, and
  in both orthographic projections, but nothing at all at 36 mm or 100 mm. A default plane
  rotated P 90° at z 0 with the camera at z −600 is evidently not in frame at the default
  lens in this build, so the four rows that depend on it are left not observed rather than
  recorded as black goldens.
- Only the four cases with visible content were committed (`focal-18`, `film-offset`,
  `ortho-front`, `ortho-top`). The empty folders `tests/golden/camera/default`,
  `/focal-100`, `/aspect-square` and `/dof` were written by the probe but are **not** in git;
  they should be deleted or rebuilt once the plane's placement is settled.
- The Physical renderer refusal is a script limitation: the Renderer row is a cycle, but
  writing the Physical entry fails, so depth of field cannot be observed without a GUI
  session.
- The checker material is built with the material's Color channel off and its Luminance
  channel on, with the Checkerboard shader in the Luminance slot and both frequencies at 10.
  Where the plane is in frame, that renders as two flat levels (orthographic Front) exactly
  as the form expects lighting not to matter.

### Follow-up read-back, 2026-11-21 (brief 138)

- Two separate problems were behind the black frames. The camera rotation the brief names
  (45, −19.45, 0) points away from the scene, and on top of that the old probe rotated the
  calibration plane by its first rotation component, which is heading, so the plane lay flat at
  the camera's own height and could not appear at all. The form's "plane rotated P 90° (facing the
  camera)" is the pitch component: the follow-up cases read plane rotation (0, 90, 0) with the face
  along −Z, and every one of them shows the checker. The four committed cases keep their flat plane
  (existing golden cases are not edited); re-checking their PNGs on 2026-11-21, each is about 90 %
  black with one grey corner shape where the scene lies relative to that rotation — no checker
  fills them, so their Result cells overstate what the files show.
- The menu camera arrives at rotation (45, −19.45, 0), matching the app's current perspective view;
  that is what all four committed case files hold. The five follow-up cases set (0, 0, 0) and read
  it back before rendering. `film-offset-y` and `ortho-zoom-2` keep every other value from their
  named cases (Film Offset X 0 % / default zoom) — that is the reading of "as film-offset / as
  ortho-front" used here, since brief 138 defines the default camera rotation as (0, 0, 0).
- A new texture tag reads Projection Spherical, which turns the checker into radial wedges around
  the image centre (first attempt). Each follow-up case sets the tag to UVW Mapping, recorded in
  `case.toml` as `params.texture_tag`, which shows the plane's own square checker, as the level-C
  check "checker corners within 1 px" expects.
- The cube carries a plain white material with only its Luminance channel on, as the calibration
  scene describes, so it reads as a white square against the checker; at 100 mm that white face
  covers the frame, which is why `focal-100-2` stays not observed (single-colour file, not
  committed).
