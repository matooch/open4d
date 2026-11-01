// Acceptance scenario S1 (docs/roadmap/README.md): build a scene from a script.
// Run: forge run tests/acceptance/s1.rhai --save s1.forge
//
// Done here: a cube with a bend deformer whose strength is animated.
// PENDING (see apps/forge/tests/acceptance.rs): the cloner with a random effector
// (briefs 065, 069) is added between the cube and the bend once those node types exist.
run("timeline.settings", #{ fps: 30, start: 0, end: 60 });

let cube = create("forge.cube", "Cube");
let bend = create("forge.bend", "Bend");
run("object.reparent", #{ node: bend, parent: cube });

// The bend grows from 0 at frame 0 to 90 degrees at frame 60, so frame 30 is 45 degrees.
run("key.set", #{ node: bend, track: "strength", frame: 0, value: 0.0 });
run("key.set", #{ node: bend, track: "strength", frame: 60, value: 90.0 });

print(`cube=${cube} bend=${bend}`);
cube
