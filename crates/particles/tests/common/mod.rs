//! Scene building shared by the particle tests.
#![allow(dead_code)]

use std::sync::Arc;

use forge_core::{Document, NodeId, Registry, Value};
use forge_particles::{EMITTER, ParticleCache, ParticleMode, ParticleToolset, Particles};

pub fn registry() -> Arc<Registry> {
    let mut r = Registry::with_builtins();
    r.install(&ParticleToolset).unwrap();
    r.install(&forge_fields::FieldToolset).unwrap();
    Arc::new(r)
}

pub struct World {
    pub doc: Document,
    pub emitter: NodeId,
}

/// A scene with one emitter whose parameters are `params`.
pub fn world(params: &[(&str, Value)]) -> World {
    let mut doc = Document::new(registry());
    let emitter = doc
        .transact("emitter", |tx| {
            let id = tx.create(EMITTER, None, None)?;
            for (k, v) in params {
                tx.set_param(id, k, v.clone())?;
            }
            Ok(id)
        })
        .unwrap();
    World { doc, emitter }
}

pub fn set(w: &mut World, id: NodeId, key: &str, v: Value) {
    let key = key.to_string();
    w.doc
        .transact("set", move |tx| tx.set_param(id, &key, v))
        .unwrap();
}

/// Adds a node of `type_id` with `params` at `pos`; returns its id.
pub fn add(w: &mut World, type_id: &str, params: &[(&str, Value)], pos: [f64; 3]) -> NodeId {
    w.doc
        .transact("add", |tx| {
            let id = tx.create(type_id, None, None)?;
            tx.set_param(id, "transform.pos", Value::Vec3(pos))?;
            for (k, v) in params {
                tx.set_param(id, k, v.clone())?;
            }
            Ok(id)
        })
        .unwrap()
}

pub fn cache(w: &World) -> ParticleCache {
    ParticleCache::build(w.doc.scene(), w.emitter, ParticleMode::Editor).unwrap()
}

pub fn f(v: f64) -> Value {
    Value::Float(v)
}

pub fn i(v: i64) -> Value {
    Value::Int(v)
}

pub fn e(v: &str) -> Value {
    Value::Enum(v.into())
}

pub fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// The sample standard deviation (n - 1 denominator).
pub fn std_dev(v: &[f64]) -> f64 {
    let m = mean(v);
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (v.len() as f64 - 1.0)).sqrt()
}

pub fn ids(p: &Particles) -> Vec<u64> {
    p.id.clone()
}
