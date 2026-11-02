//! docs/specs/NODES.md section 4: the maths of every built-in node type.

use forge_nodes::{
    EvalEnv, Graph, GraphError, GraphNodeId as Id, NodeRegistry, PortRef, PortType as T,
    PortValue as V,
};

/// Runs one node of `ty` with the given input constants and returns (graph, node, evaluation).
fn run(ty: &str, values: &[(&str, V)]) -> Result<(Id, forge_nodes::Evaluation), GraphError> {
    run_at(ty, values, 0.0)
}

fn run_at(
    ty: &str,
    values: &[(&str, V)],
    seconds: f64,
) -> Result<(Id, forge_nodes::Evaluation), GraphError> {
    let r = NodeRegistry::builtin();
    let mut g = Graph::new();
    let n = g.add_node(r, ty).unwrap();
    for (port, v) in values {
        g.set_value(r, n, port, v.clone()).unwrap();
    }
    let mut env = EvalEnv::headless(seconds);
    env.fps = 24.0;
    g.evaluate_all(r, &env).map(|e| (n, e))
}

fn out(ty: &str, values: &[(&str, V)]) -> V {
    let (n, ev) = run(ty, values).unwrap();
    let def = NodeRegistry::builtin().get(ty).unwrap().clone();
    ev.output(n, &def.outputs[0].id).unwrap().clone()
}

fn fl(ty: &str, values: &[(&str, f64)]) -> f64 {
    let values: Vec<(&str, V)> = values.iter().map(|(k, v)| (*k, V::Float(*v))).collect();
    match out(ty, &values) {
        V::Float(x) => x,
        other => panic!("{ty}: {other:?}"),
    }
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} vs {b}");
}

#[test]
fn constants_pass_their_value_through() {
    assert_eq!(
        out("const.float", &[("value", V::Float(2.5))]),
        V::Float(2.5)
    );
    assert_eq!(out("const.int", &[("value", V::Int(7))]), V::Int(7));
    assert_eq!(
        out("const.bool", &[("value", V::Bool(true))]),
        V::Bool(true)
    );
    assert_eq!(
        out("const.vec3", &[("value", V::Vec3([1.0, 2.0, 3.0]))]),
        V::Vec3([1.0, 2.0, 3.0])
    );
    assert_eq!(
        out("const.colour", &[("value", V::Colour([0.5, 1.0, 2.0]))]),
        V::Colour([0.5, 1.0, 2.0])
    );
    assert_eq!(
        out("const.string", &[("value", V::String("x".into()))]),
        V::String("x".into())
    );
    // constants convert to their port type: an int into a float constant
    assert_eq!(out("const.float", &[("value", V::Int(2))]), V::Float(2.0));
}

#[test]
fn time_node_reports_seconds_frame_and_fps() {
    let (n, ev) = run_at("time.now", &[], 1.5).unwrap();
    assert_eq!(ev.float(n, "seconds"), Some(1.5));
    assert_eq!(ev.float(n, "frame"), Some(36.0)); // 1.5 s at 24 fps
    assert_eq!(ev.float(n, "fps"), Some(24.0));
}

#[test]
fn arithmetic() {
    close(fl("math.add", &[("a", 1.5), ("b", 2.0)]), 3.5);
    close(fl("math.sub", &[("a", 1.5), ("b", 2.0)]), -0.5);
    close(fl("math.mul", &[("a", 1.5), ("b", 2.0)]), 3.0);
    close(fl("math.div", &[("a", 3.0), ("b", 2.0)]), 1.5);
    // division by zero is 0 (spec section 4)
    close(fl("math.div", &[("a", 3.0), ("b", 0.0)]), 0.0);
    // ints and bools convert implicitly on the inputs
    assert_eq!(
        out("math.add", &[("a", V::Int(2)), ("b", V::Bool(true))]),
        V::Float(3.0)
    );
}

#[test]
fn mix_clamp_and_remap() {
    close(
        fl("math.mix", &[("a", 10.0), ("b", 20.0), ("t", 0.25)]),
        12.5,
    );
    close(
        fl("math.mix", &[("a", 10.0), ("b", 20.0), ("t", 2.0)]),
        30.0,
    ); // not clamped
    close(
        fl("math.clamp", &[("value", 5.0), ("min", 0.0), ("max", 1.0)]),
        1.0,
    );
    close(
        fl("math.clamp", &[("value", -5.0), ("min", 0.0), ("max", 1.0)]),
        0.0,
    );
    close(
        fl("math.clamp", &[("value", 0.3), ("min", 0.0), ("max", 1.0)]),
        0.3,
    );
    // swapped bounds are ordered
    close(
        fl("math.clamp", &[("value", 5.0), ("min", 3.0), ("max", 1.0)]),
        3.0,
    );
    close(
        fl("math.clamp", &[("value", -5.0), ("min", 3.0), ("max", 1.0)]),
        1.0,
    );

    let remap = |value: f64, clamp: bool| match out(
        "math.remap",
        &[
            ("value", V::Float(value)),
            ("in_min", V::Float(10.0)),
            ("in_max", V::Float(20.0)),
            ("out_min", V::Float(0.0)),
            ("out_max", V::Float(100.0)),
            ("clamp", V::Bool(clamp)),
        ],
    ) {
        V::Float(x) => x,
        other => panic!("{other:?}"),
    };
    close(remap(15.0, false), 50.0);
    close(remap(30.0, false), 200.0);
    close(remap(30.0, true), 100.0);
    close(remap(0.0, true), 0.0);
    // an empty input range maps to out_min
    close(
        fl(
            "math.remap",
            &[
                ("value", 5.0),
                ("in_min", 1.0),
                ("in_max", 1.0),
                ("out_min", 7.0),
                ("out_max", 9.0),
            ],
        ),
        7.0,
    );
}

#[test]
fn trigonometry() {
    close(fl("math.sin", &[("x", std::f64::consts::FRAC_PI_2)]), 1.0);
    close(fl("math.cos", &[("x", 0.0)]), 1.0);
    close(fl("math.cos", &[("x", std::f64::consts::PI)]), -1.0);
}

#[test]
fn noise_matches_the_library_and_is_deterministic() {
    use forge_noise::{Noise, NoiseKind};
    let p = [0.3, 1.7, -2.2];
    let (n, ev) = run(
        "math.noise",
        &[
            ("position", V::Vec3(p)),
            ("time", V::Float(0.5)),
            ("seed", V::Int(42)),
            ("scale", V::Float(2.0)),
            ("kind", V::String("simplex".into())),
        ],
    )
    .unwrap();
    let expected = Noise {
        kind: NoiseKind::Simplex,
        seed: 42,
        scale: 2.0,
        octaves: 4,
        ..Noise::default()
    }
    .sample4(p.into(), 0.5);
    assert_eq!(ev.float(n, "value"), Some(expected));
    close(ev.float(n, "unit").unwrap(), 0.5 + 0.5 * expected);
    let (n2, ev2) = run(
        "math.noise",
        &[
            ("position", V::Vec3(p)),
            ("time", V::Float(0.5)),
            ("seed", V::Int(42)),
            ("scale", V::Float(2.0)),
            ("kind", V::String("simplex".into())),
        ],
    )
    .unwrap();
    assert_eq!(ev.output(n, "value"), ev2.output(n2, "value"));
    // a different seed gives a different field; every kind is in range
    let seeded = |seed: i64| {
        let (n, ev) = run(
            "math.noise",
            &[("position", V::Vec3(p)), ("seed", V::Int(seed))],
        )
        .unwrap();
        ev.float(n, "value").unwrap()
    };
    assert_ne!(seeded(1), seeded(2));
    for kind in [
        "perlin",
        "value",
        "simplex",
        "box",
        "cell",
        "voronoi1",
        "voronoi2",
        "voronoi3",
        "fbm",
        "turbulence",
        "ridged_multifractal",
        "wavy_turbulence",
        "electric",
        "gaseous",
        "blistered_turbulence",
        "displaced_turbulence",
        "displaced_voronoi",
    ] {
        assert!(forge_nodes::noise_kind(kind).is_some(), "{kind}");
        let (n, ev) = run(
            "math.noise",
            &[("position", V::Vec3(p)), ("kind", V::String(kind.into()))],
        )
        .unwrap();
        let v = ev.float(n, "value").unwrap();
        assert!((-1.0..=1.0).contains(&v), "{kind}: {v}");
    }
    assert!(forge_nodes::noise_kind("nope").is_none());
    for bad in [
        vec![("kind", V::String("nope".into()))],
        vec![("scale", V::Float(0.0))],
    ] {
        assert!(matches!(
            run("math.noise", &bad),
            Err(GraphError::Eval { .. })
        ));
    }
}

#[test]
fn vector_nodes() {
    assert_eq!(
        out(
            "vector.compose",
            &[
                ("x", V::Float(1.0)),
                ("y", V::Float(2.0)),
                ("z", V::Float(3.0))
            ]
        ),
        V::Vec3([1.0, 2.0, 3.0])
    );
    let (n, ev) = run("vector.decompose", &[("v", V::Vec3([4.0, 5.0, 6.0]))]).unwrap();
    assert_eq!(
        (ev.float(n, "x"), ev.float(n, "y"), ev.float(n, "z")),
        (Some(4.0), Some(5.0), Some(6.0))
    );
    // a float splats into the vector input
    let (n, ev) = run("vector.decompose", &[("v", V::Float(2.0))]).unwrap();
    assert_eq!(ev.float(n, "z"), Some(2.0));
    assert_eq!(
        out(
            "vector.add",
            &[
                ("a", V::Vec3([1.0, 2.0, 3.0])),
                ("b", V::Vec3([10.0, 20.0, 30.0]))
            ]
        ),
        V::Vec3([11.0, 22.0, 33.0])
    );
    assert_eq!(
        out(
            "vector.scale",
            &[("v", V::Vec3([1.0, 2.0, 3.0])), ("s", V::Float(2.0))]
        ),
        V::Vec3([2.0, 4.0, 6.0])
    );
    close(fl_v("vector.length", [3.0, 4.0, 12.0]), 13.0);
}

fn fl_v(ty: &str, v: [f64; 3]) -> f64 {
    match out(ty, &[("v", V::Vec3(v))]) {
        V::Float(x) => x,
        other => panic!("{other:?}"),
    }
}

#[test]
fn matrix_nodes() {
    let m = out(
        "matrix.from_translation",
        &[("v", V::Vec3([1.0, 2.0, 3.0]))],
    );
    let V::Matrix(cols) = &m else { panic!("{m:?}") };
    assert_eq!(&cols[12..], &[1.0, 2.0, 3.0, 1.0]);
    assert_eq!(&cols[..4], &[1.0, 0.0, 0.0, 0.0]);
    assert_eq!(
        out("matrix.translation", &[("m", m)]),
        V::Vec3([1.0, 2.0, 3.0])
    );
}

#[test]
fn comparisons_and_logic() {
    let cmp = |op: &str, a: f64, b: f64| {
        out(
            "compare.compare",
            &[
                ("a", V::Float(a)),
                ("b", V::Float(b)),
                ("op", V::String(op.into())),
            ],
        )
    };
    for (op, lt, eq, gt) in [
        ("lt", true, false, false),
        ("le", true, true, false),
        ("gt", false, false, true),
        ("ge", false, true, true),
        ("eq", false, true, false),
        ("ne", true, false, true),
    ] {
        assert_eq!(cmp(op, 1.0, 2.0), V::Bool(lt), "{op} 1 2");
        assert_eq!(cmp(op, 2.0, 2.0), V::Bool(eq), "{op} 2 2");
        assert_eq!(cmp(op, 3.0, 2.0), V::Bool(gt), "{op} 3 2");
    }
    let b = |a: bool, c: bool| [("a", V::Bool(a)), ("b", V::Bool(c))];
    assert_eq!(out("logic.and", &b(true, true)), V::Bool(true));
    assert_eq!(out("logic.and", &b(true, false)), V::Bool(false));
    assert_eq!(out("logic.or", &b(false, true)), V::Bool(true));
    assert_eq!(out("logic.or", &b(false, false)), V::Bool(false));
    assert_eq!(out("logic.not", &[("a", V::Bool(false))]), V::Bool(true));
}

#[test]
fn condition_picks_a_branch_of_any_type() {
    let pick = |c: bool, t: V, f: V| {
        out(
            "logic.condition",
            &[("condition", V::Bool(c)), ("if_true", t), ("if_false", f)],
        )
    };
    assert_eq!(pick(true, V::Float(1.0), V::Float(2.0)), V::Float(1.0));
    assert_eq!(pick(false, V::Float(1.0), V::Float(2.0)), V::Float(2.0));
    // branches may differ in type; the chosen one keeps its own
    assert_eq!(
        pick(true, V::Vec3([1.0, 2.0, 3.0]), V::Int(0)),
        V::Vec3([1.0, 2.0, 3.0])
    );
    assert_eq!(
        pick(false, V::Vec3([1.0; 3]), V::String("s".into())),
        V::String("s".into())
    );
}

#[test]
fn explicit_convert_nodes() {
    assert_eq!(
        out("convert.to_int", &[("value", V::Float(2.9))]),
        V::Int(2)
    );
    assert_eq!(
        out("convert.to_float", &[("value", V::String("1.25".into()))]),
        V::Float(1.25)
    );
    assert_eq!(
        out("convert.to_bool", &[("value", V::Float(3.0))]),
        V::Bool(true)
    );
    assert_eq!(
        out("convert.to_bool", &[("value", V::Int(0))]),
        V::Bool(false)
    );
    assert_eq!(
        out("convert.to_string", &[("value", V::Int(5))]),
        V::String("5".into())
    );
    assert_eq!(
        out("convert.to_float", &[("value", V::Vec3([0.0, 3.0, 4.0]))]),
        V::Float(5.0)
    );
    assert!(matches!(
        run("convert.to_float", &[("value", V::String("x".into()))]),
        Err(GraphError::Eval { .. })
    ));
}

#[test]
fn non_finite_results_are_errors() {
    let r = run("math.mul", &[("a", V::Float(1e308)), ("b", V::Float(10.0))]);
    assert!(matches!(r, Err(GraphError::Eval { message, .. }) if message.contains("finite")));
}

#[test]
fn a_math_graph_evaluates_end_to_end() {
    // clamp(remap(sin(t * pi), -1, 1 -> 0, 10), 0, 6) at t = 0.5 s: sin(pi/2) = 1 -> 10 -> 6
    let r = NodeRegistry::builtin();
    let mut g = Graph::new();
    let t = g.add_node(r, "time.now").unwrap();
    let mul = g.add_node(r, "math.mul").unwrap();
    let sin = g.add_node(r, "math.sin").unwrap();
    let remap = g.add_node(r, "math.remap").unwrap();
    let clamp = g.add_node(r, "math.clamp").unwrap();
    g.set_value(r, mul, "b", V::Float(std::f64::consts::PI))
        .unwrap();
    g.set_value(r, remap, "in_min", V::Float(-1.0)).unwrap();
    g.set_value(r, remap, "out_max", V::Float(10.0)).unwrap();
    g.set_value(r, clamp, "max", V::Float(6.0)).unwrap();
    let link = |g: &mut Graph, a: Id, ap: &str, b: Id, bp: &str| {
        g.connect(r, PortRef::new(a, ap), PortRef::new(b, bp))
            .unwrap();
    };
    link(&mut g, t, "seconds", mul, "a");
    link(&mut g, mul, "out", sin, "x");
    link(&mut g, sin, "out", remap, "value");
    link(&mut g, remap, "out", clamp, "value");
    let ev = g
        .evaluate_for(r, &EvalEnv::headless(0.5), &[clamp])
        .unwrap();
    close(ev.float(clamp, "out").unwrap(), 6.0);
    let ev = g
        .evaluate_for(r, &EvalEnv::headless(0.0), &[clamp])
        .unwrap();
    close(ev.float(clamp, "out").unwrap(), 5.0); // sin 0 = 0 -> remap -> 5
    // the output of a typed port is the declared type
    assert_eq!(ev.output(clamp, "out").map(V::ty), Some(T::Float));
}
