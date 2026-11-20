//! Render settings (RENDER.md R2).

use crate::engine::RenderError;
use crate::output::Aov;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PixelFilter {
    Box,
    Triangle,
    Gaussian,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderSettings {
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub max_depth: u32,
    pub seed: u64,
    pub tile: u32,
    pub filter: PixelFilter,
    pub filter_width: f64,
    pub aovs: Vec<Aov>,
    pub default_light: bool,
    pub clamp: f64,
    /// `[x0, y0, x1, y1]`, x1/y1 exclusive.
    pub region: Option<[u32; 4]>,
    pub threads: Option<usize>,
}

impl Default for RenderSettings {
    fn default() -> Self {
        RenderSettings {
            width: 320,
            height: 240,
            samples: 16,
            max_depth: 5,
            seed: 1,
            tile: 32,
            filter: PixelFilter::Box,
            filter_width: 1.0,
            aovs: Vec::new(),
            default_light: true,
            clamp: 0.0,
            region: None,
            threads: None,
        }
    }
}

impl RenderSettings {
    pub fn new(width: u32, height: u32, samples: u32) -> RenderSettings {
        RenderSettings {
            width,
            height,
            samples,
            ..RenderSettings::default()
        }
    }

    pub fn validate(&self) -> Result<(), RenderError> {
        let bad = |s: &str| Err(RenderError::Settings(s.to_string()));
        if self.width == 0 || self.height == 0 {
            return bad("the image size must be at least 1 x 1");
        }
        if self.samples == 0 {
            return bad("samples must be at least 1");
        }
        if self.tile == 0 {
            return bad("tile size must be at least 1");
        }
        if !(self.filter_width > 0.0 && self.filter_width.is_finite()) {
            return bad("filter width must be positive");
        }
        if let Some([x0, y0, x1, y1]) = self.region
            && (x0 >= x1 || y0 >= y1 || x1 > self.width || y1 > self.height)
        {
            return bad("the region is empty or outside the image");
        }
        if self.threads == Some(0) {
            return bad("threads must be at least 1");
        }
        Ok(())
    }
}
