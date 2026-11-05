//! Colour management: sRGB transfer function, colour-space tags, alpha modes
//! (docs/specs/IMAGE.md §1). The working space is linear sRGB (Rec.709 primaries).

use std::sync::OnceLock;

/// How the colour values of a texture are to be interpreted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ColorSpace {
    /// Integer files hold sRGB-encoded colour; decoded to linear on load.
    #[default]
    Srgb,
    /// Integer files hold linear values (normalised only).
    Linear,
    /// Data (normals, masks, roughness): never transformed or alpha-converted.
    Raw,
}

impl ColorSpace {
    pub fn as_str(self) -> &'static str {
        match self {
            ColorSpace::Srgb => "srgb",
            ColorSpace::Linear => "linear",
            ColorSpace::Raw => "raw",
        }
    }

    /// Parses a param string (case-insensitive).
    pub fn parse(s: &str) -> Option<ColorSpace> {
        match s.to_ascii_lowercase().as_str() {
            "srgb" => Some(ColorSpace::Srgb),
            "linear" => Some(ColorSpace::Linear),
            "raw" => Some(ColorSpace::Raw),
            _ => None,
        }
    }
}

/// Whether colour channels are already multiplied by alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AlphaMode {
    #[default]
    Straight,
    Premultiplied,
}

/// sRGB-encoded value to linear (IEC 61966-2-1), odd extension outside 0..1.
pub fn srgb_to_linear(v: f32) -> f32 {
    let a = v.abs();
    let l = if a <= 0.04045 {
        a / 12.92
    } else {
        ((a + 0.055) / 1.055).powf(2.4)
    };
    l.copysign(v)
}

/// Linear value to sRGB-encoded, odd extension outside 0..1.
pub fn linear_to_srgb(v: f32) -> f32 {
    let a = v.abs();
    let e = if a <= 0.003_130_8 {
        a * 12.92
    } else {
        1.055 * a.powf(1.0 / 2.4) - 0.055
    };
    e.copysign(v)
}

static TABLE: OnceLock<[f32; 256]> = OnceLock::new();

/// sRGB 8-bit code to linear (table).
pub fn srgb8_to_linear(code: u8) -> f32 {
    let t = TABLE.get_or_init(|| {
        let mut t = [0.0f32; 256];
        for (i, v) in t.iter_mut().enumerate() {
            // f64 so every entry is the correctly rounded f32 of the exact curve
            let e = i as f64 / 255.0;
            let l = if e <= 0.04045 {
                e / 12.92
            } else {
                ((e + 0.055) / 1.055).powf(2.4)
            };
            *v = l as f32;
        }
        t
    });
    t[code as usize]
}

/// Linear value to the nearest sRGB 8-bit code (clamped; NaN gives 0).
pub fn linear_to_srgb8(v: f32) -> u8 {
    if v.is_nan() {
        return 0;
    }
    (linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8
}

/// RGBA interleaved, in place: `rgb *= a`.
pub fn premultiply(rgba: &mut [f32]) {
    for p in rgba.as_chunks_mut::<4>().0 {
        let a = p[3];
        p[0] *= a;
        p[1] *= a;
        p[2] *= a;
    }
}

/// RGBA interleaved, in place: `rgb /= a` (alpha 0 gives rgb 0).
pub fn unpremultiply(rgba: &mut [f32]) {
    for p in rgba.as_chunks_mut::<4>().0 {
        let a = p[3];
        if a == 0.0 {
            p[0] = 0.0;
            p[1] = 0.0;
            p[2] = 0.0;
        } else {
            p[0] /= a;
            p[1] /= a;
            p[2] /= a;
        }
    }
}
