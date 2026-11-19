# Volumes II: filters, sources, fog, fields, VDB — observation (O33)

Status: observed (11 of 13 cases; `points-source` and `vdb-export` not observed) · App version: R25 25.010 · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes (open4d observer) · Date: 2026-11-19
Method: exactly as [volumes.md](volumes.md) (O32): a builder under a mesher, exported to
`tests/golden/volumes/<case>/mesh.obj`, with `observation = "docs/parity/volumes-2.md"`.
Unblocks: F/081, 082, 085, 086.

## Cases

| Case | Inputs | Changes | Notes |
|---|---|---|---|
| smooth-gaussian | a default cube | add a Smooth filter layer above the cube, type Gaussian, defaults | observed: 2096 points. A fresh Volume Filter reads Type 2, Operator 0, Offset 5.0, Iterations 1, Strength 1.0. |
| smooth-mean | a default cube | Smooth filter, type Mean | observed: 2168 points at Operator 1. |
| smooth-median | a default cube | Smooth filter, type Median | observed: 2096 points at Operator 2. |
| smooth-laplacian | a default cube | Smooth filter, type Laplacian Flow (or the closest name) | observed: 2168 points at Operator 4; the description offers no label for index 4, so the raw index is recorded. |
| smooth-strength | a default cube | Gaussian, with its strength or iterations at a non-default value | observed: 1776 points at Iterations 3. Strength is clamped: writing 5.0 reads back 1.0, so iterations carries the non-default value. |
| dilate | a default cube | a Reshape/Dilate filter with a positive offset (e.g. 10) | observed: 2168 points at Type 1, Offset +10.0. |
| erode | a default cube | the same filter with a negative offset (−10) | observed: 2168 points at Type 1, Offset −10.0; the offset sign flips the result. |
| spline-source | a default Helix spline | the spline in the list; record its radius setting | observed: 3398 points; default helix (start/end radius 200, height 200, end angle 4π, 100 subdivisions). |
| spline-radius | a default Helix spline | spline radius 20 | observed: 516 points; start and end radius 20. |
| points-source | a Matrix object (Linear, count 5, step 50) in the list, or a cloner of points | – | not observed: no scriptable point source. the scripting layer exposes only the Matrix math class, and no cloner or Matrix object type is available in this app build, and the builder description exposes no point-input port. |
| fog-input | a default sphere | input type Fog instead of SDF | observed: 1664 points; Volume Type 1 (Fog) on the builder, mesher left at its default Surface Threshold 0.01. |
| field-layer | a default cube plus a Spherical field (size 200) as a layer | the field layer's mode at default | observed: 2168 points; Spherical field layer as the only field above the cube, Size 200, mode default. |
| vdb-export | a default sphere | **also** save the builder's volume as `.vdb` through the app's VDB export (the Volume Saver object or the export command) to `tests/golden/volumes/vdb-export/volume.vdb` | not observed: no scriptable VDB export. saving through the app's VDB export blocked the app's main thread (its export dialog needs a human); the file was never written and the app stopped answering the bridge afterwards. Voxel size was 10.0 at the time of the attempt. |

The `.vdb` file is binary, which is fine: it is the app's output for our test scene, not an
asset. Keep it under 2 MB. If it is larger, use voxel size 20.

## Notes

- Filter read-back on a freshly created Volume Filter: Type 2, Operator 0, Offset 5.0,
  Iterations 1, Strength 1.0. The description exposes no cycle labels to Python, so the case
  files record the raw indices; the case names follow the UI names in the table above.
- The filter description contains three parameters labelled "Type"; only one of them changes
  the result (it reads 2 for the Smooth group, 1 for the Reshape/dilate-erode group); the other
  two always read 0. The case files record the live one.
- Setting the filter Strength to 5.0 reads back 1.0 and changes nothing (byte-identical export),
  so Smooth strength is effectively clamped; Iterations is the live knob (3 changed the mesh).
- Offset behaves differently per Type: Type 2 with Offset 10.0 is byte-identical to the fresh
  filter at Offset 5.0 (offset ignored in that group), Type 0 with Offset 10.0 changes the mesh,
  and Type 1 with ±10.0 gives the dilate/erode pair.
- A Helix at radius 20 and one at the default 200 both mesh; the spline radius feeds the builder's
  Spline Voxel Range, so the point count drops from 3398 to 516.
- The builder's Volume Type and its voxel thresholds live in the builder's own
  settings container; the per-object mode list the UI shows above the Objects list has
  no scriptable counterpart in this build (reading the Objects list raises "Parameter value not accessible").
- The mesher caches: after changing a builder parameter the previous cache can still be returned,
  so each case re-built the scene from scratch and pumped the document before meshing.
- The VDB path is blocked on a human: saving through the app's VDB export wedges the main
  thread without writing a file, and the scripting layer exposes no Volume Saver or Volume Builder object type in this build.
