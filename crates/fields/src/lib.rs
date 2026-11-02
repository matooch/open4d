//! forge-fields: fields (values in space), Linear / Spherical / Box, the Remapping tab and
//! field lists. Spec: docs/specs/FIELDS.md.
//!
//! A field is sampled at world positions with an [`EvalCtx`]; deformers (falloff) and effectors
//! take a [`FieldList`]. Field objects are scene nodes of category `Field`, registered by
//! [`FieldToolset`].

mod capsule;
mod cone;
mod cylinder;
mod node;
mod radial;
mod random;
mod remap;
mod shape;
mod torus;

use forge_core::{EvalCtx, NodeId, Scene, Value};
use glam::{DMat4, DVec3};

pub use capsule::{CAPSULE, Capsule, capsule_type};
pub use cone::{CONE, Cone, cone_type};
pub use cylinder::{CYLINDER, Cylinder, cylinder_type};
pub use node::{
    BOX, FieldToolset, LAYER_STRENGTH, LINEAR, SPHERICAL, box_type, linear_type, spherical_type,
};
pub use radial::{RADIAL, Radial, radial_type};
pub use random::{NOISE_TYPES, RANDOM, RandomField, RandomMode, RandomSpace, random_type};
pub use remap::{Contour, Remap};
pub use shape::{Direction, Shape};
pub use torus::{TORUS, Torus, torus_type};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FieldError {
    /// An observed option whose rules are not derived (yet).
    #[error("not supported yet: {0}")]
    Unsupported(&'static str),
    #[error("node {0} is not a field")]
    NotAField(u64),
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
    /// Normal blend of `s` over `self` by `strength`: `self += (s - self) * strength`.
    fn blend_normal(&mut self, s: &FieldSample, strength: f64) {
        self.value += (s.value - self.value) * strength;
        self.color =
            std::array::from_fn(|i| self.color[i] + (s.color[i] - self.color[i]) * strength);
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
    let node = scene.node(id).ok_or(FieldError::NotAField(id.0))?;
    let world = scene.world_matrix(id).ok_or(FieldError::NotAField(id.0))?;
    node::build_field(&node.type_id, &node.params, world).ok_or(FieldError::NotAField(id.0))?
}

/// How a layer combines with the layers below it. Only Normal exists so far (059).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Normal,
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

/// An ordered list of field layers, **top to bottom**. An empty list means full effect.
#[derive(Default)]
pub struct FieldList {
    pub layers: Vec<Layer>,
}

impl FieldList {
    pub fn new() -> FieldList {
        FieldList::default()
    }

    /// A list of the given field nodes, top to bottom (disabled ones left out); each layer's strength is the
    /// node's `layer_strength` (100 % by default).
    pub fn from_nodes(
        scene: &Scene,
        ids_top_to_bottom: &[NodeId],
    ) -> Result<FieldList, FieldError> {
        let layers = ids_top_to_bottom
            .iter()
            // A field switched off in the list (`object.enabled`) is not a layer (spec F-UI).
            .filter(|id| forge_core::visibility::own(scene, **id).2)
            .map(|id| {
                field_from_node(scene, *id).map(|field| {
                    let mut layer = Layer::new(field);
                    if let Some(Value::Float(s)) = scene.param(*id, LAYER_STRENGTH) {
                        layer.strength = *s;
                    }
                    layer
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
            let BlendMode::Normal = layer.blend;
            acc.blend_normal(&s, layer.strength);
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
            let BlendMode::Normal = layer.blend;
            for (a, s) in acc.iter_mut().zip(layer.field.sample_many(points, ctx)) {
                a.blend_normal(&s, layer.strength);
            }
        }
        acc
    }
}
