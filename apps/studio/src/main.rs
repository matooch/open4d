//! Forge3D editor entry point.

use std::sync::Arc;

use forge_core::Registry;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    forge_ui::run(Arc::new(Registry::with_builtins()))?;
    Ok(())
}
