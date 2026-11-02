// A sphere that moves 200 units along x and back over one second (30 fps).
run("timeline.settings", #{ fps: 30, start: 0, end: 30 });
let ball = create("forge.sphere", "Ball");
for key in [[0, 0.0], [15, 200.0], [30, 0.0]] {
    run("key.set", #{ node: ball, track: "transform.pos.x", frame: key[0], value: key[1] });
}
ball
