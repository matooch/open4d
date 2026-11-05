//! The Time effector (clone-toolset spec E21): the Plain effector with the project time in
//! seconds as the weight, so P, R and S are rates per second.

use std::collections::BTreeMap;

use forge_core::{NodeId, NodeType, Scene};
use forge_fields::FieldList;
use glam::DMat4;

use crate::effector::{Effector, EffectorCtx};
use crate::plain::{CommonDefaults, Plain, Weights, common_type, settings_from_params};
use crate::{CloneData, CloneError};

/// Type id of the Time effector.
pub const TIME: &str = "forge.effector.time";

const DEFAULTS: CommonDefaults = CommonDefaults {
    minimum: 0.0,
    position: false,
    p: [0.0; 3],
    rotation: true,
    r: [std::f64::consts::FRAC_PI_2, 0.0, 0.0],
    scale: false,
    color_mode: "off",
    uniform_scale: false,
    scale_value: 0.0,
    s: [0.0; 3],
};

/// The Time effector's node type: the Plain parameters with the Time defaults.
pub fn time_type() -> NodeType {
    common_type(TIME, "Time", &DEFAULTS)
}

/// The Time effector's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Time {
    pub base: Plain,
}

impl Default for Time {
    /// The node defaults.
    fn default() -> Self {
        from_values(&BTreeMap::new(), DMat4::IDENTITY).expect("enabled by default")
    }
}

fn from_values(values: &BTreeMap<String, forge_core::Value>, world: DMat4) -> Option<Time> {
    settings_from_params(&time_type(), values, world).map(|base| Time { base })
}

/// The Time effector of a node; `None` if the node is disabled.
pub(crate) fn from_node(scene: &Scene, id: NodeId) -> Option<Time> {
    from_values(&scene.node(id)?.params, scene.world_matrix(id)?)
}

impl Effector for Time {
    fn apply(
        &self,
        data: &mut CloneData,
        falloff: &FieldList,
        ctx: &EffectorCtx,
    ) -> Result<(), CloneError> {
        let seconds = ctx.eval.time;
        self.base.apply_weighted(data, falloff, ctx, &mut |c| {
            Weights::uniform(self.base.amount(c.value * seconds))
        })
    }
}
