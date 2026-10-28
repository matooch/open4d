//! forge-core: scene graph, data-driven parameters, undo, dirty tracking, evaluation
//! cache, native scene file and the node-type registry. Spec: docs/specs/CORE.md.

mod cache;
mod document;
pub mod file;
mod param;
mod registry;
mod scene;
pub mod smoothing;
pub mod transform;

pub use cache::EvalCache;
pub use document::{Document, EditError, History, Tx};
pub use file::{LoadError, LoadReport};
pub use param::{
    EnumOption, MeshData, ParamDesc, ParamError, ParamKind, RangeValue, UiHint, Unit, Value,
};
pub use registry::{Category, NodeType, Plugin, Registry, RegistryError};
pub use scene::{Node, NodeId, Scene};
