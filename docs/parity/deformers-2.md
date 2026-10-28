# Deformers II — observation (O9)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)). Level A (exact meshes).
Unblocks: P1 (the remaining deformers).

## Setup for every case

Use the same input as bend.md: a cube made with its menu command, size_y 400, segments
4/8/4. Make the deformer through its menu command as the cube's **child**, at the
origin. Set only what the row gives. Export the cube with the usual settings to
`tests/golden/<deformer>/<case>/mesh.obj`.

`case.toml`:
- `feature = "<deformer>"`, as in the folder name: `squash-stretch`, `wrap`, `ffd`,
  `formula`, `melt`.
- `observation = "docs/parity/deformers-2.md"`.
- `[params]` holds **every** deformer parameter under our keys.
- `[params.input]` holds the cube's params, as the bend cases do.

## Parameters (fill once per deformer)

For each deformer, list the Object tab as shown: label, our key, default, min and max.

### Squash & Stretch
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Wrap
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### FFD
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Formula
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Melt
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Deformer | Changed from default | Notes |
|---|---|---|---|
| squash-stretch/defaults | Squash & Stretch | – | |
| squash-stretch/factor | Squash & Stretch | factor 50 % | |
| squash-stretch/factor-big | Squash & Stretch | factor 150 % | |
| squash-stretch/bulge | Squash & Stretch | factor 50 %, bulge 100 % (or the closest setting) | |
| wrap/defaults | Wrap | – | |
| wrap/cylindrical | Wrap | type Cylindrical (or the second option) | |
| wrap/angles | Wrap | width angle 180°, height 300 | |
| ffd/defaults | FFD | – | the undeformed grid: does the cube change at all? |
| ffd/point-moved | FFD | default grid; move the top-front-right grid point by (0, 100, 0), done through the script | record the grid point index you moved |
| formula/defaults | Formula | – | at frame 0; also note the formula text shown |
| formula/frame-10 | Formula | – | the document time set to frame 10 at 30 fps |
| melt/defaults | Melt | – | |
| melt/strength | Melt | strength 50 % | |

## Notes
TODO
