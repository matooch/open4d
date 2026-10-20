# Generators II — mesh-input generators (`crates/generators`)

Sources: docs/parity/{symmetry,array,connect,instance}.md and their golden cases.
Spline-input generators (extrude, lathe, loft, sweep) follow in a later section once the
spline sampling is derived (docs/parity/splines.md: splines do not export).

## 0. Inputs and the exporter's weld

- A generator's **inputs** are its hierarchy children that evaluate to a mesh, in child
  order, each placed by its own local transform (as SUBDIVISION.md §1); they are
  consumed (not drawn themselves). Symmetry and Array take the first, Connect all.
- **Export weld (observed):** the reference OBJ exporter merges coincident points of the
  whole export (`symmetry/weld-off` is byte-identical to `symmetry/weld`;
  `connect/weld-off` shares the touching cubes' points). Parity tests therefore apply
  `forge_parity_harness::export_weld` to our mesh before comparing: points equal within
  1e-6 are merged into the first occurrence, later points are renumbered in order, faces
  keep their corners. Our generators themselves do not depend on it.

## 1. Symmetry (`forge.symmetry`)

Params: `mirror_plane` enum `xy, zy (default), xz`; `weld_points` true; `tolerance`
0.01 (cm); `symmetrical` true (no effect on the observed cases).
Result: the input's points, then the mirrored points (the plane's normal coordinate
negated: zy → x, xy → z, xz → y); with `weld_points`, an input point within `tolerance`
of the plane is not duplicated (its mirror is itself). Faces: the input's faces, then
each face mirrored with reversed winding keeping its first corner
(`[a, b, c, d] → [a', d', c', b']`). Faces lying on the plane are kept twice (observed,
`symmetry/weld`).

## 2. Array (`forge.array`)

Params: `radius` 250, `copies` 7 (≥ 0; the result has `copies + 1` copies), `amplitude`
0, `frequency` 0, `array_frequency` 10, `render_instance` false.
With `N = copies + 1`, copy k = 0..N−1 at angle `θ_k = 360°·k/N`: every input point p
becomes `Ry(θ_k)·p + Ry(θ_k)·(0, 0, radius) + (0, h_k, 0)`, where
`Ry(θ)(x, y, z) = (x cos θ + z sin θ, y, −x sin θ + z cos θ)` (copy 0 at +Z, copy 1
towards +X) and `h_k = amplitude · sin(360°·array_frequency·k/N)` at frame 0
(`array/wave`; the animated phase from `frequency` is not modelled: **parity-pending**).
Copies are concatenated in k order (points, then faces, per copy).

## 3. Connect (`forge.connect`)

Params: `weld` true, `tolerance` 0.1 (cm), `phong_mode` enum `manual, average, lowest,
highest, as_breaks (default)`, `textures` true, `center_axis` false (the link list
`objects` is not modelled; the children are the inputs).
Result: the inputs concatenated in child order. With `weld`: points within `tolerance`
merge into the first occurrence (later points renumbered in order), and a face whose
corner set equals that of a **later** face is dropped (`connect/defaults`: A's face at
x = 100 goes, B's stays). `center_axis = true` → `Unsupported` (not observed).

## 4. Instance (`forge.instance`)

Params: `reference` (link to an object), `instance_mode` enum `instance (default),
render_instance, multi_instance`. The local mesh is the reference's local mesh (same
point and face order); the instance's own transform places it. A missing or meshless
reference gives no geometry, and so does a cycle of instances (an instance of an
instance follows the chain to its first non-instance). The reference is not consumed
(it stays visible).

## 5. Parity tests

`crates/generators/tests/generators_parity.rs`: each case rebuilds its inputs from
`[params.input]`, `[params.input2]`, … (`type` = a primitive id suffix, `pos` the
position, other keys that primitive's params) in a scene, evaluates the generator, and
compares `export_weld(ours)` strictly (topology, 1e-4) with the export.
