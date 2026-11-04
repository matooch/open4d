# Cloner

The cloner (`forge.cloner`) repeats its child objects many times and arranges the copies.
Create one with *Create* or `object.create_cloner` (which wraps the current selection).

## Modes

A new cloner starts in **Grid Array**. The `mode` parameter chooses how clones are placed:

| Mode | Arrangement |
|---|---|
| `linear` | Along a line with a per-step offset |
| `radial` | Around a circle |
| `grid` | In a 3D grid |
| `honeycomb` | In a staggered grid |
| `object` | On the surface or points of another object |

## Order and variation

`clones_mode` chooses how children are assigned to positions: iterate in order, random,
blend or sorted. The cloner also applies a per-clone position, rotation, scale and colour,
and a seeded random mode, so results are repeatable.

## Instances and display

`instance_mode` selects whether copies are real geometry or instances, and `viewport_mode`
selects how they are drawn (off, points, matrix, bounding box or full object). Use
`view.clone_display` to switch the viewport representation.

Fields can drive the cloner's strength per clone; see [Deformers and fields](deformers-and-fields.md).
