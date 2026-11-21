//! The camera model (RENDER.md R10).

use glam::{DMat4, DVec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Film gate width `sensor_mm` spans the image width; `film_offset` shifts the principal
    /// point by fractions of the image width (+x right, +y up).
    Perspective {
        focal_mm: f64,
        sensor_mm: f64,
        film_offset: [f64; 2],
    },
    /// Parallel projection; `width` is the world width that spans the image width.
    Orthographic { width: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Camera to world, column-major. The camera looks along local +Z.
    pub transform: [f64; 16],
    pub projection: Projection,
}

impl Camera {
    pub fn new(transform: DMat4, projection: Projection) -> Camera {
        Camera {
            transform: transform.to_cols_array(),
            projection,
        }
    }

    /// Position and heading/pitch/bank in radians (R10 conventions, see [`hpb_matrix`]).
    pub fn from_hpb(position: DVec3, hpb: [f64; 3], projection: Projection) -> Camera {
        Camera::new(hpb_matrix(position, hpb), projection)
    }

    pub fn matrix(&self) -> DMat4 {
        DMat4::from_cols_array(&self.transform)
    }

    pub fn position(&self) -> DVec3 {
        self.matrix().w_axis.truncate()
    }

    pub fn default_perspective() -> Camera {
        Camera::from_hpb(
            DVec3::new(0.0, 0.0, -500.0),
            [0.0; 3],
            Projection::Perspective {
                focal_mm: 36.0,
                sensor_mm: 36.0,
                film_offset: [0.0, 0.0],
            },
        )
    }

    /// Pixel focal length and principal point for an image of `width` x `height`.
    pub fn pixel_frame(&self, width: u32, height: u32) -> Option<(f64, [f64; 2])> {
        match self.projection {
            Projection::Perspective {
                focal_mm,
                sensor_mm,
                film_offset,
            } => {
                let w = width as f64;
                let f = w * focal_mm / sensor_mm;
                Some((
                    f,
                    [
                        w * 0.5 + film_offset[0] * w,
                        height as f64 * 0.5 - film_offset[1] * w,
                    ],
                ))
            }
            Projection::Orthographic { .. } => None,
        }
    }

    /// World-space ray through the continuous pixel position `(px, py)` (row 0 at the top).
    /// The direction has unit length.
    pub fn ray(&self, px: f64, py: f64, width: u32, height: u32) -> (DVec3, DVec3) {
        let m = self.matrix();
        match self.projection {
            Projection::Perspective { .. } => {
                let (f, c) = self.pixel_frame(width, height).unwrap_or((1.0, [0.0; 2]));
                let d = DVec3::new((px - c[0]) / f, -(py - c[1]) / f, 1.0);
                (m.w_axis.truncate(), m.transform_vector3(d).normalize())
            }
            Projection::Orthographic { width: ow } => {
                let s = ow / width as f64;
                let o = DVec3::new(
                    (px - width as f64 * 0.5) * s,
                    -(py - height as f64 * 0.5) * s,
                    0.0,
                );
                (
                    m.transform_point3(o),
                    m.transform_vector3(DVec3::Z).normalize(),
                )
            }
        }
    }
}

/// `R = Rh * Rp * Rb`. Each factor is the transpose of the usual right-handed rotation, which is
/// what the observed reference does in its left-handed system: `Rh` turns +Z toward -X (a
/// heading of -15 deg looks to the right, O30), `Rp` turns +Z toward +Y (positive pitch looks
/// up) and `Rb` turns +X toward -Y. Only the heading sign is observed; pitch and bank follow
/// from it and are **parity-pending**.
pub fn hpb_matrix(position: DVec3, hpb: [f64; 3]) -> DMat4 {
    let (sh, ch) = hpb[0].sin_cos();
    let (sp, cp) = hpb[1].sin_cos();
    let (sb, cb) = hpb[2].sin_cos();
    // Columns are the images of X, Y, Z.
    let rh = glam::DMat3::from_cols(DVec3::new(ch, 0.0, sh), DVec3::Y, DVec3::new(-sh, 0.0, ch));
    let rp = glam::DMat3::from_cols(DVec3::X, DVec3::new(0.0, cp, -sp), DVec3::new(0.0, sp, cp));
    let rb = glam::DMat3::from_cols(DVec3::new(cb, -sb, 0.0), DVec3::new(sb, cb, 0.0), DVec3::Z);
    DMat4::from_mat3_translation(rh * rp * rb, position)
}
