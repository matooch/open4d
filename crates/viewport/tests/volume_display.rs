//! Volume display draw lists (docs/specs/VOLUME.md section 11.3).

use forge_viewport::volume::{
    BOX_COLOR, VolumeDisplay, box_lines, display_lines, slice_markers, value_color,
};
use glam::DVec3;

#[test]
fn the_box_has_twelve_distinct_edges_on_the_corners() {
    let (lo, hi) = (DVec3::new(-1.0, -2.0, -3.0), DVec3::new(4.0, 5.0, 6.0));
    let lines = box_lines(lo, hi, BOX_COLOR);
    assert_eq!(lines.len(), 12);
    for l in &lines {
        // an edge changes exactly one coordinate between the two bounds
        let d = (l.b - l.a).abs();
        assert_eq!(d.to_array().iter().filter(|c| **c > 0.0).count(), 1);
        for p in [l.a, l.b] {
            for (c, (a, b)) in p
                .to_array()
                .iter()
                .zip(lo.to_array().iter().zip(hi.to_array()))
            {
                assert!(*c == *a || *c == b);
            }
        }
        assert_eq!(l.color, BOX_COLOR);
    }
    let mut keys: Vec<_> = lines
        .iter()
        .map(|l| format!("{:?}{:?}", l.a.to_array(), l.b.to_array()))
        .collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 12);
}

#[test]
fn the_value_ramp_is_white_on_the_surface_blue_inside_orange_outside() {
    assert_eq!(value_color(0.0, 20.0), [1.0, 1.0, 1.0]);
    let inside = value_color(-20.0, 20.0);
    let outside = value_color(20.0, 20.0);
    assert!(inside[2] > inside[0] && outside[0] > outside[2]);
    // saturates at the band and a zero band does not divide by zero
    assert_eq!(value_color(-99.0, 20.0), inside);
    assert_eq!(value_color(5.0, 0.0), [1.0, 1.0, 1.0]);
    // halfway is halfway
    let half = value_color(10.0, 20.0);
    assert!((half[0] - (1.0 + (outside[0] - 1.0) * 0.5)).abs() < 1e-6);
}

#[test]
fn slice_markers_are_crosses_coloured_by_value() {
    let samples = [(DVec3::new(10.0, 0.0, 0.0), -20.0f32), (DVec3::ZERO, 0.0)];
    let m = slice_markers(&samples, 2.0, 20.0);
    assert_eq!(m.len(), 6);
    assert_eq!(m[0].a, DVec3::new(8.0, 0.0, 0.0));
    assert_eq!(m[0].b, DVec3::new(12.0, 0.0, 0.0));
    assert_eq!(m[0].color, value_color(-20.0, 20.0));
    assert_eq!(m[3].color, [1.0, 1.0, 1.0]);
}

#[test]
fn display_modes_choose_what_is_drawn() {
    let b = Some((DVec3::ZERO, DVec3::splat(10.0)));
    let slice = [(DVec3::new(1.0, 2.0, 3.0), 4.0f32)];
    assert_eq!(
        display_lines(b, VolumeDisplay::Box, &slice, 10.0, 20.0).len(),
        12
    );
    assert_eq!(
        display_lines(b, VolumeDisplay::Preview, &slice, 10.0, 20.0).len(),
        12
    );
    assert_eq!(
        display_lines(
            b,
            VolumeDisplay::Slice { axis: 0, index: 0 },
            &slice,
            10.0,
            20.0
        )
        .len(),
        15
    );
    assert!(display_lines(None, VolumeDisplay::Box, &slice, 10.0, 20.0).is_empty());
}
