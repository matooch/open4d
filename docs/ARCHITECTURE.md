# Architecture
core: scene graph (object tree, tags, materials), param descriptors, undo/redo commands, cache/dirty propagation, animation-ready values.
geometry: poly/n-gon meshes, half-edge, subdivision (Catmull-Clark), splines (linear/cubic/Bezier/B-spline/Akima/Hermite), booleans, normals, UVs, selections.
generators / modifiers (deformers): plugin nodes; generators produce cache geometry, deformers act on point arrays with falloff/strength/field support.
clone-engine: clone engine (cloner, matrix, fracture, tracer, mospline, motext, moinstance), effectors, MoData arrays (position, scale, rotation, color, weight, index, time).
fields: field evaluators (shapes, linear/radial, formula, random, noise, shader, sound, group w/ layers+blend modes), used by effectors, deformers, shaders.
render-api: `trait RenderEngine` (scene snapshot in, progressive tiles out), AOV model, IPR. render-cpu: BVH path tracer. render-gpu: wgpu. render-bridge (external renderer bridges): see docs/REDSHIFT.md.
viewport: wgpu, shading modes, selection, gizmos, snapping. ui: docking shell (egui + egui_dock or Slint), managers, layouts. io: glTF/OBJ/FBX/Alembic/USD + own native format (RON/MsgPack). anim: timeline, F-curves, XPresso-like node graph, dynamics (rapier). plugin: stable registration API + Rhai/Python scripting.

Crates are packaged as `forge-<dir>`; dev tools live in `tools/` (cleanroom gate). Infrastructure spec: docs/specs/INFRA.md.
