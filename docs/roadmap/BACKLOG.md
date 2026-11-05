# Backlog: every implementation brief

Implementers take the **lowest-numbered `ready` row whose dependencies are all `done`**
([../agents/PIPELINE.md](../agents/PIPELINE.md) §3). Set the row to `in-progress (<who>)` when you start.
The reviewer flips `waiting: Oxx` to `ready` once those observations are reviewed. Briefs 01–10
are done (see [../PLAN.md](../PLAN.md)). "Touches" lists the crates a brief changes, so parallel
implementers can avoid collisions.

| # | Brief | Area | Depends on | Status | Touches |
|---|---|---|---|---|---|
| 011 | [Command registry and migration of the editor commands](../agents/tasks/011-command-registry-and-migration-of-the-editor-commands.md) | A | – | done | command (new), ui, core |
| 012 | [Headless application core `forge-app`](../agents/tasks/012-headless-application-core-forge-app.md) | A | 011 | done | app (new), ui |
| 013 | [`forge` command-line tool](../agents/tasks/013-forge-command-line-tool.md) | A | 012 | done | apps/forge (new), app |
| 014 | [JSON-RPC server in the editor, plus a Rust client](../agents/tasks/014-json-rpc-server-in-the-editor-plus-a-rust-client.md) | A | 012 | done | rpc (new), ui, app |
| 015 | [MCP server `forge-mcp`](../agents/tasks/015-mcp-server-forge-mcp.md) | A | 014 | done | mcp (new) |
| 016 | [Rhai scripting: engine, bindings, `forge run`, Script Manager panel](../agents/tasks/016-rhai-scripting-engine-bindings-forge-run-script-manager-pane.md) | A | 012 | done | script (new), apps/forge, ui |
| 017 | [Python client `forge-py`](../agents/tasks/017-python-client-forge-py.md) | A | 014 | done | python/forge_py (new) |
| 018 | [RPC notifications, viewport screenshot, progress for long jobs](../agents/tasks/018-rpc-notifications-viewport-screenshot-progress-for-long-jobs.md) | A | 014 | done | rpc, ui |
| 019 | [Automation docs, generated command reference, S1/S2 acceptance tests](../agents/tasks/019-automation-docs-generated-command-reference-s1-s2-acceptance.md) | A | 013, 015, 016 | done (S1 partly pending: 065/069/164/136) | docs, apps/forge |
| 020 | [Tool framework + Extrude and Extrude Inner](../agents/tasks/020-tool-framework-extrude-inset.md) | B | 011 | done | modeling (new), ui, geometry |
| 021 | [Bevel tool (all modes)](../agents/tasks/021-bevel.md) | B | 020 | done | modeling, ui |
| 022 | [Mesh commands: Subdivide, Triangulate, Untriangulate, Reverse, Optimize, Weld, Melt, Dissolve, Split, Disconnect, Connect, Mirror, Center Axis](../agents/tasks/022-mesh-commands.md) | B | 020 | done | modeling, ui |
| 023 | [Selection commands: grow, shrink, convert, invert, connected, loop, ring](../agents/tasks/023-selection-commands.md) | B | 011 | done | geometry, ui |
| 024 | [Interactive cutting: Loop/Path Cut, Knife (line, plane, loop), Slide](../agents/tasks/024-interactive-cutting.md) | B | 020, 023 | done (parity-pending: O58, G3 item 7) | modeling, ui, viewport |
| 025 | [Bridge, Stitch and Sew, Close Polygon Hole, Polygon Pen](../agents/tasks/025-bridge-stitch-close-pen.md) | B | 020 | done | modeling, ui |
| 026 | [Soft selection and symmetry modeling](../agents/tasks/026-soft-selection-symmetry.md) | B | 023 | done (parity-pending: G3 item 6) | ui, modeling |
| 027 | [Selection tags, vertex map tags, restriction tags](../agents/tasks/027-selection-vertexmap-restriction-tags.md) | B | 023 | waiting: O54 | core, generators, ui, modifiers |
| 028 | [Editable spline object and point/tangent editing](../agents/tasks/028-editable-spline.md) | B | 003 | ready | core, geometry, generators, ui |
| 029 | [Spline tools and spline booleans](../agents/tasks/029-spline-tools.md) | B | 028 | waiting: 028 | geometry, modeling |
| 030 | [Make Editable follow-ups from O3](../agents/tasks/030-make-editable-followups.md) | B | 003, 028 | ready | generators, ui |
| 031 | [Duplicate, Arrange, Randomize, Transfer, Array command](../agents/tasks/031-duplicate-arrange-array-command.md) | B | 011 | done (parity-pending: no data; array-command probe, G3) | ui, modeling |
| 032 | [Snapping 2.0 and axis/pivot editing](../agents/tasks/032-snapping-pivot.md) | B | 006 | done (parity-pending G2/G3) | viewport, ui, modeling, generators, command |
| 033 | [N-gon handling: reference triangulation order and edge visibility](../agents/tasks/033-ngon-triangulation-order.md) | B | – | done | geometry, generators, viewport |
| 034 | [Multi-object component editing; numeric component coordinates](../agents/tasks/034-multi-object-component-editing.md) | B | 006 | done (parity-pending G3) | ui, app, command |
| 035 | [Platonic, Oil Tank, Polygon and Figure primitives](../agents/tasks/035-platonic-oiltank-polygon-figure.md) | C | 033 | done | generators |
| 036 | [Landscape primitive](../agents/tasks/036-landscape.md) | C | 063 | done | generators, noise |
| 037 | [Spline primitives II](../agents/tasks/037-spline-primitives-2.md) | C | 038 | done | generators |
| 038 | [Intermediate Points modes and R25 per-spline defaults](../agents/tasks/038-intermediate-points.md) | C | – | done | generators, geometry |
| 039 | [Text spline (fonts)](../agents/tasks/039-text-spline.md) | C | 038 | ready | generators, text (new) |
| 040 | [Metaball generator](../agents/tasks/040-metaball.md) | C | 083 | waiting: 083 | generators, volume |
| 041 | [Atom Array and Spline Mask](../agents/tasks/041-atom-array-spline-mask.md) | C | 029 | waiting: 029 | generators |
| 042 | [Sweep rail/banking/scale, Loft linear/multi, Lathe partial/movement, cap fixes](../agents/tasks/042-sweep-loft-lathe-r25.md) | C | – | done (Lathe done; Loft/Sweep R25 cases parity-pending: R20/R25 conflict, helix route) | generators |
| 043 | [Boole R25 modes and numbering, Subdivision types, Symmetry planes](../agents/tasks/043-boole-subdivision-symmetry-r25.md) | C | – | done (Boole R25 strict numbering parity-pending) | generators, geometry |
| 044 | [Shared Caps/Bevel tabs and Extrude R25 options](../agents/tasks/044-caps-bevel-tab.md) | C | 042 | waiting: 042 | generators |
| 045 | [Extrude R25 vertex order](../agents/tasks/045-extrude-r25-order.md) | C | 044 | ready | generators |
| 046 | [Null display, Instance render modes, Connect weld options](../agents/tasks/046-null-instance-connect-options.md) | C | – | ready | generators, viewport |
| 047 | [Camera, Light, Environment, Floor and Sky as scene objects](../agents/tasks/047-scene-objects-camera-light.md) | C | – | waiting: O29 | generators, core, viewport |
| 048 | [Generators as clone and volume inputs (consumed-input rules)](../agents/tasks/048-generators-in-clone-volume-contexts.md) | C | 065, 081 | ready | generators |
| 049 | [Create menu organisation](../agents/tasks/049-create-menu-organisation.md) | C | 148 | ready | ui, core |
| 050 | [Bend 3-axis gating fix + EvalCtx through evaluation](../agents/tasks/050-bend-gating-evalctx.md) | D | – | done | modifiers, generators, core |
| 051 | [Squash & Stretch and Wrap deformers](../agents/tasks/051-squash-stretch-wrap.md) | D | 050 | done (11/11 level A; Center, Aspect, Smooth, Movement, Scale Z, Tension parity-pending, see MODIFIERS.md §7) | modifiers |
| 052 | [Expression library; FFD, Formula and Melt deformers](../agents/tasks/052-expr-ffd-formula-melt.md) | D | 050, 063 | ready | expr (new), modifiers |
| 053 | [Explosion, Shatter, Wind deformers](../agents/tasks/053-explosion-shatter-wind.md) | D | 050 | ready | modifiers |
| 054 | [Smoothing deformer, Displacer, Polygon Reduction, Bevel deformer](../agents/tasks/054-smoothing-displacer-reduction-bevel.md) | D | 050, 125 | waiting: 125 | modifiers, modeling |
| 055 | [Spline Wrap, Spline and Spline Rail deformers](../agents/tasks/055-spline-deformers.md) | D | 050, 028 | waiting: 028 | modifiers |
| 056 | [Mesh (cage), Surface, Camera, Collision, Jiggle, Delta Mush, Correction deformers](../agents/tasks/056-mesh-surface-camera-collision-jiggle-deltamush-correction.md) | D | 050 | ready | modifiers |
| 057 | [Fields framework + Linear, Spherical, Box + Remapping](../agents/tasks/057-fields-framework.md) | D | 050 | done (27/27 samples; remap order and Step/Quadratic parity-pending, see FIELDS.md F5) | fields |
| 058 | [Field types II: Cylinder, Cone, Torus, Capsule, Radial, Random](../agents/tasks/058-field-types-2.md) | D | 057, 063 | done | fields, noise |
| 059 | [Field lists: layers, blend modes, groups, Formula and Shader fields](../agents/tasks/059-field-lists.md) | D | 057, 052 | ready | fields |
| 060 | [Field modifier layers: Freeze, Decay, Delay, Remap, Curve, Colorize, Step](../agents/tasks/060-field-modifier-layers.md) | D | 059 | waiting: O49 | fields |
| 061 | [Deformer falloff through field lists (all deformers)](../agents/tasks/061-deformer-falloff.md) | D | 057 | done (9/9 O24 cases at level A; object off the origin and weights outside 0..1 parity-pending, see MODIFIERS.md §10) | modifiers, fields |
| 062 | [Vertex maps from fields; restriction](../agents/tasks/062-vertex-maps-from-fields.md) | D | 027, 059 | waiting: O49 | fields, modifiers |
| 063 | [Noise library `forge-noise`](../agents/tasks/063-noise-library.md) | D | – | done (parity-pending O42) | noise (new) |
| 064 | [Field list widget and field display](../agents/tasks/064-field-list-widget.md) | D | 059, 150 | ready | ui, viewport |
| 065 | [Clone data, cloner node, Linear / Radial / Grid / Honeycomb modes, instance modes](../agents/tasks/065-cloner-core-modes.md) | E | 050 | done (19/19 samples and meshes; rules parity-pending, see MOGRAPH.md C11) | clone-engine, plugin |
| 066 | [Object-mode distributions with seeds and selections](../agents/tasks/066-cloner-object-mode.md) | E | 065, 027 | ready | clone-engine |
| 067 | [Clone order (iterate, random, blend, sort), multiple children, nesting](../agents/tasks/067-clone-order-nesting.md) | E | 065 | waiting: O50 | clone-engine |
| 068 | [Effector framework + Plain effector (modes, spaces, colour, weight, min/max)](../agents/tasks/068-effector-framework-plain.md) | E | 065, 057 | done (14/14 Plain cases at level B and A; rules parity-pending, see MOGRAPH.md E10) | clone-engine, core |
| 069 | [Random effector (all modes)](../agents/tasks/069-random-effector.md) | E | 068, 063 | done (exact: counts/visibility/untouched channels 8/8; random values statistical, reference sequence not reproduced) | clone-engine, noise |
| 070 | [Step, Formula, Time, Delay and Target effectors](../agents/tasks/070-step-formula-time-delay-target.md) | E | 068, 052 | ready | clone-engine |
| 071 | [Shader, Spline, Push Apart, Inheritance, Volume and Group effectors](../agents/tasks/071-shader-spline-pushapart-etc.md) | E | 068, 125 | ready | clone-engine |
| 072 | [Effector falloff via field lists](../agents/tasks/072-effector-field-falloff.md) | E | 068, 059 | ready | clone-engine, fields |
| 073 | [Matrix object](../agents/tasks/073-matrix-object.md) | E | 065 | waiting: O31 | clone-engine |
| 074 | [Fracture and Voronoi Fracture](../agents/tasks/074-fracture-voronoi.md) | E | 065, 033 | waiting: O31 | clone-engine, geometry |
| 075 | [Clone-toolset Text](../agents/tasks/075-clone-text.md) | E | 039, 065 | waiting: O31, O48 | clone-engine, generators |
| 076 | [Tracer and the clone-toolset spline generator](../agents/tasks/076-tracer-spline-generator.md) | E | 065, 028 | waiting: O31, O50 | clone-engine, generators |
| 077 | [Instance-over-time generator](../agents/tasks/077-instance-over-time.md) | E | 065, 088 | waiting: O50 | clone-engine, anim |
| 078 | [Clone selection and weight tags, clone cache](../agents/tasks/078-clone-tags-cache.md) | E | 065, 027 | waiting: O50 | clone-engine |
| 079 | [Clone UI: effector lists, clone display, GPU instancing hookup](../agents/tasks/079-clone-ui.md) | E | 065, 144 | done (parity-pending G4) | ui, viewport |
| 080 | [forge-volume: sparse grid, mesh/spline/points to SDF, CSG; harness compare_volume_mesh](../agents/tasks/080-forge-volume-grid-sdf.md) | F | – | waiting: O32 | volume (new), parity-harness |
| 081 | [Volume Builder node (object list, modes, SDF/fog, voxel size, hierarchy)](../agents/tasks/081-volume-builder.md) | F | 080 | waiting: O32, O33 | volume, plugin |
| 082 | [Volume filters (smooth types, dilate/erode, reshape, curve)](../agents/tasks/082-volume-filters.md) | F | 081 | waiting: O33 | volume |
| 083 | [Volume Mesher (dual contouring, adaptivity, threshold, normals)](../agents/tasks/083-volume-mesher.md) | F | 081 | waiting: O32 | volume, geometry |
| 084 | [Volume viewport display (preview iso-surface, slices)](../agents/tasks/084-volume-viewport.md) | F | 083, 143 | ready | viewport |
| 085 | [Field layers inside the volume builder](../agents/tasks/085-volume-field-layers.md) | F | 081, 059 | waiting: O33 | volume, fields |
| 086 | [VDB file read/write](../agents/tasks/086-vdb-io.md) | F | 080 | waiting: O33 | volume, io |
| 087 | [Keyframe rework: O16 rules (millisecond rounding at other fps; tangent at uneven monotone keys)](../agents/tasks/087-keyframe-rework-o16-rules-millisecond-rounding-at-other-fps.md) | G | – | done | anim |
| 088 | [Tracks for all value types + point-level animation](../agents/tasks/088-tracks-all-value-types.md) | G | 087 | waiting: O53 | core, anim |
| 089 | [Tangent model: user, broken, weighted, zero angle/length, ease presets, overshoot](../agents/tasks/089-tangent-model.md) | G | 087 | waiting: O38 | core, anim |
| 090 | [Dope sheet manager](../agents/tasks/090-dope-sheet.md) | G | 089, 011 | ready | ui |
| 091 | [Value graph (F-curve) manager](../agents/tasks/091-value-graph.md) | G | 089, 090 | waiting: O38 | ui |
| 092 | [Speed graph mode and speed editing](../agents/tasks/092-speed-graph.md) | G | 091 | ready | ui, anim |
| 093 | [Track before/after behaviours, key reduction, bake](../agents/tasks/093-track-behaviours-bake.md) | G | 089 | waiting: O27, O38 | anim |
| 094 | [Time: markers, preview range, playback modes, fps conversion, time display](../agents/tasks/094-time-markers-playback.md) | G | 010 | done (parity-pending G5) | core, ui |
| 095 | [Motion paths and onion skin](../agents/tasks/095-motion-paths-onion.md) | G | 010, 143 | ready | viewport |
| 096 | [Rotation: Euler filter, quaternion interpolation option](../agents/tasks/096-rotation-euler-quat.md) | G | 089 | waiting: O27, O53 | anim |
| 097 | [Keyframe indicators on every parameter; key context menu](../agents/tasks/097-keyframe-indicators.md) | G | 088, 150 | ready | ui |
| 098 | [Animated evaluation performance (dependency-aware re-evaluation)](../agents/tasks/098-animated-eval-performance.md) | G | 180 | done | core, anim |
| 099 | [S7 acceptance test and animation docs](../agents/tasks/099-s7-animation-acceptance.md) | G | 090–096 | ready | tests, docs |
| 100 | [Joint object, bone display, joint tools (align, mirror, chain creation)](../agents/tasks/100-joints.md) | H | 011 | waiting: O34 | rig (new), plugin, viewport |
| 101 | [Skin deformer + Weight tag + bind pose (LBS, DQS option)](../agents/tasks/101-skin-weights.md) | H | 100, 050 | waiting: O34 | rig, modifiers |
| 102 | [Weight tools: paint brush, weight manager, normalise/mirror/smooth/prune](../agents/tasks/102-weight-tools.md) | H | 101, 150 | ready | rig, ui, viewport |
| 103 | [Auto weights (bone heat)](../agents/tasks/103-auto-weights.md) | H | 101 | waiting: O34 | rig |
| 104 | [IK tag: 2-bone, chain, pole, twist, IK/FK blend](../agents/tasks/104-ik.md) | H | 100 | waiting: O35 | rig |
| 105 | [Constraint tag: PSR, Aim, Parent, Clamp](../agents/tasks/105-constraints.md) | H | 100 | waiting: O36 | rig |
| 106 | [Tags: Target, Align to Spline, Vibrate, Look at Camera, Protection, Display, Compositing](../agents/tasks/106-expression-tags.md) | H | 088 | waiting: O27, O54 | core, rig |
| 107 | [Pose Morph tag](../agents/tasks/107-pose-morph.md) | H | 088 | waiting: O37 | rig |
| 108 | [Node expression engine forge-nodes](../agents/tasks/108-forge-nodes.md) | H | 016 | done | nodes (new) |
| 109 | [Node expression editor UI](../agents/tasks/109-node-editor-ui.md) | H | 108, 150 | done (parity-pending G6; follow-up: node materials reuse the canvas, brief 124) | ui, nodes |
| 110 | [User data on any node](../agents/tasks/110-user-data.md) | H | 011, 150 | waiting: O54 | core, ui |
| 111 | [Evaluation order: tag priorities, expressions before generators](../agents/tasks/111-evaluation-order.md) | H | 104, 105, 108 | waiting: O36 | core |
| 112 | [Weight and joint display modes in the viewport](../agents/tasks/112-rig-display.md) | H | 101, 143 | ready | viewport |
| 113 | [Rig mirroring (joints and weights, left/right naming)](../agents/tasks/113-rig-mirroring.md) | H | 101 | ready | rig |
| 114 | [S6 acceptance test and rigging docs](../agents/tasks/114-s6-rigging-acceptance.md) | H | 100–113 | ready | tests, docs |
| 115 | [rapier integration: rigid body tag, shapes, scene settings, per-frame cache, scrubbing; compare_trajectory](../agents/tasks/115-rigid-bodies.md) | I | 050, 010 | waiting: O39 | dynamics (new), parity-harness |
| 116 | [Dynamics on clones](../agents/tasks/116-dynamics-on-clones.md) | I | 115, 065 | waiting: O39 | dynamics, clone-engine |
| 117 | [Connectors and springs](../agents/tasks/117-connectors-springs.md) | I | 115 | waiting: O41 | dynamics |
| 118 | [Soft bodies (PBD)](../agents/tasks/118-soft-bodies.md) | I | 115 | waiting: O41 | dynamics |
| 119 | [Particle emitter and particle data](../agents/tasks/119-particle-emitter.md) | I | 050 | waiting: O40 | dynamics |
| 120 | [Forces: gravity, wind, attractor, deflector, friction, turbulence, rotation](../agents/tasks/120-forces.md) | I | 119, 115, 063 | waiting: O40 | dynamics |
| 121 | [Particles in the clone toolset and volumes](../agents/tasks/121-particles-in-clones-volumes.md) | I | 119, 076, 081 | ready | dynamics, clone-engine, volume |
| 122 | [Bake to keys and cache tags; dynamics UI](../agents/tasks/122-dynamics-bake-ui.md) | I | 115, 093 | ready | dynamics, ui |
| 123 | [Material model: Standard material + material tag + IR compile](../agents/tasks/123-material-model.md) | J | – | waiting: O29 | material (new), plugin |
| 124 | [Node material graph + node editor UI](../agents/tasks/124-node-material.md) | J | 123, 109 | ready | material, ui |
| 125 | [Shaders I: Noise, Gradient, Color, Fresnel, Falloff, Layer, Colorizer, Bitmap](../agents/tasks/125-shaders-1.md) | J | 063, 123 | waiting: O42 | shaders (new) |
| 126 | [Shaders II: Tiles, Checkerboard, Brick, Fusion, Filter, Normalizer, Vertex Map, Field, AO](../agents/tasks/126-shaders-2.md) | J | 125 | waiting: O42 | shaders |
| 127 | [Texture tag projections + Generate UVW](../agents/tasks/127-texture-projections.md) | J | 123, 128 | waiting: O28 | material, geometry |
| 128 | [Corner UVs on PolyMesh; primitive UV parity](../agents/tasks/128-primitive-uvs.md) | J | – | waiting: O28 | geometry, generators, io |
| 129 | [UV editor (view, transform, relax LSCM, pack, seams)](../agents/tasks/129-uv-editor.md) | J | 128, 150 | ready | ui, geometry |
| 130 | [Image IO, texture cache, colour management](../agents/tasks/130-image-io-colour.md) | J | – | done | image (new) |
| 131 | [Material manager UI with preview renders](../agents/tasks/131-material-manager.md) | J | 123, 136 | ready | ui |
| 132 | [Viewport material display (PBR preview, textures)](../agents/tasks/132-viewport-materials.md) | J | 123, 143 | ready | viewport |
| 133 | [Environment: sky, HDRI, background, floor](../agents/tasks/133-environment.md) | J | 130, 137 | waiting: O29 | render-api, generators |
| 134 | [Material exchange mapping (glTF PBR to node material)](../agents/tasks/134-material-exchange-mapping.md) | J | 124, 164 | ready | io, material |
| 135 | [Render API: snapshot, settings, outputs, AOV model, progress; harness compare_image](../agents/tasks/135-render-api.md) | K | 123 | ready | render-api, parity-harness |
| 136 | [CPU path tracer core (BVH, instancing, BSDFs from the IR, MIS)](../agents/tasks/136-cpu-path-tracer.md) | K | 135 | waiting: O30 | render-cpu |
| 137 | [Lights (omni, spot, infinite, area, sky) and shadows](../agents/tasks/137-lights-shadows.md) | K | 136 | waiting: O30, O29 | render-cpu, generators |
| 138 | [Physical camera: projection, sensor, ortho, DOF, motion blur](../agents/tasks/138-physical-camera.md) | K | 136, 047 | waiting: O57, O29 | render-cpu, core |
| 139 | [Render settings, image sequences, formats, multipass/AOVs](../agents/tasks/139-render-settings-outputs.md) | K | 136, 130 | waiting: O29 | render-api, render-cpu |
| 140 | [Picture viewer and render queue](../agents/tasks/140-picture-viewer-queue.md) | K | 139 | ready | ui |
| 141 | [IPR (interactive render)](../agents/tasks/141-ipr.md) | K | 136 | ready | render-cpu, ui |
| 142 | [GPU path tracer (wgpu compute)](../agents/tasks/142-gpu-path-tracer.md) | K | 136 | waiting: O30 | render-gpu |
| 143 | [Viewport upgrades: lights, shadows, AA, display modes, 4-view layout](../agents/tasks/143-viewport-upgrades.md) | K | – | done (parity-pending G8) | viewport |
| 144 | [Viewport performance: GPU instancing, LOD, large meshes](../agents/tasks/144-viewport-performance.md) | K | 143 | done (instancing, upload cache, chunked upload, culling, LOD hook; benchmarks in VIEWPORT.md §10.10) | viewport |
| 145 | [Render parity tests against O30/O43/O57](../agents/tasks/145-render-parity-tests.md) | K | 137, 138 | waiting: O30, O43, O57 | tests |
| 146 | [S8 acceptance test and rendering docs](../agents/tasks/146-s8-rendering-acceptance.md) | K | 139, 145 | ready | tests, docs |
| 147 | [Familiar default layout: command bar, tool palette, manager arrangement](../agents/tasks/147-default-layout.md) | L | 011 | done (parity-pending G1) | ui |
| 148 | [Icon set and icon atlas](../agents/tasks/148-icon-set.md) | L | 147 | done (parity-pending G1; icons not yet drawn in the layout buttons) | ui |
| 149 | [Object manager parity: columns, dots, tags, drag/drop, search, context menus](../agents/tasks/149-object-manager.md) | L | 147 | done (parity-pending G1; follow-up: render bridge honours render visibility, layer colours) | ui |
| 150 | [Attribute manager parity: tabs, widgets, multi-edit, keyframe dots, history](../agents/tasks/150-attribute-manager.md) | L | 147 | done (parity-pending G4; follow-up: lock/history, tool panel on shared widgets) | ui |
| 151 | [Coordinates manager modes](../agents/tasks/151-coordinates-manager.md) | L | 150 | done (parity-pending G1) | ui |
| 152 | [Key map: defaults, customisation editor](../agents/tasks/152-key-map.md) | L | 011 | done (parity-pending G2) | ui |
| 153 | [Viewport navigation parity (keys, mouse, frame commands, views)](../agents/tasks/153-viewport-navigation.md) | L | 152 | done (parity-pending G2) | viewport, ui |
| 154 | [Content browser (own presets) and recent files](../agents/tasks/154-content-browser.md) | L | 163 | done (object presets only; materials kind waits for material nodes, area J) | ui |
| 155 | [Preferences window and settings storage](../agents/tasks/155-preferences.md) | L | 175 | waiting: O29 | ui |
| 156 | [Layout save/load and workspaces](../agents/tasks/156-layouts-workspaces.md) | L | 147 | done (parity-pending G1) | ui |
| 157 | [HUD and viewport overlays](../agents/tasks/157-hud-overlays.md) | L | 143 | done (parity-pending G1/G8; follow-up: a scene render size for the safe frames) | viewport |
| 158 | [Layer manager](../agents/tasks/158-layer-manager.md) | L | 149 | done (parity-pending G1/G8; follow-up: render bridge honours the render flag) | ui, core |
| 159 | [Command palette and undo history panel](../agents/tasks/159-command-palette-history.md) | L | 011 | done | ui |
| 160 | [Console panel (log + Rhai REPL)](../agents/tasks/160-console.md) | L | 016 | done | ui |
| 161 | [HiDPI, IME, themes, keyboard navigation](../agents/tasks/161-hidpi-ime-themes.md) | L | 147 | done | ui |
| 162 | [S3 acceptance test and UI docs](../agents/tasks/162-s3-ui-acceptance.md) | L | 147–153 | ready | tests, docs |
| 163 | [Native format: migrations, binary encoding, external references](../agents/tasks/163-native-format.md) | M | – | done (format version 6; follow-up: a file-path param on the first texture/cache node, compression wrapper) | core, io |
| 164 | [glTF import/export (meshes, hierarchy, materials, animation, skins, morphs)](../agents/tasks/164-gltf.md) | M | 088, 101, 107 | waiting: O44 | io |
| 165 | [FBX import (ufbx)](../agents/tasks/165-fbx-import.md) | M | 088, 101 | waiting: O44 | io |
| 166 | [FBX export (ASCII 7.4)](../agents/tasks/166-fbx-export.md) | M | 165 | waiting: O44 | io |
| 167 | [Alembic (Ogawa) import/export](../agents/tasks/167-alembic.md) | M | 088 | waiting: O44 | io |
| 168 | [USD .usda import/export](../agents/tasks/168-usd.md) | M | 123 | waiting: O44 | io |
| 169 | [STL and PLY](../agents/tasks/169-stl-ply.md) | M | – | waiting: O44 | io |
| 170 | [Camera, light and material exchange across formats](../agents/tasks/170-camera-light-material-exchange.md) | M | 047, 134 | waiting: O44 | io |
| 171 | [Exchange parity tests (S9)](../agents/tasks/171-s9-exchange-tests.md) | M | 164–169 | waiting: O44 | tests |
| 172 | [Import/export dialogs and CLI batch export](../agents/tasks/172-io-dialogs-batch.md) | M | 013, 150 | done (OBJ only; dialog layout parity-pending) | ui, io |
| 173 | [CI matrix: Linux, macOS, Windows; per-OS GPU policy](../agents/tasks/173-ci-matrix.md) | N | – | done | .github |
| 174 | [Toolchain pin and cargo-deny](../agents/tasks/174-toolchain-cargo-deny.md) | N | – | done | root |
| 175 | [Cross-platform settings, paths, fonts, file dialogs, modifiers](../agents/tasks/175-cross-platform-basics.md) | N | 173 | done | core, ui |
| 176 | [Release workflow: AppImage, dmg, zip/MSI, checksums, changelog](../agents/tasks/176-release-workflow.md) | N | 173 | done (pending first tagged run) | .github |
| 177 | [Signing and notarization hooks](../agents/tasks/177-signing-hooks.md) | N | 176 | ready | .github |
| 178 | [Crash handler and logging](../agents/tasks/178-crash-logging.md) | N | 175 | done | app |
| 179 | [Autosave and recovery](../agents/tasks/179-autosave-recovery.md) | N | 175, 163 | done | app |
| 180 | [Dependency-graph evaluation scheduler (parallel, dirty-only)](../agents/tasks/180-eval-scheduler.md) | N | 012 | done | core |
| 181 | [Benchmarks and S10 performance thresholds](../agents/tasks/181-benchmarks-s10.md) | N | 180, 144 | done | benches |
| 182 | [Fuzzing and UI soak test](../agents/tasks/182-fuzz-soak.md) | N | 014, 163 | done (nightly workflow pending first run) | fuzz |
| 183 | [User manual and generated references (mdBook, GitHub Pages)](../agents/tasks/183-manual.md) | N | 019 | done | docs |
| 184 | [Rhai plugin packages](../agents/tasks/184-rhai-plugins.md) | N | 016 | done | plugin |
| 186 | [Plugin host integration: evaluation hook and plugin folders](../agents/tasks/186-plugin-host.md) | N | 184 | done | core, generators, plugin, app, cli |
| 187 | [O58 object-command and selection parity](../agents/tasks/187-o58-object-selection-parity.md) | B | 031, 034 | ready | modeling, command, app |
| 185 | [v1.0 release checklist: S1–S10 on all three OS](../agents/tasks/185-v1-release.md) | N | everything | ready | all |
