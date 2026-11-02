//! The Squash & Stretch deformer (docs/specs/MODIFIERS.md §7.1).

use glam::DVec3;

use crate::common::SquashParams;

/// Slack added to the observed ends: the reference squashes between `top + ε` and
/// `bottom − ε` (`squash-stretch/factor`: 100.005 explains 50.0025 and 175.0025).
pub const EPSILON: f64 = 0.005;

/// The profile `g(u)` over the span, 1 in the middle and 0 at the ends; `None` for the
/// types that are not derived (`custom`, `spline`).
fn profile(kind: &str, curvature: f64, u: f64) -> Option<f64> {
    let n = match kind {
        "square" => 2,
        "cubic" => 3,
        "quartic" => 4,
        _ => return None,
    };
    let a = u.abs();
    Some(curvature * (1.0 - a.powi(n)) + (1.0 - curvature) * (1.0 - a))
}

/// Squashes or stretches `points` (in the deformer's local space) along y: the span
/// between the ends scales by `factor`, x and z widen to keep the volume (scaled by
/// `expand`), the parts beyond the ends travel with them. The unsupported curve types
/// leave the points alone.
pub fn squash_stretch(points: &mut [DVec3], p: &SquashParams) {
    let f = (p.factor / 100.0).max(1e-6);
    let e = p.expand / 100.0;
    let c = p.curvature / 100.0;
    let (ht, hb) = (p.top + EPSILON, p.bottom - EPSILON);
    let center = p.center / 100.0;
    let y0 = center * if center >= 0.0 { ht } else { -hb };
    let (lt, lb) = (ht - y0, y0 - hb);
    if lt <= 0.0 || lb <= 0.0 {
        return;
    }
    for pt in points.iter_mut() {
        let y = pt.y;
        if y > ht {
            pt.y = y + lt * (f - 1.0);
        } else if y < hb {
            pt.y = y - lb * (f - 1.0);
        } else {
            let u = if y >= y0 {
                (y - y0) / lt
            } else {
                (y - y0) / lb
            };
            let Some(g) = profile(&p.curvature_type, c, u) else {
                return;
            };
            let scale = 1.0 + e * (1.0 / f - 1.0) * g;
            pt.x *= scale;
            pt.z *= scale;
            pt.y = y0 + (y - y0) * f;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn half() -> SquashParams {
        SquashParams {
            factor: 50.0,
            ..SquashParams::default()
        }
    }

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-4, "{a} != {b}");
    }

    #[test]
    fn factor_100_changes_nothing() {
        let mut p = vec![DVec3::new(-100.0, 37.0, 80.0), DVec3::new(1.0, 300.0, 2.0)];
        let before = p.clone();
        squash_stretch(&mut p, &SquashParams::default());
        assert_eq!(p, before);
    }

    #[test]
    fn rows_match_the_factor_case() {
        let mut p = vec![
            DVec3::new(-100.0, -200.0, 0.0),
            DVec3::new(-100.0, -100.0, 0.0),
            DVec3::new(-100.0, -50.0, 0.0),
            DVec3::new(-100.0, 0.0, 0.0),
        ];
        squash_stretch(&mut p, &half());
        close(p[0].y, -149.9975);
        close(p[1].y, -50.0);
        close(p[1].x, -100.01);
        close(p[2].x, -175.0025);
        close(p[3].x, -200.0);
    }

    #[test]
    fn unsupported_types_leave_points_alone() {
        let mut p = vec![DVec3::new(-100.0, 20.0, 0.0)];
        let before = p.clone();
        squash_stretch(
            &mut p,
            &SquashParams {
                curvature_type: "spline".into(),
                ..half()
            },
        );
        assert_eq!(p, before);
    }
}
