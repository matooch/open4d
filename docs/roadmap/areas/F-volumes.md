# F · Volumes: volume builder, filters, mesher, VDB files

## Scope (v1)

- [ ] **Sparse voxel grid** (SDF and fog/density), with world transform and voxel size.
- [ ] **Volume Builder.**
  - Object list: polygon objects, splines (radius), clones and particles (points with
    radius), and other volume builders. Each entry has:
    - a mode: Union, Subtract, Intersect;
    - an input type: SDF or fog;
    - an option to use the object's hierarchy.
  - Voxel size; SDF band width; adaptive range.
  - Filter layers in the object list: Smooth (Gaussian, mean, median, Laplacian flow),
    Dilate/Erode, Reshape (morphological open/close), Curve remap, Fields.
- [ ] **Volume Mesher:**
  - threshold (iso value);
  - adaptivity (polygon reduction);
  - voxel range;
  - smooth shading output (normals).
- [ ] **Volume Loader / Saver:** `.vdb` read and write for float grids.
- [ ] **Viewport display:** a bounding box plus an iso-surface preview while editing
  (fast mesher on a coarse level), or voxel points/slices.
- [ ] **Fields drive volumes:** a field layer in the builder's list adds, subtracts or
  multiplies field values into the grid.

## Observation jobs

| Job | Form | Status |
|---|---|---|
| O32 | [volumes.md](../../parity/volumes.md): builder from cube, sphere and two-object booleans at several voxel sizes, meshed with default and changed thresholds and adaptivity | todo |
| O33 | [volumes-2.md](../../parity/volumes-2.md): filters (smooth types, dilate/erode), spline and point sources, fog vs SDF, field layer, a VDB export reference | todo |

## Implementation design

- **`crates/volume` (`forge-volume`):**
  - **`SparseGrid<T>`:** a hash map from a 3D block key (i32×3, blocks of 8³ voxels) to a
    dense block, a background value, and a transform. Iteration over active voxels;
    `get`/`set`; trilinear sampling.
  - **Mesh → SDF:**
    - for voxels within the band of each triangle (a BVH over the triangles), the exact
      unsigned distance to the closest triangle;
    - the sign comes from the generalized winding number (robust for non-closed meshes),
      already used in `forge-geometry::boolean`;
    - then a fast-sweep fill out to the band width.
  - **Spline → SDF:** distance to polyline segments minus the radius (a capsule chain).
    **Points → SDF:** a union of spheres, smooth-min optional.
  - **CSG on SDFs:** min/max/negation, with a smooth variant (polynomial smooth-min)
    where the reference has a smoothing option.
  - **Filters:** separable Gaussian over active voxels; mean and median windows;
    Laplacian flow (iterated); dilate/erode as an SDF offset followed by re-normalisation
    (a re-distance pass, fast marching).
  - **Mesher:**
    - **dual contouring** on the active band, with QEF solved by SVD (`nalgebra`) and
      quads out, which matches the "mostly quads" look of the reference volume meshes
      (check against O32);
    - fallback: marching cubes;
    - adaptivity: a vertex clustering pass weighted by curvature, or quadric-error edge
      collapse on the result (`forge-modeling`);
    - normals from the SDF gradient.
  - **VDB IO:** read and write the published OpenVDB file format for float grids
    (`RootNode`/`InternalNode` 5-4-3 tree). Our block size is 8³ = the VDB leaf. Test
    against the O33 reference `.vdb` (metadata, transform and active voxel count; values
    within tolerance).
- **Parity:**
  - Volumes are discretisation-sensitive, so exact meshes are unlikely. The comparison is
    **geometric**: Hausdorff distance below voxel size/2, surface area and volume within
    2 %, and the same genus (Euler characteristic).
  - The harness gets `compare_volume_mesh(expected, actual, voxel)` (brief 080) for this.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 080 | `forge-volume`: sparse grid, mesh/spline/points → SDF, CSG; harness `compare_volume_mesh` | – | O32 |
| 081 | Volume Builder node (object list, modes, SDF/fog, voxel size, hierarchy) | 080 | O32, O33 |
| 082 | Volume filters (smooth types, dilate/erode, reshape, curve) | 081 | O33 |
| 083 | Volume Mesher (dual contouring, adaptivity, threshold, normals); shared iso-surface module | 081 | O32 |
| 084 | Volume viewport display (preview iso-surface, slices) | 083, 143 | G4 |
| 085 | Field layers inside the builder | 081, 059 | O33 |
| 086 | VDB file read/write | 080 | O33 |

## Done when
S5 passes. Every O32/O33 case meets the geometric parity thresholds. A VDB file written
by us can be read back by our reader, and our reader reads the reference's VDB.
