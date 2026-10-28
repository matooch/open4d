//! forge-io: file formats. Spec: docs/specs/IO.md.

pub mod obj;
mod scene;

pub use scene::{export_scene, import_into};
