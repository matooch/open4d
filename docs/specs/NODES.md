# Node expression engine (`crates/nodes`, package `forge-nodes`)

Brief 108. Sources: project design (docs/roadmap/areas/H-rigging.md "Node expressions"),
the public node-graph literature (typed ports, dataflow evaluation, Kahn's topological
sort), the Rhai book (sandboxing), and the specs of this repo (CORE.md, COMMANDS.md,
SCRIPTING.md, NOISE.md). No reference-app observation is involved: behaviour that the
reference app would define (tag priorities, node names) is out of scope here and belongs to
brief 111 and the O36 observation. The editor UI is brief 109; node materials reuse the
engine in brief 124.

## 1. Model

A **graph** is a set of **graph nodes** and **links**. A graph node has a type id
(`math.add`), a stable numeric id (`GraphNodeId`, allocated from a counter that is never
reused, saved with the graph) and the **constant values** of its unconnected inputs. A
node type (`NodeDef`) declares ordered input ports and output ports; a port has a stable
string id (`a`, `out`), a label, a `PortType`, and for inputs a default constant. A link joins one
output port to one input port; an input has at most one link (connecting again replaces the
old link), an output may feed any number of inputs.

Node types are registered in a `NodeRegistry` (`register`, `install(&dyn NodePlugin)`, `get`,
`defs`), exactly like node types in `forge_core::Registry` (ids `namespace.name`, install of
a plugin is all or nothing). `NodeRegistry::builtin()` is the process-wide registry with the
built-in nodes (§4); the commands (§6) and the expression tag (§5) use it.

## 2. Ports and conversions

`PortType`: `Float`, `Int`, `Bool`, `Vec3`, `Colour` (linear RGB), `Matrix` (4x4, column
major, 16 numbers), `String`, `Node` (a link to a scene node, `Option<NodeId>`), and `Any`.
`PortValue` has one variant per concrete type (never `Any`).

**Implicit conversions** (allowed when linking and applied silently when evaluating):

| From | To | Rule |
|---|---|---|
| any type | same type | identity |
| any type | `Any` | identity |
| `Any` | any type | checked at evaluation: the value must convert implicitly, else the node fails |
| `Int` | `Float` | exact |
| `Bool` | `Int`, `Float` | 0 / 1 |
| `Float`, `Int` | `Vec3` | splat to all three components |
| `Float`, `Int` | `Colour` | grey (splat) |
| `Vec3` | `Colour` | component copy |
| `Colour` | `Vec3` | component copy |

**Explicit conversions** (only through the `convert.*` nodes, §4, or `convert_explicit`)
add: `Float` to `Int` (truncate toward zero, non-finite is an error), `Float`/`Int` to
`Bool` (non-zero), `Vec3` to `Float` (length), `Colour` to `Float` (luminance
`0.2126 r + 0.7152 g + 0.0722 b`), `Vec3` to `Matrix` (translation) and `Matrix` to `Vec3`
(translation column), anything to `String` (decimal text), `String` to `Float`, `Int`,
`Bool` (parse, error if it does not parse). A link between types with no implicit
conversion is refused by `connect` with `TypeMismatch`.

## 3. Graph operations and evaluation

All operations validate fully and leave the graph unchanged on error.

- `add_node(type)` allocates an id and fills no values (defaults apply).
- `remove_node(id)` also removes every link that touches it.
- `connect(from, to)`: both ports must exist (`from` an output, `to` an input), the types must
  be implicitly convertible, and the link must not close a cycle (a link from a node to itself
  or to one of its ancestors is `Cycle`). It returns the link it replaced, if any.
- `disconnect(to)` removes the link into an input. `set_value(node, port, v)` converts `v`
  implicitly to the port type and stores it as the constant (a constant equal to the default
  is still stored). The value of an input is its link if any, else its constant, else the
  default.
- `topo_order()` is Kahn's algorithm with the ready set ordered by node id, so the order is
  deterministic; `Err(Cycle)` if the graph (e.g. loaded from a damaged file) has a cycle.
- **Evaluation** (`evaluate`, `evaluate_all`, `evaluate_for`) runs the needed nodes in topological
  order. A node runs **at most once per evaluation**; its outputs are cached for the rest of
  that evaluation (the `Evaluation` result lists the nodes in the order they ran, so tests
  can see it). `evaluate` runs the *sink* nodes (types flagged `sink`: the parameter-write
  node) and everything they depend on; nodes nobody needs do not run. `evaluate_all` runs every
  node; `evaluate_for(ids)` runs the given nodes and their dependencies. A node error stops
  the evaluation and names the node (`GraphError::Eval { node, message }`).
- `EvalEnv` carries the read-only inputs of an evaluation: the scene (optional), the
  time in seconds and the fps, the host node (`object` inputs that are left empty mean the
  host, the object that carries the expression tag), and the Rhai `SandboxLimits`.
- Nodes that write the scene do not mutate it; they append `ParamWrite { node, param, value }`
  to the evaluation. The value is converted to the target parameter's kind with the implicit
  rules (a `Float` into a `Vec3` param splats). Values are in **stored units** (angles in
  radians, like the scene), not display units.

## 4. Built-in node types

Ports are listed `id: Type`; `Any` outputs take their type from the data. Division by zero
yields 0 so a graph never produces a non-finite value from it.

| Type | Inputs | Outputs | Rule |
|---|---|---|---|
| `const.float` `const.int` `const.bool` `const.vec3` `const.colour` `const.string` | `value` | `out` | the constant |
| `time.now` | - | `seconds: Float`, `frame: Float`, `fps: Float` | evaluation time; `frame = seconds * fps` |
| `math.add` `math.sub` `math.mul` `math.div` | `a: Float`, `b: Float` | `out: Float` | `a+b`, `a-b`, `a*b`, `a/b` (0 if `b == 0`) |
| `math.mix` | `a`, `b`, `t: Float` (0, 1, 0) | `out` | `a + (b-a) t` (not clamped) |
| `math.clamp` | `value`, `min`, `max` (0, 0, 1) | `out` | `min(max(value, lo), hi)` with `lo = min(min,max)`, `hi = max(min,max)` |
| `math.remap` | `value`, `in_min`, `in_max`, `out_min`, `out_max`, `clamp: Bool` (0,1,0,1,false) | `out` | `t = (value-in_min)/(in_max-in_min)` (0 if the input range is empty), clamped to [0,1] when `clamp`, `out_min + (out_max-out_min) t` |
| `math.sin` `math.cos` | `x: Float` (radians) | `out` | `sin x`, `cos x` |
| `math.noise` | `position: Vec3`, `time: Float`, `seed: Int`, `scale: Float` (1), `kind: String` ("perlin"), `octaves: Int` (4) | `value: Float` in [-1,1], `unit: Float` in [0,1] | `forge_noise::Noise::sample4(position, time)` (NOISE.md N1). `kind` is one of the 17 kinds in snake case (`perlin`, `value`, `simplex`, `box`, `cell`, `voronoi1`, `voronoi2`, `voronoi3`, `fbm`, `turbulence`, `ridged_multifractal`, `wavy_turbulence`, `electric`, `gaseous`, `blistered_turbulence`, `displaced_turbulence`, `displaced_voronoi`). `scale` must be > 0, `octaves` is clamped to 1..=16, the seed is taken modulo 2^32 |
| `vector.compose` | `x`, `y`, `z: Float` | `out: Vec3` | `[x, y, z]` |
| `vector.decompose` | `v: Vec3` | `x`, `y`, `z: Float` | the components |
| `vector.add` `vector.scale` `vector.length` | `a`,`b: Vec3`; `v: Vec3`, `s: Float` (1); `v: Vec3` | `Vec3`, `Vec3`, `Float` | `a+b`, `v*s`, `|v|` |
| `matrix.translation` | `m: Matrix` | `out: Vec3` | the translation column |
| `matrix.from_translation` | `v: Vec3` | `out: Matrix` | translation matrix |
| `compare.compare` | `a`, `b: Float`, `op: String` ("lt") | `out: Bool` | `lt le gt ge eq ne` |
| `logic.and` `logic.or` | `a`, `b: Bool` | `out: Bool` | |
| `logic.not` | `a: Bool` | `out: Bool` | |
| `logic.condition` | `condition: Bool`, `if_true: Any`, `if_false: Any` | `out: Any` | the chosen input; both inputs are evaluated (graphs have no laziness) |
| `convert.to_float` `convert.to_int` `convert.to_bool` `convert.to_string` | `value: Any` | `out` | explicit conversions of §2 |
| `object.self` | - | `object: Node` | the host (error without one) |
| `object.get_param` | `object: Node` (empty = host), `param: String` | `value: Any` | reads the parameter of the *evaluated* scene (animation applied, earlier expressions applied). Float, int, bool, vec3, colour, string; an enum gives its option id, a link gives `Node`; other kinds are an error |
| `object.world_matrix` | `object: Node` | `out: Matrix` | world matrix of the evaluated scene |
| `object.set_param` | `object: Node` (empty = host), `param: String`, `value: Any`, `enabled: Bool` (true) | - (sink) | appends a `ParamWrite` when `enabled` |
| `script.rhai` | `source: String`, `a`,`b`,`c`,`d: Float`, `v: Vec3`, `flag: Bool` | `result: Float`, `vec: Vec3`, `flag: Bool` | §4.1 |

### 4.1 The Rhai node

`script.rhai` runs `source` in a fresh sandboxed engine. The variables `a b c d` (floats),
`v` (an array `[x, y, z]`), `flag` (bool), `time` (seconds) and `frame` are in scope. The
script value is mapped to the typed outputs: a number sets `result`; a bool sets `flag`
(and `result` to 0/1); an array of three numbers sets `vec`; an object map may set any of
the keys `result`, `vec`, `flag` (missing ones are 0, `[0,0,0]`, false; an unknown key or a
wrongly typed value is an error); `()` is an error. The script is a pure function of its
inputs: it has **no command access and no scene access**.

Sandbox rules are those of docs/specs/SCRIPTING.md §4 and §5 (`forge-script`, brief 016):
no modules (`no_module`), no clock (`no_time`), `eval` disabled, strict variables, and the
limits `SandboxLimits::default()` = 5 000 000 operations, 10 s wall clock (checked every 1024
operations), call depth 48, expression depth 64/32, strings 1 MiB, arrays and maps 100 000.
A test in the repository asserts that the numbers equal `forge_script::ScriptOptions::default()`.
`forge-nodes` cannot depend on `forge-script` (that crate depends on `forge-app`, which depends
on this one), so it pins the same `rhai` version and repeats the rules. A limit stops the node
with an error that names the limit.

## 5. The expression tag

Node type `forge.expression` (category Tag, plugin `NodesToolset`), params:

| id | kind | default | note |
|---|---|---|---|
| `enabled` | bool | true | a disabled tag does nothing |
| `graph` | string, hidden | the empty graph | the graph as JSON (`Graph::to_json`) |

The graph lives in a string parameter so that it is saved by the native file format with no
new value kind and edited by `set_param`, which makes every graph edit one undo step. The
JSON is deterministic (ordered maps, the id counter, nodes by id, links by target).

**`apply_expressions(scene, seconds) -> (Scene, Vec<Diagnostic>)`** is the evaluation hook. It
visits the objects in document order (an object, then its tags in tag order, then its children,
depth first). For each enabled `forge.expression` tag it evaluates the graph with the tag's host as
`host`, and applies the resulting `ParamWrite`s with `Scene::with_params` before the next tag runs,
so a later expression sees the writes of an earlier one. A write whose node or param does not exist
or whose value does not validate is skipped; a graph that fails to parse or evaluate is skipped;
each is reported as a `Diagnostic { tag, message }` and never aborts the pass. The result is a copy of
the scene (not an undoable edit), exactly like `forge_anim::scene_at`.

**Evaluation order** (area H, CORE.md §5a): for one frame, `forge-app` produces the evaluated scene as

1. animation tracks: `scene_at(scene, t)`;
2. expression tags: `apply_expressions(evaluated, t)`, in document order;
3. generators and deformers, which read the evaluated scene's parameters (they run when a
   consumer asks for geometry, after the two steps above).

So an expression overrides an animated value of the same parameter, and a generator always sees
the expression's output. Priorities between expression tags and the other tag kinds (constraints, IK)
are brief 111; until then expression tags run in document order and before everything else.

## 6. Commands

All take `tag` (the id of a `forge.expression` node); every mutating command is one undo step
(`Edit Expression`-style labels) and fails without changing anything.

| id | arguments | result | undoable |
|---|---|---|---|
| `graph.add_node` | `tag`, `type`, `values?` (port id to JSON value), `pos?` (`[x, y]`) | `{node}` | yes |
| `graph.remove` | `tag`, `node?`, `nodes?` (one or a list, one undo step) | `{removed_links}` | yes |
| `graph.connect` | `tag`, `from_node`, `from_port`, `to_node`, `to_port`, `release_node?`, `release_port?` | `{replaced}` | yes |
| `graph.move` | `tag`, `moves` (list of `{node, x, y}`), `gesture?` | `{moved}` | yes |
| `graph.disconnect` | `tag`, `to_node`, `to_port` | null | yes |
| `graph.set_value` | `tag`, `node`, `port`, `value`, `gesture?` | null | yes |
| `graph.get` | `tag` | `{nodes: [{id, type, values, pos}], links: [{from, to}], order}` (`order` is null if the graph has a cycle) | no |
| `graph.node_types` | - | the registered node types with their ports | no |
| `graph.evaluate` | `tag` | `{outputs, writes, ran}` of a dry run of every node at the timeline's current frame, on the evaluated scene; changes nothing | no |

**Editor additions (brief 109).** A graph node has an editor position `pos` (`[x, y]` in canvas
units, default `[0, 0]`, saved in the graph JSON as `pos` only when it is not `[0, 0]`, so graphs
that were never positioned keep their old text). It has no effect on evaluation. `graph.move`
sets the positions of several nodes at once (absolute values; a non-finite number or an unknown
node fails and changes nothing). `graph.add_node`
`pos` places the new node. `graph.remove` takes `node` or `nodes` (at least one; all or none),
removing every link that touches them in one step. `graph.connect` `release_node` and
`release_port` name an input whose link is removed in the same step before the new link is made
(moving a link from one input to another; if the connection is refused nothing changes, the
released link stays). `gesture` (a number) on `graph.move` and `graph.set_value` merges edits
with the same gesture, tag and nodes into one undo step (`graph-move:<tag>:<gesture>`,
`graph-value:<tag>:<node>:<port>:<gesture>`), like `param.edit`.

JSON values follow the port type: number (Float; Int needs an integer), bool, `[x, y, z]`
(Vec3, Colour), 16 numbers (Matrix), string, node id or null (Node). For `Any` ports a JSON
integer is an Int, another number a Float, `[x,y,z]` a Vec3, null a Node. `object.create`
with type `forge.expression` adds the tag to the active object, like any tag.

## 7. File format

Brief 108 raises the native file format to **version 8** (CORE.md §6.7). Nothing in the scene
schema changes: the graph is an ordinary string parameter. The bump declares the new node type
and parameter encoding, so that builds without this feature reject the file instead of loading
a scene whose expression tag they would silently drop (unknown types are kept as-is but not
evaluated). Step 7 -> 8 has nothing to rewrite.

## 8. Tests

`crates/nodes/tests/` (spec section in brackets):

- `ports.rs`: the implicit and explicit conversion tables, `Any` rules, refusal of unsupported
  links [§2].
- `graph.rs`: connect/disconnect/set_value/remove validation, stable ids, cycle rejection (self,
  two-node, long), topological order and its determinism, caching (a diamond runs the shared node
  once), evaluating only what sinks need, a cyclic graph loaded from JSON is an error, JSON round trip [§3].
- `nodes.rs`: the maths of every node type [§4].
- `rhai.rs`: typed inputs/outputs of the Rhai node, the sandbox (operations, call depth, no modules,
  no `eval`, strings), the limits equal `ScriptOptions::default()` [§4.1].
- `expression.rs` (headless `App`): an expression tag built only with commands moves a cube from time;
  an expression overrides a keyed value; order of two expressions; undo of each command; a failing graph is a
  diagnostic; save/load round trip; v7 file migration; evaluation by `App::evaluated` [§5-§7].
- `crates/nodes/tests/editor_commands.rs` (brief 109): `pos` round trip and the unchanged JSON of
  unpositioned graphs, `graph.move` (several nodes, one step, gesture merge, bad input changes nothing),
  `graph.add_node` `pos`, `graph.remove` with a list, `graph.connect` with `release_*` (one step,
  refusal keeps the old link), `graph.set_value` gesture merge, `graph.get` `pos`.
