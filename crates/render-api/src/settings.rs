//! Render settings (RENDER.md R2).

use crate::engine::RenderError;
use crate::output::Aov;

/// Pixel reconstruction filters; `filter_width` is the full support width in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PixelFilter {
    Box,
    Triangle,
    /// Truncated at the support, standard deviation `filter_width / 4`.
    Gaussian,
    /// The cubic B-spline.
    Cubic,
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
    /// Trace camera rays through the camera's thin lens (RENDER.md R15). Off by default: a
    /// camera is then a pinhole and the image is sharp everywhere.
    pub depth_of_field: bool,
    /// Post effects of the real-time raster tier (RENDER.md R22). Off by default, so the other
    /// engines and the R21 parity tests are unchanged; they ignore the field.
    pub post: PostSettings,
    /// Optional ray-traced effects for the real-time engine (RENDER.md R24).
    pub hybrid: HybridSettings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HybridSettings {
    /// Master switch. When off, use the raster approximations.
    pub enabled: bool,
    /// Replace screen-space AO with ray-traced ambient visibility.
    pub ao: bool,
    /// Maximum AO ray distance in world units. Zero leaves ambient unchanged.
    pub ao_radius: f64,
}

impl Default for HybridSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            ao: false,
            ao_radius: 40.0,
        }
    }
}

/// The tier-1 post chain (RENDER.md R22.1). Every effect is its own toggle, all off by default;
/// the values are fixed so two renders of one scene stay bit-identical (R21.6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostSettings {
    /// Screen-space ambient occlusion (R22.2).
    pub ssao: bool,
    /// World-space sampling radius of the AO samples.
    pub ssao_radius: f64,
    /// 0 leaves the ambient term alone, 1 lets full occlusion black it out.
    pub ssao_strength: f64,
    /// Screen-space reflections (R22.3).
    pub ssr: bool,
    /// March steps per reflection ray.
    pub ssr_steps: u32,
    /// Bloom over the threshold (R22.4).
    pub bloom: bool,
    /// Linear luminance above which a pixel blooms.
    pub bloom_threshold: f64,
    /// Weight of the blurred bright pass added back.
    pub bloom_strength: f64,
    /// Temporal anti-aliasing: jittered subframes averaged per render (R22.5).
    pub taa: bool,
    /// Subframes per render.
    pub taa_samples: u32,
    /// Filmic tone mapping of the beauty (R22.6).
    pub tonemap: bool,
    /// Linear multiplier before the tone curve.
    pub exposure: f64,
}

impl Default for PostSettings {
    fn default() -> Self {
        PostSettings {
            ssao: false,
            ssao_radius: 40.0,
            ssao_strength: 1.0,
            ssr: false,
            ssr_steps: 16,
            bloom: false,
            bloom_threshold: 1.0,
            bloom_strength: 0.25,
            taa: false,
            taa_samples: 4,
            tonemap: false,
            exposure: 1.0,
        }
    }
}

impl PostSettings {
    /// True when no effect is switched on: the raster tier then renders exactly as R21.
    pub fn is_off(&self) -> bool {
        !(self.ssao || self.ssr || self.bloom || self.taa || self.tonemap)
    }

    pub fn validate(&self) -> Result<(), RenderError> {
        let bad = |s: &str| Err(RenderError::Settings(s.to_string()));
        if !(self.ssao_radius > 0.0 && self.ssao_radius.is_finite()) {
            return bad("the AO radius must be positive");
        }
        if !(0.0..=1.0).contains(&self.ssao_strength) {
            return bad("the AO strength must be in [0, 1]");
        }
        if !(1..=128).contains(&self.ssr_steps) {
            return bad("ssr steps must be in 1..=128");
        }
        if !(1..=32).contains(&self.taa_samples) {
            return bad("taa samples must be in 1..=32");
        }
        if !(self.bloom_threshold >= 0.0 && self.bloom_threshold.is_finite()) {
            return bad("the bloom threshold must not be negative");
        }
        if !(self.bloom_strength >= 0.0 && self.bloom_strength.is_finite()) {
            return bad("the bloom strength must not be negative");
        }
        if !(self.exposure > 0.0 && self.exposure.is_finite()) {
            return bad("the exposure must be positive");
        }
        Ok(())
    }
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
            depth_of_field: false,
            post: PostSettings::default(),
            hybrid: HybridSettings::default(),
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
        self.post.validate()?;
        if self.hybrid.enabled
            && self.hybrid.ao
            && !(self.hybrid.ao_radius >= 0.0 && self.hybrid.ao_radius.is_finite())
        {
            return bad("the ray-traced AO radius must be nonnegative and finite");
        }
        Ok(())
    }
}
