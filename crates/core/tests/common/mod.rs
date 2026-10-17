//! Synthetic node types shared by the core tests (not reference-app types).
#![allow(dead_code)]

use std::sync::Arc;

use forge_core::{Category, Document, NodeType, ParamDesc, Plugin, Registry, RegistryError, Unit};

pub struct TestPlugin;

impl Plugin for TestPlugin {
    fn id(&self) -> &str {
        "test"
    }

    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        r.register(
            NodeType::new("test.box", "Box", Category::Generator)
                .param(
                    ParamDesc::float("size", "Size", 100.0)
                        .range(0.0, 1e6)
                        .unit(Unit::Distance),
                )
                .param(ParamDesc::int("segments", "Segments", 1).range(1, 1000))
                .param(ParamDesc::enumeration(
                    "mode",
                    "Mode",
                    "solid",
                    &[("solid", "Solid"), ("wire", "Wire")],
                ))
                .param(ParamDesc::link(
                    "target",
                    "Target",
                    &[Category::Object, Category::Generator],
                ))
                .param(ParamDesc::bool("enabled", "Enabled", true))
                .param(ParamDesc::vec3("offset", "Offset", [0.0; 3]).range(-10.0, 10.0))
                .param(ParamDesc::color("tint", "Tint", [1.0, 1.0, 1.0]))
                .param(ParamDesc::string("note", "Note", "")),
        )?;
        r.register(
            NodeType::new("test.bend", "Bend", Category::Deformer)
                .param(ParamDesc::float("strength", "Strength", 0.0).unit(Unit::Angle)),
        )?;
        r.register(
            NodeType::new("test.sel", "Selection", Category::Tag)
                .param(ParamDesc::string("set", "Set", "")),
        )?;
        r.register(NodeType::new("test.mat", "Material", Category::Material))?;
        Ok(())
    }
}

pub fn registry() -> Arc<Registry> {
    let mut r = Registry::with_builtins();
    r.install(&TestPlugin).expect("test plugin installs");
    Arc::new(r)
}

pub fn doc() -> Document {
    Document::new(registry())
}
