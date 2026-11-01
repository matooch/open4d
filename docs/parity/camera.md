# Camera projection — observation (O57)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: as [render-reference.md](render-reference.md) (O30): Standard renderer, 320 × 240.

**Calibration scene:** a default plane rotated P 90° (facing the camera) at z 0, carrying a
Checkerboard material with U/V frequency 10 (Luminance channel only, so lighting doesn't
matter), plus a default cube at (0, 0, −100).

Output: `tests/golden/camera/<case>/render.png` and `mesh.obj`. `case.toml` holds every
camera parameter. Use `feature = "camera"`.

Unblocks: K/138, C/047. Level C (silhouette and checker corners within 1 px).

Record the camera's parameter table once (Object, Physical and Details tabs).

## Cases

| Case | Camera |
|---|---|
| default | a camera made through its menu command at (0, 0, −600), defaults |
| focal-18 | the same, focal length 18 mm |
| focal-100 | focal length 100 mm |
| sensor | sensor size changed to the second preset (record it) |
| film-offset | film offset X 20 % |
| ortho-front | projection Front (orthographic), zoom default |
| ortho-top | projection Top, camera at (0, 600, 0) |
| aspect-square | render size 240 × 240 |
| dof | Physical renderer (if available) with f-stop 1.4 and focus distance 600 (skip if the Physical renderer isn't scriptable) |

## Notes
TODO
