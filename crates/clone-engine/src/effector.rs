//! The effector framework: the [`Effector`] trait, the cloner's effector list and its
//! application in list order (clone-toolset spec E1).

use forge_core::{Category, EvalCtx, NodeId, Scene, Value};
use forge_fields::FieldList;
use glam::DMat4;

use crate::{CloneData, CloneError};
use crate::{plain, random};

/// What an effector may read besides the clone data.
#[derive(Debug, Clone, Copy)]
pub struct EffectorCtx<'a> {
    /// Time, fps and seed of the evaluation.
    pub eval: &'a EvalCtx,
    /// World matrix of the cloner: clone matrices are in cloner space, fields are sampled
    /// at world positions.
    pub cloner_world: DMat4,
}

/// Something that modifies clone data. Effectors are `Category::Effector` nodes linked from
/// the cloner's `effectors` list and run in list order.
pub trait Effector {
    /// Modifies `data` in place. `falloff` is the effector's field list (an empty list means
    /// full effect, value 1). Options without derived rules give [`CloneError::Unsupported`].
    fn apply(
        &self,
        data: &mut CloneData,
        falloff: &FieldList,
        ctx: &EffectorCtx,
    ) -> Result<(), CloneError>;
}

/// The effector of node `id`, or `None` for a node that is missing, disabled or not an
/// effector. Unknown effector types are `Unsupported`.
pub fn effector_from_node(
    scene: &Scene,
    id: NodeId,
) -> Result<Option<Box<dyn Effector>>, CloneError> {
    let Some(node) = scene.node(id) else {
        return Ok(None);
    };
    let Some(ty) = scene.node_type(id) else {
        return Ok(None);
    };
    if ty.category != Category::Effector {
        return Ok(None);
    }
    match node.type_id.as_str() {
        plain::PLAIN => Ok(plain::from_node(scene, id).map(|p| Box::new(p) as Box<dyn Effector>)),
        random::RANDOM => {
            Ok(random::from_node(scene, id).map(|r| Box::new(r) as Box<dyn Effector>))
        }
        _ => Err(CloneError::Unsupported("this effector type")),
    }
}

/// The field list of an effector node (its `fields` list, top to bottom).
fn falloff_of(scene: &Scene, id: NodeId) -> Result<FieldList, CloneError> {
    let ids: Vec<NodeId> = match scene.param(id, "fields") {
        Some(Value::LinkList(l)) => l.clone(),
        _ => Vec::new(),
    };
    // A link to a deleted or non-field node is skipped, like an empty slot.
    let ids: Vec<NodeId> = ids
        .into_iter()
        .filter(|f| {
            scene
                .node_type(*f)
                .is_some_and(|t| t.category == Category::Field)
        })
        .collect();
    FieldList::from_nodes(scene, &ids)
        .map_err(|_| CloneError::Unsupported("a field type the effector falloff cannot read"))
}

/// Runs the effectors linked from the cloner `cloner`, in list order, on `data`.
pub fn apply_effectors(
    scene: &Scene,
    cloner: NodeId,
    data: &mut CloneData,
    eval: &EvalCtx,
) -> Result<(), CloneError> {
    let list = match scene.param(cloner, "effectors") {
        Some(Value::LinkList(l)) => l.clone(),
        _ => return Ok(()),
    };
    let ctx = EffectorCtx {
        eval,
        cloner_world: scene.world_matrix(cloner).unwrap_or(DMat4::IDENTITY),
    };
    for id in list {
        if let Some(effector) = effector_from_node(scene, id)? {
            effector.apply(data, &falloff_of(scene, id)?, &ctx)?;
        }
    }
    Ok(())
}
