//! Pane layout maths and per-pane cameras (docs/specs/VIEWPORT.md §9). Pure, no GPU.

use forge_viewport::camera::{Camera, Handedness, Projection};
use forge_viewport::panes::{Layout, Pane, PaneRect, arrange, pane_at, pane_camera, visible};
use glam::DVec3;

#[test]
fn ids_round_trip_and_indices_follow_all() {
    for (i, p) in Pane::ALL.into_iter().enumerate() {
        assert_eq!(p.index(), i);
        assert_eq!(Pane::from_id(p.id()), Some(p));
    }
    assert_eq!(Pane::from_id("back"), None);
    assert_eq!(Pane::Perspective.label(), "Perspective");
}

#[test]
fn visible_panes_per_layout() {
    use Pane::*;
    for active in Pane::ALL {
        assert_eq!(visible(Layout::Single, active, None), vec![active]);
        // maximise does nothing in the single layout
        assert_eq!(visible(Layout::Single, active, Some(Top)), vec![active]);
        assert_eq!(
            visible(Layout::Four, active, None),
            vec![Perspective, Top, Right, Front]
        );
        assert_eq!(visible(Layout::Four, active, Some(Front)), vec![Front]);
    }
}

fn overlap(a: &PaneRect, b: &PaneRect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

#[test]
fn four_panes_tile_the_area_with_gaps() {
    for (w, h, gap) in [
        (800.0, 600.0, 2.0),
        (801.0, 601.0, 2.0),
        (1279.0, 719.0, 4.0),
        (100.0, 50.0, 0.0),
        (333.0, 777.0, 3.0),
    ] {
        let r = arrange(Layout::Four, Pane::Perspective, None, w, h, gap);
        assert_eq!(r.len(), 4);
        let order: Vec<Pane> = r.iter().map(|p| p.pane).collect();
        assert_eq!(order, Pane::QUAD);
        for (i, a) in r.iter().enumerate() {
            assert!(a.w > 0.0 && a.h > 0.0);
            assert!(a.x >= 0.0 && a.y >= 0.0 && a.x + a.w <= w && a.y + a.h <= h);
            for b in &r[i + 1..] {
                assert!(!overlap(a, b), "{a:?} {b:?}");
            }
        }
        // corners: top-left starts at the origin, bottom-right ends at the area's corner
        assert_eq!((r[0].x, r[0].y), (0.0, 0.0));
        assert_eq!((r[3].x + r[3].w, r[3].y + r[3].h), (w, h));
        // columns and rows are separated by exactly the gap
        assert_eq!(r[1].x - (r[0].x + r[0].w), gap);
        assert_eq!(r[2].y - (r[0].y + r[0].h), gap);
        assert_eq!(r[1].y, 0.0);
        assert_eq!(r[2].x, 0.0);
        // covered area + the gap strips = the whole area
        let covered: f32 = r.iter().map(|p| p.w * p.h).sum();
        let strips = gap * w + gap * h - gap * gap;
        assert!((covered + strips - w * h).abs() < 1.0, "{w}x{h} gap {gap}");
        // widths differ by at most one pixel
        assert!((r[0].w - r[1].w).abs() <= 1.0 && (r[0].h - r[2].h).abs() <= 1.0);
    }
}

#[test]
fn one_visible_pane_gets_the_whole_area() {
    let single = arrange(Layout::Single, Pane::Right, None, 640.0, 480.0, 2.0);
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].pane, Pane::Right);
    assert_eq!(
        (single[0].x, single[0].y, single[0].w, single[0].h),
        (0.0, 0.0, 640.0, 480.0)
    );
    let max = arrange(
        Layout::Four,
        Pane::Perspective,
        Some(Pane::Top),
        640.0,
        480.0,
        2.0,
    );
    assert_eq!(max.len(), 1);
    assert_eq!(max[0].pane, Pane::Top);
    assert_eq!((max[0].w, max[0].h), (640.0, 480.0));
}

#[test]
fn pane_at_finds_interiors_and_rejects_gaps() {
    let r = arrange(Layout::Four, Pane::Perspective, None, 802.0, 602.0, 2.0);
    // 400 + 2 + 400 wide, 300 + 2 + 300 tall
    assert_eq!(pane_at(&r, 10.0, 10.0), Some(Pane::Perspective));
    assert_eq!(pane_at(&r, 600.0, 10.0), Some(Pane::Top));
    assert_eq!(pane_at(&r, 10.0, 500.0), Some(Pane::Right));
    assert_eq!(pane_at(&r, 700.0, 500.0), Some(Pane::Front));
    // first pixel of a pane belongs to it, the last pixel before the gap too
    assert_eq!(pane_at(&r, 0.0, 0.0), Some(Pane::Perspective));
    assert_eq!(pane_at(&r, 399.9, 299.9), Some(Pane::Perspective));
    assert_eq!(pane_at(&r, 402.0, 302.0), Some(Pane::Front));
    // gaps and outside
    assert_eq!(pane_at(&r, 400.5, 10.0), None);
    assert_eq!(pane_at(&r, 10.0, 301.0), None);
    assert_eq!(pane_at(&r, 802.0, 10.0), None);
    assert_eq!(pane_at(&r, -1.0, 10.0), None);
    // a single pane covers everything inside
    let one = arrange(Layout::Single, Pane::Front, None, 100.0, 100.0, 2.0);
    assert_eq!(pane_at(&one, 50.0, 99.0), Some(Pane::Front));
}

fn base() -> Camera {
    Camera {
        yaw: 0.68,
        pitch: 0.52,
        distance: 1000.0,
        handedness: Handedness::Left,
        target: DVec3::new(10.0, 20.0, 30.0),
        ..Camera::default()
    }
}

#[test]
fn default_pane_cameras() {
    let b = base();
    assert_eq!(pane_camera(Pane::Perspective, &b), b);
    let top = pane_camera(Pane::Top, &b);
    let front = pane_camera(Pane::Front, &b);
    let right = pane_camera(Pane::Right, &b);
    for c in [&top, &front, &right] {
        assert!(matches!(c.projection, Projection::Orthographic { .. }));
        assert_eq!(c.target, b.target, "the pivot is kept");
    }
    assert!(
        (top.forward_world() - DVec3::NEG_Y).length() < 1e-3,
        "{:?}",
        top.forward_world()
    );
    // Right: eye on world +X; Front: the side the default camera sits on (-Z, left-handed)
    assert!((right.eye_world() - right.target_world()).x > 0.0);
    assert!((right.forward_world() - DVec3::NEG_X).length() < 1e-6);
    assert!((front.eye_world() - front.target_world()).z < 0.0);
    assert!((front.forward_world() - DVec3::Z).length() < 1e-6);
    // the three differ
    assert!(top != front && front != right);
    // the same in a right-handed world: Front looks along -Z
    let rh = Camera {
        handedness: Handedness::Right,
        ..b
    };
    assert!((pane_camera(Pane::Front, &rh).forward_world() - DVec3::NEG_Z).length() < 1e-6);
}
