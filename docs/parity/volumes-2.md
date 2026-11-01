# Volumes II: filters, sources, fog, fields, VDB — observation (O33)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [volumes.md](volumes.md) (O32): a builder under a mesher, exported to
`tests/golden/volumes/<case>/mesh.obj`, with `observation = "docs/parity/volumes-2.md"`.
Unblocks: F/081, 082, 085, 086.

## Cases

| Case | Inputs | Changes | Notes |
|---|---|---|---|
| smooth-gaussian | a default cube | add a Smooth filter layer above the cube, type Gaussian, defaults | Record the filter's parameter table |
| smooth-mean | a default cube | Smooth filter, type Mean | |
| smooth-median | a default cube | Smooth filter, type Median | |
| smooth-laplacian | a default cube | Smooth filter, type Laplacian Flow (or the closest name) | |
| smooth-strength | a default cube | Gaussian, with its strength or iterations at a non-default value | |
| dilate | a default cube | a Reshape/Dilate filter with a positive offset (e.g. 10) | |
| erode | a default cube | the same filter with a negative offset (−10) | |
| spline-source | a default Helix spline | the spline in the list; record its radius setting | |
| spline-radius | a default Helix spline | spline radius 20 | |
| points-source | a Matrix object (Linear, count 5, step 50) in the list, or a cloner of points | – | If neither works, record why |
| fog-input | a default sphere | input type Fog instead of SDF | The mesher may need a threshold change. Record what was used |
| field-layer | a default cube plus a Spherical field (size 200) as a layer | the field layer's mode at default | |
| vdb-export | a default sphere | **also** save the builder's volume as `.vdb` through the app's VDB export (the Volume Saver object or the export command) to `tests/golden/volumes/vdb-export/volume.vdb` | Record the voxel size and grid name |

The `.vdb` file is binary, which is fine: it is the app's output for our test scene, not an
asset. Keep it under 2 MB. If it is larger, use voxel size 20.

## Notes
TODO
