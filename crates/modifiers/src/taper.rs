//! The Taper deformer (docs/specs/MODIFIERS.md §3).

use glam::DVec3;

use crate::common::{TaperParams, placement, taper_curve};

/// Tapers `points` (in the deformer's local space): `x` and `z` are scaled toward the
/// y axis by `1 − s·f(t)` — anchored at the bottom of the box, so a positive `strength`
/// narrows the top — and y never moves.
pub fn taper(points: &mut [DVec3], params: &TaperParams) {
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
        let scale = 1.0 - s * taper_curve(t, c, params.fillet);
        point.x *= scale;
        point.z *= scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `taper/strength`: strength 50, curvature 100, limited.
    fn params() -> TaperParams {
        TaperParams {
            strength: 50.0,
            ..TaperParams::default()
        }
    }

    #[test]
    fn zero_strength_changes_nothing() {
        let mut p = vec![DVec3::new(-100.0, 0.0, 200.0)];
        let before = p.clone();
        taper(&mut p, &TaperParams::default());
        assert_eq!(p, before);
    }

    #[test]
    fn the_scale_is_the_quadratic_curve_anchored_at_the_box_bottom() {
        // The middle row (y = 0, t = 1/2): scale 1 − 0.5·0.25 = 0.875 on x and z.
        let mut points = vec![DVec3::new(-100.0, 0.0, -100.0)];
        taper(&mut points, &params());
        assert!((points[0].x + 87.5).abs() < 1e-9, "{:?}", points[0]);
        assert!((points[0].z + 87.5).abs() < 1e-9, "{:?}", points[0]);
        assert_eq!(points[0].y, 0.0); // y never moves
        // The bottom row is the anchor: f(0) = 0.
        let mut bottom = vec![DVec3::new(-100.0, -125.0, 100.0)];
        taper(&mut bottom, &params());
        assert_eq!(bottom[0], DVec3::new(-100.0, -125.0, 100.0));
    }

    #[test]
    fn limited_holds_the_top_scale_above_the_box_and_ignores_below() {
        let mut points = vec![
            DVec3::new(-100.0, -200.0, 0.0),
            DVec3::new(-100.0, 200.0, 0.0),
        ];
        taper(&mut points, &params());
        assert_eq!(points[0].x, -100.0);
        // Above the box: the scale at t = 1, 1 − 0.5 = 0.5.
        assert!((points[1].x + 50.0).abs() < 1e-9, "{:?}", points[1]);
    }

    #[test]
    fn curvature_blends_the_linear_and_quadratic_profiles() {
        // `taper/curvature` (c = 1/2): f(1/10) = 0.5·0.01 + 0.5·0.1 = 0.055.
        let p = TaperParams {
            strength: 50.0,
            curvature: 50.0,
            ..TaperParams::default()
        };
        let mut points = vec![DVec3::new(100.0, -100.0, 0.0)];
        taper(&mut points, &p);
        assert!((points[0].x - 97.25).abs() < 1e-9, "{:?}", points[0]);
    }

    #[test]
    fn fillet_replaces_the_blend_with_smoothstep() {
        // `taper/fillet`: f(1/10) = 3t² − 2t³ = 0.028.
        let p = TaperParams {
            strength: 50.0,
            fillet: true,
            ..TaperParams::default()
        };
        let mut points = vec![DVec3::new(100.0, -100.0, 0.0)];
        taper(&mut points, &p);
        assert!((points[0].x - 98.6).abs() < 1e-9, "{:?}", points[0]);
    }

    #[test]
    fn a_zero_sized_box_does_not_produce_nan() {
        let p = TaperParams {
            size: DVec3::new(250.0, 0.0, 250.0),
            strength: 50.0,
            ..TaperParams::default()
        };
        let mut points = vec![DVec3::new(1.0, 2.0, 3.0)];
        taper(&mut points, &p);
        assert_eq!(points[0], DVec3::new(1.0, 2.0, 3.0));
    }
}
