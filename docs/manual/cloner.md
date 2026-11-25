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

## Walkthrough: a motion-graphics scene from a script

The S4 acceptance scenario (roadmap `README.md`) builds a ring of fractured letters in one Rhai
script: [`tests/acceptance/s4_motion_graphics.rhai`](../../tests/acceptance/s4_motion_graphics.rhai). The
acceptance test runs exactly that file through the CLI, so the script doubles as an automation
example.

```rhai
run("timeline.settings", #{ fps: 30, start: 0, end: 90 });

let text = create("forge.clone_text", "Text");
set_param(text, "text", "FORGE");
set_param(text, "height", 100.0);
set_param(text, "depth", 20.0);

let random = create("forge.effector.random", "Random");
set_param(random, "seed", 4242);
set_param(random, "position", false);
set_param(random, "rotation", true);
set_param(random, "r", vec3(25, 35, 15));

let fracture = create("forge.fracture", "Fracture");
run("object.reparent", #{ node: text, parent: fracture });
set_param(fracture, "mode", "explode_segments");
set_param(fracture, "effectors", [random]);

let circle = create("forge.circle_spline", "Circle");
set_param(circle, "plane", "xz");
set_param(circle, "radius", 200.0);

let cloner = create("forge.cloner", "Cloner");
set_param(cloner, "mode", "object");
set_param(cloner, "object", circle);
set_param(cloner, "distribution", "vertex");
set_param(cloner, "align_clone", false);
run("object.reparent", #{ node: fracture, parent: cloner });

let field = create("forge.field.linear", "Sweep");
set_param(field, "length", 50.0);
set_param(field, "direction", "x_pos");
run("key.set", #{ node: field, track: "transform.pos.x", frame: 0, value: 250.0 });
run("key.set", #{ node: field, track: "transform.pos.x", frame: 90, value: 150.0 });

let plain = create("forge.effector.plain", "Plain");
set_param(plain, "fields", [field]);
set_param(plain, "position", true);
set_param(plain, "p", vec3(0, 100, 0));
set_param(cloner, "effectors", [plain]);
```

Run it and export or render a few frames:

```sh
forge run tests/acceptance/s4_motion_graphics.rhai --save s4.forge
forge eval s4.forge --frame 0 --export s4-0000.obj
forge render s4.forge --start 0 --end 90 --out "frames/frame_{frame:04}.png" \
    --width 1920 --height 1080
```

What the pieces do:

- **Text → Fracture.** `forge.clone_text` makes one extruded glyph per letter; `Fracture` in
  Explode Segments mode splits it into its polygon islands, so "FORGE" gives five pieces.
- **Circle spline → Cloner (Object, Vertex).** A spline has no mesh of its own: the cloner uses
  the spline's sampled points. The Circle's default *Intermediate Points* (Uniform, Number 8)
  samples four quarter arcs into `4 x (8 + 1) = 36` points, so the cloner makes 36 clones, one on
  each point. With `align_clone` off the clones keep world axes.
- **Random effector on the fracture.** It turns each fragment about its own centre with the
  fixed seed; Position is off, so the fragments stay on their clone's point.
- **Plain effector with a Linear field.** The field's `length` is 50, so its value runs from 0 to
  1 over 100 cm along X. Keyed from x = 250 at frame 0 to x = 150 at frame 90, every clone's
  falloff is 0 at frame 0 (nothing moves, all clones sit on the spline) and by frame 90 the ring
  section that the field covers has risen by `P = (0, 100, 0)` times its falloff value.

The acceptance test reads the result back from the files: five pieces in the fracture export,
36 clones of them, every clone origin on the circle at frame 0 within 1e-3, exactly the covered
clones lifted at frame 90, byte-identical OBJ exports from two runs at frames 0, 45 and 90, and a
1920×1080 PNG sequence at 1 sample whose alpha shows the letters. The full 91-frame render at the
default sample count is a local check, not part of the test budget.
