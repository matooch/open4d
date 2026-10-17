//! Synthetic node types for UI tests (not reference-app types).
#![allow(dead_code)]

use std::sync::Arc;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Plugin, Registry, RegistryError, Unit};
use forge_ui::commands::{Command, execute};
use forge_ui::state::EditorState;

pub struct TestPlugin;

impl Plugin for TestPlugin {
    fn id(&self) -> &str {
        "test"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        r.register(
            NodeType::new("test.box", "Box", Category::Generator)
                .param(
                    ParamDesc::float("size", "Size", 2.0)
                        .range(0.0, 1e6)
                        .unit(Unit::Distance)
                        .group("Object"),
                )
                .param(
                    ParamDesc::float("twist", "Twist", 0.5)
                        .unit(Unit::Angle)
                        .group("Object"),
                )
                .param(
                    ParamDesc::float("ratio", "Ratio", 0.25)
                        .range(0.0, 1.0)
                        .unit(Unit::Percent)
                        .group("Shape"),
                )
                .param(
                    ParamDesc::int("segments", "Segments", 1)
                        .range(1, 100)
                        .group("Object"),
                )
                .param(ParamDesc::enumeration(
                    "mode",
                    "Mode",
                    "solid",
                    &[("solid", "Solid"), ("wire", "Wire")],
                ))
                .param(ParamDesc::link("target", "Target", &[Category::Object]))
                .param(ParamDesc::bool("enabled", "Enabled", true).group("Shape"))
                .param(ParamDesc::vec3("offset", "Offset", [0.0; 3]))
                .param(ParamDesc::color("tint", "Tint", [1.0, 0.5, 0.25]))
                .param(ParamDesc::string("note", "Note", ""))
                .param(ParamDesc::int("internal", "Internal", 0).hidden()),
        )?;
        r.register(NodeType::new("test.bend", "Bend", Category::Deformer))?;
        r.register(NodeType::new("test.sel", "Selection", Category::Tag))?;
        r.register(NodeType::new("test.mat", "Material", Category::Material))?;
        Ok(())
    }
}

pub fn registry() -> Arc<Registry> {
    let mut r = Registry::with_builtins();
    r.install(&TestPlugin).unwrap();
    r.install(&forge_generators::Primitives).unwrap();
    Arc::new(r)
}

pub fn state() -> EditorState {
    EditorState::new(registry())
}

/// Creates a root node of `ty` and returns its id (it becomes the selection).
pub fn create(s: &mut EditorState, ty: &str) -> NodeId {
    execute(s, Command::Create(ty.into())).unwrap();
    s.selection.active.unwrap()
}
