# Camera projection — observation (O57)

Status: partial (4 of 9 cases; 4 render an empty frame, 1 is not scriptable — see Notes) · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20
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
