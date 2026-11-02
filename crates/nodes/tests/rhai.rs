//! docs/specs/NODES.md section 4.1: the Rhai node and its sandbox.

use std::time::Duration;

use forge_nodes::{
    EvalEnv, Graph, GraphError, GraphNodeId as Id, NodeRegistry, PortRef, PortValue as V,
    SandboxLimits,
};

fn script(
    source: &str,
    inputs: &[(&str, V)],
    env: &EvalEnv,
) -> Result<(Id, forge_nodes::Evaluation), GraphError> {
    let r = NodeRegistry::builtin();
    let mut g = Graph::new();
    let n = g.add_node(r, "script.rhai").unwrap();
    g.set_value(r, n, "source", V::String(source.into()))
        .unwrap();
    for (port, v) in inputs {
        g.set_value(r, n, port, v.clone()).unwrap();
    }
    g.evaluate_all(r, env).map(|e| (n, e))
}

fn run(source: &str, inputs: &[(&str, V)]) -> Result<(Id, forge_nodes::Evaluation), GraphError> {
    script(source, inputs, &EvalEnv::headless(2.0))
}

fn message(r: Result<(Id, forge_nodes::Evaluation), GraphError>) -> String {
    match r {
        Err(GraphError::Eval { message, .. }) => message,
        other => panic!("expected an eval error, got {other:?}"),
    }
}

#[test]
fn numbers_set_the_result_output() {
    let (n, ev) = run("a * 2.0 + b", &[("a", V::Float(3.0)), ("b", V::Float(1.0))]).unwrap();
    assert_eq!(ev.float(n, "result"), Some(7.0));
    assert_eq!(ev.output(n, "vec"), Some(&V::Vec3([0.0; 3])));
    assert_eq!(ev.output(n, "flag"), Some(&V::Bool(false)));
    // integers are accepted
    let (n, ev) = run("40 + 2", &[]).unwrap();
    assert_eq!(ev.float(n, "result"), Some(42.0));
}

#[test]
fn inputs_time_and_frame_are_in_scope() {
    let (n, ev) = run(
        "a + b + c + d + v[0] + v[1] + v[2] + time + frame + (if flag { 1000.0 } else { 0.0 })",
        &[
            ("a", V::Float(1.0)),
            ("b", V::Float(2.0)),
            ("c", V::Float(3.0)),
            ("d", V::Float(4.0)),
            ("v", V::Vec3([10.0, 20.0, 30.0])),
            ("flag", V::Bool(true)),
        ],
    )
    .unwrap();
    // time 2.0 s at 30 fps: frame 60
    assert_eq!(
        ev.float(n, "result"),
        Some(1.0 + 2.0 + 3.0 + 4.0 + 60.0 + 2.0 + 60.0 + 1000.0)
    );
}

#[test]
fn bools_arrays_and_maps_set_the_matching_outputs() {
    let (n, ev) = run("a > 1.0", &[("a", V::Float(2.0))]).unwrap();
    assert_eq!(ev.output(n, "flag"), Some(&V::Bool(true)));
    assert_eq!(ev.float(n, "result"), Some(1.0));
    let (n, ev) = run("[a, a * 2.0, 3]", &[("a", V::Float(1.5))]).unwrap();
    assert_eq!(ev.output(n, "vec"), Some(&V::Vec3([1.5, 3.0, 3.0])));
    let (n, ev) = run("#{ result: 5, vec: [1.0, 2.0, 3.0], flag: true }", &[]).unwrap();
    assert_eq!(ev.float(n, "result"), Some(5.0));
    assert_eq!(ev.output(n, "vec"), Some(&V::Vec3([1.0, 2.0, 3.0])));
    assert_eq!(ev.output(n, "flag"), Some(&V::Bool(true)));
    // missing keys take their defaults
    let (n, ev) = run("#{ result: 2.5 }", &[]).unwrap();
    assert_eq!(ev.float(n, "result"), Some(2.5));
    assert_eq!(ev.output(n, "flag"), Some(&V::Bool(false)));
}

#[test]
fn wrong_output_types_are_errors() {
    assert!(message(run("()", &[])).contains("nothing"));
    assert!(message(run("\"text\"", &[])).contains("expected"));
    assert!(message(run("[1.0, 2.0]", &[])).contains("expected"));
    assert!(message(run("#{ result: \"x\" }", &[])).contains("result"));
    assert!(message(run("#{ vec: [1, 2] }", &[])).contains("vec"));
    assert!(message(run("#{ flag: 1 }", &[])).contains("flag"));
    assert!(message(run("#{ other: 1 }", &[])).contains("other"));
    assert!(message(run("let x = ;", &[])).contains("syntax"));
    assert!(
        message(run("undefined_name + 1", &[]))
            .to_lowercase()
            .contains("undefined_name")
    );
}

#[test]
fn typed_ports_connect_to_the_script() {
    let r = NodeRegistry::builtin();
    let mut g = Graph::new();
    let t = g.add_node(r, "time.now").unwrap();
    let s = g.add_node(r, "script.rhai").unwrap();
    let c = g.add_node(r, "const.vec3").unwrap();
    g.set_value(r, s, "source", V::String("a * v[2]".into()))
        .unwrap();
    g.set_value(r, c, "value", V::Vec3([0.0, 0.0, 4.0]))
        .unwrap();
    g.connect(r, PortRef::new(t, "seconds"), PortRef::new(s, "a"))
        .unwrap();
    g.connect(r, PortRef::new(c, "out"), PortRef::new(s, "v"))
        .unwrap();
    // a string output cannot feed a float input
    let str_node = g.add_node(r, "const.string").unwrap();
    assert!(
        g.connect(r, PortRef::new(str_node, "out"), PortRef::new(s, "a"))
            .is_err()
    );
    let ev = g.evaluate_for(r, &EvalEnv::headless(1.5), &[s]).unwrap();
    assert_eq!(ev.float(s, "result"), Some(6.0));
}

#[test]
fn scripts_are_deterministic() {
    let src = "let s = 0.0; for i in 0..100 { s += sin(i * a); } s";
    let one = run(src, &[("a", V::Float(0.37))]).unwrap();
    let two = run(src, &[("a", V::Float(0.37))]).unwrap();
    assert_eq!(one.1.output(one.0, "result"), two.1.output(two.0, "result"));
}

#[test]
fn the_sandbox_stops_runaway_scripts() {
    let mut env = EvalEnv::headless(0.0);
    env.limits.max_operations = 10_000;
    let m = message(script("loop {}", &[], &env));
    assert!(m.contains("operation limit (10000)"), "{m}");

    let mut env = EvalEnv::headless(0.0);
    env.limits.max_call_levels = 8;
    let m = message(script("fn f(n) { f(n + 1) } f(0)", &[], &env));
    assert!(m.contains("call depth limit (8)"), "{m}");

    let mut env = EvalEnv::headless(0.0);
    env.limits.timeout = Duration::ZERO;
    let m = message(script("loop {}", &[], &env));
    assert!(m.contains("time limit"), "{m}");

    // data size: a string that doubles hits the 1 MiB cap long before the operation limit
    let m = message(run("let s = \"xxxxxxxx\"; loop { s += s; }", &[]));
    assert!(m.contains("too large"), "{m}");
    let m = message(run("let a = [1, 2]; loop { a += a; }", &[]));
    assert!(m.contains("too large"), "{m}");
}

#[test]
fn the_sandbox_has_no_modules_eval_clock_or_commands() {
    for src in [
        "import \"x\" as m; 1",
        "eval(\"1 + 1\")",
        "timestamp()",
        "run(\"object.create\", #{ type: \"forge.cube\" })",
        "create(\"forge.cube\")",
        "scene()",
    ] {
        let m = message(run(src, &[]));
        assert!(!m.is_empty(), "{src}");
    }
    // print and debug are accepted and discarded
    let (n, ev) = run("print(\"hello\"); debug(1); 3", &[]).unwrap();
    assert_eq!(ev.float(n, "result"), Some(3.0));
}

#[test]
fn the_limits_equal_the_script_crates_options() {
    let ours = SandboxLimits::default();
    let theirs = forge_script::ScriptOptions::default();
    assert_eq!(ours.max_operations, theirs.max_operations);
    assert_eq!(ours.timeout, theirs.timeout);
    assert_eq!(ours.max_call_levels, theirs.max_call_levels);
}
