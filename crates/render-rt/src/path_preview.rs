//! Spatial reconstruction for progressive path-traced previews (R25.2).

use forge_image::Image;
use forge_render_api::RenderError;
use glam::Vec3;

/// Geometry at the primary ray, independent of the noisy radiance estimate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PrimaryGuide {
    Inactive,
    Miss,
    Hit {
        /// Positive linear depth in world units.
        depth: f32,
        /// Unit shading normal in world space.
        normal: [f32; 3],
    },
}

impl PrimaryGuide {
    fn compatible(self, neighbour: Self) -> bool {
        let (
            Self::Hit { depth, normal },
            Self::Hit {
                depth: other_depth,
                normal: other_normal,
            },
        ) = (self, neighbour)
        else {
            return false;
        };
        (depth - other_depth).abs() <= 0.05 * depth.min(other_depth)
            && Vec3::from_array(normal).dot(Vec3::from_array(other_normal)) >= 0.9
    }
}

/// Filter a completed batch's linear mean without mutating it or changing alpha mode.
/// `guides` contains one row-major primary guide per pixel, including inactive pixels.
pub fn denoise(raw: &Image, guides: &[PrimaryGuide], samples: u32) -> Result<Image, RenderError> {
    let width = raw.width as usize;
    let height = raw.height as usize;
    let (pixels, remainder) = raw.data.as_chunks::<4>();
    if samples == 0
        || width.checked_mul(height) != Some(pixels.len())
        || !remainder.is_empty()
        || guides.len() != pixels.len()
    {
        return Err(RenderError::Output(
            "preview requires a completed batch and one primary guide per pixel".into(),
        ));
    }

    let mut preview = raw.clone();
    let strength = f64::from(samples).sqrt().recip();
    for (i, dst) in preview.data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        if !matches!(guides[i], PrimaryGuide::Hit { .. }) {
            continue;
        }
        let x = i % width;
        let y = i / width;
        let centre = pixels[i];
        let mut weight = 4.0;
        let mut sum = [centre[0], centre[1], centre[2]].map(|v| f64::from(v) * weight);
        for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
            for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                let j = ny * width + nx;
                let p = pixels[j];
                if i == j || p[3] != centre[3] || !guides[i].compatible(guides[j]) {
                    continue;
                }
                let w = if x == nx || y == ny { 2.0 } else { 1.0 };
                weight += w;
                for c in 0..3 {
                    sum[c] += f64::from(p[c]) * w;
                }
            }
        }
        if weight > 4.0 {
            for c in 0..3 {
                let original = f64::from(centre[c]);
                dst[c] = ((sum[c] / weight - original) * strength + original) as f32;
            }
        }
    }
    Ok(preview)
}
