// Twenty cubes at seeded random positions: the same scene on every run and platform.
let r = rand(7);
for i in 0..20 {
    let cube = create("forge.cube", `random_${i}`);
    set_param(cube, "size_x", r.range(20.0, 80.0));
    set_transform(cube, #{ pos: vec3(r.range(-500, 500), r.range(-500, 500), r.range(-500, 500)) });
}
scene().nodes().len()
