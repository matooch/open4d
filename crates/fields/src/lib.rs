//! forge-fields: fields (values in space), Linear / Spherical / Box, the Remapping tab and
//! field lists. Spec: docs/specs/FIELDS.md.
//!
//! A field is sampled at world positions with an [`EvalCtx`]; deformers (falloff) and effectors
//! take a [`FieldList`]. Field objects are scene nodes of category `Field`, registered by
//! [`FieldToolset`].

mod blend;
mod capsule;
mod cone;
mod cylinder;
mod formula;
mod group;
mod node;
mod radial;
mod random;
mod remap;
mod shader;
mod shape;
mod torus;

use forge_core::{Category, EvalCtx, LinkRow, NodeId, Scene, Value};
use glam::{DMat4, DVec3};

pub use blend::{BLEND_OPTIONS, BlendMode};
pub use capsule::{CAPSULE, Capsule, capsule_type};
pub use cone::{CONE, Cone, cone_type};
pub use cylinder::{CYLINDER, Cylinder, cylinder_type};
pub use formula::{FORMULA, FormulaField, formula_type};
pub use group::{GROUP, GroupField, group_type};
pub use node::{
    BOX, FIELD_LIST, FieldToolset, LINEAR, SPHERICAL, box_type, field_list_param, linear_type,
    spherical_type,
};
pub use radial::{RADIAL, Radial, radial_type};
pub use random::{
    NOISE_TYPES, RANDOM, RandomField, RandomMode, RandomSpace, random_type, reads_time,
};
pub use remap::{Contour, Remap};
pub use shader::{SHADER, shader_type};
pub use shape::{Direction, Shape};
pub use torus::{TORUS, Torus, torus_type};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FieldError {
    /// An observed option whose rules are not derived (yet).
    #[error("not supported yet: {0}")]
    Unsupported(&'static str),
    #[error("node {0} is not a field")]
    NotAField(u64),
    /// The Formula field's text does not parse.
    #[error("formula: {0}")]
    Formula(String),
}

/// The channels of one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldSample {
    /// The remapped value.
    pub value: f64,
    /// Colour channel: the field's constant Color (spec F2; `[value; 3]` for fields without one).
    pub color: [f64; 3],
    /// Direction channel (zero until it is observed).
    pub direction: DVec3,
}

impl FieldSample {
    /// `s` blended over `self` with `mode` at `strength`, channel by channel (the colour with
    /// the same function as the value, the direction always Normal: **parity-pending**).
    fn blend(&mut self, s: &FieldSample, mode: BlendMode, strength: f64) {
        self.value = mode.mix(self.value, s.value, strength);
        self.color = std::array::from_fn(|i| mode.mix(self.color[i], s.color[i], strength));
        self.direction += (s.direction - self.direction) * strength;
    }

    pub fn from_value(value: f64) -> FieldSample {
        FieldSample {
            value,
            color: [value; 3],
            direction: DVec3::ZERO,
        }
    }
}

/// Values in space.
pub trait Field: Send + Sync {
    /// The sample at a world position.
    fn sample(&self, p_world: DVec3, ctx: &EvalCtx) -> FieldSample;

    /// Samples many points; fields may override this with a batched version.
    fn sample_many(&self, points: &[DVec3], ctx: &EvalCtx) -> Vec<FieldSample> {
        points.iter().map(|p| self.sample(*p, ctx)).collect()
    }
}

/// A shape with its Remapping tab and world placement: the Linear, Spherical and Box fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeField {
    pub shape: Shape,
    pub remap: Remap,
    pub clip_to_shape: bool,
    /// The colour channel: a constant colour (the field's Color; white by default).
    pub color: [f64; 3],
    /// World to field space.
    pub world_to_local: DMat4,
}

impl ShapeField {
    /// The field with its Remapping tab checked; `world` is the field's world matrix.
    pub fn new(
        shape: Shape,
        remap: Remap,
        clip_to_shape: bool,
        world: DMat4,
    ) -> Result<ShapeField, FieldError> {
        remap.check()?;
        Ok(ShapeField {
            shape,
            remap,
            clip_to_shape,
            color: [1.0; 3],
            world_to_local: world.inverse(),
        })
    }

    /// The same field with a colour channel.
    pub fn with_color(mut self, color: [f64; 3]) -> ShapeField {
        self.color = color;
        self
    }

    /// The remapped value at a world position.
    pub fn value(&self, p_world: DVec3) -> f64 {
        let p = self.world_to_local.transform_point3(p_world);
        if self.clip_to_shape && self.shape.outside(p) {
            return 0.0;
        }
        self.remap.apply(self.shape.ramp(p))
    }
}

impl Field for ShapeField {
    fn sample(&self, p_world: DVec3, _ctx: &EvalCtx) -> FieldSample {
        FieldSample {
            color: self.color,
            ..FieldSample::from_value(self.value(p_world))
        }
    }
}

/// Builds the field of a `Category::Field` node from its parameters and world matrix.
pub fn field_from_node(scene: &Scene, id: NodeId) -> Result<Box<dyn Field>, FieldError> {
    field_from_node_at(scene, id, 0)
}

/// Group nesting deeper than this (or a group that contains itself) is refused.
const MAX_GROUP_DEPTH: usize = 16;

fn field_from_node_at(
    scene: &Scene,
    id: NodeId,
    depth: usize,
) -> Result<Box<dyn Field>, FieldError> {
    let node = scene.node(id).ok_or(FieldError::NotAField(id.0))?;
    if node.type_id == GROUP {
        if depth >= MAX_GROUP_DEPTH {
            return Err(FieldError::Unsupported("group nested too deeply"));
        }
        let rows = match node.params.get(FIELD_LIST) {
            Some(Value::LinkRows(rows)) => rows.clone(),
            _ => Vec::new(),
        };
        let list = FieldList::from_rows_at(scene, &rows, depth + 1)?;
        return group::field(&node.params, list).map(|f| Box::new(f) as Box<dyn Field>);
    }
    let world = scene.world_matrix(id).ok_or(FieldError::NotAField(id.0))?;
    node::build_field(&node.type_id, &node.params, world).ok_or(FieldError::NotAField(id.0))?
}

/// One row of a field list.
pub struct Layer {
    pub field: Box<dyn Field>,
    /// Layer strength (fraction).
    pub strength: f64,
    pub blend: BlendMode,
}

impl Layer {
    pub fn new(field: Box<dyn Field>) -> Layer {
        Layer {
            field,
            strength: 1.0,
            blend: BlendMode::Normal,
        }
    }
}

/// An ordered list of field layers, **top to bottom**, evaluated bottom to top with each layer's
/// blend mode and strength (F7). An empty list means full effect.
#[derive(Default)]
pub struct FieldList {
    pub layers: Vec<Layer>,
}

impl FieldList {
    pub fn new() -> FieldList {
        FieldList::default()
    }

    /// A list of the given field nodes, top to bottom, every row Normal at 100 % (disabled
    /// nodes left out, F-UI).
    pub fn from_nodes(
        scene: &Scene,
        ids_top_to_bottom: &[NodeId],
    ) -> Result<FieldList, FieldError> {
        let rows: Vec<LinkRow> = ids_top_to_bottom
            .iter()
            .map(|id| {
                match scene.node_type(*id) {
                    Some(t) if t.category == Category::Field => {}
                    _ => return Err(FieldError::NotAField(id.0)),
                }
                Ok(LinkRow::new(*id, BlendMode::Normal.id()))
            })
            .collect::<Result<_, _>>()?;
        FieldList::from_rows(scene, &rows)
    }

    /// The list stored in the `LinkRows` parameter `param` of `owner` (a deformer's or an
    /// effector's `fields`); an unset or other kind of value is an empty list.
    pub fn from_param(scene: &Scene, owner: NodeId, param: &str) -> Result<FieldList, FieldError> {
        match scene.param(owner, param) {
            Some(Value::LinkRows(rows)) => FieldList::from_rows(scene, rows),
            _ => Ok(FieldList::new()),
        }
    }

    /// A list from rows, top to bottom (F7). A row that is switched off, whose node is gone or is
    /// not a field, or whose node is disabled (`object.enabled`, F-UI) is left out; the others
    /// keep their blend mode and strength.
    pub fn from_rows(scene: &Scene, rows: &[LinkRow]) -> Result<FieldList, FieldError> {
        FieldList::from_rows_at(scene, rows, 0)
    }

    fn from_rows_at(
        scene: &Scene,
        rows: &[LinkRow],
        depth: usize,
    ) -> Result<FieldList, FieldError> {
        let layers = rows
            .iter()
            .filter(|r| r.enabled)
            .filter(|r| {
                scene
                    .node_type(r.target)
                    .is_some_and(|t| t.category == Category::Field)
            })
            .filter(|r| forge_core::visibility::own(scene, r.target).2)
            .map(|r| {
                field_from_node_at(scene, r.target, depth).map(|field| Layer {
                    field,
                    strength: r.strength,
                    blend: BlendMode::from_id(&r.blend).unwrap_or_default(),
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(FieldList { layers })
    }

    /// Sets the strength of layer `index` (counted from the top).
    pub fn with_strength(mut self, index: usize, strength: f64) -> FieldList {
        if let Some(l) = self.layers.get_mut(index) {
            l.strength = strength;
        }
        self
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// The list at a world position: bottom to top, `acc += (value - acc) * strength`;
    /// an empty list gives value 1.
    pub fn sample(&self, p_world: DVec3, ctx: &EvalCtx) -> FieldSample {
        if self.layers.is_empty() {
            return FieldSample::from_value(1.0);
        }
        let mut acc = FieldSample::from_value(0.0);
        for layer in self.layers.iter().rev() {
            let s = layer.field.sample(p_world, ctx);
            acc.blend(&s, layer.blend, layer.strength);
        }
        acc
    }

    /// The list at many world positions; each layer samples the whole batch (a Sorted Random field
    /// needs all points at once).
    pub fn sample_many(&self, points: &[DVec3], ctx: &EvalCtx) -> Vec<FieldSample> {
        if self.layers.is_empty() {
            return vec![FieldSample::from_value(1.0); points.len()];
        }
        let mut acc = vec![FieldSample::from_value(0.0); points.len()];
        for layer in self.layers.iter().rev() {
            for (a, s) in acc.iter_mut().zip(layer.field.sample_many(points, ctx)) {
                a.blend(&s, layer.blend, layer.strength);
            }
        }
        acc
    }
}
