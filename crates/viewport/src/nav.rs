//! Viewport navigation as pure camera functions (docs/specs/VIEWPORT.md §7).
//! Directions, speeds, margins and view orientations are our own design, parity-pending G2.

use std::f64::consts::FRAC_PI_4;

use glam::DVec3;

use crate::camera::{AxisView, Camera, Handedness, Projection};

/// Orbit speed of a drag, radians per pixel.
pub const ORBIT_RAD_PER_PX: f64 = 0.01;
/// Dolly speed of a drag: the distance scales by `exp(-DOLLY_PER_PX · dx)`.
pub const DOLLY_PER_PX: f64 = 0.01;
/// Wheel zoom: distance scales by `WHEEL_BASE^points`.
pub const WHEEL_BASE: f64 = 0.999;
/// Space kept around framed geometry (radius factor).
pub const FRAME_MARGIN: f64 = 1.1;
/// Orientation of the default perspective view (the editor's default camera).
pub const DEFAULT_YAW: f64 = 0.68;
pub const DEFAULT_PITCH: f64 = 0.52;

const DEGENERATE_RADIUS: f64 = 1e-9;

/// The named views of `view.set_view`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardView {
    Perspective,
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

/// Alt + primary drag: orbit around the pivot. Pivot and distance are unchanged.
pub fn orbit_drag(cam: &Camera, dx_px: f64, dy_px: f64) -> Camera {
    let mut c = *cam;
    c.orbit(-ORBIT_RAD_PER_PX * dx_px, ORBIT_RAD_PER_PX * dy_px);
    c
}

/// Middle drag: the pivot moves in the view plane so content follows the cursor.
pub fn pan_drag(cam: &Camera, dx_px: f64, dy_px: f64, viewport_h: f64) -> Camera {
    let mut c = *cam;
    c.pan(dx_px, dy_px, viewport_h);
    c
}

/// Alt + secondary drag: dragging right moves closer.
pub fn dolly_drag(cam: &Camera, dx_px: f64) -> Camera {
    let mut c = *cam;
    c.dolly((-DOLLY_PER_PX * dx_px).exp());
    c
}

/// Distance factor of a wheel motion in points (positive zooms in).
pub fn wheel_factor(points: f64) -> f64 {
    WHEEL_BASE.powf(points)
}

/// Dollies by `factor` and moves the pivot so the point of the pivot plane under `cursor`
/// (pixels, y down, in a `size` viewport) stays under it.
pub fn zoom_at(cam: &Camera, cursor: [f64; 2], size: [f64; 2], factor: f64) -> Camera {
    if factor <= 0.0 || !factor.is_finite() || size[0] <= 0.0 || size[1] <= 0.0 {
        return *cam;
    }
    let wpp = cam.world_per_pixel(size[1]);
    let (ox, oy) = (
        (cursor[0] - size[0] / 2.0) * wpp,
        (size[1] / 2.0 - cursor[1]) * wpp,
    );
    let mut c = *cam;
    c.dolly(factor);
    c.target += (cam.right() * ox + cam.up() * oy) * (1.0 - factor);
    c
}

/// Centres a world box and fits its bounding sphere (with margin) into a viewport of the
/// given aspect ratio. Orientation and projection kind are kept.
pub fn frame_box(cam: &Camera, min: DVec3, max: DVec3, aspect: f64) -> Camera {
    let mut c = *cam;
    c.target = (min + max) / 2.0 * c.mirror();
    let radius = (max - min).length() / 2.0 * FRAME_MARGIN;
    if radius < DEGENERATE_RADIUS || !radius.is_finite() {
        return c;
    }
    let aspect = if aspect > 0.0 { aspect } else { 1.0 };
    match &mut c.projection {
        Projection::Perspective { fov_y } => {
            let half_y = *fov_y / 2.0;
            let half_x = (half_y.tan() * aspect).atan();
            c.distance = radius / half_y.min(half_x).sin();
        }
        Projection::Orthographic { height } => {
            *height = 2.0 * radius * (1.0f64).max(1.0 / aspect);
            c.distance = c.distance.max(2.0 * radius);
        }
    }
    c
}

/// Switches to a named view, keeping the pivot (docs/specs/VIEWPORT.md §7).
pub fn view_camera(cam: &Camera, view: StandardView) -> Camera {
    let mut c = *cam;
    if view == StandardView::Perspective {
        if let Projection::Orthographic { height } = c.projection {
            let fov_y = FRAC_PI_4;
            c.projection = Projection::Perspective { fov_y };
            c.distance = height / (2.0 * (fov_y / 2.0).tan());
            c.yaw = DEFAULT_YAW;
            c.pitch = DEFAULT_PITCH;
        }
        return c;
    }
    if let Projection::Perspective { fov_y } = c.projection {
        c.projection = Projection::Orthographic {
            height: 2.0 * c.distance * (fov_y / 2.0).tan(),
        };
    }
    let front = match c.handedness {
        Handedness::Left => AxisView::NegZ,
        Handedness::Right => AxisView::PosZ,
    };
    let back = match c.handedness {
        Handedness::Left => AxisView::PosZ,
        Handedness::Right => AxisView::NegZ,
    };
    c.look_along(match view {
        StandardView::Front => front,
        StandardView::Back => back,
        StandardView::Right => AxisView::PosX,
        StandardView::Left => AxisView::NegX,
        StandardView::Top => AxisView::PosY,
        StandardView::Bottom => AxisView::NegY,
        StandardView::Perspective => unreachable!("handled above"),
    });
    c
}
