//! The Bulge deformer (docs/specs/MODIFIERS.md §4).

use glam::DVec3;

use crate::common::{BulgeParams, bulge_curve, placement};

/// Bulges `points` (in the deformer's local space): `x` and `z` are scaled by
/// `1 + s·g(u)`, with `u` centred on the box (−1 at the bottom, +1 at the top), so the
/// bulge is strongest in the middle and vanishes at the ends; y never moves.
pub fn bulge(points: &mut [DVec3], params: &BulgeParams) {
    let h = params.size.y;
    if h <= 0.0 {
        return;
    }
    let s = params.strength / 100.0;
    let c = params.curvature / 100.0;
    for point in points.iter_mut() {
        let Some(t) = placement(&params.mode, point.y, h).t() else {
            continue;
        };
        // t = 0 at the bottom of the box and 1 at the top ⇔ u = −1 … +1.
        let scale = 1.0 + s * bulge_curve(2.0 * t - 1.0, c, params.fillet);
        point.x *= scale;
        point.z *= scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `bulge/strength`: strength 50, curvature 100, limited.
    fn params() -> BulgeParams {
        BulgeParams {
            strength: 50.0,
            ..BulgeParams::default()
        }
    }

    #[test]
    fn zero_strength_changes_nothing() {
        let mut p = vec![DVec3::new(-100.0, 0.0, 200.0)];
        let before = p.clone();
        bulge(&mut p, &BulgeParams::default());
        assert_eq!(p, before);
    }

    #[test]
    fn the_bulge_is_strongest_in_the_middle_and_zero_at_the_box_ends() {
        // u = −0.8 (y = −100): scale 1 + 0.5·0.36 = 1.18; u = 0: 1.5.
        let mut points = vec![
            DVec3::new(-100.0, -100.0, 0.0),
            DVec3::new(-100.0, 0.0, 0.0),
            DVec3::new(-100.0, -125.0, 0.0),
        ];
        bulge(&mut points, &params());
        assert!((points[0].x + 118.0).abs() < 1e-9, "{:?}", points[0]);
        assert!((points[1].x + 150.0).abs() < 1e-9, "{:?}", points[1]);
        // g(−1) = 0: the bottom row of the box is untouched.
        assert_eq!(points[2].x, -100.0);
    }

    #[test]
    fn negative_strength_pinches_and_unlimited_extrapolates() {
        // `bulge/negative` (u = 0): scale 0.5.
        let neg = BulgeParams {
            strength: -50.0,
            ..BulgeParams::default()
        };
        let mut points = vec![DVec3::new(-100.0, 0.0, 0.0)];
        bulge(&mut points, &neg);
        assert!((points[0].x + 50.0).abs() < 1e-9, "{:?}", points[0]);
        // `bulge/mode-unlimited` (y = −200, u = −1.6): 1 + 0.5·(−1.56) = 0.22.
        let unlim = BulgeParams {
            strength: 50.0,
            mode: "unlimited".to_string(),
            ..BulgeParams::default()
        };
        let mut points = vec![DVec3::new(-100.0, -200.0, 0.0)];
        bulge(&mut points, &unlim);
        assert!((points[0].x + 22.0).abs() < 1e-9, "{:?}", points[0]);
    }

    #[test]
    fn curvature_and_fillet_pick_the_profile() {
        // `bulge/curvature` (c = 1/2) and `bulge/fillet` at u = −0.8.
        let blend = BulgeParams {
            strength: 50.0,
            curvature: 50.0,
            ..BulgeParams::default()
        };
        let fillet = BulgeParams {
            strength: 50.0,
            fillet: true,
            ..BulgeParams::default()
        };
        let mut points = vec![DVec3::new(-100.0, -100.0, 0.0)];
        bulge(&mut points, &blend);
        assert!((points[0].x + 114.0).abs() < 1e-9, "{:?}", points[0]);
        let mut points = vec![DVec3::new(-100.0, -100.0, 0.0)];
        bulge(&mut points, &fillet);
        assert!((points[0].x + 105.2).abs() < 1e-9, "{:?}", points[0]);
    }

    #[test]
    fn a_zero_sized_box_does_not_produce_nan() {
        let p = BulgeParams {
            size: DVec3::new(250.0, 0.0, 250.0),
            strength: 50.0,
            ..BulgeParams::default()
        };
        let mut points = vec![DVec3::new(1.0, 2.0, 3.0)];
        bulge(&mut points, &p);
        assert_eq!(points[0], DVec3::new(1.0, 2.0, 3.0));
    }
}
