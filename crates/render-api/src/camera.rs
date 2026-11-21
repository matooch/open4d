//! The camera model (RENDER.md R10).

use glam::{DMat4, DVec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Film gate width `sensor_mm` spans the image width; `film_offset` shifts the principal
    /// point by a fraction of the image width in x (+x right) and of the image height in y
    /// (+y up): `film-offset` (X) and `film-offset-y` (RENDER.md R15).
    Perspective {
        focal_mm: f64,
        sensor_mm: f64,
        film_offset: [f64; 2],
    },
    /// Parallel projection; `width` is the world width that spans the image width.
    Orthographic { width: f64 },
}

/// A thin lens for depth of field (RENDER.md R15): rays start on a disc of `radius` (scene units)
/// in the camera's XY plane and meet at the plane `focus_distance` ahead of the camera. Only
/// perspective cameras use it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aperture {
    pub radius: f64,
    pub focus_distance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Camera to world, column-major. The camera looks along local +Z.
    pub transform: [f64; 16],
    pub projection: Projection,
    /// The lens, used when the render settings ask for depth of field. `None`: a pinhole.
    pub aperture: Option<Aperture>,
}

impl Camera {
    pub fn new(transform: DMat4, projection: Projection) -> Camera {
        Camera {
            transform: transform.to_cols_array(),
            projection,
            aperture: None,
        }
    }

    /// The same camera with a thin lens.
    pub fn with_aperture(mut self, aperture: Aperture) -> Camera {
        self.aperture = Some(aperture);
        self
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
                        height as f64 * 0.5 - film_offset[1] * height as f64,
                    ],
                ))
            }
            Projection::Orthographic { .. } => None,
        }
    }

    /// Like [`Camera::ray`] through the thin lens: the origin is the point `u` (two numbers in
    /// `[0, 1)`) of the aperture disc (concentric mapping), the direction aims at the point of
    /// the focus plane that the pinhole ray through `(px, py)` hits. Without an aperture, or for
    /// an orthographic camera, this is [`Camera::ray`].
    pub fn ray_lens(
        &self,
        px: f64,
        py: f64,
        width: u32,
        height: u32,
        u: (f64, f64),
    ) -> (DVec3, DVec3) {
        let (Some(ap), Projection::Perspective { .. }) = (self.aperture, self.projection) else {
            return self.ray(px, py, width, height);
        };
        let (f, c) = self.pixel_frame(width, height).unwrap_or((1.0, [0.0; 2]));
        let d = DVec3::new((px - c[0]) / f, -(py - c[1]) / f, 1.0);
        // the pinhole ray reaches z = focus_distance at d * focus_distance
        let focus = d * ap.focus_distance;
        let (lx, ly) = concentric_disc(u);
        let origin = DVec3::new(lx * ap.radius, ly * ap.radius, 0.0);
        let m = self.matrix();
        (
            m.transform_point3(origin),
            m.transform_vector3((focus - origin).normalize()),
        )
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

/// Shirley-Chiu concentric map of the unit square to the unit disc.
pub fn concentric_disc(u: (f64, f64)) -> (f64, f64) {
    let (a, b) = (2.0 * u.0 - 1.0, 2.0 * u.1 - 1.0);
    if a == 0.0 && b == 0.0 {
        return (0.0, 0.0);
    }
    let (r, th) = if a.abs() > b.abs() {
        (a, std::f64::consts::FRAC_PI_4 * (b / a))
    } else {
        (
            b,
            std::f64::consts::FRAC_PI_2 - std::f64::consts::FRAC_PI_4 * (a / b),
        )
    };
    (r * th.cos(), r * th.sin())
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
