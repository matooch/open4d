// Ring Kit: a torus-shaped generator and a command that creates one.
// Only the function definitions are used; see docs/specs/PLUGINS.md.

// Node type `ring_kit.ring`. Runs in a pure sandbox: params in, mesh out.
fn build_ring(p) {
    let seg = p.segments;
    let sides = p.sides;
    let points = [];
    let polys = [];
    for i in 0..seg {
        let a = 2.0 * PI() * i.to_float() / seg.to_float();
        for j in 0..sides {
            let b = 2.0 * PI() * j.to_float() / sides.to_float();
            let r = p.radius + p.tube * cos(b);
            points.push([r * cos(a), p.tube * sin(b), r * sin(a)]);
        }
    }
    for i in 0..seg {
        let i2 = (i + 1) % seg;
        for j in 0..sides {
            let j2 = (j + 1) % sides;
            polys.push([i * sides + j, i2 * sides + j, i2 * sides + j2, i * sides + j2]);
        }
    }
    #{ points: points, polys: polys }
}

// Command `ring_kit.add_ring`: creates a ring, optionally with a radius.
fn add_ring(args) {
    let id = create("ring_kit.ring", "Ring");
    if args.contains("radius") {
        set_param(id, "radius", args.radius);
    }
    id
}
