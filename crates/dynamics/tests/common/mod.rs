//! Scene building and golden-track reading shared by the dynamics parity tests.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, Registry, Tx, Value, transform};
use forge_dynamics::{BodyKey, DynamicsCache, DynamicsToolset, RIGID_BODY};
use forge_generators::{Primitives, evaluate};
use forge_parity_harness::{Samples, parse_samples, track_values};
use glam::{DMat3, DMat4, DVec3};

pub fn registry() -> Arc<Registry> {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r.install(&forge_clone_engine::CloneToolset).unwrap();
    r.install(&DynamicsToolset).unwrap();
    Arc::new(r)
}

pub fn golden(feature: &str, case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden")
        .join(feature)
        .join(case)
}

pub fn case_table(feature: &str, case: &str) -> toml::Table {
    let p = golden(feature, case).join("case.toml");
    std::fs::read_to_string(p).unwrap().parse().unwrap()
}

/// The values of `samples-<object>-<param>.json`, one per frame from 0.
pub fn track(feature: &str, case: &str, object: &str, param: &str) -> Vec<f64> {
    let p = golden(feature, case).join(format!("samples-{object}-{param}.json"));
    let s: Samples = parse_samples(&std::fs::read_to_string(p).unwrap()).unwrap();
    track_values(&s).expect("a track with frames 0, 1, 2, ...")
}

/// Degrees, (h, p, b) of a world matrix as the reference reads them.
pub fn hpb_deg(m: &DMat4) -> [f64; 3] {
    let r = transform::hpb_from_rotation(transform::rotation_part(m));
    [r.x.to_degrees(), r.y.to_degrees(), r.z.to_degrees()]
}

pub fn rot_matrix_from_deg(h: f64, p: f64, b: f64) -> DMat3 {
    transform::Transform {
        pos: DVec3::ZERO,
        rot: DVec3::new(h.to_radians(), p.to_radians(), b.to_radians()),
        scale: DVec3::ONE,
    }
    .rotation()
}

/// What one object of a case scene looks like.
#[derive(Clone)]
pub struct Obj {
    pub name: &'static str,
    pub kind: &'static str, // forge.cube / forge.sphere / forge.plane
    pub size: f64,          // cube edge or sphere radius
    pub pos: [f64; 3],
    pub rot_deg: [f64; 3],
    pub dynamic: &'static str, // on / off
    pub bounce: f64,
    pub friction: f64,
    pub velocity: Option<[f64; 3]>,
}

impl Obj {
    pub fn plane() -> Obj {
        Obj {
            name: "plane",
            kind: "forge.plane",
            size: 0.0,
            pos: [0.0; 3],
            rot_deg: [0.0; 3],
            dynamic: "off",
            bounce: 1.0,
            friction: 1.0,
            velocity: None,
        }
    }

    pub fn cube(name: &'static str, pos: [f64; 3]) -> Obj {
        Obj {
            name,
            kind: "forge.cube",
            size: 50.0,
            pos,
            rot_deg: [0.0; 3],
            dynamic: "on",
            bounce: 0.05,
            friction: 0.9,
            velocity: None,
        }
    }
}

pub struct Built {
    pub doc: Document,
    pub ids: BTreeMap<&'static str, NodeId>,
}

pub fn add_object(tx: &mut Tx, o: &Obj) -> NodeId {
    let id = tx.create(o.kind, None, None).unwrap();
    match o.kind {
        "forge.cube" => {
            for a in ["size_x", "size_y", "size_z"] {
                tx.set_param(id, a, Value::Float(o.size)).unwrap();
            }
        }
        "forge.sphere" => tx.set_param(id, "radius", Value::Float(o.size)).unwrap(),
        _ => {}
    }
    tx.set_param(id, "transform.pos", Value::Vec3(o.pos))
        .unwrap();
    let rot = o.rot_deg.map(f64::to_radians);
    tx.set_param(id, "transform.rot", Value::Vec3(rot)).unwrap();
    tx.rename(id, o.name).unwrap();
    let t = tx.attach_tag(RIGID_BODY, id, None).unwrap();
    tx.set_param(t, "dynamic", Value::Enum(o.dynamic.into()))
        .unwrap();
    tx.set_param(t, "bounce", Value::Float(o.bounce)).unwrap();
    tx.set_param(t, "friction", Value::Float(o.friction))
        .unwrap();
    if let Some(v) = o.velocity {
        tx.set_param(t, "custom_initial_velocity", Value::Bool(true))
            .unwrap();
        tx.set_param(t, "initial_linear_velocity", Value::Vec3(v))
            .unwrap();
    }
    id
}

pub fn build(objs: &[Obj]) -> Built {
    let mut doc = Document::new(registry());
    let ids = doc
        .transact("scene", |tx| {
            Ok(objs.iter().map(|o| (o.name, add_object(tx, o))).collect())
        })
        .unwrap();
    Built { doc, ids }
}

pub fn mesh_fn(doc: &Document) -> impl Fn(NodeId) -> Option<forge_geometry::PolyMesh> + '_ {
    move |id| evaluate(doc.scene(), id, &EvalCtx::default()).unwrap()
}

/// Simulates `frames` frames (0 ..= frames) and returns, per body, the world matrix per frame.
pub fn simulate(b: &Built, frames: i64) -> BTreeMap<NodeId, Vec<DMat4>> {
    let mut cache = DynamicsCache::build(b.doc.scene(), &mesh_fn(&b.doc)).unwrap();
    let mut out: BTreeMap<NodeId, Vec<DMat4>> = BTreeMap::new();
    for f in 0..=frames {
        for (k, m) in cache.at(f) {
            if let BodyKey::Node(id) = k {
                out.entry(id).or_default().push(m);
            }
        }
    }
    out
}

pub fn column(ms: &[DMat4], axis: usize) -> Vec<f64> {
    ms.iter().map(|m| m.w_axis[axis]).collect()
}

pub fn set(doc: &mut Document, id: NodeId, key: &str, v: Value) {
    let key = key.to_string();
    doc.transact("set", move |tx| tx.set_param(id, &key, v))
        .unwrap();
}

/// Adds a connector of `kind` between `a` and `b` at the origin (the default position).
pub fn add_connector(tx: &mut Tx, kind: &str, a: Option<NodeId>, b: Option<NodeId>) -> NodeId {
    let c = tx.create(forge_dynamics::CONNECTOR, None, None).unwrap();
    tx.set_param(c, "type", Value::Enum(kind.into())).unwrap();
    tx.set_param(c, "object_a", Value::Link(a)).unwrap();
    tx.set_param(c, "object_b", Value::Link(b)).unwrap();
    c
}
