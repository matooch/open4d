//! docs/specs/CORE.md section 5b: the evaluation scheduler. Serial vs parallel equality
//! over the parity scenes and seeded random scenes, dirty propagation (counting
//! evaluations), time dependence, cycles.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use forge_app::animate::Evaluated;
use forge_app::schedule::{Frame, NodeOutput, Scheduler, Threads, drawn_nodes, node_output};
use forge_app::{App, default_registry};
use forge_clone_engine::SplitMix64;
use forge_core::{Document, EvalCtx, Key, NodeId, Scene, Value};
use forge_parity_harness::discover_cases;
use serde_json::json;

#[path = "../../generators/tests/common/mod.rs"]
mod common;

type Bits = (Option<(Vec<[u64; 3]>, Vec<Vec<u32>>)>, Option<String>);

fn bits(o: &NodeOutput) -> Bits {
    (
        o.mesh.as_ref().map(|m| {
            (
                m.points
                    .iter()
                    .map(|p| p.to_array().map(f64::to_bits))
                    .collect(),
                m.polys.clone(),
            )
        }),
        o.error.as_ref().map(|e| format!("{e:?}")),
    )
}

/// The existing path: `Evaluated` (tracks, expressions) then serial `evaluate` per drawn node.
fn reference(scene: &Scene, frame: i64) -> BTreeMap<NodeId, Bits> {
    let ev = Evaluated::default().get(scene, frame);
    let ctx = EvalCtx::at_frame(frame, f64::from(scene.settings().fps));
    drawn_nodes(&ev)
        .into_iter()
        .map(|id| (id, bits(&node_output(&ev, id, &ctx))))
        .collect()
}

fn frame_bits(f: &Frame) -> BTreeMap<NodeId, Bits> {
    f.outputs.iter().map(|(k, v)| (*k, bits(v))).collect()
}

fn registry() -> Arc<forge_core::Registry> {
    Arc::new(default_registry().unwrap())
}

// --- the parity scene corpus ---------------------------------------------------------

const PRIMITIVES: &[&str] = &[
    "plane", "cube", "sphere", "cylinder", "cone", "disc", "torus", "tube", "capsule", "pyramid",
];
const GENERATORS: &[&str] = &[
    "symmetry", "array", "connect", "instance", "extrude", "lathe", "loft", "sweep",
];
const DEFORMERS: &[&str] = &["bend", "twist", "taper", "bulge", "shear", "spherify"];

type Table = toml::map::Map<String, toml::Value>;

fn quietly<T>(f: impl FnOnce() -> T + std::panic::UnwindSafe) -> Option<T> {
    std::panic::catch_unwind(f).ok()
}

/// Scenes rebuilt from the golden case files: plain primitives, the modeling generators,
/// the simple deformers and the cloner modes. Cases the builders here cannot express are
/// skipped (the parity tests of those crates cover them).
fn parity_corpus() -> Vec<(String, Document)> {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut out = Vec::new();
    for case in discover_cases(&root).unwrap() {
        let key = format!("{}/{}", case.feature, case.name);
        let Ok(m) = case.manifest() else { continue };
        let f = case.feature.as_str();
        let reg = reg.clone();
        let built = quietly(std::panic::AssertUnwindSafe(|| {
            let mut doc = Document::new(reg.clone());
            if PRIMITIVES.contains(&f) {
                let ty = format!("forge.{f}");
                let p = common::typed_params(&reg, &ty, &common::resolved_params(&case, &m));
                doc.transact("c", |tx| {
                    let id = tx.create(&ty, None, None)?;
                    for (k, v) in p {
                        tx.set_param(id, &k, v)?;
                    }
                    Ok(())
                })
                .ok()?;
                return Some(doc);
            }
            if GENERATORS.contains(&f) {
                let params = common::resolved_params(&case, &m);
                let mut inputs: Vec<(&String, &Table)> = params
                    .iter()
                    .filter_map(|(k, v)| v.as_table().map(|t| (k, t)))
                    .collect();
                inputs.sort_by_key(|(k, _)| {
                    k.trim_start_matches("input").parse::<u32>().unwrap_or(1)
                });
                let gen_type = format!("forge.{f}");
                doc.transact("case", |tx| {
                    let g = tx.create(&gen_type, None, None)?;
                    common::apply(tx, &reg, g, &gen_type, &params);
                    for (_, t) in &inputs {
                        let ty = format!("forge.{}", t["type"].as_str().unwrap());
                        let parent = if f == "instance" { None } else { Some(g) };
                        let child = tx.create(&ty, parent, None)?;
                        common::apply(tx, &reg, child, &ty, t);
                        if f == "instance" {
                            tx.set_param(g, "reference", Value::Link(Some(child)))?;
                        }
                    }
                    Ok(())
                })
                .ok()?;
                return Some(doc);
            }
            if DEFORMERS.contains(&f) {
                let params = common::resolved_params(&case, &m);
                let input = params.get("input")?.as_table()?;
                let ty = format!("forge.{}", input["type"].as_str()?);
                let dty = format!("forge.{f}");
                doc.transact("case", |tx| {
                    let o = tx.create(&ty, None, None)?;
                    common::apply(tx, &reg, o, &ty, &known(&reg, &ty, input));
                    let d = tx.create(&dty, Some(o), None)?;
                    common::apply(tx, &reg, d, &dty, &known(&reg, &dty, &params));
                    Ok(())
                })
                .ok()?;
                return Some(doc);
            }
            if f == "cloner" {
                let mode = m.params.get("mode")?.as_str()?.to_string();
                if mode == "object" || m.params.contains_key("effector") {
                    return None;
                }
                let ty = reg.get("forge.cloner")?;
                doc.transact("case", |tx| {
                    let c = tx.create("forge.cloner", None, None)?;
                    tx.set_param(c, "mode", Value::Enum(mode.clone()))?;
                    for (k, v) in &m.params {
                        // keys that are cloner params verbatim; the others belong to the
                        // cloner parity test's translation table
                        let Some(desc) = ty.find_param(k) else {
                            continue;
                        };
                        let value = match (&desc.kind, v) {
                            (forge_core::ParamKind::Bool, toml::Value::Boolean(b)) => {
                                Value::Bool(*b)
                            }
                            (forge_core::ParamKind::Int { .. }, toml::Value::Integer(i)) => {
                                Value::Int(*i)
                            }
                            _ => continue,
                        };
                        let _ = tx.set_param(c, k, value);
                    }
                    let cube = tx.create("forge.cube", Some(c), None)?;
                    tx.set_param(cube, "size_x", Value::Float(20.0))?;
                    Ok(())
                })
                .ok()?;
                return Some(doc);
            }
            None
        }))
        .flatten();
        if let Some(doc) = built {
            out.push((key, doc));
        }
    }
    out
}

/// The entries of a case table that are params of `ty` (or `pos`).
fn known(reg: &forge_core::Registry, ty: &str, t: &Table) -> Table {
    let nt = reg.get(ty).unwrap();
    t.iter()
        .filter(|(k, v)| !v.is_table() && (k.as_str() == "pos" || nt.find_param(k).is_some()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn check_scene(name: &str, scene: &Scene, frames: &[i64]) {
    let mut serial = Scheduler::new(Threads::Serial);
    let mut par = Scheduler::new(Threads::Fixed(4));
    let mut auto = Scheduler::new(Threads::Auto);
    for &fr in frames {
        let want = reference(scene, fr);
        let s = serial.evaluate(scene, fr);
        let p = par.evaluate(scene, fr);
        let a = auto.evaluate(scene, fr);
        assert_eq!(frame_bits(&s), want, "{name} frame {fr}: serial scheduler");
        assert_eq!(frame_bits(&p), want, "{name} frame {fr}: 4 threads");
        assert_eq!(frame_bits(&a), want, "{name} frame {fr}: global pool");
    }
}

#[test]
fn parity_scenes_serial_and_parallel_are_bit_identical() {
    let corpus = parity_corpus();
    assert!(
        corpus.len() >= 150,
        "only {} parity scenes built",
        corpus.len()
    );
    for (key, doc) in &corpus {
        check_scene(key, doc.scene(), &[0, 7]);
    }
    println!("parity scenes checked: {}", corpus.len());
}

// --- hand-built cloner scene and random scenes ------------------------------------------

fn vec3(v: [f64; 3]) -> Value {
    Value::Vec3(v)
}

/// A cloner with a Plain effector, a Random effector animated by noise, a spherical field,
/// a bend with a field falloff, a sibling deformer under a null, an animated cube.
fn cloner_scene() -> Document {
    let mut doc = Document::new(registry());
    doc.transact("cloner_rig", |tx| {
        let cloner = tx.create("forge.cloner", None, None)?;
        tx.set_param(cloner, "mode", Value::Enum("linear".into()))?;
        tx.set_param(cloner, "linear_count", Value::Int(6))?;
        let cube = tx.create("forge.cube", Some(cloner), None)?;
        tx.set_param(cube, "size_x", Value::Float(20.0))?;
        let field = tx.create("forge.field.spherical", None, None)?;
        let plain = tx.create("forge.effector.plain", None, None)?;
        tx.set_param(plain, "p", vec3([0.0, 40.0, 0.0]))?;
        tx.set_param(plain, "fields", Value::LinkList(vec![field]))?;
        let random = tx.create("forge.effector.random", None, None)?;
        tx.set_param(random, "random_mode", Value::Enum("noise".into()))?;
        tx.set_param(cloner, "effectors", Value::LinkList(vec![plain, random]))?;
        let sphere = tx.create("forge.sphere", None, None)?;
        let bend = tx.create("forge.bend", Some(sphere), None)?;
        tx.set_param(bend, "fields", Value::LinkList(vec![field]))?;
        let null = tx.create("forge.null", None, None)?;
        tx.create("forge.cube", Some(null), None)?;
        tx.create("forge.twist", Some(null), None)?;
        let moving = tx.create("forge.cube", None, None)?;
        tx.set_key(moving, "transform.pos.x", Key::new(0.0, 0.0))?;
        tx.set_key(moving, "transform.pos.x", Key::new(1.0, 100.0))?;
        Ok(())
    })
    .unwrap();
    doc
}

#[test]
fn cloner_scene_serial_and_parallel_match() {
    check_scene("cloner_rig", cloner_scene().scene(), &[0, 5, 15, 30, 3]);
}

const KINDS: &[&str] = &[
    "cube",
    "sphere",
    "cylinder",
    "null",
    "cloner",
    "subdivision",
    "plane",
];

pub fn random_scene(seed: u64) -> Document {
    let mut rng = SplitMix64::new(seed);
    let mut doc = Document::new(registry());
    doc.transact("random", |tx| {
        let mut parents: Vec<Option<NodeId>> = vec![None];
        let mut effectors: Vec<NodeId> = Vec::new();
        let mut fields: Vec<NodeId> = Vec::new();
        for _ in 0..(2 + (rng.next_u64() % 3)) {
            let f = tx.create("forge.field.spherical", None, None)?;
            tx.set_param(
                f,
                "transform.pos",
                vec3([rng.next_signed() * 100.0, 0.0, 0.0]),
            )?;
            fields.push(f);
            let e = tx.create(
                if rng.next_u64().is_multiple_of(2) {
                    "forge.effector.plain"
                } else {
                    "forge.effector.random"
                },
                None,
                None,
            )?;
            tx.set_param(e, "fields", Value::LinkList(vec![f]))?;
            if tx
                .scene()
                .node(e)
                .is_some_and(|n| n.type_id.ends_with("random"))
                && rng.next_u64().is_multiple_of(2)
            {
                tx.set_param(e, "random_mode", Value::Enum("noise".into()))?;
            }
            effectors.push(e);
        }
        let count = 6 + (rng.next_u64() % 10) as usize;
        for _ in 0..count {
            let parent = parents[(rng.next_u64() as usize) % parents.len()];
            let kind = KINDS[(rng.next_u64() as usize) % KINDS.len()];
            let id = tx.create(&format!("forge.{kind}"), parent, None)?;
            tx.set_param(
                id,
                "transform.pos",
                vec3([
                    rng.next_signed() * 200.0,
                    rng.next_signed() * 50.0,
                    rng.next_signed() * 200.0,
                ]),
            )?;
            if rng.next_u64().is_multiple_of(3) {
                tx.set_param(id, "transform.rot", vec3([rng.next_signed(), 0.0, 0.0]))?;
            }
            if rng.next_u64().is_multiple_of(3) {
                tx.set_key(
                    id,
                    "transform.pos.x",
                    Key::new(0.0, rng.next_signed() * 100.0),
                )?;
                tx.set_key(
                    id,
                    "transform.pos.x",
                    Key::new(1.0, rng.next_signed() * 100.0),
                )?;
            }
            match kind {
                "cloner" => {
                    tx.set_param(id, "mode", Value::Enum("linear".into()))?;
                    tx.set_param(
                        id,
                        "linear_count",
                        Value::Int(3 + (rng.next_u64() % 12) as i64),
                    )?;
                    let e = effectors[(rng.next_u64() as usize) % effectors.len()];
                    tx.set_param(id, "effectors", Value::LinkList(vec![e]))?;
                    tx.create("forge.cube", Some(id), None)?;
                }
                "subdivision" => {
                    tx.create("forge.cube", Some(id), None)?;
                }
                "null" => parents.push(Some(id)),
                _ => {}
            }
            if rng.next_u64().is_multiple_of(4)
                && matches!(kind, "cube" | "sphere" | "cylinder" | "plane")
            {
                let d = tx.create(
                    if rng.next_u64().is_multiple_of(2) {
                        "forge.bend"
                    } else {
                        "forge.twist"
                    },
                    Some(id),
                    None,
                )?;
                if rng.next_u64().is_multiple_of(2) {
                    let f = fields[(rng.next_u64() as usize) % fields.len()];
                    tx.set_param(d, "fields", Value::LinkList(vec![f]))?;
                }
            }
        }
        Ok(())
    })
    .unwrap();
    doc
}

fn all_ids(scene: &Scene) -> Vec<NodeId> {
    forge_core::depgraph::all_nodes(scene)
}

#[test]
fn random_scenes_and_edits_match_the_full_serial_evaluation() {
    for seed in 0..12u64 {
        let mut doc = random_scene(seed);
        let mut inc_serial = Scheduler::new(Threads::Serial);
        let mut inc_par = Scheduler::new(Threads::Fixed(3));
        let mut rng = SplitMix64::new(seed ^ 0xabcdef);
        for step in 0..10 {
            // a frame change or an edit, evaluated incrementally and from scratch
            let frame = (rng.next_u64() % 40) as i64;
            if step % 2 == 1 {
                let ids = all_ids(doc.scene());
                let id = ids[(rng.next_u64() as usize) % ids.len()];
                let is_hier = doc.scene().node(id).is_some_and(|n| !n.is_tag());
                if is_hier {
                    let v = vec3([rng.next_signed() * 80.0, 0.0, rng.next_signed() * 80.0]);
                    let _ = doc.transact("edit", |tx| tx.set_param(id, "transform.pos", v));
                }
            }
            let want = reference(doc.scene(), frame);
            let name = format!("seed {seed} step {step}");
            assert_eq!(
                frame_bits(&inc_serial.evaluate(doc.scene(), frame)),
                want,
                "{name} serial"
            );
            assert_eq!(
                frame_bits(&inc_par.evaluate(doc.scene(), frame)),
                want,
                "{name} parallel"
            );
        }
    }
}

// --- dirty propagation: count evaluations ----------------------------------------------

/// Three groups, each a null with two cubes; a loose cube; a cloner linked to a field.
struct Groups {
    app: App,
    nulls: [NodeId; 3],
    cubes: [[NodeId; 2]; 3],
    loose: NodeId,
    cloner: NodeId,
    field: NodeId,
    proto: NodeId,
    plain: NodeId,
}

fn groups() -> Groups {
    let mut app = App::new_default();
    let mut nulls = [NodeId(0); 3];
    let mut cubes = [[NodeId(0); 2]; 3];
    let (loose, cloner, field, proto, plain) = app
        .doc
        .transact("groups", |tx| {
            for g in 0..3 {
                nulls[g] = tx.create("forge.null", None, None)?;
                for cube in &mut cubes[g] {
                    *cube = tx.create("forge.cube", Some(nulls[g]), None)?;
                }
            }
            let loose = tx.create("forge.sphere", None, None)?;
            let cloner = tx.create("forge.cloner", None, None)?;
            let proto = tx.create("forge.cube", Some(cloner), None)?;
            let field = tx.create("forge.field.spherical", None, None)?;
            let plain = tx.create("forge.effector.plain", None, None)?;
            tx.set_param(plain, "fields", Value::LinkList(vec![field]))?;
            tx.set_param(cloner, "effectors", Value::LinkList(vec![plain]))?;
            Ok((loose, cloner, field, proto, plain))
        })
        .unwrap();
    Groups {
        app,
        nulls,
        cubes,
        loose,
        cloner,
        field,
        proto,
        plain,
    }
}

fn sorted(mut v: Vec<NodeId>) -> Vec<NodeId> {
    v.sort();
    v
}

#[test]
fn editing_one_node_evaluates_exactly_its_dependants() {
    let mut g = groups();
    let mut s = Scheduler::new(Threads::Fixed(2));
    let first = s.evaluate(g.app.doc.scene(), 0);
    assert_eq!(
        first.stats.evaluated.len(),
        first.outputs.len(),
        "first pass evaluates all"
    );

    // nothing changed: no work
    let again = s.evaluate(g.app.doc.scene(), 0);
    assert!(again.stats.unchanged);
    assert!(again.stats.evaluated.is_empty());

    // a shape param of a child: the child and its group null (the null draws its children)
    let c = g.cubes[1][0];
    g.app
        .doc
        .transact("e", |tx| tx.set_param(c, "size_x", Value::Float(33.0)))
        .unwrap();
    let f = s.evaluate(g.app.doc.scene(), 0);
    assert_eq!(f.stats.evaluated, sorted(vec![c, g.nulls[1]]));
    assert_eq!(f.stats.reused, f.outputs.len() - 2);

    // a transform of the null: itself, both children
    let n = g.nulls[2];
    g.app
        .doc
        .transact("e", |tx| {
            tx.set_param(n, "transform.pos", vec3([5.0, 0.0, 0.0]))
        })
        .unwrap();
    let f = s.evaluate(g.app.doc.scene(), 0);
    assert_eq!(
        f.stats.evaluated,
        sorted(vec![n, g.cubes[2][0], g.cubes[2][1]])
    );

    // a field that a cloner reaches through an effector: the cloner (and the field, which is
    // not drawn); not the prototype, which the cloner consumes
    let fl = g.field;
    g.app
        .doc
        .transact("e", |tx| {
            tx.set_param(fl, "transform.pos", vec3([10.0, 0.0, 0.0]))
        })
        .unwrap();
    let f = s.evaluate(g.app.doc.scene(), 0);
    assert!(f.stats.evaluated.contains(&g.cloner));
    assert!(!f.stats.evaluated.contains(&g.proto));
    assert!(f.stats.dirty.contains(&fl));
    assert_eq!(f.stats.evaluated, sorted(vec![g.cloner, g.plain, fl]));

    // an unrelated sphere does not move any other result
    let lo = g.loose;
    g.app
        .doc
        .transact("e", |tx| tx.set_param(lo, "radius", Value::Float(77.0)))
        .unwrap();
    let f = s.evaluate(g.app.doc.scene(), 0);
    assert_eq!(f.stats.evaluated, vec![lo]);

    // the result after all these incremental steps equals a fresh evaluation
    let want = reference(g.app.doc.scene(), 0);
    assert_eq!(frame_bits(&f), want);
}

#[test]
fn removing_a_linked_node_dirties_its_former_dependants() {
    let mut g = groups();
    let mut s = Scheduler::new(Threads::Serial);
    s.evaluate(g.app.doc.scene(), 0);
    let fl = g.field;
    g.app.doc.transact("rm", |tx| tx.delete(fl)).unwrap();
    let f = s.evaluate(g.app.doc.scene(), 0);
    assert!(f.stats.evaluated.contains(&g.cloner));
    assert_eq!(frame_bits(&f), reference(g.app.doc.scene(), 0));
}

#[test]
fn a_sibling_deformer_dirties_the_objects_it_deforms() {
    let mut app = App::new_default();
    let (null, cube, bend, other) = app
        .doc
        .transact("s", |tx| {
            let null = tx.create("forge.null", None, None)?;
            let cube = tx.create("forge.cube", Some(null), None)?;
            let bend = tx.create("forge.bend", Some(null), None)?;
            let other = tx.create("forge.sphere", None, None)?;
            Ok((null, cube, bend, other))
        })
        .unwrap();
    let mut s = Scheduler::new(Threads::Serial);
    s.evaluate(app.doc.scene(), 0);
    app.doc
        .transact("e", |tx| tx.set_param(bend, "strength", Value::Float(0.5)))
        .unwrap();
    let f = s.evaluate(app.doc.scene(), 0);
    // the deformer node is itself a drawn (geometry-less) node
    assert_eq!(f.stats.evaluated, sorted(vec![null, cube, bend]));
    assert!(!f.stats.evaluated.contains(&other));
    assert_eq!(frame_bits(&f), reference(app.doc.scene(), 0));
}

// --- time ----------------------------------------------------------------------------------

#[test]
fn a_static_scene_does_no_work_on_a_frame_change() {
    let mut g = groups();
    let mut s = Scheduler::new(Threads::Fixed(2));
    s.evaluate(g.app.doc.scene(), 0);
    for fr in [1, 2, 50] {
        let f = s.evaluate(g.app.doc.scene(), fr);
        assert!(f.stats.evaluated.is_empty(), "frame {fr}");
        assert_eq!(frame_bits(&f), reference(g.app.doc.scene(), fr));
    }
    assert!(s.time_dependent().is_empty());
    let _ = &mut g;
}

#[test]
fn only_the_animated_chain_is_re_evaluated_on_a_frame_change() {
    let mut g = groups();
    let moving = g.cubes[0][1];
    g.app
        .doc
        .transact("k", |tx| {
            tx.set_key(moving, "transform.pos.x", Key::new(0.0, 0.0))?;
            tx.set_key(moving, "transform.pos.x", Key::new(1.0, 90.0))
        })
        .unwrap();
    let mut s = Scheduler::new(Threads::Fixed(2));
    s.evaluate(g.app.doc.scene(), 0);
    let f = s.evaluate(g.app.doc.scene(), 10);
    assert_eq!(f.stats.evaluated, sorted(vec![moving, g.nulls[0]]));
    assert_eq!(frame_bits(&f), reference(g.app.doc.scene(), 10));
    assert_eq!(
        s.time_dependent(),
        [moving, g.nulls[0]].into_iter().collect()
    );
    // a held key: the value repeats, nothing changes
    let f = s.evaluate(g.app.doc.scene(), 40);
    assert_eq!(f.stats.evaluated, sorted(vec![moving, g.nulls[0]]));
    let f2 = s.evaluate(g.app.doc.scene(), 50);
    assert!(
        f2.stats.evaluated.is_empty(),
        "past the last key the value is constant"
    );
}

fn effector_scene(speed: f64) -> (Document, NodeId, NodeId, NodeId) {
    let mut doc = Document::new(registry());
    let (cloner, other, effector) = doc
        .transact("s", |tx| {
            let cloner = tx.create("forge.cloner", None, None)?;
            tx.set_param(cloner, "mode", Value::Enum("linear".into()))?;
            tx.create("forge.cube", Some(cloner), None)?;
            let e = tx.create("forge.effector.random", None, None)?;
            tx.set_param(e, "random_mode", Value::Enum("noise".into()))?;
            tx.set_param(e, "animation_speed", Value::Float(speed))?;
            tx.set_param(cloner, "effectors", Value::LinkList(vec![e]))?;
            let other = tx.create("forge.sphere", None, None)?;
            Ok((cloner, other, e))
        })
        .unwrap();
    (doc, cloner, other, effector)
}

#[test]
fn a_noise_effector_makes_its_cloner_time_dependent() {
    let (doc, cloner, other, effector) = effector_scene(1.0);
    let mut s = Scheduler::new(Threads::Fixed(2));
    s.evaluate(doc.scene(), 0);
    let f = s.evaluate(doc.scene(), 12);
    assert_eq!(f.stats.evaluated, sorted(vec![cloner, effector]));
    assert!(s.time_dependent().contains(&cloner));
    assert!(!s.time_dependent().contains(&other));
    assert_eq!(frame_bits(&f), reference(doc.scene(), 12));
    let (still, ..) = effector_scene(0.0);
    let mut s = Scheduler::new(Threads::Serial);
    s.evaluate(still.scene(), 0);
    assert!(s.evaluate(still.scene(), 9).stats.evaluated.is_empty());
    assert!(s.time_dependent().is_empty());
}

// --- expressions stay one ordered stage -----------------------------------------------------

fn expr_add(app: &mut App, tag: NodeId, ty: &str, values: serde_json::Value) -> u64 {
    app.run(
        "graph.add_node",
        json!({"tag": tag.0, "type": ty, "values": values}),
    )
    .unwrap()["node"]
        .as_u64()
        .unwrap()
}

fn expr_link(app: &mut App, tag: NodeId, from: (u64, &str), to: (u64, &str)) {
    app.run(
        "graph.connect",
        json!({"tag": tag.0, "from_node": from.0, "from_port": from.1, "to_node": to.0, "to_port": to.1}),
    )
    .unwrap();
}

#[test]
fn expressions_run_in_order_and_drive_dirtiness() {
    let mut app = App::new_default();
    let mut cubes = Vec::new();
    for _ in 0..3 {
        let out = app
            .run("object.create", json!({"type": "forge.cube"}))
            .unwrap();
        cubes.push(NodeId(out["node"].as_u64().unwrap()));
    }
    // cube 0 follows the time; no other cube has a tag
    app.selection.click(cubes[0]);
    let tag = NodeId(
        app.run("object.create", json!({"type": forge_nodes::TAG}))
            .unwrap()["node"]
            .as_u64()
            .unwrap(),
    );
    let t = expr_add(&mut app, tag, "time.now", json!({}));
    let v = expr_add(&mut app, tag, "vector.compose", json!({}));
    let w = expr_add(
        &mut app,
        tag,
        "object.set_param",
        json!({"param": "transform.pos"}),
    );
    expr_link(&mut app, tag, (t, "seconds"), (v, "x"));
    expr_link(&mut app, tag, (v, "out"), (w, "value"));
    let mut s = Scheduler::new(Threads::Fixed(2));
    s.evaluate(app.doc.scene(), 0);
    let f = s.evaluate(app.doc.scene(), 15);
    assert!(f.stats.expressions_ran);
    assert_eq!(f.stats.evaluated, vec![cubes[0]], "only the host moves");
    assert_eq!(frame_bits(&f), reference(app.doc.scene(), 15));
    assert!(s.time_dependent().contains(&cubes[0]) && s.time_dependent().contains(&tag));
    assert!(!s.time_dependent().contains(&cubes[1]));
}

// --- cycles ------------------------------------------------------------------------------------

#[test]
fn link_cycles_are_reported_and_broken_the_same_way_every_time() {
    let run = || {
        let mut app = App::new_default();
        let (a, b, plain) = app
            .doc
            .transact("c", |tx| {
                let cube = tx.create("forge.cube", None, None)?;
                let a = tx.create("forge.instance", None, None)?;
                let b = tx.create("forge.instance", None, None)?;
                tx.set_param(a, "reference", Value::Link(Some(b)))?;
                tx.set_param(b, "reference", Value::Link(Some(a)))?;
                let _ = cube;
                let plain = tx.create("forge.sphere", None, None)?;
                Ok((a, b, plain))
            })
            .unwrap();
        let mut s = Scheduler::new(Threads::Fixed(2));
        let f = s.evaluate(app.doc.scene(), 0);
        (
            a,
            b,
            plain,
            f.stats.cycles.clone(),
            frame_bits(&f),
            reference(app.doc.scene(), 0),
        )
    };
    let (a, b, plain, cycles, got, want) = run();
    assert_eq!(cycles.len(), 1, "{cycles:?}");
    assert_eq!((cycles[0].from.node, cycles[0].to.node), (b, a));
    assert_eq!(got, want);
    assert!(got.contains_key(&plain));
    let (_, _, _, cycles2, ..) = run();
    assert_eq!(cycles, cycles2);
}

#[test]
fn changing_fps_or_loading_another_document_starts_over() {
    let mut g = groups();
    let mut s = Scheduler::new(Threads::Serial);
    s.evaluate(g.app.doc.scene(), 0);
    g.app
        .run(
            "timeline.settings",
            json!({"fps": 24, "start": 0, "end": 90}),
        )
        .unwrap();
    let f = s.evaluate(g.app.doc.scene(), 0);
    assert_eq!(f.stats.evaluated.len(), f.outputs.len());
    s.clear();
    let f = s.evaluate(g.app.doc.scene(), 0);
    assert_eq!(f.stats.evaluated.len(), f.outputs.len());
}

#[test]
fn the_app_exposes_the_scheduler() {
    let mut g = groups();
    let f = g.app.scheduled(0);
    assert_eq!(frame_bits(&f), reference(g.app.doc.scene(), 0));
}
