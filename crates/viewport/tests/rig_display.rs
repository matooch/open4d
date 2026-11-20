//! Rig display draw lists (docs/specs/VIEWPORT.md section 13). Pure: no GPU.

use forge_viewport::camera::Camera;
use forge_viewport::render::Palette;
use forge_viewport::rig::{
    BoneStyle, CIRCLE_SEGMENTS, JOINT_COLOR, JointDraw, JointStyle, bone_edges, joint_lines,
    weight_color, weight_colors, weight_overlay,
};
use glam::{DMat4, DVec3};

const SIZE: [f64; 2] = [800.0, 600.0];

fn cam() -> Camera {
    Camera {
        distance: 10.0,
        ..Camera::default()
    }
}

fn joint(selected: bool) -> JointDraw {
    JointDraw {
        world: DMat4::from_translation(DVec3::new(0.0, 1.0, 0.0)),
        end: Some(DVec3::new(0.0, 3.0, 0.0)),
        length: 1.0,
        selected,
    }
}

#[test]
fn bone_styles_have_the_documented_edge_counts() {
    let (a, b) = (DVec3::ZERO, DVec3::new(0.0, 2.0, 0.0));
    assert_eq!(bone_edges(a, b, BoneStyle::None).len(), 0);
    assert_eq!(bone_edges(a, b, BoneStyle::Line), vec![[a, b]]);
    assert_eq!(bone_edges(a, b, BoneStyle::Standard).len(), 12);
    assert_eq!(bone_edges(a, b, BoneStyle::Polygon).len(), 12);
    assert_eq!(bone_edges(a, b, BoneStyle::Stick).len(), 12);
    assert_eq!(bone_edges(a, b, BoneStyle::Box).len(), 12);
    assert!(bone_edges(a, a, BoneStyle::Standard).is_empty());
    assert!(bone_edges(a, a, BoneStyle::Box).is_empty());
}

#[test]
fn the_standard_bone_is_an_octahedron_around_the_axis() {
    let (a, b) = (DVec3::ZERO, DVec3::new(0.0, 2.0, 0.0));
    let edges = bone_edges(a, b, BoneStyle::Standard);
    // eight edges touch an apex, the other four are the waist at y = 0.2, radius 0.2
    let waist: Vec<_> = edges
        .iter()
        .filter(|e| e[0] != a && e[0] != b && e[1] != a && e[1] != b)
        .collect();
    assert_eq!(waist.len(), 4);
    for e in waist {
        for p in e {
            assert!((p.y - 0.2).abs() < 1e-12);
            assert!((p.x.hypot(p.z) - 0.2).abs() < 1e-12);
        }
    }
    // a stick is thinner than the standard bone
    let stick = bone_edges(a, b, BoneStyle::Stick);
    let width = |edges: &[[DVec3; 2]]| {
        edges
            .iter()
            .flatten()
            .map(|p| p.x.hypot(p.z))
            .fold(0.0, f64::max)
    };
    assert!(width(&stick) < width(&edges));
}

#[test]
fn joint_lines_draw_the_bone_then_the_marker() {
    let pal = Palette::default();
    let c = cam();
    let j = joint(false);
    let count = |bone, marker| joint_lines(&j, bone, marker, &pal, &c).len();
    assert_eq!(count(BoneStyle::None, JointStyle::None), 0);
    assert_eq!(count(BoneStyle::Line, JointStyle::None), 1);
    assert_eq!(count(BoneStyle::Standard, JointStyle::Axis), 12 + 3);
    assert_eq!(
        count(BoneStyle::None, JointStyle::SphereWire),
        3 * CIRCLE_SEGMENTS
    );
    assert_eq!(
        count(BoneStyle::None, JointStyle::Sphere),
        3 * CIRCLE_SEGMENTS
    );
    assert_eq!(count(BoneStyle::None, JointStyle::Circle), CIRCLE_SEGMENTS);
}

#[test]
fn colours_follow_selection_and_axes() {
    let pal = Palette::default();
    let c = cam();
    let plain = joint_lines(&joint(false), BoneStyle::Line, JointStyle::Axis, &pal, &c);
    assert_eq!(plain[0].color, JOINT_COLOR);
    // axis marker lines keep the x, y, z colours and start at the joint
    assert_eq!(plain[1].color, pal.gizmo_x);
    assert_eq!(plain[2].color, pal.gizmo_y);
    assert_eq!(plain[3].color, pal.gizmo_z);
    assert_eq!(plain[1].a, DVec3::new(0.0, 1.0, 0.0));
    // marker size is 0.15 of the bone length 2
    assert!(((plain[1].b - plain[1].a).length() - 0.3).abs() < 1e-12);
    let sel = joint_lines(&joint(true), BoneStyle::Line, JointStyle::None, &pal, &c);
    assert_eq!(sel[0].color, pal.selection);
}

#[test]
fn a_bone_without_a_child_runs_along_the_joint_z_axis() {
    let j = JointDraw {
        world: DMat4::from_translation(DVec3::new(1.0, 0.0, 0.0)),
        end: None,
        length: 4.0,
        selected: false,
    };
    assert_eq!(j.bone_end(), DVec3::new(1.0, 0.0, 4.0));
    let lines = joint_lines(
        &j,
        BoneStyle::Line,
        JointStyle::None,
        &Palette::default(),
        &cam(),
    );
    assert_eq!(lines[0].b, DVec3::new(1.0, 0.0, 4.0));
}

#[test]
fn the_weight_ramp_runs_blue_cyan_green_yellow_red() {
    assert_eq!(weight_color(0.0), [0.0, 0.0, 1.0]);
    assert_eq!(weight_color(0.25), [0.0, 1.0, 1.0]);
    assert_eq!(weight_color(0.5), [0.0, 1.0, 0.0]);
    assert_eq!(weight_color(0.75), [1.0, 1.0, 0.0]);
    assert_eq!(weight_color(1.0), [1.0, 0.0, 0.0]);
    // clamped outside the range, linear between stops
    assert_eq!(weight_color(-3.0), weight_color(0.0));
    assert_eq!(weight_color(7.0), weight_color(1.0));
    let mid = weight_color(0.125);
    assert!((mid[1] - 0.5).abs() < 1e-6 && (mid[2] - 1.0).abs() < 1e-6);
    // red never falls once it starts rising at 0.5
    let mut prev = -1.0;
    for k in 0..=10 {
        let r = weight_color(0.5 + 0.05 * f64::from(k))[0];
        assert!(r >= prev);
        prev = r;
    }
    assert_eq!(
        weight_colors(&[0.0, 1.0]),
        vec![[0.0, 0.0, 1.0], [1.0, 0.0, 0.0]]
    );
}

#[test]
fn the_overlay_has_one_marker_per_vertex_and_can_hide_zero_weights() {
    let pts = [
        DVec3::ZERO,
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let w = [0.0, 0.5, 1.0];
    let c = cam();
    let all = weight_overlay(&pts, &w, false, &c, SIZE);
    assert_eq!(all.len(), 12);
    assert!(all[..4].iter().all(|l| l.color == weight_color(0.0)));
    assert!(all[8..].iter().all(|l| l.color == weight_color(1.0)));
    let shown = weight_overlay(&pts, &w, true, &c, SIZE);
    assert_eq!(shown.len(), 8);
    // a missing weight counts as 0
    assert_eq!(weight_overlay(&pts, &[1.0], true, &c, SIZE).len(), 4);
    // the square is WEIGHT_PX wide on screen: much smaller than a unit step at distance 10
    let side = (all[0].b - all[0].a).length();
    assert!(side > 0.0 && side < 1.0);
}
