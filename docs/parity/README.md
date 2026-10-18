# Observation kit

How to turn the reference app into golden data, fast. You (the observer) run the
reference app as a **black box**; implementers only ever read what lands in this
folder and in `tests/golden/`. Policy: [docs/CLEANROOM.md](../CLEANROOM.md),
method: [docs/PARITY.md](../PARITY.md).

## 0. Before you start (once)

- [ ] Check the reference app's EULA allows this kind of use and export. Stop if not.
- [ ] Note the exact app version and OS — every observation file records it.
- [ ] Never open the app's install folders, resource/preset/string/icon files, or
      plugins. Only the running UI, its documented export formats, and the public
      manual.
- [ ] Screenshots are for *understanding* behaviour; don't commit them, don't trace
      them.

## 1. Set up the OBJ exporter once, then never change it

Record every exporter option you see (name and value) in
[export-settings.md](export-settings.md). Recommended:

| Option (whatever the app calls it) | Set to | Why |
|---|---|---|
| Scale / units | 1 (no rescale) or a known factor | we undo it with `[import] scale` |
| Axis / handedness flip | leave at default, **record it** | `[import] flip_z` undoes a Z flip |
| Triangulate | off | we compare n-gons and vertex order exactly |
| Normals | on | needed for smoothing parity |
| UVs | on | needed later (materials) |
| Materials / textures | off | not needed |
| Export selected only | on | one object per file |

If the exporter offers no option for something, write "not offered".

## 2. The loop (per case, ~2 minutes)

1. New, empty scene. Create the object. Change only the parameters the case lists.
2. Write the parameter values you set into the observation file (case table).
3. Export the object as OBJ to
   `tests/golden/<feature>/<case>/mesh.obj` (case names: lowercase, dashes).
4. Run `cargo run -p forge-parity-harness --bin golden -- info tests/golden/<feature>/<case>/mesh.obj`
   and paste the output into the case row (vertex/face counts, size).
5. Create `tests/golden/<feature>/<case>/case.toml` (template below).
6. When a feature is done: `cargo run -p forge-parity-harness --bin golden -- check`
   must print `ok: N cases`.

```toml
feature = "cube"                          # same as the folder name
observation = "docs/parity/cube.md"
description = "defaults"
mesh = "mesh.obj"

[params]                                  # exactly what you typed/left in the app
size_x = 200.0

[import]                                  # from export-settings.md
scale = 1.0
flip_z = false
```

Parameter keys in `[params]`: lowercase words joined by `_`, named after what the
parameter *does* (e.g. `size_x`, `segments_y`, `fillet_radius`), not copied labels.
Write the app's own label in the observation file's parameter table instead.

## 3. Order of work

Do these in order; each unblocks code that is waiting on it.

| # | File | Unblocks | Cases |
|---|---|---|---|
| 1 | [export-settings.md](export-settings.md) | everything | – |
| 2 | [transform.md](transform.md) | transforms, rotate gizmo, coordinate panel | 8 |
| 3 | [cube.md](cube.md) | first primitive, harness end-to-end | 5 |
| 4 | [plane.md](plane.md) | primitives | 4 |
| 5 | [sphere.md](sphere.md) | primitives, sphere types | 6+ |
| 6 | [smoothing.md](smoothing.md) | normal weighting, smoothing angle | 4 |
| 7 | [subdivision.md](subdivision.md) | subdivision surfaces | 7 |
| 8 | [boole.md](boole.md) | booleans | 6 |
| 9 | [defaults.md](defaults.md) | the parity-pending UI/tool defaults | – |
| 10 | [viewport.md](viewport.md) | un-mirrored viewport display (handedness) | – |
| 11 | [cylinder.md](cylinder.md) | primitives II | 10 |
| 12 | [cone.md](cone.md) | primitives II | 9 |
| 13 | [disc.md](disc.md) | primitives II | 6 |
| 14 | [torus.md](torus.md) | primitives II | 5 |
| 15 | [tube.md](tube.md) | primitives II | 6 |
| 16 | [capsule.md](capsule.md) | primitives II | 4 |
| 17 | [pyramid.md](pyramid.md) | primitives II | 4 |
| 18 | [splines.md](splines.md) | spline export, spline inputs | 6 |
| 19 | [symmetry.md](symmetry.md) | generators II | 5 |
| 20 | [array.md](array.md) | generators II | 4 |
| 21 | [connect.md](connect.md) | generators II | 4 |
| 22 | [instance.md](instance.md) | generators II | 1 |
| 23 | [extrude.md](extrude.md) | generators II (spline input) | 6 |
| 24 | [lathe.md](lathe.md) | generators II (spline input) | 4 |
| 25 | [loft.md](loft.md) | generators II (spline input) | 4 |
| 26 | [sweep.md](sweep.md) | generators II (spline input) | 5 |

[TEMPLATE.md](TEMPLATE.md) is the blank form for any other feature.

## 4. What "good" observations look like

- **Numbers, not impressions.** "Default radius 100 (unit cm shown in the field)",
  not "medium size".
- **Ranges:** type a huge and a negative value into each numeric field and record
  what the field clamps to. That gives min/max.
- **Units:** record the unit suffix the field shows and the scene unit setting.
- **Every option of an enum** listed with its label as shown.
- **Anything surprising** (e.g. a parameter that hides others) goes under "Notes".
- One object per export; nothing else selected.

## 5. Handing it back

Commit on a branch (e.g. `observations-1`) and push, or just tell me the files are
there. I'll write specs from them, add the parity tests, and implement until the
harness diffs pass. Missing info is fine: mark it `TODO` and I'll ask.
