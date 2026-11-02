//! The script API as data, for the generated reference (docs/specs/SCRIPTING.md §3).
//!
//! [`API`] is the one place that describes what scripts can call. A unit test compares it
//! with the functions the engine actually registers (name and parameter count), so a
//! binding added, removed or re-shaped without updating this table fails the build.

/// One documented function, property or operator.
pub struct ApiEntry {
    /// Section heading in the reference.
    pub group: &'static str,
    /// How a script writes it.
    pub signature: &'static str,
    /// What it does.
    pub summary: &'static str,
    /// The engine functions this entry covers: `(registered name, parameters including
    /// the receiver)`. Property getters are `get$name`, setters `set$name`.
    pub engine: &'static [(&'static str, usize)],
}

const fn e(
    group: &'static str,
    signature: &'static str,
    summary: &'static str,
    engine: &'static [(&'static str, usize)],
) -> ApiEntry {
    ApiEntry {
        group,
        signature,
        summary,
        engine,
    }
}

const CMD: &str = "Commands";
const SCENE: &str = "Scene queries (read-only)";
const NODE: &str = "Node handles";
const TIME: &str = "Time";
const MATHS: &str = "Maths: vec3 and deg";
const RNG: &str = "Seeded random numbers";

/// Every function a script can call, in reference order.
pub const API: &[ApiEntry] = &[
    e(
        CMD,
        "run(id)",
        "Runs command `id` without arguments; returns the command's result as a Rhai value. A failing command throws (`id: <error>`); catch it with `try`/`catch`.",
        &[("run", 1)],
    ),
    e(
        CMD,
        "run(id, #{ ... })",
        "Runs command `id` with the map as its arguments (units as in the command reference).",
        &[("run", 2)],
    ),
    e(
        CMD,
        "commands()",
        "Array of every command id the host registered.",
        &[("commands", 0)],
    ),
    e(
        CMD,
        "create(type)",
        "Creates a node (`object.create`) and returns its id.",
        &[("create", 1)],
    ),
    e(
        CMD,
        "create(type, name)",
        "Creates a node, renames it (`object.rename`) and returns its id.",
        &[("create", 2)],
    ),
    e(
        CMD,
        "set_param(node, name, value)",
        "`param.set`. `value` is a number, bool, string, `vec3`, array or `()`.",
        &[("set_param", 3)],
    ),
    e(
        CMD,
        "set_transform(node, #{ pos, rot, scale })",
        "`transform.set`. The keys are optional; values are `vec3` or `[x, y, z]`.",
        &[("set_transform", 2)],
    ),
    e(
        SCENE,
        "scene()",
        "A handle to the live document; queries see edits made earlier in the same script.",
        &[("scene", 0)],
    ),
    e(
        SCENE,
        "scene().nodes()",
        "All non-tag node ids in tree order.",
        &[("nodes", 1)],
    ),
    e(SCENE, "scene().roots()", "Root node ids.", &[("roots", 1)]),
    e(
        SCENE,
        "scene().find(name)",
        "Id of the first node named `name` in tree order, or `()`.",
        &[("find", 2)],
    ),
    e(
        SCENE,
        "scene().find_all(name)",
        "Array of the ids of all nodes named `name`.",
        &[("find_all", 2)],
    ),
    e(
        SCENE,
        "scene().node(id)",
        "A node handle; throws for an unknown id.",
        &[("node", 2)],
    ),
    e(
        SCENE,
        "scene().selection()",
        "Array of the selected node ids.",
        &[("selection", 1)],
    ),
    e(
        SCENE,
        "scene().len()",
        "Number of nodes, tags included.",
        &[("len", 1)],
    ),
    e(
        SCENE,
        "scene().fps`, `.start`, `.end",
        "Project settings.",
        &[("get$fps", 1), ("get$start", 1), ("get$end", 1)],
    ),
    e(
        NODE,
        "node.id`, `.name`, `.type",
        "Properties; `type` is e.g. `forge.cube`.",
        &[("get$id", 1), ("get$name", 1), ("get$type", 1)],
    ),
    e(
        NODE,
        "node.parent()",
        "Parent id or `()`.",
        &[("parent", 1)],
    ),
    e(
        NODE,
        "node.children()",
        "Array of child ids.",
        &[("children", 1)],
    ),
    e(
        NODE,
        "node.param(name)",
        "Current stored value in display units (angles in degrees, vectors as `vec3`); throws for an unknown parameter.",
        &[("param", 2)],
    ),
    e(
        NODE,
        "node.params()",
        "Map of every parameter.",
        &[("params", 1)],
    ),
    e(TIME, "frame()", "The current frame.", &[("frame", 0)]),
    e(
        TIME,
        "frame(n)",
        "`timeline.set_frame`; returns the clamped frame.",
        &[("frame", 1)],
    ),
    e(
        MATHS,
        "vec3(x, y, z)",
        "A vector with fields `x`, `y`, `z` (readable and writable). Converts to `[x, y, z]` in command arguments.",
        &[
            ("vec3", 3),
            ("get$x", 1),
            ("get$y", 1),
            ("get$z", 1),
            ("set$x", 2),
            ("set$y", 2),
            ("set$z", 2),
        ],
    ),
    e(
        MATHS,
        "a + b`, `a - b`, `-a",
        "Component-wise vector arithmetic.",
        &[("+", 2), ("-", 2), ("-", 1)],
    ),
    e(
        MATHS,
        "a * s`, `s * a`, `a / s",
        "Vector times or divided by a number.",
        &[("*", 2), ("*", 2), ("/", 2)],
    ),
    e(
        MATHS,
        "a == b`, `a != b",
        "Vector comparison.",
        &[("==", 2), ("!=", 2)],
    ),
    e(MATHS, "v.len()", "Length.", &[("len", 1)]),
    e(MATHS, "a.dot(b)", "Dot product.", &[("dot", 2)]),
    e(MATHS, "a.cross(b)", "Cross product.", &[("cross", 2)]),
    e(
        MATHS,
        "v.normalized()",
        "Unit vector (zero stays zero).",
        &[("normalized", 1)],
    ),
    e(MATHS, "v.to_array()", "`[x, y, z]`.", &[("to_array", 1)]),
    e(
        MATHS,
        "v.to_string()`, `v.to_debug()",
        "Text form, used by `print`.",
        &[("to_string", 1), ("to_debug", 1)],
    ),
    e(
        MATHS,
        "deg(x)",
        "Degrees to radians, for scripts that do their own trigonometry. Commands take degrees.",
        &[("deg", 1)],
    ),
    e(MATHS, "to_deg(x)", "Radians to degrees.", &[("to_deg", 1)]),
    e(
        RNG,
        "rand(seed)",
        "A generator (SplitMix64). The same seed gives the same sequence on every platform.",
        &[("rand", 1)],
    ),
    e(RNG, "r.float()", "Float in `[0, 1)`.", &[("float", 1)]),
    e(RNG, "r.range(a, b)", "Float in `[a, b)`.", &[("range", 3)]),
    e(
        RNG,
        "r.int(a, b)",
        "Integer in `[a, b]`; `a > b` is an error.",
        &[("int", 3)],
    ),
];

/// The group headings in reference order.
pub fn groups() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for entry in API {
        if !out.contains(&entry.group) {
            out.push(entry.group);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use super::*;
    use crate::host::Host;

    /// `(name, parameter count)` of every function the engine has, with multiplicity.
    fn registered() -> BTreeMap<(String, usize), usize> {
        let mut engine = rhai::Engine::new();
        let host = Rc::new(Host::new(crate::default_commands(), 10));
        crate::bind::register(&mut engine, &host);
        let mut out = BTreeMap::new();
        for sig in engine.gen_fn_signatures(false) {
            let open = sig.find('(').expect("signature has parameters");
            let close = sig.find(')').expect("signature has parameters");
            let params = sig[open + 1..close].trim();
            let count = if params.is_empty() {
                0
            } else {
                params.split(',').count()
            };
            *out.entry((sig[..open].to_string(), count)).or_insert(0) += 1;
        }
        out
    }

    #[test]
    fn the_api_table_matches_the_registered_functions() {
        let mut documented: BTreeMap<(String, usize), usize> = BTreeMap::new();
        for entry in API {
            for (name, arity) in entry.engine {
                *documented.entry((name.to_string(), *arity)).or_insert(0) += 1;
            }
        }
        let registered = registered();
        let missing: Vec<_> = registered
            .iter()
            .filter(|(k, n)| documented.get(*k) != Some(*n))
            .collect();
        let stale: Vec<_> = documented
            .iter()
            .filter(|(k, n)| registered.get(*k) != Some(*n))
            .collect();
        assert!(
            missing.is_empty() && stale.is_empty(),
            "script API table out of date: registered but undocumented {missing:?}, \
             documented but not registered {stale:?}"
        );
    }

    #[test]
    fn every_entry_has_text() {
        for entry in API {
            assert!(!entry.signature.is_empty() && !entry.summary.is_empty());
            assert!(!entry.engine.is_empty());
        }
    }
}
