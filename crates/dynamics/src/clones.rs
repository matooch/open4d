//! Dynamics on clones (docs/specs/DYNAMICS.md §6): a cloner that carries a rigid body tag
//! makes every visible clone its own body; the simulated poses go back into the clone data.

use forge_clone_engine::CLONER;
use forge_core::{EvalCtx, NodeId, Scene};
use forge_generators::cloner_instances;

use crate::build::{body_tag, kind_of, objects_in_order, spec_from_tag};
use crate::sim::{BodyKey, BodySpec};

fn ctx(scene: &Scene) -> EvalCtx {
    EvalCtx {
        time: 0.0,
        fps: f64::from(scene.settings().fps),
        seed: 0,
    }
}

/// True for a cloner node.
pub fn is_cloner(scene: &Scene, id: NodeId) -> bool {
    scene.node(id).is_some_and(|n| n.type_id == CLONER)
}

/// One body per visible clone of every cloner with an enabled rigid body tag, in cloner order
/// then clone order. The clone's world matrix is the cloner's times the clone matrix; its shape
/// comes from the clone's prototype.
pub fn clone_bodies(scene: &Scene) -> Vec<BodySpec> {
    let mut out = Vec::new();
    let ctx = ctx(scene);
    for id in objects_in_order(scene) {
        if !is_cloner(scene, id) {
            continue;
        }
        let Some(tag) = body_tag(scene, id) else {
            continue;
        };
        let Some(kind) = kind_of(scene, tag) else {
            continue;
        };
        let Some(world) = scene.world_matrix(id) else {
            continue;
        };
        let Ok(Some(inst)) = cloner_instances(scene, id, &ctx) else {
            continue;
        };
        for i in 0..inst.clones.len() {
            if !inst.clones.visible[i] {
                continue;
            }
            let mesh = inst.prototypes[inst.clones.child[i] as usize].clone();
            out.push(spec_from_tag(
                scene,
                tag,
                BodyKey::Clone {
                    cloner: id,
                    index: i as u32,
                },
                kind,
                world * inst.clones.matrices[i],
                mesh,
            ));
        }
    }
    out
}
