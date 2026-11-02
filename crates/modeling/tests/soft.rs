//! Soft selection weights and symmetry (docs/specs/EDITABLE.md §5).

use forge_geometry::PolyMesh;
use forge_modeling::{
    Axis, ComponentPlan, ComponentSettings, Distance, Falloff, SoftSelection, Symmetry, partners,
    transform_components, weights,
};
use glam::{DMat4, DVec3};
use serde_json::json;

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

/// 11 points on the x axis, spacing 10, joined by edges (a path of degenerate "polys" is not
/// allowed, so the edges come from thin quads along z).
fn row() -> PolyMesh {
    let mut points = Vec::new();
    for i in 0..11 {
        points.push(DVec3::new(i as f64 * 10.0, 0.0, 0.0));
    }
    for i in 0..11 {
        points.push(DVec3::new(i as f64 * 10.0, 0.0, 10.0));
    }
    let polys = (0..10u32).map(|i| vec![i, i + 1, 12 + i, 11 + i]).collect();
    PolyMesh { points, polys }
}

fn soft(falloff: Falloff) -> SoftSelection {
    SoftSelection {
        enabled: true,
        radius: 100.0,
        falloff,
        ..SoftSelection::default()
    }
}

fn settings(soft: SoftSelection, symmetry: Symmetry) -> ComponentSettings {
    ComponentSettings { soft, symmetry }
}

#[test]
fn falloff_shapes_at_sample_points() {
    let at = |f: Falloff, t: f64| soft(f).weight_at(t);
    for f in Falloff::ALL {
        close(at(f, 0.0), 1.0);
        close(at(f, 1.0), 0.0);
        close(at(f, 1.5), 0.0);
    }
    close(at(Falloff::Linear, 0.25), 0.75);
    close(at(Falloff::Linear, 0.5), 0.5);
    close(at(Falloff::Dome, 0.5), 0.75f64.sqrt());
    close(at(Falloff::Bell, 0.5), 0.5);
    close(
        at(Falloff::Bell, 0.25),
        0.5 * (1.0 + (std::f64::consts::PI / 4.0).cos()),
    );
    // the default spline passes through its samples
    close(at(Falloff::Spline, 0.5), 0.5);
    close(at(Falloff::Spline, 0.25), 0.896);
}

#[test]
fn spline_curve_follows_the_user_samples_and_is_clamped() {
    let mut s = soft(Falloff::Spline);
    s.curve = vec![1.0, 1.0, 0.0];
    close(s.weight_at(0.0), 1.0);
    close(s.weight_at(0.5), 1.0);
    for k in 0..100 {
        let w = s.weight_at(k as f64 / 100.0);
        assert!((0.0..=1.0).contains(&w), "{w}");
    }
    s.curve = vec![0.5];
    assert!(s.validate().is_err());
    s.curve = vec![0.5, 1.5];
    assert!(s.validate().is_err());
    s.curve = vec![0.0; 65];
    assert!(s.validate().is_err());
}

#[test]
fn a_move_with_radius_100_moves_neighbours_by_the_falloff_fraction() {
    let mesh = row();
    for f in Falloff::ALL {
        let s = settings(soft(f), Symmetry::default());
        let out = transform_components(&mesh, &[0], DMat4::from_translation(DVec3::Y * 50.0), &s);
        for (i, o) in out.iter().enumerate().take(11) {
            let expect = soft(f).weight_at(i as f64 * 10.0 / 100.0) * 50.0;
            close(o.y, expect);
            close(o.x, mesh.points[i].x);
        }
        // the far row is 10 away in z: distance sqrt(x^2 + 100)
        for (o, p) in out.iter().zip(&mesh.points).skip(11) {
            close(
                o.y,
                soft(f).weight_at(p.distance(mesh.points[0]) / 100.0) * 50.0,
            );
        }
    }
}

#[test]
fn selected_points_always_move_fully_and_off_means_selected_only() {
    let mesh = row();
    let m = DMat4::from_translation(DVec3::Z * 7.0);
    let off = settings(SoftSelection::default(), Symmetry::default());
    let out = transform_components(&mesh, &[3, 4], m, &off);
    for (i, (o, p)) in out.iter().zip(&mesh.points).enumerate() {
        let moved = i == 3 || i == 4;
        close(o.z - p.z, if moved { 7.0 } else { 0.0 });
    }
    let on = settings(soft(Falloff::Linear), Symmetry::default());
    let out = transform_components(&mesh, &[3, 4], m, &on);
    close(out[3].z, 7.0);
    close(out[4].z, 7.0);
    close(out[5].z - mesh.points[5].z, 7.0 * 0.9);
}

#[test]
fn the_nearest_selected_point_sets_the_distance() {
    let mesh = row();
    let w = weights(&mesh, &[0, 10], &soft(Falloff::Linear));
    close(w[5], 0.5);
    close(w[2], 0.8);
    close(w[8], 0.8);
}

#[test]
fn rotation_and_scale_blend_positions() {
    let mesh = row();
    let s = settings(soft(Falloff::Linear), Symmetry::default());
    let m = DMat4::from_scale(DVec3::splat(2.0));
    let out = transform_components(&mesh, &[0], m, &s);
    // point 0 is at the origin: it stays; point 5 moves by w * (2p - p)
    close(out[5].x, 50.0 + 0.5 * 50.0);
}

#[test]
fn surface_distance_does_not_cross_a_gap_that_volume_does() {
    // two separate strips 20 apart in z
    let mut points = vec![];
    for z in [0.0, 20.0] {
        for x in [0.0, 10.0] {
            points.push(DVec3::new(x, 0.0, z));
            points.push(DVec3::new(x, 10.0, z));
        }
    }
    let mesh = PolyMesh {
        points,
        polys: vec![vec![0, 1, 3, 2], vec![4, 5, 7, 6]],
    };
    let mut s = soft(Falloff::Linear);
    s.distance = Distance::Volume;
    let vol = weights(&mesh, &[0], &s);
    s.distance = Distance::Surface;
    let surf = weights(&mesh, &[0], &s);
    assert!(vol[4] > 0.0, "volume reaches the other strip");
    close(surf[4], 0.0);
    close(surf[1], 0.9);
    close(vol[1], 0.9);
}

#[test]
fn surface_distance_follows_the_edges() {
    // an L: the corner cuts the straight line
    let mesh = PolyMesh {
        points: vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(30.0, 0.0, 0.0),
            DVec3::new(30.0, 40.0, 0.0),
            DVec3::new(0.0, 40.0, 0.0),
        ],
        polys: vec![vec![0, 1, 2, 3]],
    };
    let mut s = soft(Falloff::Linear);
    s.radius = 200.0;
    s.distance = Distance::Surface;
    // 0 -> 2 along edges is 70, straight is 50
    close(weights(&mesh, &[0], &s)[2], 1.0 - 70.0 / 200.0);
    s.distance = Distance::Volume;
    close(weights(&mesh, &[0], &s)[2], 1.0 - 50.0 / 200.0);
}

fn sym(axis: Axis) -> Symmetry {
    Symmetry {
        enabled: true,
        axis,
        offset: 0.0,
        tolerance: 0.01,
    }
}

/// Points at x = -2..=2 (y = 0) and a ring of y = 10 on a symmetric quad strip.
fn strip() -> PolyMesh {
    let mut points = vec![];
    for y in [0.0, 10.0] {
        for x in -2..=2 {
            points.push(DVec3::new(x as f64 * 10.0, y, 0.0));
        }
    }
    let polys = (0..4u32).map(|i| vec![i, i + 1, 6 + i, 5 + i]).collect();
    PolyMesh { points, polys }
}

#[test]
fn partners_are_found_within_the_tolerance() {
    let mesh = strip();
    let p = partners(&mesh, &sym(Axis::X));
    assert_eq!(p[0], Some(4));
    assert_eq!(p[4], Some(0));
    assert_eq!(p[2], Some(2));
    assert_eq!(p[5], Some(9));
    let mut m = mesh.clone();
    m.points[4].x += 0.5;
    assert_eq!(partners(&m, &sym(Axis::X))[0], None);
    let mut loose = sym(Axis::X);
    loose.tolerance = 1.0;
    assert_eq!(partners(&m, &loose)[0], Some(4));
    // along y nothing matches except itself on the plane y = 0 (offset 0)
    let py = partners(&mesh, &sym(Axis::Y));
    assert_eq!(py[0], Some(0));
    assert_eq!(py[5], None);
}

fn assert_symmetric(points: &[DVec3], tol: f64) {
    for p in points {
        let r = DVec3::new(-p.x, p.y, p.z);
        assert!(
            points.iter().any(|q| q.distance(r) < tol),
            "no mirror for {p}"
        );
    }
}

#[test]
fn moving_one_side_moves_the_mirror_side_too() {
    let mesh = strip();
    let s = settings(SoftSelection::default(), sym(Axis::X));
    let m = DMat4::from_translation(DVec3::new(5.0, 3.0, 2.0));
    let out = transform_components(&mesh, &[4], m, &s);
    assert_eq!(out[4], mesh.points[4] + DVec3::new(5.0, 3.0, 2.0));
    assert_eq!(out[0], mesh.points[0] + DVec3::new(-5.0, 3.0, 2.0));
    for i in [1, 2, 3, 5, 6, 7, 8, 9] {
        assert_eq!(out[i], mesh.points[i]);
    }
    assert_symmetric(&out, 1e-9);
}

#[test]
fn points_on_the_plane_stay_on_it() {
    let mesh = strip();
    let s = settings(SoftSelection::default(), sym(Axis::X));
    let m = DMat4::from_translation(DVec3::new(5.0, 3.0, 2.0));
    let out = transform_components(&mesh, &[2], m, &s);
    close(out[2].x, 0.0);
    close(out[2].y, 3.0);
    close(out[2].z, 2.0);
}

#[test]
fn selecting_both_sides_mirrors_a_rotation_and_a_scale() {
    let mesh = strip();
    let s = settings(SoftSelection::default(), sym(Axis::X));
    let rot = DMat4::from_translation(DVec3::new(0.0, 5.0, 0.0))
        * DMat4::from_rotation_z(0.3)
        * DMat4::from_translation(DVec3::new(0.0, -5.0, 0.0));
    let out = transform_components(&mesh, &[0, 4, 1, 3], rot, &s);
    assert_symmetric(&out, 1e-9);
    let sc = DMat4::from_scale(DVec3::new(1.5, 0.5, 1.0));
    let out = transform_components(&mesh, &[0, 4, 5, 9], sc, &s);
    assert_symmetric(&out, 1e-9);
}

#[test]
fn symmetric_soft_moves_keep_a_symmetric_mesh_symmetric() {
    let mesh = strip();
    let s = settings(soft(Falloff::Bell), sym(Axis::X));
    let mut r = s.clone();
    r.soft.radius = 25.0;
    for sel in [vec![4u32], vec![0], vec![3, 9], vec![2]] {
        for m in [
            DMat4::from_translation(DVec3::new(4.0, 6.0, -2.0)),
            DMat4::from_rotation_y(0.4),
            DMat4::from_scale(DVec3::new(2.0, 1.0, 3.0)),
        ] {
            assert_symmetric(&transform_components(&mesh, &sel, m, &r), 1e-9);
        }
    }
}

#[test]
fn soft_selection_near_the_plane_reaches_across_it_symmetrically() {
    let mesh = strip();
    let mut st = settings(soft(Falloff::Linear), sym(Axis::X));
    st.soft.radius = 20.0;
    let plan = ComponentPlan::new(&mesh, &[3], &st);
    let out = plan.apply(&mesh.points, DMat4::from_translation(DVec3::Y * 10.0));
    close(out[3].y, 10.0);
    close(out[1].y, 10.0); // the mirror of the selected point
    close(out[2].y, 5.0); // 10 away from both: weight 0.5
    close(out[4].y, 5.0);
    close(out[0].y, 5.0);
    assert_eq!(plan.partners().unwrap()[1], Some(3));
}

#[test]
fn symmetry_off_leaves_the_mirror_side_alone() {
    let mesh = strip();
    let mut s = settings(SoftSelection::default(), sym(Axis::X));
    s.symmetry.enabled = false;
    let out = transform_components(&mesh, &[4], DMat4::from_translation(DVec3::Y), &s);
    assert_eq!(out[0], mesh.points[0]);
}

#[test]
fn settings_json_sets_keys_and_rejects_bad_values_without_changing_anything() {
    let mut s = SoftSelection::default();
    let obj = |v: serde_json::Value| v.as_object().unwrap().clone();
    s.apply_json(&obj(
        json!({"enabled": true, "radius": 50, "falloff": "dome"}),
    ))
    .unwrap();
    assert!(s.enabled && s.radius == 50.0 && s.falloff == Falloff::Dome);
    let before = s.clone();
    assert!(s.apply_json(&obj(json!({"radius": 0}))).is_err());
    assert!(
        s.apply_json(&obj(json!({"falloff": "box", "radius": 5})))
            .is_err()
    );
    assert!(s.apply_json(&obj(json!({"nope": 1}))).is_err());
    assert_eq!(s, before);
    let mut y = Symmetry::default();
    y.apply_json(&obj(
        json!({"enabled": true, "axis": "z", "offset": 2.5, "tolerance": 0.5}),
    ))
    .unwrap();
    assert_eq!((y.axis, y.offset, y.tolerance), (Axis::Z, 2.5, 0.5));
    assert!(y.apply_json(&obj(json!({"axis": "w"}))).is_err());
    assert!(y.apply_json(&obj(json!({"tolerance": -1}))).is_err());
}
