//! Render settings stored in the scene (docs/specs/SCENE-OBJECTS.md section S6, RENDER.md R2).
//! Defaults are the O29 section 5 values of a new document; a scene without a stored
//! configuration (`Scene::render_config() == None`) renders with the caller's own defaults.

use serde::{Deserialize, Serialize};

/// Pixel filter of the final image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderFilter {
    Box,
    Triangle,
    Gaussian,
    Cubic,
}

impl RenderFilter {
    pub fn id(self) -> &'static str {
        match self {
            RenderFilter::Box => "box",
            RenderFilter::Triangle => "triangle",
            RenderFilter::Gaussian => "gaussian",
            RenderFilter::Cubic => "cubic",
        }
    }

    pub fn parse(s: &str) -> Option<RenderFilter> {
        Some(match s {
            "box" => RenderFilter::Box,
            "triangle" => RenderFilter::Triangle,
            "gaussian" => RenderFilter::Gaussian,
            "cubic" => RenderFilter::Cubic,
            _ => return None,
        })
    }
}

/// The document's render settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderConfig {
    pub width: u32,
    pub height: u32,
    /// Samples per pixel.
    pub samples: u32,
    pub max_depth: u32,
    pub seed: u64,
    /// Light a scene without lights with the engine's default light (RENDER.md R11).
    pub default_light: bool,
    pub filter: RenderFilter,
    /// Full support width of the filter in pixels.
    pub filter_width: f64,
    /// Trace camera rays through the camera's thin lens (RENDER.md R15). A pure addition to the
    /// stored form: files written before it read as `false`.
    #[serde(default)]
    pub depth_of_field: bool,
}

impl Default for RenderConfig {
    /// O29 section 5: 1280 x 720, Cubic filter of width 2; sampling follows RENDER.md R2.
    fn default() -> Self {
        RenderConfig {
            width: 1280,
            height: 720,
            samples: 16,
            max_depth: 5,
            seed: 1,
            default_light: true,
            filter: RenderFilter::Cubic,
            filter_width: 2.0,
            depth_of_field: false,
        }
    }
}

impl RenderConfig {
    pub const MAX_SIZE: u32 = 128_000;

    /// Why the configuration is not valid, if it is not.
    pub fn check(&self) -> Result<(), String> {
        if !(1..=Self::MAX_SIZE).contains(&self.width)
            || !(1..=Self::MAX_SIZE).contains(&self.height)
        {
            return Err(format!(
                "size must be 1..={} pixels, got {} x {}",
                Self::MAX_SIZE,
                self.width,
                self.height
            ));
        }
        if !(1..=1_000_000).contains(&self.samples) {
            return Err(format!("samples must be 1..=1000000, got {}", self.samples));
        }
        if !(1..=256).contains(&self.max_depth) {
            return Err(format!("max_depth must be 1..=256, got {}", self.max_depth));
        }
        if !(self.filter_width.is_finite() && self.filter_width > 0.0 && self.filter_width <= 64.0)
        {
            return Err(format!(
                "filter_width must be in (0, 64], got {}",
                self.filter_width
            ));
        }
        Ok(())
    }
}
