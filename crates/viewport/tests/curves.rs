//! Spline draw lists and picking (docs/specs/VIEWPORT.md section 11). Pure: no GPU.

use forge_viewport::camera::Camera;
use forge_viewport::curves::{
    ControlHit, Controls, DrawLine, HANDLE_COLOR, Loop, POINT_COLOR, SplineDraw, draw_list, pick,
};
use forge_viewport::render::{Palette, Shading};
use glam::DVec3;

const SIZE: [f64; 2] = [800.0, 600.0];

fn cam() -> Camera {
    Camera {
        distance: 10.0,
        ..Camera::default()
    }
}

fn v(x: f64, y: f64) -> DVec3 {
    DVec3::new(x, y, 0.0)
}

fn open_curve() -> SplineDraw {
    SplineDraw {
        loops: vec![Loop {
            points: vec![v(-2.0, 0.0), v(0.0, 1.0), v(2.0, 0.0)],
            closed: false,
        }],
        selected: false,
        controls: None,
    }
}

fn controls(handles: bool) -> Controls {
    let points = vec![v(-2.0, 0.0), v(0.0, 1.0), v(2.0, 0.0)];
    let off = |dx: f64| if handles { v(dx, 0.0) } else { DVec3::ZERO };
    Controls {
        left: points.iter().map(|p| *p - off(0.5)).collect(),
        right: points.iter().map(|p| *p + off(0.5)).collect(),
        points,
        selected: vec![false, true, false],
        handles,
    }
}

fn lines(d: &SplineDraw, shading: Shading) -> Vec<DrawLine> {
    draw_list(d, shading, &Palette::default(), &cam(), SIZE)
}

fn count(l: &[DrawLine], color: [f32; 3]) -> usize {
    l.iter().filter(|x| x.color == color).count()
}

#[test]
fn the_curve_is_the_polyline_in_the_object_colour_in_every_non_box_mode() {
    let pal = Palette::default();
    let d = open_curve();
    for mode in Shading::ALL {
        if mode == Shading::BoundingBox {
            continue;
        }
        let l = lines(&d, mode);
        assert_eq!(l.len(), 2, "{mode:?}");
        assert!(l.iter().all(|x| x.color == pal.object), "{mode:?}");
        assert_eq!((l[0].a, l[0].b), (v(-2.0, 0.0), v(0.0, 1.0)));
    }
}

#[test]
fn a_selected_spline_uses_the_selection_colour() {
    let mut d = open_curve();
    d.selected = true;
    let l = lines(&d, Shading::Smooth);
    assert!(l.iter().all(|x| x.color == Palette::default().selection));
}

#[test]
fn a_closed_loop_joins_its_last_point_to_its_first() {
    let mut d = open_curve();
    d.loops[0].closed = true;
    let l = lines(&d, Shading::Wire);
    assert_eq!(l.len(), 3);
    assert_eq!((l[2].a, l[2].b), (v(2.0, 0.0), v(-2.0, 0.0)));
    // a loop whose sampler already repeated the first point does not get a zero-length line
    d.loops[0].points.push(v(-2.0, 0.0));
    assert_eq!(lines(&d, Shading::Wire).len(), 3);
}

#[test]
fn bounding_box_mode_draws_the_twelve_box_edges_instead_of_the_curve() {
    let mut d = open_curve();
    d.loops[0].points.push(v(2.0, 3.0));
    let l = lines(&d, Shading::BoundingBox);
    assert_eq!(l.len(), 12);
    let lo = l
        .iter()
        .fold(DVec3::splat(f64::MAX), |m, x| m.min(x.a).min(x.b));
    let hi = l
        .iter()
        .fold(DVec3::splat(f64::MIN), |m, x| m.max(x.a).max(x.b));
    assert_eq!((lo, hi), (v(-2.0, 0.0), v(2.0, 3.0)));
}

#[test]
fn controls_add_point_squares_and_bezier_handles() {
    let mut d = open_curve();
    d.controls = Some(controls(true));
    let pal = Palette::default();
    let l = lines(&d, Shading::Smooth);
    // 2 curve lines, 6 handle lines + 6 handle squares (24 lines), 3 point squares (12 lines)
    assert_eq!(l.len(), 2 + 6 + 24 + 12);
    assert_eq!(count(&l, HANDLE_COLOR), 6 + 24);
    assert_eq!(count(&l, POINT_COLOR), 8, "two unselected points");
    assert_eq!(count(&l, pal.selection), 4, "the selected point");
    // the order: curve, handle lines, handle squares, point squares
    assert_eq!(l[0].color, pal.object);
    assert_eq!(l[2].color, HANDLE_COLOR);
    assert_eq!(l.last().unwrap().color, POINT_COLOR);
}

#[test]
fn squares_have_the_documented_pixel_size() {
    let mut d = open_curve();
    d.controls = Some(controls(false));
    let c = cam();
    let l = lines(&d, Shading::Smooth);
    // the first point square: skip the 2 curve lines
    let sq = &l[2..6];
    let xs: Vec<f64> = sq
        .iter()
        .flat_map(|x| [x.a, x.b])
        .map(|p| c.project(p, SIZE[0], SIZE[1]).unwrap().x)
        .collect();
    let width =
        xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
    assert!((width - 7.0).abs() < 0.05, "{width}");
}

#[test]
fn non_bezier_splines_and_zero_handles_draw_no_handles() {
    let mut d = open_curve();
    d.controls = Some(controls(false));
    let l = lines(&d, Shading::Smooth);
    assert_eq!(count(&l, HANDLE_COLOR), 0);
    assert_eq!(l.len(), 2 + 12);
    // bezier with some zero handles: only the non-zero ones draw
    let mut c = controls(true);
    c.right[0] = c.points[0];
    c.left[2] = c.points[2];
    d.controls = Some(c);
    assert_eq!(count(&lines(&d, Shading::Smooth), HANDLE_COLOR), 4 + 16);
}

#[test]
fn controls_show_in_bounding_box_mode_too() {
    let mut d = open_curve();
    d.controls = Some(controls(false));
    assert_eq!(lines(&d, Shading::BoundingBox).len(), 12 + 12);
}

#[test]
fn points_behind_the_camera_draw_no_squares() {
    let mut d = open_curve();
    let mut c = controls(false);
    c.points[0] = DVec3::new(0.0, 0.0, 50.0); // behind the eye at z = 10
    c.left[0] = c.points[0];
    c.right[0] = c.points[0];
    d.controls = Some(c);
    assert_eq!(lines(&d, Shading::Smooth).len(), 2 + 8);
}

fn px(p: DVec3) -> (f64, f64) {
    let s = cam().project(p, SIZE[0], SIZE[1]).unwrap();
    (s.x, s.y)
}

#[test]
fn picking_finds_points_and_handles_within_the_radius() {
    let mut d = open_curve();
    d.controls = Some(controls(true));
    let at = |p: DVec3, dx: f64| {
        let (x, y) = px(p);
        pick(&d, &cam(), (x + dx, y), (SIZE[0], SIZE[1]), 8.0)
    };
    assert_eq!(at(v(0.0, 1.0), 3.0), Some(ControlHit::Point(1)));
    assert_eq!(at(v(0.0, 1.0), 9.5), None, "outside the radius");
    assert_eq!(at(v(0.5, 1.0), 0.0), Some(ControlHit::Right(1)));
    assert_eq!(at(v(-0.5, 1.0), 0.0), Some(ControlHit::Left(1)));
    assert_eq!(
        pick(
            &open_curve(),
            &cam(),
            (400.0, 300.0),
            (SIZE[0], SIZE[1]),
            8.0
        ),
        None
    );
}

#[test]
fn nearer_wins_and_ties_prefer_point_then_right_then_left() {
    let mut d = open_curve();
    let mut c = controls(true);
    // the right handle of point 1 lies 4 px from the point
    let wpp = cam().world_per_pixel(SIZE[1]);
    c.right[1] = c.points[1] + v(4.0 * wpp, 0.0);
    d.controls = Some(c);
    let (x, y) = px(v(0.0, 1.0));
    let near_handle = pick(&d, &cam(), (x + 3.5, y), (SIZE[0], SIZE[1]), 8.0);
    assert_eq!(near_handle, Some(ControlHit::Right(1)));
    let near_point = pick(&d, &cam(), (x + 0.5, y), (SIZE[0], SIZE[1]), 8.0);
    assert_eq!(near_point, Some(ControlHit::Point(1)));
    // exact tie: a point and both handles at the same place -> Point
    let mut c = controls(true);
    c.right[1] = c.points[1] + v(1e-6, 0.0);
    c.left[1] = c.points[1] - v(1e-6, 0.0);
    d.controls = Some(c);
    let tie = pick(&d, &cam(), (x, y), (SIZE[0], SIZE[1]), 8.0);
    assert_eq!(tie, Some(ControlHit::Point(1)));
}
