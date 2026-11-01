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
    if suppressed(points, params) {
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
        let placed = if outside_xz(&params.mode, point, params.size) {
            Placement::Unchanged
        } else {
            placement(&params.mode, y, h)
        };
        let (qx, qy) = match placed {
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

/// `within_box` gates on x and z too (MODIFIERS.md §0): a point beyond the box half-size
/// in either axis keeps its place. `point` is in the deformer's space, before `angle`.
fn outside_xz(mode: &str, point: &DVec3, size: DVec3) -> bool {
    mode == "within_box" && (point.x.abs() > size.x / 2.0 || point.z.abs() > size.z / 2.0)
}

/// `within_box` does nothing at all when the box spans the object's x/z extent in
/// neither axis (MODIFIERS.md §0, `bend/mode-box-narrow`).
fn suppressed(points: &[DVec3], params: &BendParams) -> bool {
    if params.mode != "within_box" || points.is_empty() {
        return false;
    }
    let extent = |axis: fn(&DVec3) -> f64| {
        let (lo, hi) = points
            .iter()
            .map(axis)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
        hi - lo
    };
    params.size.x < extent(|p| p.x) && params.size.z < extent(|p| p.z)
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

    fn within_box(size: DVec3) -> BendParams {
        BendParams {
            size,
            mode: "within_box".to_string(),
            ..params()
        }
    }

    /// A 200 x 400 x 200 object as its corner and centre points.
    fn object() -> Vec<DVec3> {
        vec![
            DVec3::new(-100.0, 0.0, -100.0),
            DVec3::new(100.0, 0.0, 100.0),
            DVec3::new(0.0, 0.0, 0.0),
        ]
    }

    #[test]
    fn within_box_gates_each_axis_inclusively() {
        // Spans z (250 >= 200), narrow in x: the x = +-100 points keep their place.
        let mut pts = object();
        bend(&mut pts, &within_box(DVec3::new(100.0, 250.0, 250.0)));
        assert_eq!(pts[0], DVec3::new(-100.0, 0.0, -100.0));
        assert_ne!(pts[2], DVec3::ZERO);
        // z only; |z| = size_z/2 is outside nothing: boundary is inclusive.
        let mut pts = vec![DVec3::new(0.0, 0.0, 50.0), DVec3::new(0.0, 0.0, 50.1)];
        bend(&mut pts, &within_box(DVec3::new(250.0, 250.0, 100.0)));
        assert_ne!(pts[0], DVec3::new(0.0, 0.0, 50.0));
        assert_eq!(pts[1], DVec3::new(0.0, 0.0, 50.1));
    }

    #[test]
    fn within_box_is_suppressed_when_the_box_spans_neither_axis() {
        let mut pts = object();
        bend(&mut pts, &within_box(DVec3::new(100.0, 250.0, 100.0)));
        assert_eq!(pts, object());
        // (120, 250, 190) is also narrower than the 200 x 200 object in both axes.
        bend(&mut pts, &within_box(DVec3::new(120.0, 250.0, 190.0)));
        assert_eq!(pts, object());
        // Spanning one axis (size == extent) deforms.
        bend(&mut pts, &within_box(DVec3::new(100.0, 250.0, 200.0)));
        assert_ne!(pts[2], DVec3::ZERO);
    }

    #[test]
    fn the_suppression_follows_the_object_extent_and_within_box_only() {
        // A 100 x 100 object with box (120, 250, 150) spans it: it bends.
        let mut pts = vec![DVec3::new(-50.0, 0.0, -50.0), DVec3::new(50.0, 0.0, 50.0)];
        bend(&mut pts, &within_box(DVec3::new(120.0, 250.0, 150.0)));
        assert_ne!(pts[1], DVec3::new(50.0, 0.0, 50.0));
        // `limited` ignores the x/z size entirely.
        let mut pts = object();
        let limited = BendParams {
            size: DVec3::new(100.0, 250.0, 100.0),
            ..params()
        };
        bend(&mut pts, &limited);
        assert_ne!(pts[2], DVec3::ZERO);
    }
}
