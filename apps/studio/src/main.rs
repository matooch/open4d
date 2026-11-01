//! Forge3D editor entry point.

use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = forge_app::default_registry()?;
    forge_ui::run(Arc::new(registry))?;
    Ok(())
}
