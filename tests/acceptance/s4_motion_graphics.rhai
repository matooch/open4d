// Acceptance scenario S4 (docs/roadmap/README.md): motion graphics through the CLI.
// Run: forge run tests/acceptance/s4_motion_graphics.rhai --save s4.forge
//
// "FORGE" is fractured into one piece per letter, the fracture is cloned onto the 36 sampled
// points of a Circle spline, a Random effector turns every fragment and a Plain effector with
// a keyed Linear field falloff sweeps a rise across the ring over 90 frames.
//
// Frame 0: the field sits beyond the ring, so every clone's falloff is 0 and each clone sits
// exactly on the spline. By frame 90 the field has swept in to x = 150 and lifts the clones it
// covers (x > 100) by the effector's P (0, 100, 0); the rest stay on the spline.
run("timeline.settings", #{ fps: 30, start: 0, end: 90 });

let text = create("forge.clone_text", "Text");
set_param(text, "text", "FORGE");
set_param(text, "height", 100.0);
set_param(text, "depth", 20.0);

// A fixed seed keeps the fragment turns byte-identical between runs.
let random = create("forge.effector.random", "Random");
set_param(random, "seed", 4242);
set_param(random, "position", false);
set_param(random, "rotation", true);
set_param(random, "r", vec3(25, 35, 15));

let fracture = create("forge.fracture", "Fracture");
run("object.reparent", #{ node: text, parent: fracture });
set_param(fracture, "mode", "explode_segments");
set_param(fracture, "effectors", [random]);

// The Circle's default Intermediate Points (Uniform, 8) sample it into 36 points on the curve.
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

print(`text=${text} fracture=${fracture} circle=${circle} cloner=${cloner} field=${field} plain=${plain} random=${random}`);
cloner
