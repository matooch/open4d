// Builds a small hierarchy and prints it with the scene queries.
let root = create("forge.null", "Root");
for name in ["A", "B"] {
    let child = create("forge.cube", name);
    run("object.reparent", #{ node: child, parent: root });
}
set_param(scene().find("B"), "size_x", 50.0);

fn show(id, depth) {
    let n = scene().node(id);
    let indent = "";
    for i in 0..depth { indent += "  "; }
    print(`${indent}${n.name} (${n.type})`);
    for child in n.children() { show(child, depth + 1); }
}
for id in scene().roots() { show(id, 0); }
print(`B size = ${scene().node(scene().find("B")).param("size_x")}`);
