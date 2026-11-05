//! The Surface deformer, projection type (docs/specs/MODIFIERS.md §14.2).

use glam::DVec3;

use crate::helper::closest_on_triangle;

/// `(e1, e2, n)` of a triangle; `None` when it is degenerate.
fn frame(a: DVec3, b: DVec3, c: DVec3) -> Option<[DVec3; 3]> {
    let n = (b - a).cross(c - a);
    if n.length() < 1e-12 {
        return None;
    }
    let n = n.normalize();
    let e1 = (b - a).normalize();
    Some([e1, n.cross(e1), n])
}

fn area(a: DVec3, b: DVec3, c: DVec3) -> f64 {
    0.5 * (b - a).cross(c - a).length()
}

/// Binds every point to the nearest triangle of the `bind` surface and follows the
/// `current` one. `tris` index both point lists.
pub fn surface_deform(
    points: &mut [DVec3],
    bind: &[DVec3],
    current: &[DVec3],
    tris: &[[u32; 3]],
    strength: f64,
    proportional: bool,
) {
    if bind.len() != current.len() || tris.is_empty() {
        return;
    }
    for p in points.iter_mut() {
        let mut best: Option<(f64, usize, DVec3, [f64; 3])> = None;
        for (ti, t) in tris.iter().enumerate() {
            let [a, b, c] = t.map(|i| bind[i as usize]);
            let (q, bary) = closest_on_triangle(*p, a, b, c);
            let d = (*p - q).length_squared();
            if best.is_none_or(|(bd, ..)| d < bd) {
                best = Some((d, ti, q, bary));
            }
        }
        let Some((_, ti, q, bary)) = best else {
            continue;
        };
        let t = tris[ti].map(|i| i as usize);
        let [ba, bb, bc] = t.map(|i| bind[i]);
        let [ca, cb, cc] = t.map(|i| current[i]);
        let on_now = ca * bary[0] + cb * bary[1] + cc * bary[2];
        let off = *p - q;
        let moved = match (frame(ba, bb, bc), frame(ca, cb, cc)) {
            (Some(fb), Some(fc)) => {
                let mut o =
                    fc[0] * off.dot(fb[0]) + fc[1] * off.dot(fb[1]) + fc[2] * off.dot(fb[2]);
                if proportional {
                    o *= (area(ca, cb, cc) / area(ba, bb, bc)).sqrt();
                }
                on_now + o
            }
            _ => on_now + off,
        };
        *p += (moved - *p) * strength;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad() -> (Vec<DVec3>, Vec<[u32; 3]>) {
        (
            vec![
                DVec3::new(-100.0, 0.0, -100.0),
                DVec3::new(100.0, 0.0, -100.0),
                DVec3::new(100.0, 0.0, 100.0),
                DVec3::new(-100.0, 0.0, 100.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    #[test]
    fn it_follows_a_translated_and_a_scaled_surface() {
        let (b, t) = quad();
        let p0 = vec![DVec3::new(10.0, 30.0, 20.0), DVec3::new(-50.0, -20.0, 70.0)];
        let up: Vec<DVec3> = b.iter().map(|q| *q + DVec3::new(0.0, 40.0, 0.0)).collect();
        let mut a = p0.clone();
        surface_deform(&mut a, &b, &up, &t, 1.0, false);
        for (x, y) in a.iter().zip(&p0) {
            assert!((*x - (*y + DVec3::new(0.0, 40.0, 0.0))).length() < 1e-9);
        }
        let mut s = p0.clone();
        let big: Vec<DVec3> = b.iter().map(|q| *q * 2.0).collect();
        surface_deform(&mut s, &b, &big, &t, 1.0, true);
        // proportional: the offset above the plane doubles with the linear scale
        assert!((s[0].y - 60.0).abs() < 1e-9, "{:?}", s[0]);
        let mut u = p0.clone();
        surface_deform(&mut u, &b, &b, &t, 1.0, false);
        for (x, y) in u.iter().zip(&p0) {
            assert!((*x - *y).length() < 1e-9);
        }
    }
}
