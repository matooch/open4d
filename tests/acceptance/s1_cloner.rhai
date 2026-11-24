// Acceptance scenario S1, second half (docs/roadmap/README.md): a cloner with a random effector.
// Run: forge run tests/acceptance/s1_cloner.rhai --save s1c.forge
//
// Five cubes in a linear row, scattered by a seeded random effector; the bend under the cube is
// animated as in s1.rhai, so frame 30 is bent halfway.
run("timeline.settings", #{ fps: 30, start: 0, end: 60 });

let cloner = create("forge.cloner", "Cloner");
set_param(cloner, "mode", "linear");
set_param(cloner, "linear_count", 5);
set_param(cloner, "transform_position", vec3(250, 0, 0));

let cube = create("forge.cube", "Cube");
run("object.reparent", #{ node: cube, parent: cloner });
let bend = create("forge.bend", "Bend");
run("object.reparent", #{ node: bend, parent: cube });
run("key.set", #{ node: bend, track: "strength", frame: 0, value: 0.0 });
run("key.set", #{ node: bend, track: "strength", frame: 60, value: 90.0 });

let random = create("forge.effector.random", "Random");
set_param(random, "seed", 4242);
set_param(random, "p", vec3(40, 40, 40));
set_param(cloner, "effectors", [random]);

print(`cloner=${cloner} cube=${cube} bend=${bend} random=${random}`);
cloner
