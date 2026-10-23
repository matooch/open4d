//! The Bend deformer (docs/specs/MODIFIERS.md §1).

use glam::DVec3;

use crate::common::{BendParams, Placement, placement};

/// Bends `points` (in the deformer's local space) in the XY plane, turning the whole
/// bend direction by `params.angle` about Y.
///
/// The neutral line (x = 0) follows the circle with centre `(R, -h/2)` and radius
/// `R = h/α + (keep_y_axis_length ? 0 : size_x/2)`; points keep their distance `x`
/// along the circle's normal, and the part above the box in `limited` continues along
/// the tangent at the top of the box.
pub fn bend(points: &mut [DVec3], params: &BendParams) {
    let h = params.size.y;
    let alpha = params.strength;
    if h <= 0.0 || alpha == 0.0 {
        return;
    }
    let half = h / 2.0;
    // Radius of the neutral line (MODIFIERS.md §1): with Keep Y-Axis Length the arc
    // length of the centre line is the box height, so R = h/α.
    let radius = h / alpha
        + if params.keep_y_axis_length {
            0.0
        } else {
            params.size.x / 2.0
        };
    // The Angle param turns the bend direction: p' = R(β) · bend(R(-β) · p).
    let (cb, sb) = (params.angle.cos(), params.angle.sin());

    for point in points.iter_mut() {
        // Into the bend frame, R(-β) on (x, z).
        let (x, y, z) = (
            point.x * cb + point.z * sb,
            point.y,
            -point.x * sb + point.z * cb,
        );
        let (qx, qy) = match placement(&params.mode, y, h) {
            Placement::Unchanged => (x, y),
            Placement::Deform(t) => arc(radius, half, alpha * t, x),
            Placement::Carry => {
                let (ax, ay) = arc(radius, half, alpha, x);
                // Translate along the tangent at the end of the arc (φ = α in
                // `limited`, the only mode that gives `Carry`).
                let (s, c) = (alpha.sin(), alpha.cos());
                (ax + (y - half) * s, ay + (y - half) * c)
            }
        };
        // Back out of the bend frame, R(β) on (x, z).
        *point = DVec3::new(qx * cb - z * sb, qy, qx * sb + z * cb);
    }
}

/// The bent position of a point at signed distance `x` from the neutral line, on the
/// arc at angle `φ`: `A(φ) + x·N(φ)`.
fn arc(radius: f64, half: f64, phi: f64, x: f64) -> (f64, f64) {
    let (c, s) = (phi.cos(), phi.sin());
    (radius - radius * c + x * c, -half + radius * s - x * s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> BendParams {
        BendParams {
            strength: std::f64::consts::FRAC_PI_2,
            ..BendParams::default()
        }
    }

    #[test]
    fn zero_strength_changes_nothing() {
        let mut p = vec![DVec3::new(-100.0, 200.0, -100.0), DVec3::ZERO];
        let before = p.clone();
        bend(&mut p, &BendParams::default());
        assert_eq!(p, before);
    }

    #[test]
    fn the_bottom_of_the_box_stays_put_and_the_top_follows_the_arc() {
        let p = params();
        let mut points = vec![DVec3::new(-100.0, -125.0, 0.0), DVec3::new(0.0, 125.0, 0.0)];
        bend(&mut points, &p);
        // The anchor row of the box is unchanged.
        assert_eq!(points[0], DVec3::new(-100.0, -125.0, 0.0));
        // The neutral line at the top of the box is at (R, y0 + R), R = h/α + sx/2.
        let r = 250.0 / (std::f64::consts::FRAC_PI_2) + 125.0;
        assert!((points[1].x - r).abs() < 1e-9, "{}", points[1].x);
        assert!((points[1].y - (-125.0 + r)).abs() < 1e-9, "{}", points[1].y);
    }

    #[test]
    fn keep_y_axis_length_drops_the_box_half_width_from_the_radius() {
        let mut on = params();
        on.keep_y_axis_length = true;
        let mut points = vec![DVec3::new(0.0, 125.0, 0.0)];
        bend(&mut points, &on);
        let r = 250.0 / (std::f64::consts::FRAC_PI_2);
        assert!((points[0].x - r).abs() < 1e-9, "{}", points[0].x);
        assert!((points[0].y - (-125.0 + r)).abs() < 1e-9, "{}", points[0].y);
    }

    #[test]
    fn limited_carries_the_part_above_the_box_along_the_tangent() {
        let p = params();
        let mut points = vec![DVec3::new(0.0, 200.0, 0.0)];
        bend(&mut points, &p);
        let r = 250.0 / (std::f64::consts::FRAC_PI_2) + 125.0;
        // The tangent at the end of a 90° arc points along +x.
        assert!((points[0].x - (r + 75.0)).abs() < 1e-9, "{}", points[0].x);
        assert!((points[0].y - (-125.0 + r)).abs() < 1e-9, "{}", points[0].y);
    }

    #[test]
    fn a_zero_sized_box_does_not_produce_nan() {
        let p = BendParams {
            size: DVec3::new(250.0, 0.0, 250.0),
            strength: 1.0,
            ..BendParams::default()
        };
        let mut points = vec![DVec3::new(1.0, 2.0, 3.0)];
        bend(&mut points, &p);
        assert_eq!(points[0], DVec3::new(1.0, 2.0, 3.0));
    }
}
