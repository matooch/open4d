# Phased plan (tick boxes only after parity test passes)
0. [x] Workspace, CI, parity-harness, golden-data layout, CLEANROOM checks (CI fails on forbidden strings/trademarks).
1. [ ] core: scene graph, param descriptors, undo, serialization, dirty/cache system, plugin registry.
2. [ ] geometry: mesh/spline types, subdivision, normals, selections, booleans.
3. [ ] viewport (wgpu): navigation, shading modes, selection, gizmos, snapping, grid.
4. [ ] ui: docking, Object/Attribute/Material/Coordinate/Timeline managers, layouts, modern dark theme.
5. [ ] generators: all primitives + spline primitives (docs/specs/GENERATORS.md).
6. [ ] generators II: Extrude/Lathe/Loft/Sweep/SDS/Boole/Symmetry/Array/Connect/Instance/Metaball etc.
7. [ ] modifiers: all deformers (docs/specs/MODIFIERS.md) with falloff.
8. [ ] fields system, then MoGraph: Cloner, Matrix, effectors, Fracture/Voronoi, MoSpline/MoText/Tracer/MoInstance (docs/specs/MOGRAPH.md).
9. [ ] materials: node-based PBR + legacy-channel-style material, shaders (noise, gradient, fresnel, etc.), UV tools, texture tags.
10. [ ] render-cpu path tracer + AOVs + IPR; then render-gpu (wgpu).
11. [ ] animation: keyframes, F-curve editor, tags/constraints, node-based expressions, dynamics (rapier), cache.
12. [ ] Redshift track (docs/REDSHIFT.md): spike A/B/C decision.
13. [ ] io: glTF/OBJ/FBX/Alembic/USD; importer for reference-app scenes ONLY via officially documented/exported formats.
14. [ ] scripting (Rhai/Python), AppImage/Flatpak packaging, docs.
