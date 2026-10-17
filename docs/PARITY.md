# Black-box parity method
For each feature:
1. Build a minimal scene in the reference app by hand (or via its documented scripting API if licensed for it).
2. Export geometry (OBJ/FBX/Alembic) and render stills at fixed settings; save under tests/golden/<feature>/.
3. Record parameter defaults, ranges, and behavior in docs/parity/<feature>.md.
4. Reimplement; `parity-harness` loads the golden scene, builds ours, and diffs: vertex counts, topology, bbox, per-vertex position error <1e-4, matrix transforms, image PSNR for shading.
5. Failing diffs are tracked in docs/parity/STATUS.md.
Screenshots of the reference UI may be used to understand layout and workflow only, not traced for icons or art.
