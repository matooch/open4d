//! The Twist deformer (docs/specs/MODIFIERS.md §2).

use glam::DVec3;

use crate::common::{Placement, TwistParams, placement};

/// Twists `points` (in the deformer's local space) about the deformer's Y axis: the
/// turn is linear in y, 0° at the bottom of the box and `params.angle` at the top.
pub fn twist(points: &mut [DVec3], params: &TwistParams) {
    let h = params.size.y;
    let alpha = params.angle;
    if h <= 0.0 || alpha == 0.0 {
        return;
    }
    for point in points.iter_mut() {
        let phi = match placement(&params.mode, point.y, h) {
            Placement::Unchanged => continue,
            Placement::Deform(t) => alpha * t,
            // `limited` above the box: the full turn, no displacement (y is kept).
            Placement::Carry => alpha,
        };
        let (c, s) = (phi.cos(), phi.sin());
        let (x, z) = (point.x, point.z);
        *point = DVec3::new(x * c - z * s, point.y, x * s + z * c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> TwistParams {
        TwistParams {
            angle: std::f64::consts::FRAC_PI_2,
            ..TwistParams::default()
        }
    }

    #[test]
    fn zero_angle_changes_nothing() {
        let mut p = vec![DVec3::new(-100.0, 0.0, -100.0)];
        let before = p.clone();
        twist(&mut p, &TwistParams::default());
        assert_eq!(p, before);
    }

    #[test]
    fn the_turn_is_linear_in_y_and_never_moves_y() {
        // `twist/angle`: at y = 0 (t = 1/2) the corner (-100, 0, -100) ends at
        // (0, 0, -141.42), a 45° turn.
        let mut points = vec![DVec3::new(-100.0, 0.0, -100.0)];
        twist(&mut points, &params());
        let d = points[0];
        assert!(d.x.abs() < 1e-9, "{d:?}");
        assert_eq!(d.y, 0.0);
        assert!(
            (d.z + 100.0 * std::f64::consts::SQRT_2).abs() < 1e-9,
            "{d:?}"
        );
    }

    #[test]
    fn limited_keeps_the_points_below_the_box_at_rest_and_turns_those_above() {
        let mut points = vec![
            DVec3::new(-100.0, -200.0, -100.0),
            DVec3::new(-100.0, 200.0, -100.0),
        ];
        twist(&mut points, &params());
        assert_eq!(points[0], DVec3::new(-100.0, -200.0, -100.0));
        // Above the box the full 90°: (-100, -100) -> (100, -100).
        assert!((points[1].x - 100.0).abs() < 1e-9, "{:?}", points[1]);
        assert!((points[1].z + 100.0).abs() < 1e-9, "{:?}", points[1]);
    }

    #[test]
    fn a_zero_sized_box_does_not_produce_nan() {
        let p = TwistParams {
            size: DVec3::new(250.0, 0.0, 250.0),
            angle: 1.0,
            ..TwistParams::default()
        };
        let mut points = vec![DVec3::new(1.0, 2.0, 3.0)];
        twist(&mut points, &p);
        assert_eq!(points[0], DVec3::new(1.0, 2.0, 3.0));
    }
}
