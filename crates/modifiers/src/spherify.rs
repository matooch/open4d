//! The Spherify deformer (docs/specs/MODIFIERS.md §6).

use glam::DVec3;

use crate::common::SpherifyParams;

/// Spherifies `points` (in the deformer's local space): each point moves along its own
/// radius toward the sphere of `params.radius` centred on the origin, by `strength`
/// percent of the way there. Every point is treated — Spherify has no box and no mode.
pub fn spherify(points: &mut [DVec3], params: &SpherifyParams) {
    let s = params.strength / 100.0;
    if s == 0.0 {
        return;
    }
    for point in points.iter_mut() {
        let len = point.length();
        if len == 0.0 {
            continue; // The origin has no direction to project along; it stays there.
        }
        *point += (point.normalize() * params.radius - *point) * s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `spherify/defaults`: radius 200, strength 50.
    fn params() -> SpherifyParams {
        SpherifyParams::default()
    }

    #[test]
    fn zero_strength_changes_nothing() {
        let mut p = vec![DVec3::new(-100.0, -200.0, -100.0)];
        let before = p.clone();
        spherify(
            &mut p,
            &SpherifyParams {
                strength: 0.0,
                ..params()
            },
        );
        assert_eq!(p, before);
    }

    #[test]
    fn strength_blends_toward_the_projection() {
        // `spherify/defaults`: the corner (|p| = 244.949) ends halfway to its
        // projection (−81.65, −163.30, −81.65).
        let mut points = vec![DVec3::new(-100.0, -200.0, -100.0)];
        spherify(&mut points, &params());
        let d = points[0];
        assert!((d.x + 90.8248290463863).abs() < 1e-9, "{d:?}");
        assert!((d.y + 181.6496580927726).abs() < 1e-9, "{d:?}");
        assert!((d.z + 90.8248290463863).abs() < 1e-9, "{d:?}");
    }

    #[test]
    fn full_strength_puts_every_point_on_the_sphere() {
        // `spherify/strength-full`: each point is at exactly `radius` from the origin.
        let points = vec![
            DVec3::new(-100.0, -200.0, -100.0),
            DVec3::new(-100.0, 0.0, 0.0),
            DVec3::new(0.0, 200.0, 0.0),
        ];
        let mut moved = points.clone();
        spherify(
            &mut moved,
            &SpherifyParams {
                strength: 100.0,
                ..params()
            },
        );
        for p in &moved {
            assert!((p.length() - 200.0).abs() < 1e-9, "{p:?}");
        }
        // The direction is kept.
        assert!(
            (moved[0].x - -81.64965809277261).abs() < 1e-9,
            "{:?}",
            moved[0]
        );
        // A point already on the sphere stays put (`spherify/strength-full` keeps the
        // y corners at ±200).
        assert_eq!(moved[2], DVec3::new(0.0, 200.0, 0.0));
    }

    #[test]
    fn a_smaller_radius_pulls_closer_and_the_origin_stays() {
        // `spherify/radius` (radius 100 at 100 %).
        let mut points = vec![DVec3::new(-100.0, 0.0, 0.0)];
        spherify(
            &mut points,
            &SpherifyParams {
                radius: 100.0,
                strength: 100.0,
            },
        );
        assert!((points[0].length() - 100.0).abs() < 1e-9, "{:?}", points[0]);
        // The origin has no direction: it stays (never seen in a golden case).
        let mut origin = vec![DVec3::ZERO];
        spherify(&mut origin, &params());
        assert_eq!(origin[0], DVec3::ZERO);
    }
}
