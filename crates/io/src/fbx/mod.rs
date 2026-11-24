//! FBX exchange (docs/specs/IO.md §8): import through the `ufbx` crate (brief 165).

mod apply;
mod read;

pub use read::{ImportScene, read};

use std::collections::BTreeMap;

use forge_core::Track;
use forge_geometry::{CornerUvs, PolyMesh};
use forge_rig::weights::WeightMap;
use glam::{DMat4, DVec3};

/// Files above this size are rejected before parsing.
pub(crate) const MAX_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid FBX: {0}")]
    Invalid(String),
}

/// One object of the file in our conventions (left-handed Y-up, centimetres by default).
pub(crate) struct Node {
    pub name: String,
    pub kind: Kind,
    pub children: Vec<usize>,
    pub pos: DVec3,
    /// `(h, p, b)` in radians.
    pub rot: DVec3,
    pub scale: DVec3,
    pub tracks: BTreeMap<String, Track>,
}

pub(crate) enum Kind {
    Null,
    Joint,
    Mesh(Box<MeshPart>),
    Camera(Camera),
    Light(Light),
}

pub(crate) struct MeshPart {
    pub mesh: PolyMesh,
    pub uv: Option<CornerUvs>,
    /// Add a smoothing tag (the file shades the mesh smoothly or has no normals).
    pub smooth: bool,
    /// Material slot of every polygon.
    pub face_slot: Vec<u32>,
    /// Material index (into `ImportScene::materials`) of every slot.
    pub slots: Vec<Option<usize>>,
    pub skin: Option<Skin>,
}

pub(crate) struct Skin {
    /// Node indices, in cluster order.
    pub joints: Vec<usize>,
    pub weights: WeightMap,
    /// Joint bind matrices in the mesh's space.
    pub bind: Vec<DMat4>,
}

pub(crate) struct Camera {
    pub sensor: f64,
    pub fov: f64,
    pub near: f64,
    pub far: f64,
    pub focus: Option<f64>,
    /// `Some(zoom)` for an orthographic camera.
    pub ortho_zoom: Option<f64>,
}

pub(crate) struct Light {
    pub kind: &'static str,
    pub color: [f64; 3],
    pub intensity: f64,
    pub inner: f64,
    pub outer: f64,
}

pub(crate) struct Material {
    pub name: String,
    pub graph: String,
}
