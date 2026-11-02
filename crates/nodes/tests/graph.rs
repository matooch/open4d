//! docs/specs/NODES.md section 3: graph edits, cycles, topological order, caching.

use std::sync::atomic::{AtomicUsize, Ordering};

use forge_nodes::{
    EvalEnv, Graph, GraphError, GraphNodeId as Id, NodeDef, NodePlugin, NodeRegistry, PortRef,
    PortType as T, PortValue as V, RegistryError,
};

fn reg() -> NodeRegistry {
    NodeRegistry::with_builtins()
}

fn p(node: Id, port: &str) -> PortRef {
    PortRef::new(node, port)
}

fn env() -> EvalEnv<'static> {
    EvalEnv::headless(0.0)
}

#[test]
fn connect_checks_ports_and_types() {
    let r = reg();
    let mut g = Graph::new();
    let s = g.add_node(&r, "const.string").unwrap();
    let i = g.add_node(&r, "const.int").unwrap();
    let fl = g.add_node(&r, "const.float").unwrap();
    let add = g.add_node(&r, "math.add").unwrap();
    let v = g.add_node(&r, "vector.compose").unwrap();
    let vc = g.add_node(&r, "const.vec3").unwrap();
    let cond = g.add_node(&r, "logic.condition").unwrap();

    // string -> float is refused
    assert_eq!(
        g.connect(&r, p(s, "out"), p(add, "a")),
        Err(GraphError::TypeMismatch {
            from: T::String,
            to: T::Float
        })
    );
    // float -> int needs an explicit conversion
    assert!(matches!(
        g.connect(&r, p(add, "out"), p(i, "value")),
        Err(GraphError::TypeMismatch { .. })
    ));
    // implicit: int -> float, float -> vec3, vec3 -> vec3
    assert_eq!(g.connect(&r, p(i, "out"), p(add, "a")), Ok(None));
    assert_eq!(g.connect(&r, p(fl, "out"), p(vc, "value")), Ok(None));
    assert_eq!(g.connect(&r, p(v, "out"), p(cond, "if_true")), Ok(None)); // vec3 -> any
    // any -> typed is allowed (checked at run time)
    assert_eq!(g.connect(&r, p(cond, "out"), p(add, "b")), Ok(None));
    // unknown ports and nodes
    assert!(matches!(
        g.connect(&r, p(fl, "nope"), p(add, "a")),
        Err(GraphError::UnknownPort { kind: "output", .. })
    ));
    assert!(matches!(
        g.connect(&r, p(fl, "out"), p(add, "nope")),
        Err(GraphError::UnknownPort { kind: "input", .. })
    ));
    // an output port is not an input
    assert!(matches!(
        g.connect(&r, p(fl, "out"), p(add, "out")),
        Err(GraphError::UnknownPort { kind: "input", .. })
    ));
    assert_eq!(
        g.connect(&r, p(fl, "out"), p(Id(99), "a")),
        Err(GraphError::UnknownNode(Id(99)))
    );
    assert!(matches!(
        g.add_node(&r, "nope.nope"),
        Err(GraphError::UnknownType(_))
    ));
}

#[test]
fn an_input_has_one_link_and_connecting_again_replaces_it() {
    let r = reg();
    let mut g = Graph::new();
    let a = g.add_node(&r, "const.float").unwrap();
    let b = g.add_node(&r, "const.float").unwrap();
    let add = g.add_node(&r, "math.add").unwrap();
    assert_eq!(g.connect(&r, p(a, "out"), p(add, "a")), Ok(None));
    let old = g.connect(&r, p(b, "out"), p(add, "a")).unwrap().unwrap();
    assert_eq!(old.from, p(a, "out"));
    assert_eq!(g.links().len(), 1);
    // an output may feed several inputs
    g.connect(&r, p(b, "out"), p(add, "b")).unwrap();
    assert_eq!(g.links().len(), 2);
    let gone = g.disconnect(&p(add, "a")).unwrap();
    assert_eq!(gone.from, p(b, "out"));
    assert!(matches!(
        g.disconnect(&p(add, "a")),
        Err(GraphError::NotLinked(_))
    ));
}

#[test]
fn set_value_converts_to_the_port_type() {
    let r = reg();
    let mut g = Graph::new();
    let add = g.add_node(&r, "math.add").unwrap();
    g.set_value(&r, add, "a", V::Int(3)).unwrap();
    assert_eq!(g.constant_of(&r, add, "a"), Ok(V::Float(3.0)));
    assert_eq!(g.constant_of(&r, add, "b"), Ok(V::Float(0.0))); // the default
    assert!(g.set_value(&r, add, "a", V::String("x".into())).is_err());
    assert!(g.set_value(&r, add, "a", V::Float(f64::INFINITY)).is_err());
    assert!(matches!(
        g.set_value(&r, add, "zz", V::Float(1.0)),
        Err(GraphError::UnknownPort { .. })
    ));
    // the failed edits changed nothing
    assert_eq!(g.constant_of(&r, add, "a"), Ok(V::Float(3.0)));
}

#[test]
fn cycles_are_rejected_and_leave_the_graph_unchanged() {
    let r = reg();
    let mut g = Graph::new();
    let a = g.add_node(&r, "math.add").unwrap();
    // a node cannot feed itself
    let before = g.clone();
    assert!(matches!(
        g.connect(&r, p(a, "out"), p(a, "a")),
        Err(GraphError::Cycle(path)) if path == vec![a]
    ));
    assert_eq!(g, before);

    // two nodes
    let b = g.add_node(&r, "math.add").unwrap();
    g.connect(&r, p(a, "out"), p(b, "a")).unwrap();
    let before = g.clone();
    assert!(matches!(
        g.connect(&r, p(b, "out"), p(a, "b")),
        Err(GraphError::Cycle(path)) if path == vec![a, b]
    ));
    assert_eq!(g, before);

    // a long chain, closed at the end
    let mut chain = vec![b];
    for _ in 0..20 {
        let n = g.add_node(&r, "math.add").unwrap();
        g.connect(&r, p(*chain.last().unwrap(), "out"), p(n, "a"))
            .unwrap();
        chain.push(n);
    }
    assert!(matches!(
        g.connect(&r, p(*chain.last().unwrap(), "out"), p(a, "a")),
        Err(GraphError::Cycle(_))
    ));
    assert!(g.topo_order().is_ok());
    // the same pair in the other direction is fine (a diamond is not a cycle)
    g.connect(&r, p(a, "out"), p(*chain.last().unwrap(), "b"))
        .unwrap();
}

#[test]
fn topological_order_respects_links_and_is_deterministic() {
    let r = reg();
    let mut g = Graph::new();
    // ids 1..=5 created in an order unrelated to the data flow
    let n: Vec<Id> = (0..5)
        .map(|_| g.add_node(&r, "math.add").unwrap())
        .collect();
    g.connect(&r, p(n[4], "out"), p(n[2], "a")).unwrap();
    g.connect(&r, p(n[2], "out"), p(n[0], "a")).unwrap();
    g.connect(&r, p(n[4], "out"), p(n[1], "a")).unwrap();
    g.connect(&r, p(n[1], "out"), p(n[0], "b")).unwrap();
    let order = g.topo_order().unwrap();
    assert_eq!(order, g.topo_order().unwrap());
    let pos = |id: Id| order.iter().position(|x| *x == id).unwrap();
    for l in g.links() {
        assert!(pos(l.from.node) < pos(l.to.node), "{l:?}");
    }
    // sources first, ties by id: n3 (isolated) and n4 are ready first
    assert_eq!(order, vec![n[3], n[4], n[1], n[2], n[0]]);
}

#[test]
fn a_cyclic_graph_loaded_from_json_cannot_be_ordered_or_run() {
    let r = reg();
    let mut g = Graph::new();
    let a = g.add_node(&r, "math.add").unwrap();
    let b = g.add_node(&r, "math.add").unwrap();
    g.connect(&r, p(a, "out"), p(b, "a")).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&g.to_json()).unwrap();
    json["links"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "from": {"node": b.0, "port": "out"},
            "to": {"node": a.0, "port": "a"},
        }));
    let bad = Graph::from_json(&json.to_string()).unwrap();
    assert!(matches!(bad.topo_order(), Err(GraphError::Cycle(ids)) if ids == vec![a, b]));
    assert!(matches!(
        bad.evaluate_all(&r, &env()),
        Err(GraphError::Cycle(_))
    ));
    assert!(Graph::from_json("not json").is_err());
}

#[test]
fn removing_a_node_drops_its_links_and_ids_are_never_reused() {
    let r = reg();
    let mut g = Graph::new();
    let a = g.add_node(&r, "const.float").unwrap();
    let b = g.add_node(&r, "math.add").unwrap();
    let c = g.add_node(&r, "math.add").unwrap();
    g.connect(&r, p(a, "out"), p(b, "a")).unwrap();
    g.connect(&r, p(b, "out"), p(c, "a")).unwrap();
    assert_eq!(g.remove_node(b), Ok(2));
    assert!(g.links().is_empty());
    assert_eq!(g.remove_node(b), Err(GraphError::UnknownNode(b)));
    let d = g.add_node(&r, "math.add").unwrap();
    assert!(d > c, "{d} {c}");
    // the counter survives serialization
    let g2 = Graph::from_json(&g.to_json()).unwrap();
    let mut g2 = g2;
    let e = g2.add_node(&r, "math.add").unwrap();
    assert!(e > d);
}

#[test]
fn graphs_round_trip_through_json_deterministically() {
    let r = reg();
    let mut g = Graph::new();
    let t = g.add_node(&r, "time.now").unwrap();
    let m = g.add_node(&r, "math.mul").unwrap();
    g.connect(&r, p(t, "seconds"), p(m, "a")).unwrap();
    g.set_value(&r, m, "b", V::Float(100.0)).unwrap();
    let n = g.add_node(&r, "const.string").unwrap();
    g.set_value(&r, n, "value", V::String("hi".into())).unwrap();
    let text = g.to_json();
    let back = Graph::from_json(&text).unwrap();
    assert_eq!(back, g);
    assert_eq!(back.to_json(), text);
    let again = {
        let mut h = Graph::new();
        let t = h.add_node(&r, "time.now").unwrap();
        let m = h.add_node(&r, "math.mul").unwrap();
        h.connect(&r, p(t, "seconds"), p(m, "a")).unwrap();
        h.set_value(&r, m, "b", V::Float(100.0)).unwrap();
        let n = h.add_node(&r, "const.string").unwrap();
        h.set_value(&r, n, "value", V::String("hi".into())).unwrap();
        h.to_json()
    };
    assert_eq!(again, text, "same edits give the same text");
}

static COUNT: AtomicUsize = AtomicUsize::new(0);

struct CountingNodes;

impl NodePlugin for CountingNodes {
    fn id(&self) -> &str {
        "test.counting"
    }

    fn register(&self, r: &mut NodeRegistry) -> Result<(), RegistryError> {
        r.register(
            NodeDef::new("test.count", "Count", "test", |c| {
                COUNT.fetch_add(1, Ordering::SeqCst);
                Ok(vec![V::Float(c.float("x") + 1.0)])
            })
            .input("x", T::Float, V::Float(0.0))
            .output("out", T::Float),
        )
    }
}

#[test]
fn a_shared_node_runs_once_per_evaluation() {
    let mut r = reg();
    r.install(&CountingNodes).unwrap();
    let mut g = Graph::new();
    // diamond: src -> (left, right) -> sum
    let src = g.add_node(&r, "test.count").unwrap();
    let left = g.add_node(&r, "math.mul").unwrap();
    let right = g.add_node(&r, "math.add").unwrap();
    let sum = g.add_node(&r, "math.add").unwrap();
    g.connect(&r, p(src, "out"), p(left, "a")).unwrap();
    g.connect(&r, p(src, "out"), p(right, "a")).unwrap();
    g.connect(&r, p(left, "out"), p(sum, "a")).unwrap();
    g.connect(&r, p(right, "out"), p(sum, "b")).unwrap();
    g.set_value(&r, left, "b", V::Float(2.0)).unwrap();
    COUNT.store(0, Ordering::SeqCst);
    let ev = g.evaluate_for(&r, &env(), &[sum]).unwrap();
    assert_eq!(COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(ev.ran.iter().filter(|n| **n == src).count(), 1);
    assert_eq!(ev.ran.len(), 4);
    // src = 1; left = 2; right = 1; sum = 3
    assert_eq!(ev.float(sum, "out"), Some(3.0));
    // each evaluation starts fresh: nothing is cached across evaluations
    g.evaluate_for(&r, &env(), &[sum]).unwrap();
    assert_eq!(COUNT.load(Ordering::SeqCst), 2);
}

#[test]
fn only_what_the_sinks_need_runs() {
    let mut r = reg();
    r.install(&CountingNodes).unwrap();
    let mut g = Graph::new();
    let unused = g.add_node(&r, "test.count").unwrap();
    let c = g.add_node(&r, "const.float").unwrap();
    let ev = g.evaluate(&r, &env()).unwrap();
    assert!(ev.ran.is_empty(), "no sink, nothing runs");
    let all = g.evaluate_all(&r, &env()).unwrap();
    assert_eq!(all.ran, vec![unused, c]);
    let some = g.evaluate_for(&r, &env(), &[c]).unwrap();
    assert_eq!(some.ran, vec![c]);
    assert_eq!(
        g.evaluate_for(&r, &env(), &[Id(77)]).err(),
        Some(GraphError::UnknownNode(Id(77)))
    );
}

#[test]
fn eval_errors_name_the_node_and_stop_the_run() {
    let r = reg();
    let mut g = Graph::new();
    let c = g.add_node(&r, "compare.compare").unwrap();
    g.set_value(&r, c, "op", V::String("bogus".into())).unwrap();
    match g.evaluate_all(&r, &env()) {
        Err(GraphError::Eval { node, message }) => {
            assert_eq!(node, c);
            assert!(message.contains("bogus"), "{message}");
        }
        other => panic!("{other:?}"),
    }
    // an Any output that cannot convert to the input it feeds fails at run time
    let mut g = Graph::new();
    let cond = g.add_node(&r, "logic.condition").unwrap();
    let add = g.add_node(&r, "math.add").unwrap();
    g.set_value(&r, cond, "if_true", V::String("text".into()))
        .unwrap();
    g.set_value(&r, cond, "condition", V::Bool(true)).unwrap();
    g.connect(&r, p(cond, "out"), p(add, "a")).unwrap();
    match g.evaluate_all(&r, &env()) {
        Err(GraphError::Eval { node, .. }) => assert_eq!(node, add),
        other => panic!("{other:?}"),
    }
    // and works when the value happens to fit
    g.set_value(&r, cond, "if_true", V::Int(4)).unwrap();
    let ev = g.evaluate_all(&r, &env()).unwrap();
    assert_eq!(ev.float(add, "out"), Some(4.0));
}

#[test]
fn registry_validates_definitions() {
    let mut r = NodeRegistry::new();
    let ok =
        |id: &str| NodeDef::new(id, "x", "t", |_| Ok(vec![])).input("a", T::Float, V::Float(0.0));
    assert!(matches!(
        r.register(ok("nodots")),
        Err(RegistryError::InvalidTypeId(_))
    ));
    assert!(matches!(
        r.register(ok("Up.case")),
        Err(RegistryError::InvalidTypeId(_))
    ));
    r.register(ok("t.one")).unwrap();
    assert!(matches!(
        r.register(ok("t.one")),
        Err(RegistryError::Duplicate(_))
    ));
    let dup = NodeDef::new("t.dup", "x", "t", |_| Ok(vec![]))
        .input("a", T::Float, V::Float(0.0))
        .input("a", T::Float, V::Float(0.0));
    assert!(matches!(
        r.register(dup),
        Err(RegistryError::InvalidPort { .. })
    ));
    let bad_default =
        NodeDef::new("t.bad", "x", "t", |_| Ok(vec![])).input("a", T::Float, V::String("x".into()));
    assert!(matches!(
        r.register(bad_default),
        Err(RegistryError::InvalidDefault { .. })
    ));
    // a plugin installs all or nothing
    struct Half;
    impl NodePlugin for Half {
        fn id(&self) -> &str {
            "half"
        }
        fn register(&self, r: &mut NodeRegistry) -> Result<(), RegistryError> {
            r.register(NodeDef::new("t.fresh", "x", "t", |_| Ok(vec![])))?;
            r.register(NodeDef::new("t.one", "x", "t", |_| Ok(vec![])))
        }
    }
    assert!(r.install(&Half).is_err());
    assert!(r.get("t.fresh").is_none());
    // the built-in registry is complete and well formed
    let b = NodeRegistry::builtin();
    assert!(b.defs().count() >= 35);
    for d in b.defs() {
        assert!(!d.label.is_empty());
        assert!(d.sink || !d.outputs.is_empty(), "{} has no outputs", d.id);
    }
}
