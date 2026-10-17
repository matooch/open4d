//! docs/specs/VIEWPORT.md §2. Default camera: eye (0,0,10) looking at a ±1 box.

mod common;

use common::{H, W, close, unit_box};
use forge_geometry::Edge;
use forge_geometry::selection::Mode;
use forge_viewport::camera::{Camera, Ray};
use forge_viewport::picking::{
    Element, pick_edge, pick_point, pick_poly, ray_triangle, rect_select,
};
use glam::DVec3;

fn px(c: &Camera, p: DVec3) -> (f64, f64) {
    let s = c.project(p, W, H).unwrap();
    (s.x, s.y)
}

#[test]
fn ray_triangle_is_two_sided() {
    let (a, b, c) = (DVec3::ZERO, DVec3::X, DVec3::Y);
    let down = Ray {
        origin: DVec3::new(0.2, 0.2, 3.0),
        dir: DVec3::NEG_Z,
    };
    assert!((ray_triangle(&down, a, b, c).unwrap() - 3.0).abs() < 1e-12);
    let up = Ray {
        origin: DVec3::new(0.2, 0.2, -2.0),
        dir: DVec3::Z,
    };
    assert!((ray_triangle(&up, a, b, c).unwrap() - 2.0).abs() < 1e-12);
    let miss = Ray {
        origin: DVec3::new(0.8, 0.8, 3.0),
        dir: DVec3::NEG_Z,
    };
    assert!(ray_triangle(&miss, a, b, c).is_none());
    let away = Ray {
        origin: DVec3::new(0.2, 0.2, 3.0),
        dir: DVec3::Z,
    };
    assert!(ray_triangle(&away, a, b, c).is_none());
}

#[test]
fn pick_poly_returns_the_nearest_face() {
    let m = unit_box();
    let c = Camera::default();
    let hit = pick_poly(&m, &c.ray(W / 2.0, H / 2.0, W, H)).unwrap();
    assert_eq!(hit.poly, 3, "z = +1 face");
    assert!((hit.t - 9.0).abs() < 1e-9);
    close(hit.point, DVec3::new(0., 0., 1.), 1e-9);
    assert!(pick_poly(&m, &c.ray(5.0, 5.0, W, H)).is_none());
}

#[test]
fn pick_point_by_screen_distance_and_visibility() {
    let m = unit_box();
    let c = Camera::default();
    let (fx, fy) = px(&c, DVec3::new(1., 1., 1.)); // vertex 7, front
    assert_eq!(
        pick_point(&m, &c, fx + 2.0, fy - 2.0, W, H, 5.0, true),
        Some(7)
    );
    let (bx, by) = px(&c, DVec3::new(1., 1., -1.)); // vertex 3, hidden behind the front face
    assert_eq!(pick_point(&m, &c, bx, by, W, H, 3.0, false), Some(3));
    assert_eq!(pick_point(&m, &c, bx, by, W, H, 3.0, true), None);
    assert_eq!(pick_point(&m, &c, 5.0, 5.0, W, H, 3.0, false), None);
}

#[test]
fn pick_edge_by_distance_to_projected_segment() {
    let m = unit_box();
    let c = Camera::default();
    let id = m.edges().iter().position(|e| *e == Edge(6, 7)).unwrap() as u32;
    let (x, y) = px(&c, DVec3::new(0.3, 1., 1.));
    assert_eq!(pick_edge(&m, &c, x, y + 2.0, W, H, 4.0, true), Some(id));
    assert_eq!(pick_edge(&m, &c, W / 2.0, H / 2.0, W, H, 4.0, true), None);
}

#[test]
fn rect_select_points_edges_polys() {
    let m = unit_box();
    let c = Camera::default();
    let all = [0.0, 0.0, W, H];
    let ids = |s: forge_geometry::selection::IndexSet| s.iter().collect::<Vec<_>>();
    assert_eq!(
        ids(rect_select(
            &m,
            &c,
            all,
            W,
            H,
            Element::Points,
            Mode::All,
            false
        ))
        .len(),
        8
    );
    assert_eq!(
        ids(rect_select(
            &m,
            &c,
            all,
            W,
            H,
            Element::Points,
            Mode::All,
            true
        )),
        [4, 5, 6, 7]
    );
    assert_eq!(
        ids(rect_select(
            &m,
            &c,
            all,
            W,
            H,
            Element::Polys,
            Mode::All,
            true
        )),
        [3]
    );
    assert_eq!(
        ids(rect_select(
            &m,
            &c,
            all,
            W,
            H,
            Element::Polys,
            Mode::Any,
            false
        ))
        .len(),
        6
    );
    assert_eq!(
        ids(rect_select(
            &m,
            &c,
            all,
            W,
            H,
            Element::Edges,
            Mode::All,
            true
        ))
        .len(),
        4
    );
    let reversed = [W, H, 0.0, 0.0];
    assert_eq!(
        ids(rect_select(
            &m,
            &c,
            reversed,
            W,
            H,
            Element::Points,
            Mode::All,
            false
        ))
        .len(),
        8
    );
    let centre = [390.0, 290.0, 410.0, 310.0];
    assert!(rect_select(&m, &c, centre, W, H, Element::Points, Mode::All, false).is_empty());
}
