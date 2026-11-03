// A 3x3x3 grid of cubes under a null called "Grid". Returns the id of the null.
let grid = create("forge.null", "Grid");
let step = 300.0;
for x in 0..3 {
    for y in 0..3 {
        for z in 0..3 {
            let cube = create("forge.cube", `cube_${x}${y}${z}`);
            run("object.reparent", #{ node: cube, parent: grid });
            set_transform(cube, #{
                pos: vec3(x.to_float() * step, y.to_float() * step, z.to_float() * step)
            });
        }
    }
}
print(`built ${scene().node(grid).children().len()} cubes`);
grid
