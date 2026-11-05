//! The effector framework: the [`Effector`] trait, the cloner's effector list and its
//! application in list order (clone-toolset spec E1).

use forge_core::{Category, EvalCtx, NodeId, Scene, Value};
use forge_fields::FieldList;
use glam::DMat4;

use crate::{CloneData, CloneError};
use crate::{delay, formula, plain, random, step, target, time};

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
        step::STEP => Ok(step::from_node(scene, id).map(|e| Box::new(e) as Box<dyn Effector>)),
        formula::FORMULA => {
            Ok(formula::from_node(scene, id).map(|e| Box::new(e) as Box<dyn Effector>))
        }
        time::TIME => Ok(time::from_node(scene, id).map(|e| Box::new(e) as Box<dyn Effector>)),
        target::TARGET => {
            Ok(target::from_node(scene, id).map(|e| Box::new(e) as Box<dyn Effector>))
        }
        delay::DELAY => Ok(delay::from_node(scene, id).map(|e| Box::new(e) as Box<dyn Effector>)),
        _ => Err(CloneError::Unsupported("this effector type")),
    }
}

/// The field list of an effector node (its `fields` rows, top to bottom). A row that is off, or
/// a link to a deleted or non-field node, is skipped, like an empty slot.
fn falloff_of(scene: &Scene, id: NodeId) -> Result<FieldList, CloneError> {
    FieldList::from_param(scene, id, "fields")
        .map_err(|_| CloneError::Unsupported("a field type the effector falloff cannot read"))
}

/// Runs the effectors linked from the cloner `cloner`, in list order, on `data`.
///
/// A Delay effector (spec E23) needs the input of the earlier frames: they are replayed
/// from the cloner and the effectors before it, so no state is kept between evaluations.
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
    if list
        .iter()
        .filter(|id| delay::is_delay(scene, **id))
        .count()
        > 1
    {
        return Err(CloneError::Unsupported("more than one Delay effector"));
    }
    let ctx = EffectorCtx {
        eval,
        cloner_world: scene.world_matrix(cloner).unwrap_or(DMat4::IDENTITY),
    };
    for (k, id) in list.iter().enumerate() {
        if delay::is_delay(scene, *id) {
            if let Some(d) = delay::from_node(scene, *id) {
                let history = replay(scene, cloner, &list[..k], eval)?;
                d.smooth(&history, data, &falloff_of(scene, *id)?, &ctx)?;
            }
        } else if let Some(effector) = effector_from_node(scene, *id)? {
            effector.apply(data, &falloff_of(scene, *id)?, &ctx)?;
        }
    }
    Ok(())
}

/// The clone data the effectors `before` leave at the frames before the current one, oldest
/// first, at most [`delay::WINDOW`] frames (none at frame 0).
fn replay(
    scene: &Scene,
    cloner: NodeId,
    before: &[NodeId],
    eval: &EvalCtx,
) -> Result<Vec<CloneData>, CloneError> {
    if eval.fps <= 0.0 || !eval.time.is_finite() {
        return Ok(Vec::new());
    }
    let frame = (eval.time * eval.fps).round() as i64;
    let Some(node) = scene.node(cloner) else {
        return Ok(Vec::new());
    };
    let settings = crate::settings_from_params(&node.params);
    let mut out = Vec::new();
    for g in (frame - delay::WINDOW).max(0)..frame {
        let at = EvalCtx {
            time: g as f64 / eval.fps,
            fps: eval.fps,
            seed: eval.seed,
        };
        let mut data = crate::generate(&settings, &at)?;
        let ctx = EffectorCtx {
            eval: &at,
            cloner_world: scene.world_matrix(cloner).unwrap_or(DMat4::IDENTITY),
        };
        for id in before {
            if let Some(effector) = effector_from_node(scene, *id)? {
                effector.apply(&mut data, &falloff_of(scene, *id)?, &ctx)?;
            }
        }
        out.push(data);
    }
    Ok(out)
}
