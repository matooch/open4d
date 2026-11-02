# Material renders — observation (O43)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [render-reference.md](render-reference.md) (O30), using its fixed
camera, 320 × 240 Standard renderer and render settings. The scene is a default sphere at
(0, 100, 0) on a default plane, with one Area light at (300, 400, −300) with shadow Area.
Each case changes only the sphere's material.

Output: `tests/golden/render/<case>/render.png` and `mesh.obj`, with
`observation = "docs/parity/material-renders.md"`. Level C (PSNR). Unblocks: J/123, 125,
K/145.

## Cases

| Case | Sphere material |
|---|---|
| mat-default | a new default material |
| mat-red-matte | colour (0.8, 0.1, 0.1); reflectance layer removed or brightness 0 |
| mat-glossy | colour (0.2, 0.2, 0.8); a GGX reflection layer, roughness 10 % |
| mat-rough | as mat-glossy, roughness 60 % |
| mat-metal | reflectance only (colour channel off), GGX, roughness 20 %, layer colour (0.9, 0.7, 0.3) |
| mat-bump-noise | default colour, Bump channel with a Noise shader, strength 50 % |
| mat-transparent | transparency on, refraction 1.5, brightness 80 % |
| mat-luminance | luminance channel on, colour (1, 0.5, 0) |
| mat-fresnel | Color channel = Fresnel shader |
| mat-checker-uv | Color = Checkerboard, UVW projection |
| mat-checker-cubic | Color = Checkerboard, texture tag projection Cubic |

## Notes
TODO
