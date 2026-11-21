//! forge-core: scene graph, data-driven parameters, undo, dirty tracking, evaluation
//! cache, native scene file and the node-type registry. Spec: docs/specs/CORE.md.

mod cache;
pub mod depgraph;
mod document;
mod eval;
pub mod file;
pub mod layers;
mod param;
mod registry;
pub mod render_config;
mod scene;
pub mod smoothing;
pub mod tags;
pub mod time;
mod track;
pub mod transform;
pub mod visibility;

pub use cache::EvalCache;
pub use document::{Document, EditError, History, Tx};
pub use eval::EvalCtx;
pub use file::{LoadError, LoadReport};
pub use layers::{Layer, LayerId, Layers};
pub use param::{
    EnumOption, LinkRow, MeshData, ParamDesc, ParamError, ParamKind, RangeValue, UiHint, Unit,
    Value,
};
pub use registry::{Category, NodeEvaluator, NodeType, Plugin, Registry, RegistryError};
pub use render_config::{RenderConfig, RenderFilter};
pub use scene::{Node, NodeId, Scene, SceneSettings};
pub use time::{Marker, PlayMode, TimeConfig, TimeDisplay};
pub use track::{
    Behavior, Handle, Interp, Key, KeyTangent, TangentMode, Target, Track, TrackError, TrackTarget,
    parse_target, parse_track_target,
};
