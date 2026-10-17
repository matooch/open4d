//! Forge3D editor entry point.

use std::sync::Arc;

use forge_core::Registry;
use forge_generators::Primitives;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::with_builtins();
    registry.install(&Primitives)?;
    forge_ui::run(Arc::new(registry))?;
    Ok(())
}
