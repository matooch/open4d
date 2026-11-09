//! Motion path and onion skin draw lists (docs/specs/VIEWPORT.md section 12). Pure: no GPU.

mod common;

use common::unit_box;
use forge_geometry::PolyMesh;
use forge_viewport::camera::Camera;
use forge_viewport::curves::DrawLine;
use forge_viewport::motion::{
    GHOST_AFTER, GHOST_BEFORE, GhostDraw, KEY_COLOR, MAX_GHOST_POLYS, MotionDraw, PATH_FUTURE,
    PATH_PAST, PathDraw, ghost_color, ghost_geometry, ghost_strength, overlay_lines,
};
use forge_viewport::render::Palette;
use glam::DVec3;

const SIZE: [f64; 2] = [800.0, 600.0];

fn cam() -> Camera {
    Camera {
        distance: 10.0,
        ..Camera::default()
    }
}

fn path(current: Option<(f64, DVec3)>) -> PathDraw {
    PathDraw {
        samples: (0..=4)
            .map(|i| (f64::from(i) * 10.0, DVec3::new(f64::from(i), 0.0, 0.0)))
            .collect(),
        keys: vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(4.0, 0.0, 0.0)],
        current,
    }
}

fn lines(d: &MotionDraw) -> Vec<DrawLine> {
    overlay_lines(d, &Palette::default(), &cam(), SIZE)
}

fn count_color(l: &[DrawLine], c: [f32; 3]) -> usize {
    l.iter().filter(|x| x.color == c).count()
}

#[test]
fn segments_are_past_up_to_the_current_frame_and_future_after() {
    let d = MotionDraw {
        paths: vec![path(Some((20.0, DVec3::new(2.0, 0.0, 0.0))))],
        ghosts: vec![],
    };
    let l = lines(&d);
    assert_eq!(
        count_color(&l, PATH_PAST),
        2,
        "segments ending at 10 and 20"
    );
    assert_eq!(count_color(&l, PATH_FUTURE), 2);
    // without a current frame everything is future
    let d = MotionDraw {
        paths: vec![path(None)],
        ghosts: vec![],
    };
    let l = lines(&d);
    assert_eq!(count_color(&l, PATH_FUTURE), 4);
    assert_eq!(count_color(&l, PATH_PAST), 0);
}

#[test]
fn key_and_current_squares() {
    let pal = Palette::default();
    let with = MotionDraw {
        paths: vec![path(Some((20.0, DVec3::new(2.0, 0.0, 0.0))))],
        ghosts: vec![],
    };
    let l = lines(&with);
    assert_eq!(
        count_color(&l, KEY_COLOR),
        8,
        "two key squares of four lines"
    );
    assert_eq!(count_color(&l, pal.selection), 4, "one current square");
    // order: segments, key squares, current square
    let first_key = l.iter().position(|x| x.color == KEY_COLOR).unwrap();
    let first_cur = l.iter().position(|x| x.color == pal.selection).unwrap();
    let last_seg = l
        .iter()
        .rposition(|x| x.color == PATH_PAST || x.color == PATH_FUTURE)
        .unwrap();
    assert!(last_seg < first_key && first_key < first_cur);
    // a current square is 9 px wide: half of it from the centre
    let sq: Vec<&DrawLine> = l.iter().filter(|x| x.color == pal.selection).collect();
    let w = sq.iter().map(|x| x.a.x).fold(f64::MIN, f64::max)
        - sq.iter().map(|x| x.a.x).fold(f64::MAX, f64::min);
    let px = cam().world_per_pixel(SIZE[1]);
    assert!((w - 9.0 * px).abs() < 1e-9, "{w} vs {}", 9.0 * px);
    // no current marker without a current frame
    let none = MotionDraw {
        paths: vec![path(None)],
        ghosts: vec![],
    };
    assert_eq!(count_color(&lines(&none), pal.selection), 0);
}

#[test]
fn squares_behind_the_camera_are_skipped() {
    let c = cam();
    let behind = c.eye_world() - c.forward_world() * 5.0;
    let d = MotionDraw {
        paths: vec![PathDraw {
            samples: vec![(0.0, DVec3::ZERO), (1.0, DVec3::X)],
            keys: vec![behind],
            current: Some((0.0, behind)),
        }],
        ghosts: vec![],
    };
    let l = lines(&d);
    assert_eq!(count_color(&l, KEY_COLOR), 0);
    assert_eq!(count_color(&l, Palette::default().selection), 0);
    assert_eq!(l.len(), 1, "only the segment");
}

#[test]
fn ghost_geometry_is_the_unique_edges_or_the_box() {
    let g = ghost_geometry(&unit_box());
    assert_eq!(g.len(), 12);
    assert!(ghost_geometry(&PolyMesh::default()).is_empty());
    // a heavy mesh falls back to the 12 edges of its bounding box
    let n = MAX_GHOST_POLYS + 1;
    let heavy = PolyMesh {
        points: (0..n * 3)
            .map(|i| DVec3::new(i as f64, (i % 7) as f64, 0.0))
            .collect(),
        polys: (0..n)
            .map(|i| vec![(i * 3) as u32, (i * 3 + 1) as u32, (i * 3 + 2) as u32])
            .collect(),
    };
    let g = ghost_geometry(&heavy);
    assert_eq!(g.len(), 12);
    let xmax = g
        .iter()
        .flat_map(|e| e.iter())
        .map(|p| p.x)
        .fold(f64::MIN, f64::max);
    assert_eq!(xmax, (n * 3 - 1) as f64);
}

#[test]
fn ghost_fade_is_monotone_and_tinted_by_side() {
    assert_eq!(ghost_strength(1, 1), 0.8);
    assert_eq!(ghost_strength(1, 3), 0.8);
    assert!(ghost_strength(1, 3) > ghost_strength(2, 3));
    assert!(ghost_strength(2, 3) > ghost_strength(3, 3));
    assert!((ghost_strength(3, 3) - 0.3).abs() < 1e-6);
    let pal = Palette::default();
    let near = ghost_color(false, 1, 3, &pal);
    let far = ghost_color(false, 3, 3, &pal);
    // the far ghost is closer to the background
    let dist = |a: [f32; 3]| {
        (0..3)
            .map(|i| (a[i] - pal.background[i]).abs())
            .sum::<f32>()
    };
    assert!(dist(near) > dist(far));
    assert_ne!(ghost_color(true, 1, 3, &pal), near);
    let _ = (GHOST_AFTER, GHOST_BEFORE);
}

#[test]
fn ghosts_draw_first_with_their_colour() {
    let pal = Palette::default();
    let ghost = |forward, rank| GhostDraw {
        forward,
        rank,
        of: 2,
        lines: ghost_geometry(&unit_box()),
    };
    let d = MotionDraw {
        paths: vec![path(None)],
        ghosts: vec![ghost(false, 1), ghost(true, 2)],
    };
    let l = lines(&d);
    assert_eq!(l.len(), 12 + 12 + 4 + 8);
    assert_eq!(l[0].color, ghost_color(false, 1, 2, &pal));
    assert_eq!(l[12].color, ghost_color(true, 2, 2, &pal));
    assert_eq!(l[24].color, PATH_FUTURE);
}

#[test]
fn empty_draw_has_no_lines() {
    assert!(lines(&MotionDraw::default()).is_empty());
}
