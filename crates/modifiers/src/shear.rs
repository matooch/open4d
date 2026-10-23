//! The Shear deformer (docs/specs/MODIFIERS.md §5).

use glam::DVec3;

use crate::common::{ShearParams, placement, taper_curve};

/// Shears `points` (in the deformer's local space): every point is offset along
/// `angle` (turned about Y, so +X at 0) by `s·(size_y/2)·f(t)` — the same curve as
/// Taper's — and y never moves.
pub fn shear(points: &mut [DVec3], params: &ShearParams) {
    let h = params.size.y;
    if h <= 0.0 {
        return;
    }
    let magnitude = params.strength / 100.0 * (h / 2.0);
    // The offset direction: R(angle) applied to +X (MODIFIERS.md §0).
    let (dx, dz) = (params.angle.cos(), params.angle.sin());
    let c = params.curvature / 100.0;
    for point in points.iter_mut() {
        let Some(t) = placement(&params.mode, point.y, h).t() else {
            continue;
        };
        let offset = magnitude * taper_curve(t, c, params.fillet);
        point.x += offset * dx;
        point.z += offset * dz;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `shear/strength`: strength 50, angle 0, curvature 100, limited.
    fn params() -> ShearParams {
        ShearParams {
            strength: 50.0,
            ..ShearParams::default()
        }
    }

    #[test]
    fn zero_strength_changes_nothing() {
        let mut p = vec![DVec3::new(-100.0, 0.0, 200.0)];
        let before = p.clone();
        shear(&mut p, &ShearParams::default());
        assert_eq!(p, before);
    }

    #[test]
    fn the_offset_follows_the_taper_curve_and_never_moves_y() {
        // The middle row (t = 1/2): 62.5·0.25 = 15.625 along +X.
        let mut points = vec![DVec3::new(-100.0, 0.0, -100.0)];
        shear(&mut points, &params());
        assert!((points[0].x + 84.375).abs() < 1e-9, "{:?}", points[0]);
        assert_eq!(points[0].y, 0.0);
        assert_eq!(points[0].z, -100.0);
        // Above the box `limited` holds the offset at t = 1: 62.5.
        let mut top = vec![DVec3::new(-100.0, 200.0, 0.0)];
        shear(&mut top, &params());
        assert!((top[0].x + 37.5).abs() < 1e-9, "{:?}", top[0]);
    }

    #[test]
    fn angle_turns_the_offset_toward_z() {
        // `shear/angle` (45°): both components 15.625·cos 45°.
        let p = ShearParams {
            strength: 50.0,
            angle: std::f64::consts::FRAC_PI_4,
            ..ShearParams::default()
        };
        let mut points = vec![DVec3::new(-100.0, 0.0, 0.0)];
        shear(&mut points, &p);
        let shift = 15.625 * std::f64::consts::FRAC_1_SQRT_2;
        assert!(
            (points[0].x - (-100.0 + shift)).abs() < 1e-9,
            "{:?}",
            points[0]
        );
        assert!((points[0].z - shift).abs() < 1e-9, "{:?}", points[0]);
    }

    #[test]
    fn curvature_and_fillet_pick_the_profile_like_taper() {
        // `shear/curvature` (c = 1/2, offset 62.5·0.055) and `shear/fillet` (62.5·0.028)
        // on the y = −100 row (t = 1/10).
        let blend = ShearParams {
            strength: 50.0,
            curvature: 50.0,
            ..ShearParams::default()
        };
        let fillet = ShearParams {
            strength: 50.0,
            fillet: true,
            ..ShearParams::default()
        };
        let mut points = vec![DVec3::new(-100.0, -100.0, 0.0)];
        shear(&mut points, &blend);
        assert!((points[0].x + 96.5625).abs() < 1e-9, "{:?}", points[0]);
        let mut points = vec![DVec3::new(-100.0, -100.0, 0.0)];
        shear(&mut points, &fillet);
        assert!((points[0].x + 98.25).abs() < 1e-9, "{:?}", points[0]);
    }

    #[test]
    fn unlimited_extrapolates_the_parabola() {
        // `shear/mode-unlimited` (y = 200, t = 1.3): 62.5·1.69 = 105.625.
        let p = ShearParams {
            strength: 50.0,
            mode: "unlimited".to_string(),
            ..ShearParams::default()
        };
        let mut points = vec![DVec3::new(-100.0, 200.0, 0.0)];
        shear(&mut points, &p);
        assert!((points[0].x - 5.625).abs() < 1e-9, "{:?}", points[0]);
    }

    #[test]
    fn a_zero_sized_box_does_not_produce_nan() {
        let p = ShearParams {
            size: DVec3::new(250.0, 0.0, 250.0),
            strength: 50.0,
            ..ShearParams::default()
        };
        let mut points = vec![DVec3::new(1.0, 2.0, 3.0)];
        shear(&mut points, &p);
        assert_eq!(points[0], DVec3::new(1.0, 2.0, 3.0));
    }
}
