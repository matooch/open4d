//! The Collision deformer (docs/specs/MODIFIERS.md §14.4): points are pushed out of
//! closed collider volumes.

use glam::DVec3;

use crate::helper::closest_on_triangle;

/// The ray used by the inside test: skewed so it does not run along mesh edges.
const RAY: DVec3 = DVec3::new(1.0, 0.0123, 0.0071);

fn ray_hits(o: DVec3, a: DVec3, b: DVec3, c: DVec3) -> bool {
    // Möller–Trumbore
    let (e1, e2) = (b - a, c - a);
    let h = RAY.cross(e2);
    let det = e1.dot(h);
    if det.abs() < 1e-14 {
        return false;
    }
    let f = 1.0 / det;
    let s = o - a;
    let u = f * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = s.cross(e1);
    let v = f * RAY.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    f * e2.dot(q) > 0.0
}

/// Pushes `points` (world) out of the collider (`pts`/`tris`, world) to `offset` from its
/// surface, with `strength` in 0…1 blending.
pub fn collide(points: &mut [DVec3], pts: &[DVec3], tris: &[[u32; 3]], offset: f64, strength: f64) {
    // Orientation: a negative signed volume means the triangles face inwards.
    let volume: f64 = tris
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| pts[i as usize]);
            a.dot(b.cross(c))
        })
        .sum();
    let flip = if volume < 0.0 { -1.0 } else { 1.0 };
    for p in points.iter_mut() {
        let mut best: Option<(f64, usize, DVec3)> = None;
        let mut crossings = 0;
        for (ti, t) in tris.iter().enumerate() {
            let [a, b, c] = t.map(|i| pts[i as usize]);
            if ray_hits(*p, a, b, c) {
                crossings += 1;
            }
            let (q, _) = closest_on_triangle(*p, a, b, c);
            let d = (*p - q).length_squared();
            if best.is_none_or(|(bd, ..)| d < bd) {
                best = Some((d, ti, q));
            }
        }
        let Some((d2, ti, q)) = best else { continue };
        let inside = crossings % 2 == 1;
        if !inside && d2.sqrt() >= offset {
            continue;
        }
        let [a, b, c] = tris[ti].map(|i| pts[i as usize]);
        let tn = (b - a).cross(c - a) * flip;
        let normal = if !inside && d2.sqrt() > 1e-9 {
            (*p - q) / d2.sqrt()
        } else if tn.length() > 1e-12 {
            tn.normalize()
        } else {
            continue;
        };
        let target = q + normal * offset;
        *p += (target - *p) * strength;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A closed octahedron of radius 100 at the origin.
    fn octa() -> (Vec<DVec3>, Vec<[u32; 3]>) {
        let p = vec![
            DVec3::X * 100.0,
            -DVec3::X * 100.0,
            DVec3::Y * 100.0,
            -DVec3::Y * 100.0,
            DVec3::Z * 100.0,
            -DVec3::Z * 100.0,
        ];
        let t = vec![
            [0, 2, 4],
            [2, 1, 4],
            [1, 3, 4],
            [3, 0, 4],
            [2, 0, 5],
            [1, 2, 5],
            [3, 1, 5],
            [0, 3, 5],
        ];
        (p, t)
    }

    #[test]
    fn inside_points_land_on_the_surface_and_outside_ones_stay() {
        let (p, t) = octa();
        let mut pts = vec![
            DVec3::new(10.0, 5.0, 5.0),
            DVec3::new(300.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 0.0),
        ];
        let far = pts[1];
        collide(&mut pts, &p, &t, 0.0, 1.0);
        assert_eq!(pts[1], far);
        for q in [pts[0], pts[2]] {
            let l1 = q.x.abs() + q.y.abs() + q.z.abs();
            assert!((l1 - 100.0).abs() < 1e-6, "{q:?}");
        }
        // an offset keeps outside points at a distance
        let mut o = vec![DVec3::new(100.0, 0.0, 0.0)];
        collide(&mut o, &p, &t, 10.0, 1.0);
        assert!(o[0].x > 105.0);
    }

    #[test]
    fn the_orientation_of_the_collider_does_not_matter() {
        let (p, t) = octa();
        let rev: Vec<[u32; 3]> = t.iter().map(|t| [t[0], t[2], t[1]]).collect();
        let (mut a, mut b) = (
            vec![DVec3::new(30.0, 10.0, 0.0)],
            vec![DVec3::new(30.0, 10.0, 0.0)],
        );
        collide(&mut a, &p, &t, 0.0, 1.0);
        collide(&mut b, &p, &rev, 0.0, 1.0);
        assert!((a[0] - b[0]).length() < 1e-9);
    }
}
