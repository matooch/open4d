//! The Wrap deformer (docs/specs/MODIFIERS.md §7.2).

use glam::DVec3;

use crate::common::WrapParams;

/// Rolls the plane of `points` (in the deformer's local space) onto a cylinder or a
/// sphere of `radius` centred at `z = radius`; x runs along the longitude span, y along
/// the latitude span (spherical only).
pub fn wrap(points: &mut [DVec3], p: &WrapParams) {
    let spherical = p.wrap == "spherical";
    if p.width <= 0.0 || (spherical && p.height <= 0.0) {
        return;
    }
    let lon_span = p.longitude_end - p.longitude_start;
    let lon_mid = 0.5 * (p.longitude_start + p.longitude_end);
    let lat_span = p.latitude_end - p.latitude_start;
    let lat_mid = 0.5 * (p.latitude_start + p.latitude_end);
    let sz = p.scale_z / 100.0;
    let tension = p.tension / 100.0;
    let quarter_turn = 1.5 * std::f64::consts::PI; // 270 degrees faces -z
    for pt in points.iter_mut() {
        let phi = lon_mid + (pt.x + p.movement) / p.width * lon_span - quarter_turn;
        let d = p.radius - pt.z * sz;
        let wrapped = if spherical {
            let theta = lat_mid + pt.y / p.height * lat_span;
            DVec3::new(
                d * theta.cos() * phi.sin(),
                d * theta.sin(),
                p.radius - d * theta.cos() * phi.cos(),
            )
        } else {
            DVec3::new(d * phi.sin(), pt.y, p.radius - d * phi.cos())
        };
        *pt += (wrapped - *pt) * tension;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: DVec3, b: DVec3) {
        assert!((a - b).length() < 1e-4, "{a:?} != {b:?}");
    }

    #[test]
    fn cylindrical_default_rolls_a_half_turn() {
        let mut pts = vec![
            DVec3::new(-100.0, 7.0, -100.0),
            DVec3::new(0.0, 0.0, -100.0),
        ];
        wrap(&mut pts, &WrapParams::default());
        close(pts[0], DVec3::new(-212.132034, 7.0, -12.132034));
        close(pts[1], DVec3::new(0.0, 0.0, -100.0));
    }

    #[test]
    fn spherical_lifts_y_by_latitude() {
        let mut pts = vec![DVec3::new(-100.0, 200.0, -100.0)];
        wrap(
            &mut pts,
            &WrapParams {
                wrap: "spherical".into(),
                ..WrapParams::default()
            },
        );
        close(pts[0], DVec3::new(-150.0, 212.132034, 50.0));
    }

    #[test]
    fn zero_tension_and_zero_width_leave_the_points() {
        let mut pts = vec![DVec3::new(-100.0, 5.0, -100.0)];
        wrap(
            &mut pts,
            &WrapParams {
                tension: 0.0,
                ..WrapParams::default()
            },
        );
        assert_eq!(pts[0], DVec3::new(-100.0, 5.0, -100.0));
        wrap(
            &mut pts,
            &WrapParams {
                width: 0.0,
                ..WrapParams::default()
            },
        );
        assert_eq!(pts[0], DVec3::new(-100.0, 5.0, -100.0));
    }
}
