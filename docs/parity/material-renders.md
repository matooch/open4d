# Material renders — observation (O43)

Status: observed (11 of 11 cases) · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20
Method: exactly as [render-reference.md](render-reference.md) (O30), using its fixed
camera, 320 × 240 Standard renderer and render settings. The scene is a default sphere at
(0, 100, 0) on a default plane, with one Area light at (300, 400, −300) with shadow Area.
Each case changes only the sphere's material.

Output: `tests/golden/render/<case>/render.png` and `mesh.obj`, with
`observation = "docs/parity/material-renders.md"`. Level C (PSNR). Unblocks: J/123, 125,
K/145.

Every case carries the full material parameter table (all channels, including the default
specular layer) and the texture tag's projection in `case.toml`, plus the camera and the two
scene objects. The images are 320 × 240, eight-bit RGB.

## Cases

| Case | Sphere material | Result |
|---|---|---|
| mat-default | a new default material | 12 368 bytes, 1140 distinct colours; the sphere's brightest pixel is 188/255 and the background is black (66 112 of 76 800 pixels) |
| mat-red-matte | colour (0.8, 0.1, 0.1); reflectance layer removed or brightness 0 | 10 882 bytes, 564 colours; surface peak (201, 25, 25), reflectance switched off — see Notes |
| mat-glossy | colour (0.2, 0.2, 0.8); a GGX reflection layer, roughness 10 % | 11 662 bytes, 662 colours; a small bright highlight (49, 49, 197) against the blue body — the layer's Width field carries the roughness (Notes) |
| mat-rough | as mat-glossy, roughness 60 % | 12 575 bytes, 2381 colours — six times the tonal range of mat-glossy, the highlight spread over the whole surface |
| mat-metal | reflectance only (colour channel off), GGX, roughness 20 %, layer colour (0.9, 0.7, 0.3) | 1978 bytes, 250 colours; with the colour channel off and no environment the sphere is almost black, only 725 pixels differ from the background — the Area light's reflection alone |
| mat-bump-noise | default colour, Bump channel with a Noise shader, strength 50 % | 28 650 bytes, 1391 colours against the 1140 of mat-default, and the brightest surface drops from 188 to 171 as the noise breaks up the shading |
| mat-transparent | transparency on, refraction 1.5, brightness 80 % | 12 061 bytes, 990 colours; the sphere reads 136/255 — dimmer than mat-default because the plane shows through |
| mat-luminance | luminance channel on, colour (1, 0.5, 0) | 11 309 bytes, 942 colours; the surface peaks at (255, 220, 188), so the luminance channel lifts it past white in red |
| mat-fresnel | Color channel = Fresnel shader | 13 157 bytes, 974 colours; the body picks up a tint the default material does not have (first non-black colour (89, 89, 89)) |
| mat-checker-uv | Color = Checkerboard, UVW projection | 20 086 bytes, 1609 colours; 729 pure-white pixels — the checker's white squares land on the sphere |
| mat-checker-cubic | Color = Checkerboard, texture tag projection Cubic | 25 259 bytes, 1618 colours; 660 white pixels and a different pattern from the UVW case, so the projection really changes the mapping |

## Notes

- The scene, camera, light and render settings are the O30 ones; only the material and (for the
  two checker cases) the texture tag's projection differ.
- The renderer's reflection layer has no field labelled "Roughness" in this build. Its Width
  field is the roughness control: Width 10 % gives a tight highlight, 60 % spreads it over the
  whole sphere, 20 % sits between. The case files record the Width value under its own label.
- There is a "Roughness" field on the material too, but it belongs to the diffuse (Oren-Nayar)
  model: changing it from 10 % to 60 % leaves the render byte-identical, so it is not the
  reflection roughness.
- Reflectance could not be edited layer by layer (its list behaves like the Layer shader's), so
  mat-red-matte switches the material's reflectance off rather than deleting a layer; that is
  the closest literal reading of "reflectance layer removed or brightness 0".
- mat-metal is nearly black by design of the setup: with the colour channel off and no
  environment object, a metal can only show the Area light's reflection, which is 725 pixels.
- The Color, Luminance, Transparency and Bump channels each take a shader through their own
  Texture slot; the noise in the bump channel and the checkerboard in the colour channel both
  show up in the render, and the two checker projections differ from each other.
