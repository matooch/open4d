//! forge-material: the Standard material, the texture tag and the shading IR.
//! Spec: docs/specs/MATERIAL.md.

pub mod ir;
pub mod material;
pub mod params;

pub use ir::{
    Alpha, Bump, Diffuse, DiffuseModel, Displacement, Fog, IrNode, MixMode, NodeIx, NormalMap,
    Outputs, ShadeCtx, ShadingGraph, Specular, TextureSampler, Transmission, compile, mix,
};
pub use material::Material;
pub use params::{MATERIAL, MaterialPlugin, TEXTURE_TAG, standard_type, texture_tag_type};
