// Acceptance scenario S6 (docs/roadmap/README.md): rigging through the CLI.
// Run: forge run tests/acceptance/s6_rigging.rhai --save s6.forge
//
// A joint chain of three joints (J1 at y = -100, J2 at y = 0, J3 at y = 100, bones of length
// 100) bound with automatic weights to an editable cylinder (radius 20, height 200, 8 height
// segments). An IK tag on J1 (end J3) reaches for the Goal null and bends towards the Pole null
// (docs/specs/RIG.md section 10: the elbow goes to the side opposite the pole). A Pose Morph tag on
// the cylinder has one target that widens the upper half (RIG.md section 15, own design D20).
// Everything is animated over 60 frames at 30 fps:
//   Goal  (60, 60, 0) -> (0, 20, 80) -> (-40, 70, 40)       frames 0, 30, 60
//   Pole  (0, 0, 300) -> (300, 0, 0) -> (0, 0, -300)         frames 0, 30, 60
//   morph strength 0 % -> 100 % -> 50 %                      frames 0, 30, 60
// Two small cubes ride on J2 and J3 so that the CLI can read the joints' world positions from an
// export (a joint has no geometry of its own).
run("timeline.settings", #{ fps: 30, start: 0, end: 60 });

let cyl = create("forge.cylinder", "Body");
set_param(cyl, "radius", 20.0);
set_param(cyl, "height", 200.0);
set_param(cyl, "height_segments", 8);
set_param(cyl, "rotation_segments", 12);
let made = run("mesh.make_editable", #{ nodes: [cyl] });
let body = made.made[0];

let chain = run("joint.create_chain", #{ points: [[0, -100, 0], [0, 0, 0], [0, 100, 0]], prefix: "J", align: "y" });
let j1 = chain.nodes[0];
let j2 = chain.nodes[1];
let j3 = chain.nodes[2];

// automatic weights, bind pose, skin deformer
run("weights.auto_bind", #{ object: body, joints: [j1, j2, j3] });

// markers: tiny cubes that follow the joints
let m2 = create("forge.cube", "Elbow marker");
set_param(m2, "size_x", 2.0);
set_param(m2, "size_y", 2.0);
set_param(m2, "size_z", 2.0);
run("object.reparent", #{ node: m2, parent: j2 });
set_transform(m2, #{ pos: vec3(0, 0, 0) });
let m3 = create("forge.cube", "Tip marker");
set_param(m3, "size_x", 2.0);
set_param(m3, "size_y", 2.0);
set_param(m3, "size_z", 2.0);
run("object.reparent", #{ node: m3, parent: j3 });
set_transform(m3, #{ pos: vec3(0, 0, 0) });

// IK with a pole
let goal = create("forge.null", "Goal");
let pole = create("forge.null", "Pole");
set_transform(goal, #{ pos: vec3(60, 60, 0) });
set_transform(pole, #{ pos: vec3(0, 0, 300) });
let ik = run("tag.attach", #{ type: "forge.ik_tag", node: j1 }).node;
set_param(ik, "end", j3);
set_param(ik, "goal", goal);
set_param(ik, "pole", pole);

// pose morph (RIG.md section 15): the tag takes the rest pose as its base and as target 0, the
// upper half of the cylinder is widened (x and z doubled) and captured as the target, then the
// object goes back to the rest pose. Cylinder points: 0 and 1 are the cap centres, then 9 points
// (y = -100 + 25 j, j = 0..8) per rotation segment.
let upper = [1];
for i in 0..12 {
    for j in 5..9 {
        upper.push(2 + i * 9 + j);
    }
}
let morph = run("pose_morph.add_target", #{ object: body, name: "Wide top" });
let tag = morph.tag;
run("mesh.transform", #{ node: body, points: upper, pivot: [0, 0, 0], scale: [2, 1, 2], soft: false });
run("pose_morph.update_target", #{ tag: tag, slot: 0 });
run("mesh.transform", #{ node: body, points: upper, pivot: [0, 0, 0], scale: [0.5, 1, 0.5], soft: false });

// keys
run("key.set", #{ node: goal, track: "transform.pos.x", frame: 0, value: 60.0 });
run("key.set", #{ node: goal, track: "transform.pos.y", frame: 0, value: 60.0 });
run("key.set", #{ node: goal, track: "transform.pos.z", frame: 0, value: 0.0 });
run("key.set", #{ node: goal, track: "transform.pos.x", frame: 30, value: 0.0 });
run("key.set", #{ node: goal, track: "transform.pos.y", frame: 30, value: 20.0 });
run("key.set", #{ node: goal, track: "transform.pos.z", frame: 30, value: 80.0 });
run("key.set", #{ node: goal, track: "transform.pos.x", frame: 60, value: -40.0 });
run("key.set", #{ node: goal, track: "transform.pos.y", frame: 60, value: 70.0 });
run("key.set", #{ node: goal, track: "transform.pos.z", frame: 60, value: 40.0 });
run("key.set", #{ node: pole, track: "transform.pos.x", frame: 0, value: 0.0 });
run("key.set", #{ node: pole, track: "transform.pos.z", frame: 0, value: 300.0 });
run("key.set", #{ node: pole, track: "transform.pos.x", frame: 30, value: 300.0 });
run("key.set", #{ node: pole, track: "transform.pos.z", frame: 30, value: 0.0 });
run("key.set", #{ node: pole, track: "transform.pos.x", frame: 60, value: 0.0 });
run("key.set", #{ node: pole, track: "transform.pos.z", frame: 60, value: -300.0 });
run("key.set", #{ node: tag, track: "strength_0", frame: 0, value: 0.0 });
run("key.set", #{ node: tag, track: "strength_0", frame: 30, value: 100.0 });
run("key.set", #{ node: tag, track: "strength_0", frame: 60, value: 50.0 });

print(`body=${body} j1=${j1} j2=${j2} j3=${j3} m2=${m2} m3=${m3} goal=${goal} pole=${pole} ik=${ik} morph=${tag}`);
