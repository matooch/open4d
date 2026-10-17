//! Orbit camera, projection and pixel/ray mapping (docs/specs/VIEWPORT.md §1).
//! Engine convention: right-handed, +Y up.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use glam::dcamera::rh::{proj::directx, view::look_at_mat4};
use glam::{DMat4, DVec3, DVec4};

const PITCH_LIMIT: f64 = FRAC_PI_2 - 1e-4;
const MIN_SIZE: f64 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Vertical field of view in radians.
    Perspective { fov_y: f64 },
    /// World units visible vertically.
    Orthographic { height: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: DVec3,
    /// Unit length.
    pub dir: DVec3,
}

impl Ray {
    pub fn at(&self, t: f64) -> DVec3 {
        self.origin + self.dir * t
    }
}

/// Eye placement along a world axis, relative to the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisView {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub target: DVec3,
    pub distance: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub projection: Projection,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            target: DVec3::ZERO,
            distance: 10.0,
            yaw: 0.0,
            pitch: 0.0,
            projection: Projection::Perspective { fov_y: FRAC_PI_4 },
        }
    }
}

impl Camera {
    /// Unit vector from the target towards the eye.
    fn offset_dir(&self) -> DVec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        DVec3::new(cp * sy, sp, cp * cy)
    }

    pub fn eye(&self) -> DVec3 {
        self.target + self.offset_dir() * self.distance
    }

    pub fn forward(&self) -> DVec3 {
        -self.offset_dir()
    }

    pub fn right(&self) -> DVec3 {
        self.forward().cross(DVec3::Y).normalize()
    }

    pub fn up(&self) -> DVec3 {
        self.right().cross(self.forward())
    }

    pub fn view(&self) -> DMat4 {
        look_at_mat4(self.eye(), self.target, DVec3::Y)
    }

    pub fn projection(&self, aspect: f64) -> DMat4 {
        match self.projection {
            Projection::Perspective { fov_y } => {
                let near = (self.distance * 1e-3).max(1e-6);
                directx::perspective(fov_y, aspect, near, self.distance * 1e4)
            }
            Projection::Orthographic { height } => {
                let (hh, far) = (height / 2.0, self.distance * 1e4);
                directx::orthographic(-hh * aspect, hh * aspect, -hh, hh, -far, far)
            }
        }
    }

    pub fn view_proj(&self, aspect: f64) -> DMat4 {
        self.projection(aspect) * self.view()
    }

    pub fn orbit(&mut self, d_yaw: f64, d_pitch: f64) {
        self.yaw += d_yaw;
        self.pitch = (self.pitch + d_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// World size of one pixel at the target's depth.
    pub fn world_per_pixel(&self, viewport_h: f64) -> f64 {
        match self.projection {
            Projection::Perspective { fov_y } => {
                2.0 * self.distance * (fov_y / 2.0).tan() / viewport_h
            }
            Projection::Orthographic { height } => height / viewport_h,
        }
    }

    /// Moves the view so content follows the cursor by `(dx, dy)` pixels.
    pub fn pan(&mut self, dx_px: f64, dy_px: f64, viewport_h: f64) {
        let wpp = self.world_per_pixel(viewport_h);
        self.target += -self.right() * dx_px * wpp + self.up() * dy_px * wpp;
    }

    /// Scales distance (perspective) or visible height (orthographic).
    pub fn dolly(&mut self, factor: f64) {
        if factor <= 0.0 {
            return;
        }
        match &mut self.projection {
            Projection::Perspective { .. } => {
                self.distance = (self.distance * factor).max(MIN_SIZE)
            }
            Projection::Orthographic { height } => *height = (*height * factor).max(MIN_SIZE),
        }
    }

    /// Centres the box and fits its bounding sphere into view.
    pub fn frame(&mut self, min: DVec3, max: DVec3) {
        self.target = (min + max) / 2.0;
        let radius = ((max - min).length() / 2.0).max(MIN_SIZE);
        match &mut self.projection {
            Projection::Perspective { fov_y } => self.distance = radius / (*fov_y / 2.0).sin(),
            Projection::Orthographic { height } => {
                *height = 2.0 * radius;
                self.distance = self.distance.max(2.0 * radius);
            }
        }
    }

    pub fn look_along(&mut self, view: AxisView) {
        (self.yaw, self.pitch) = match view {
            AxisView::PosX => (FRAC_PI_2, 0.0),
            AxisView::NegX => (-FRAC_PI_2, 0.0),
            AxisView::PosZ => (0.0, 0.0),
            AxisView::NegZ => (PI, 0.0),
            AxisView::PosY => (0.0, PITCH_LIMIT),
            AxisView::NegY => (0.0, -PITCH_LIMIT),
        };
    }

    /// `(x_px, y_px, depth)`; `None` behind the camera.
    pub fn project(&self, world: DVec3, w: f64, h: f64) -> Option<DVec3> {
        let clip = self.view_proj(w / h) * world.extend(1.0);
        if clip.w <= 1e-12 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(DVec3::new(
            (ndc.x + 1.0) / 2.0 * w,
            (1.0 - ndc.y) / 2.0 * h,
            ndc.z,
        ))
    }

    /// World ray through pixel `(x, y)`.
    pub fn ray(&self, x_px: f64, y_px: f64, w: f64, h: f64) -> Ray {
        let ndc_x = 2.0 * x_px / w - 1.0;
        let ndc_y = 1.0 - 2.0 * y_px / h;
        let inv = self.view_proj(w / h).inverse();
        let unproject = |z: f64| {
            let p: DVec4 = inv * DVec4::new(ndc_x, ndc_y, z, 1.0);
            p.truncate() / p.w
        };
        match self.projection {
            Projection::Perspective { .. } => {
                let eye = self.eye();
                Ray {
                    origin: eye,
                    dir: (unproject(0.0) - eye).normalize(),
                }
            }
            Projection::Orthographic { .. } => Ray {
                origin: unproject(0.0),
                dir: self.forward(),
            },
        }
    }
}
