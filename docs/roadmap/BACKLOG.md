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
| 027 | [Selection tags, vertex map tags, restriction tags](../agents/tasks/027-selection-vertexmap-restriction-tags.md) | B | 023 | done | core, generators, ui, modifiers |
| 028 | [Editable spline object and point/tangent editing](../agents/tasks/028-editable-spline.md) | B | 003 | done | core, geometry, generators, ui |
| 029 | [Spline tools and spline booleans](../agents/tasks/029-spline-tools.md) | B | 028 | done | geometry, modeling |
| 030 | [Make Editable follow-ups from O3](../agents/tasks/030-make-editable-followups.md) | B | 003, 028 | done | generators, ui |
| 031 | [Duplicate, Arrange, Randomize, Transfer, Array command](../agents/tasks/031-duplicate-arrange-array-command.md) | B | 011 | done (parity-pending: no data; array-command probe, G3) | ui, modeling |
| 032 | [Snapping 2.0 and axis/pivot editing](../agents/tasks/032-snapping-pivot.md) | B | 006 | done (parity-pending G2/G3) | viewport, ui, modeling, generators, command |
| 033 | [N-gon handling: reference triangulation order and edge visibility](../agents/tasks/033-ngon-triangulation-order.md) | B | – | done | geometry, generators, viewport |
| 034 | [Multi-object component editing; numeric component coordinates](../agents/tasks/034-multi-object-component-editing.md) | B | 006 | done (parity-pending G3) | ui, app, command |
| 035 | [Platonic, Oil Tank, Polygon and Figure primitives](../agents/tasks/035-platonic-oiltank-polygon-figure.md) | C | 033 | done | generators |
| 036 | [Landscape primitive](../agents/tasks/036-landscape.md) | C | 063 | done | generators, noise |
| 037 | [Spline primitives II](../agents/tasks/037-spline-primitives-2.md) | C | 038 | done | generators |
| 038 | [Intermediate Points modes and R25 per-spline defaults](../agents/tasks/038-intermediate-points.md) | C | – | done | generators, geometry |
| 039 | [Text spline (fonts)](../agents/tasks/039-text-spline.md) | C | 038 | done (9/9 O48 cases on font-independent invariants; glyph shapes parity-pending until a re-observation with a licensed font) | generators, text (new) |
| 040 | [Metaball generator](../agents/tasks/040-metaball.md) | C | 083 | done | generators, volume |
| 041 | [Atom Array and Spline Mask](../agents/tasks/041-atom-array-spline-mask.md) | C | 029 | done (4/4 O20 cases; other mask modes, axes, oblique atom edges parity-pending) | generators |
| 042 | [Sweep rail/banking/scale, Loft linear/multi, Lathe partial/movement, cap fixes](../agents/tasks/042-sweep-loft-lathe-r25.md) | C | – | done (Lathe done; Loft/Sweep R25 cases parity-pending: R20/R25 conflict, helix route) | generators |
| 043 | [Boole R25 modes and numbering, Subdivision types, Symmetry planes](../agents/tasks/043-boole-subdivision-symmetry-r25.md) | C | – | done (Boole R25 strict numbering parity-pending) | generators, geometry |
| 044 | [Shared Caps/Bevel tabs and Extrude R25 options](../agents/tasks/044-caps-bevel-tab.md) | C | 042 | done (parity-pending: Delaunay/Regular Grid caps, bevels pinned) | generators |
| 045 | [Extrude R25 vertex order](../agents/tasks/045-extrude-r25-order.md) | C | 044 | done (parity-pending: larger outlines) | generators |
| 046 | [Null display, Instance render modes, Connect weld options](../agents/tasks/046-null-instance-connect-options.md) | C | – | waiting: G8 (O20 follow-ups are in) | generators, viewport |
| 047 | [Camera, Light, Environment, Floor and Sky as scene objects](../agents/tasks/047-scene-objects-camera-light.md) | C | – | done (camera and light defaults equal O29 for all 9 light types; Environment/Floor/Sky are our design; stereo, composition helpers, volumetric light params not yet; render bridge and `forge render` use the scene objects; render settings stored in v12) | generators, core, viewport |
| 048 | [Generators as clone and volume inputs (consumed-input rules)](../agents/tasks/048-generators-in-clone-volume-contexts.md) | C | 065, 081 | done | generators |
| 049 | [Create menu organisation](../agents/tasks/049-create-menu-organisation.md) | C | 148 | done | ui, core |
| 050 | [Bend 3-axis gating fix + EvalCtx through evaluation](../agents/tasks/050-bend-gating-evalctx.md) | D | – | done | modifiers, generators, core |
| 051 | [Squash & Stretch and Wrap deformers](../agents/tasks/051-squash-stretch-wrap.md) | D | 050 | done (11/11 level A; Center, Aspect, Smooth, Movement, Scale Z, Tension parity-pending, see MODIFIERS.md §7) | modifiers |
| 052 | [Expression library; FFD, Formula and Melt deformers](../agents/tasks/052-expr-ffd-formula-melt.md) | D | 050, 063 | done (5 exact + 2 statistical of 7; Melt values parity-pending, see MODIFIERS.md §16-19) | expr (new), modifiers |
| 053 | [Explosion, Shatter, Wind deformers](../agents/tasks/053-explosion-shatter-wind.md) | D | 050 | done (2 exact + 3 statistical of 5; random stream and wind noise parity-pending, see MODIFIERS.md §11-13) | modifiers |
| 054 | [Smoothing deformer, Displacer, Polygon Reduction, Bevel deformer](../agents/tasks/054-smoothing-displacer-reduction-bevel.md) | D | 050, 125 | done (parity-pending: O19) | modifiers, modeling |
| 055 | [Spline Wrap, Spline and Spline Rail deformers](../agents/tasks/055-spline-deformers.md) | D | 050, 028 | done (0 exact of 4, all pinned: sampler digits and unresolved helix frame / Spline pairing; see MODIFIERS.md §20-21) | modifiers |
| 056 | [Mesh (cage), Surface, Camera, Collision, Jiggle, Delta Mush, Correction deformers](../agents/tasks/056-mesh-surface-camera-collision-jiggle-deltamush-correction.md) | D | 050 | done (3 exact of 8, one vacuous; 5 pinned: the reference export did not run the solvers; algorithms parity-pending, see MODIFIERS.md §14-15) | modifiers |
| 057 | [Fields framework + Linear, Spherical, Box + Remapping](../agents/tasks/057-fields-framework.md) | D | 050 | done (27/27 samples; remap order and Step/Quadratic parity-pending, see FIELDS.md F5) | fields |
| 058 | [Field types II: Cylinder, Cone, Torus, Capsule, Radial, Random](../agents/tasks/058-field-types-2.md) | D | 057, 063 | done | fields, noise |
| 059 | [Field lists: layers, blend modes, groups, Formula and Shader fields](../agents/tasks/059-field-lists.md) | D | 057, 052 | done (16/17 exact; Shader field Unsupported until 125; native format 11, see FIELDS.md F7) | fields |
| 060 | [Field modifier layers: Freeze, Decay, Delay, Remap, Curve, Colorize, Step](../agents/tasks/060-field-modifier-layers.md) | D | 059 | done (parity-pending: Step, Decay, Delay, Freeze pinned, FIELDS.md F8) | fields |
| 061 | [Deformer falloff through field lists (all deformers)](../agents/tasks/061-deformer-falloff.md) | D | 057 | done (9/9 O24 cases at level A; object off the origin and weights outside 0..1 parity-pending, see MODIFIERS.md §10) | modifiers, fields |
| 062 | [Vertex maps from fields; restriction](../agents/tasks/062-vertex-maps-from-fields.md) | D | 027, 059 | done | fields, modifiers |
| 063 | [Noise library `forge-noise`](../agents/tasks/063-noise-library.md) | D | – | done (parity-pending O42) | noise (new) |
| 064 | [Field list widget and field display](../agents/tasks/064-field-list-widget.md) | D | 059, 150 | done (parity-pending G4: looks, columns) | ui, viewport |
| 065 | [Clone data, cloner node, Linear / Radial / Grid / Honeycomb modes, instance modes](../agents/tasks/065-cloner-core-modes.md) | E | 050 | done (19/19 samples and meshes; rules parity-pending, see MOGRAPH.md C11) | clone-engine, plugin |
| 066 | [Object-mode distributions with seeds and selections](../agents/tasks/066-cloner-object-mode.md) | E | 065, 027 | done (parity-pending: aligned object-vertex and object-polycenter roll pinned) | clone-engine |
| 067 | [Clone order (iterate, random, blend, sort), multiple children, nesting](../agents/tasks/067-clone-order-nesting.md) | E | 065 | done (random order statistical) | clone-engine |
| 068 | [Effector framework + Plain effector (modes, spaces, colour, weight, min/max)](../agents/tasks/068-effector-framework-plain.md) | E | 065, 057 | done (14/14 Plain cases at level B and A; rules parity-pending, see MOGRAPH.md E10) | clone-engine, core |
| 069 | [Random effector (all modes)](../agents/tasks/069-random-effector.md) | E | 068, 063 | done (exact: counts/visibility/untouched channels 8/8; random values statistical, reference sequence not reproduced) | clone-engine, noise |
| 070 | [Step, Formula, Time, Delay and Target effectors](../agents/tasks/070-step-formula-time-delay-target.md) | E | 068, 052 | done (level B 6/6, level A 5/6: `step-position` known mismatch; Delay has no golden case, all its rules parity-pending, see MOGRAPH.md E18-E25) | clone-engine |
| 071 | [Shader, Spline, Push Apart, Inheritance, Volume and Group effectors](../agents/tasks/071-shader-spline-pushapart-etc.md) | E | 068, 125 | done (parity-pending: five O22 pins) | clone-engine |
| 072 | [Effector falloff via field lists](../agents/tasks/072-effector-field-falloff.md) | E | 068, 059 | done (parity-pending: multi-layer lists, no observation) | clone-engine, fields |
| 073 | [Matrix object](../agents/tasks/073-matrix-object.md) | E | 065 | done | clone-engine |
| 074 | [Fracture and Voronoi Fracture](../agents/tasks/074-fracture-voronoi.md) | E | 065, 033 | done (Fracture; Voronoi blocked: not observed) | clone-engine, geometry |
| 075 | [Clone-toolset Text](../agents/tasks/075-clone-text.md) | E | 039, 065 | done (parity-pending: font metrics and outlines) | clone-engine, generators |
| 076 | [Tracer and the clone-toolset spline generator](../agents/tasks/076-tracer-spline-generator.md) | E | 065, 028 | done (Turtle unsupported; tracer positions within 1e-3) | clone-engine, generators |
| 077 | [Instance-over-time generator](../agents/tasks/077-instance-over-time.md) | E | 065, 088 | done (parity-pending: Trail mode) | clone-engine, anim |
| 078 | [Clone selection and weight tags, clone cache](../agents/tasks/078-clone-tags-cache.md) | E | 065, 027 | done (parity-pending: O50 tag and cache captures) | clone-engine |
| 079 | [Clone UI: effector lists, clone display, GPU instancing hookup](../agents/tasks/079-clone-ui.md) | E | 065, 144 | done (parity-pending G4) | ui, viewport |
| 080 | [forge-volume: sparse grid, mesh/spline/points to SDF, CSG; harness compare_volume_mesh](../agents/tasks/080-forge-volume-grid-sdf.md) | F | – | done | volume (new), parity-harness |
| 081 | [Volume Builder node (object list, modes, SDF/fog, voxel size, hierarchy)](../agents/tasks/081-volume-builder.md) | F | 080 | done (parity-pending: spline radius factor, fog density, subtract/intersect not observed) | volume, plugin |
| 082 | [Volume filters (smooth types, dilate/erode, reshape, curve)](../agents/tasks/082-volume-filters.md) | F | 081 | done (parity-pending: 3-iteration Gaussian is a known mismatch; Laplacian flow, offset type and curve remap not observed) volume |
| 083 | [Volume Mesher (dual contouring, adaptivity, threshold, normals)](../agents/tasks/083-volume-mesher.md) | F | 081 | done (parity-pending: adaptive tolerance factor, absolute iso; the mesher is a cell mesher with octree block merge, not dual contouring: see VOLUME.md 7) | volume, geometry |
| 084 | [Volume viewport display (preview iso-surface, slices)](../agents/tasks/084-volume-viewport.md) | F | 083, 143 | done for the logic (preview worker, coarse build, draw lists; parity-pending G4); the editor panel does not call it yet viewport |
| 085 | [Field layers inside the volume builder](../agents/tasks/085-volume-field-layers.md) | F | 081, 059 | done (parity-pending: scale K, threshold 0.5, subtract / multiply modes: one case) volume, fields |
| 086 | [VDB file read/write](../agents/tasks/086-vdb-io.md) | F | 080 | done (parity-pending: layout from the public description, no reference file; vdb-export case needs a human; read/write tested against our own files and the observed .vdb if present) | volume, io |
| 087 | [Keyframe rework: O16 rules (millisecond rounding at other fps; tangent at uneven monotone keys)](../agents/tasks/087-keyframe-rework-o16-rules-millisecond-rounding-at-other-fps.md) | G | – | done | anim |
| 088 | [Tracks for all value types + point-level animation](../agents/tasks/088-tracks-all-value-types.md) | G | 087 | done (parity-pending) | core, anim |
| 089 | [Tangent model: user, broken, weighted, zero angle/length, ease presets, overshoot](../agents/tasks/089-tangent-model.md) | G | 087 | done (parity-pending) | core, anim |
| 090 | [Dope sheet manager](../agents/tasks/090-dope-sheet.md) | G | 089, 011 | done (parity-pending) | ui |
| 091 | [Value graph (F-curve) manager](../agents/tasks/091-value-graph.md) | G | 089, 090 | done (parity-pending) | ui |
| 092 | [Speed graph mode and speed editing](../agents/tasks/092-speed-graph.md) | G | 091 | done (parity-pending: G5) | ui, anim |
| 093 | [Track before/after behaviours, key reduction, bake](../agents/tasks/093-track-behaviours-bake.md) | G | 089 | done | anim |
| 094 | [Time: markers, preview range, playback modes, fps conversion, time display](../agents/tasks/094-time-markers-playback.md) | G | 010 | done (parity-pending G5) | core, ui |
| 095 | [Motion paths and onion skin](../agents/tasks/095-motion-paths-onion.md) | G | 010, 143 | done | viewport |
| 096 | [Rotation: Euler filter, quaternion interpolation option](../agents/tasks/096-rotation-euler-quat.md) | G | 089 | done (parity-pending) | anim |
| 097 | [Keyframe indicators on every parameter; key context menu](../agents/tasks/097-keyframe-indicators.md) | G | 088, 150 | done (parity-pending: G4) | ui |
| 098 | [Animated evaluation performance (dependency-aware re-evaluation)](../agents/tasks/098-animated-eval-performance.md) | G | 180 | done | core, anim |
| 099 | [S7 acceptance test and animation docs](../agents/tasks/099-s7-animation-acceptance.md) | G | 090–096 | done | tests, docs |
| 100 | [Joint object, bone display, joint tools (align, mirror, chain creation)](../agents/tasks/100-joints.md) | H | 011 | done (O34 mesh cases: bind-pose exact, deformed cases pinned to the reference's 1e-3 noise, RIG.md §8; command wrappers not registered, see RIG.md §1.3) | rig (new), plugin, viewport |
| 101 | [Skin deformer + Weight tag + bind pose (LBS, DQS option)](../agents/tasks/101-skin-weights.md) | H | 100, 050 | done (parity-pending: 4 of 5 posed O34 cases differ from the reference by its own noise, RIG.md §8) | rig, modifiers |
| 102 | [Weight tools: paint brush, weight manager, normalise/mirror/smooth/prune](../agents/tasks/102-weight-tools.md) | H | 101, 150 | done (parity-pending G6) | rig, ui, viewport |
| 103 | [Auto weights (bone heat)](../agents/tasks/103-auto-weights.md) | H | 101 | done (parity-pending: no reference auto-weight data, O34 case not observed; level C checked against the nearest bone, RIG.md §4) | rig |
| 104 | [IK tag: 2-bone, chain, pole, twist, IK/FK blend](../agents/tasks/104-ik.md) | H | 100 | done | rig |
| 105 | [Constraint tag: PSR, Aim, Parent, Clamp](../agents/tasks/105-constraints.md) | H | 100 | done | rig |
| 106 | [Tags: Target, Align to Spline, Vibrate, Look at Camera, Protection, Display, Compositing](../agents/tasks/106-expression-tags.md) | H | 088 | done | core, rig |
| 107 | [Pose Morph tag](../agents/tasks/107-pose-morph.md) | H | 088 | waiting: O37 | rig |
| 108 | [Node expression engine forge-nodes](../agents/tasks/108-forge-nodes.md) | H | 016 | done | nodes (new) |
| 109 | [Node expression editor UI](../agents/tasks/109-node-editor-ui.md) | H | 108, 150 | done (parity-pending G6; follow-up: node materials reuse the canvas, brief 124) | ui, nodes |
| 110 | [User data on any node](../agents/tasks/110-user-data.md) | H | 011, 150 | done | core, ui |
| 111 | [Evaluation order: tag priorities, expressions before generators](../agents/tasks/111-evaluation-order.md) | H | 104, 105, 108 | done | core |
| 112 | [Weight and joint display modes in the viewport](../agents/tasks/112-rig-display.md) | H | 101, 143 | done (parity-pending: own design; draw lists and ramp as pure code, weight overlay as markers; VIEWPORT.md §13) | viewport |
| 113 | [Rig mirroring (joints and weights, left/right naming)](../agents/tasks/113-rig-mirroring.md) | H | 101 | done (own design: no observation; symmetric-deformation unit test, RIG.md §5) | rig |
| 114 | [S6 acceptance test and rigging docs](../agents/tasks/114-s6-rigging-acceptance.md) | H | 100–113 | ready | tests, docs |
| 115 | [rapier integration: rigid body tag, shapes, scene settings, per-frame cache, scrubbing; compare_trajectory](../agents/tasks/115-rigid-bodies.md) | I | 050, 010 | done (parity-pending: scene gravity/steps are a fit; bounce and friction combine rules) | dynamics (new), parity-harness |
| 116 | [Dynamics on clones](../agents/tasks/116-dynamics-on-clones.md) | I | 115, 065 | done (parity-pending: clone drift and edge balance are engine-specific) | dynamics, clone-engine |
| 117 | [Connectors and springs](../agents/tasks/117-connectors-springs.md) | I | 115 | done (parity-pending: connector axis choice, spring stiffness scale) | dynamics |
| 118 | [Soft bodies (PBD)](../agents/tasks/118-soft-bodies.md) | I | 115 | done (resting height pinned: the reference sinks through the plane; constants parity-pending) | dynamics |
| 119 | [Particle emitter and particle data](../agents/tasks/119-particle-emitter.md) | I | 050 | waiting: O40 | dynamics |
| 120 | [Forces: gravity, wind, attractor, deflector, friction, turbulence, rotation](../agents/tasks/120-forces.md) | I | 119, 115, 063 | waiting: O40 | dynamics |
| 121 | [Particles in the clone toolset and volumes](../agents/tasks/121-particles-in-clones-volumes.md) | I | 119, 076, 081 | ready | dynamics, clone-engine, volume |
| 122 | [Bake to keys and cache tags; dynamics UI](../agents/tasks/122-dynamics-bake-ui.md) | I | 115, 093 | done | dynamics, ui |
| 123 | [Material model: Standard material + material tag + IR compile](../agents/tasks/123-material-model.md) | J | – | done (8 tests; scene hosting of materials needs a core slot) | material (new), plugin |
| 124 | [Node material graph + node editor UI](../agents/tasks/124-node-material.md) | J | 123, 109 | done (parity-pending: G7) | material, ui |
| 125 | [Shaders I: Noise, Gradient, Color, Fresnel, Falloff, Layer, Colorizer, Bitmap](../agents/tasks/125-shaders-1.md) | J | 063, 123 | done (6/17 exact; noise basis known mismatch) | shaders (new) |
| 126 | [Shaders II: Tiles, Checkerboard, Brick, Fusion, Filter, Normalizer, Vertex Map, Field, AO](../agents/tasks/126-shaders-2.md) | J | 125 | done (checker, tiles exact; brick, field shader unsupported) | shaders |
| 127 | [Texture tag projections + Generate UVW](../agents/tasks/127-texture-projections.md) | J | 123, 128 | done (6/7 exact; frontal known mismatch; no Generate UVW command yet) | material, geometry |
| 128 | [Corner UVs on PolyMesh; primitive UV parity](../agents/tasks/128-primitive-uvs.md) | J | – | done (10/10 primitives per corner; CornerUvs beside PolyMesh, no OBJ vt IO yet) | geometry, generators, io |
| 129 | [UV editor (view, transform, relax LSCM, pack, seams)](../agents/tasks/129-uv-editor.md) | J | 128, 150 | done (parity-pending: G7) | ui, geometry |
| 130 | [Image IO, texture cache, colour management](../agents/tasks/130-image-io-colour.md) | J | – | done | image (new) |
| 131 | [Material manager UI with preview renders](../agents/tasks/131-material-manager.md) | J | 123, 136 | done (parity-pending: G7) | ui |
| 132 | [Viewport material display (PBR preview, textures)](../agents/tasks/132-viewport-materials.md) | J | 123, 143 | done | viewport |
| 133 | [Environment: sky, HDRI, background, floor](../agents/tasks/133-environment.md) | J | 130, 137 | done (HDRI loading, importance sampling: 32.8 dB at 64 spp vs 4096; sky, background and floor objects; no O29 observation of these objects, own design) | render-api, generators |
| 134 | [Material exchange mapping (glTF PBR to node material)](../agents/tasks/134-material-exchange-mapping.md) | J | 124, 164 | done | io, material |
| 135 | [Render API: snapshot, settings, outputs, AOV model, progress; harness compare_image](../agents/tasks/135-render-api.md) | K | 123 | done | render-api, parity-harness |
| 136 | [CPU path tracer core (BVH, instancing, BSDFs from the IR, MIS)](../agents/tasks/136-cpu-path-tracer.md) | K | 135 | done | render-cpu |
| 137 | [Lights (omni, spot, infinite, area, sky) and shadows](../agents/tasks/137-lights-shadows.md) | K | 136 | done (light node types not registered; parity: silhouettes 15/19) | render-cpu, generators |
| 138 | [Physical camera: projection, sensor, ortho, DOF, motion blur](../agents/tasks/138-physical-camera.md) | K | 136, 047 | done (4/4 O57 cases within 1 px; thin-lens DOF own design, no parity data; motion blur, lens distortion, vignetting not done) | render-cpu, core |
| 139 | [Render settings, image sequences, formats, multipass/AOVs](../agents/tasks/139-render-settings-outputs.md) | K | 136, 130 | done (settings by CLI flags; not yet stored in the scene) | render-api, render-cpu |
| 140 | [Picture viewer and render queue](../agents/tasks/140-picture-viewer-queue.md) | K | 139 | done (parity-pending: G8) | ui |
| 141 | [IPR (interactive render)](../agents/tasks/141-ipr.md) | K | 136 | done | render-cpu, ui |
| 142 | [GPU path tracer (wgpu compute)](../agents/tasks/142-gpu-path-tracer.md) | K | 136 | done (CPU vs GPU 18/18 O30 cases at 71-158 dB; 5 shader-node cases unsupported until brief 207) | render-gpu |
| 143 | [Viewport upgrades: lights, shadows, AA, display modes, 4-view layout](../agents/tasks/143-viewport-upgrades.md) | K | – | done (parity-pending G8) | viewport |
| 144 | [Viewport performance: GPU instancing, LOD, large meshes](../agents/tasks/144-viewport-performance.md) | K | 143 | done (instancing, upload cache, chunked upload, culling, LOD hook; benchmarks in VIEWPORT.md §10.10) | viewport |
| 145 | [Render parity tests against O30/O43/O57](../agents/tasks/145-render-parity-tests.md) | K | 137, 138 | done (parity-pending: existing silhouette and lighting gaps) | tests |
| 146 | [S8 acceptance test and rendering docs](../agents/tasks/146-s8-rendering-acceptance.md) | K | 139, 145 | done | tests, docs |
| 147 | [Familiar default layout: command bar, tool palette, manager arrangement](../agents/tasks/147-default-layout.md) | L | 011 | done (parity-pending G1) | ui |
| 148 | [Icon set and icon atlas](../agents/tasks/148-icon-set.md) | L | 147 | done (parity-pending G1; icons not yet drawn in the layout buttons) | ui |
| 149 | [Object manager parity: columns, dots, tags, drag/drop, search, context menus](../agents/tasks/149-object-manager.md) | L | 147 | done (parity-pending G1; follow-up: render bridge honours render visibility, layer colours) | ui |
| 150 | [Attribute manager parity: tabs, widgets, multi-edit, keyframe dots, history](../agents/tasks/150-attribute-manager.md) | L | 147 | done (parity-pending G4; follow-up: lock/history, tool panel on shared widgets) | ui |
| 151 | [Coordinates manager modes](../agents/tasks/151-coordinates-manager.md) | L | 150 | done (parity-pending G1) | ui |
| 152 | [Key map: defaults, customisation editor](../agents/tasks/152-key-map.md) | L | 011 | done (parity-pending G2) | ui |
| 153 | [Viewport navigation parity (keys, mouse, frame commands, views)](../agents/tasks/153-viewport-navigation.md) | L | 152 | done (parity-pending G2) | viewport, ui |
| 154 | [Content browser (own presets) and recent files](../agents/tasks/154-content-browser.md) | L | 163 | done (object presets only; materials kind waits for material nodes, area J) | ui |
| 155 | [Preferences window and settings storage](../agents/tasks/155-preferences.md) | L | 175 | done | ui |
| 156 | [Layout save/load and workspaces](../agents/tasks/156-layouts-workspaces.md) | L | 147 | done (parity-pending G1) | ui |
| 157 | [HUD and viewport overlays](../agents/tasks/157-hud-overlays.md) | L | 143 | done (parity-pending G1/G8; follow-up: a scene render size for the safe frames) | viewport |
| 158 | [Layer manager](../agents/tasks/158-layer-manager.md) | L | 149 | done (parity-pending G1/G8; follow-up: render bridge honours the render flag) | ui, core |
| 159 | [Command palette and undo history panel](../agents/tasks/159-command-palette-history.md) | L | 011 | done | ui |
| 160 | [Console panel (log + Rhai REPL)](../agents/tasks/160-console.md) | L | 016 | done | ui |
| 161 | [HiDPI, IME, themes, keyboard navigation](../agents/tasks/161-hidpi-ime-themes.md) | L | 147 | done | ui |
| 162 | [S3 acceptance test and UI docs](../agents/tasks/162-s3-ui-acceptance.md) | L | 147–153 | done (parity-pending G1/G2/G4) | tests, docs |
| 163 | [Native format: migrations, binary encoding, external references](../agents/tasks/163-native-format.md) | M | – | done (format version 6; follow-up: a file-path param on the first texture/cache node, compression wrapper) | core, io |
| 164 | [glTF import/export (meshes, hierarchy, materials, animation, skins, morphs)](../agents/tasks/164-gltf.md) | M | 088, 101, 107 | done | io |
| 165 | [FBX import (ufbx)](../agents/tasks/165-fbx-import.md) | M | 088, 101 | done | io |
| 166 | [FBX export (ASCII 7.4)](../agents/tasks/166-fbx-export.md) | M | 165 | done (parity-pending: no reference ASCII FBX; validated by re-import) | io |
| 167 | [Alembic (Ogawa) import/export](../agents/tasks/167-alembic.md) | M | 088 | done | io |
| 168 | [USD .usda import/export](../agents/tasks/168-usd.md) | M | 123 | done (parity-pending: no reference USD) | io |
| 169 | [STL and PLY](../agents/tasks/169-stl-ply.md) | M | – | done (parity-pending: PLY not observed) | io |
| 170 | [Camera, light and material exchange across formats](../agents/tasks/170-camera-light-material-exchange.md) | M | 047, 134 | done (parity-pending: USD) | io |
| 171 | [Exchange parity tests (S9)](../agents/tasks/171-s9-exchange-tests.md) | M | 164–169 | done (parity-pending: glTF skinned-arm f15/f30 bounds, KNOWN_MISMATCH ≤ 5e-4) | tests |
| 172 | [Import/export dialogs and CLI batch export](../agents/tasks/172-io-dialogs-batch.md) | M | 013, 150 | done (OBJ only; dialog layout parity-pending) | ui, io |
| 173 | [CI matrix: Linux, macOS, Windows; per-OS GPU policy](../agents/tasks/173-ci-matrix.md) | N | – | done | .github |
| 174 | [Toolchain pin and cargo-deny](../agents/tasks/174-toolchain-cargo-deny.md) | N | – | done | root |
| 175 | [Cross-platform settings, paths, fonts, file dialogs, modifiers](../agents/tasks/175-cross-platform-basics.md) | N | 173 | done | core, ui |
| 176 | [Release workflow: AppImage, dmg, zip/MSI, checksums, changelog](../agents/tasks/176-release-workflow.md) | N | 173 | done (pending first tagged run) | .github |
| 177 | [Signing and notarization hooks](../agents/tasks/177-signing-hooks.md) | N | 176 | done (pending first signed run) | .github |
| 178 | [Crash handler and logging](../agents/tasks/178-crash-logging.md) | N | 175 | done | app |
| 179 | [Autosave and recovery](../agents/tasks/179-autosave-recovery.md) | N | 175, 163 | done | app |
| 180 | [Dependency-graph evaluation scheduler (parallel, dirty-only)](../agents/tasks/180-eval-scheduler.md) | N | 012 | done | core |
| 181 | [Benchmarks and S10 performance thresholds](../agents/tasks/181-benchmarks-s10.md) | N | 180, 144 | done | benches |
| 182 | [Fuzzing and UI soak test](../agents/tasks/182-fuzz-soak.md) | N | 014, 163 | done (nightly workflow pending first run) | fuzz |
| 183 | [User manual and generated references (mdBook, GitHub Pages)](../agents/tasks/183-manual.md) | N | 019 | done | docs |
| 184 | [Rhai plugin packages](../agents/tasks/184-rhai-plugins.md) | N | 016 | done | plugin |
| 186 | [Plugin host integration: evaluation hook and plugin folders](../agents/tasks/186-plugin-host.md) | N | 184 | done | core, generators, plugin, app, cli |
| 187 | [O58 object-command and selection parity](../agents/tasks/187-o58-object-selection-parity.md) | B | 031, 034 | done | modeling, command, app |
| 188 | [Adopt the R25 Loft ring and quad order (D18)](../agents/tasks/188-r25-loft-adoption.md) | C | 042 | done | generators, io |
| 189 | [Native format v10 for list-valued parameters](../agents/tasks/189-format-v10.md) | M | 028, 163 | done | core, io |
| 190 | [Viewport splines: drawing, points, handles and dragging](../agents/tasks/190-viewport-splines.md) | E | 028, 143 | done | viewport, ui, command |
| 191 | [R25 Helix Bézier construction (sampling parity)](../agents/tasks/191-helix-sampling.md) | B | 028, 042 | done | generators |
| 192 | [Volume and rig nodes in the editor: evaluation hook, commands, display](../agents/tasks/192-volume-rig-integration.md) | F, H | 081, 083, 100, 101, 112, 084 | done | generators, app, command, ui, viewport, volume, rig |
| 193 | [Parity tests use Extrude's Auto direction](../agents/tasks/193-extrude-tests-use-auto.md) | C | 044, 045 | done | generators (tests) |
| 194 | [Materials and shaders in the document: hosting, links, Generate UVW command](../agents/tasks/194-material-hosting.md) | J | 123, 125, 127 | done | core, material, shaders, app, command, ui, io |
| 195 | [Grow Selection: Use Edge option for polygons](../agents/tasks/195-grow-use-edge.md) | B | 023 | done | geometry, command |
| 196 | [OBJ texture coordinates (vt) import and export](../agents/tasks/196-obj-uvs.md) | M | 004, 128 | done | io |
| 197 | [Shading IR to WGSL codegen and shader cache (part 1: IR core)](../agents/tasks/197-shading-ir-wgsl.md) | R | 123 | done | material, shaders |
| 207 | [Shader nodes to WGSL, GPU comparison over all nodes (part 2)](../agents/tasks/207-shader-nodes-wgsl.md) | R | 197, 125, 126 | done | shaders, material |
| 198 | [Real-time raster PBR engine (render-rt)](../agents/tasks/198-realtime-raster.md) | R | 197, 133, 137 | waiting: 197, 133, 137 | render-rt (new), viewport, render-api |
| 199 | [Post effects and viewport Render mode](../agents/tasks/199-realtime-post.md) | R | 198 | waiting: 198 | render-rt, viewport, ui |
| 200 | [GPU scene: acceleration structures and compute fallback](../agents/tasks/200-gpu-scene-rt.md) | R | 142, 198 | waiting: 142, 198 | render-rt, render-gpu |
| 201 | [Hybrid ray-traced shadows, reflections, AO and GI](../agents/tasks/201-hybrid-rt.md) | R | 200 | waiting: 200 | render-rt |
| 202 | [Real-time path tracing with denoising](../agents/tasks/202-realtime-pathtrace.md) | R | 200, 201 | waiting: 200, 201 | render-rt |
| 203 | [Consistency tests across render tiers](../agents/tasks/203-rt-consistency.md) | R | 198, 202 | waiting: 198, 202 | tests, render-rt |
| 204 | [Real-time performance targets](../agents/tasks/204-rt-performance.md) | R | 199, 201 | waiting: 199, 201 | render-rt, bench |
| 205 | [S4 acceptance test and motion-graphics docs](../agents/tasks/205-s4-mograph-acceptance.md) | E | 066, 072, 074, 075, 136 | done | tests, docs |
| 206 | [S5 acceptance test and volume docs](../agents/tasks/206-s5-volume-acceptance.md) | F | 081, 082, 083, 003 | done | tests, docs |
| 208 | [Volume mesher: manifold output on thin features](../agents/tasks/208-volume-mesher-manifold.md) | F | 083, 206 | done | volume |
| 185 | [v1.0 release checklist: S1–S10 on all three OS](../agents/tasks/185-v1-release.md) | N | everything | ready | all |
