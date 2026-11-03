//! docs/specs/MANIPULATE.md §6: snap settings, targets, priority and the world-space solver.

use forge_geometry::PolyMesh;
use forge_modeling::snap::{
    SnapKind, SnapMode, SnapSettings, SnapSource, Workplane, closest_on_triangle, normal_basis,
    snap_point,
};
use glam::DVec3;
use serde_json::{Map, Value, json};

/// Cube from -1 to 1; point index = x + 2y + 4z.
fn cube() -> PolyMesh {
    let p = |i: u32| {
        DVec3::new(
            if i & 1 == 1 { 1.0 } else { -1.0 },
            if i & 2 == 2 { 1.0 } else { -1.0 },
            if i & 4 == 4 { 1.0 } else { -1.0 },
        )
    };
    PolyMesh {
        points: (0..8).map(p).collect(),
        polys: vec![
            vec![0, 1, 5, 4],
            vec![2, 6, 7, 3],
            vec![0, 2, 3, 1],
            vec![4, 5, 7, 6],
            vec![0, 4, 6, 2],
            vec![1, 3, 7, 5],
        ],
    }
}

fn source() -> Vec<SnapSource> {
    vec![SnapSource {
        mesh: cube(),
        origin: Some(DVec3::ZERO),
        skip_points: vec![],
    }]
}

fn only(kinds: &[SnapKind]) -> SnapSettings {
    let mut s = SnapSettings::default();
    for k in SnapKind::ALL {
        s.targets.set(k, kinds.contains(&k));
    }
    s
}

fn obj(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

fn close(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 1e-12, "{a} != {b}");
}

#[test]
fn defaults_json_and_validation() {
    let mut s = SnapSettings::default();
    assert!(!s.enabled);
    assert_eq!(s.mode, SnapMode::D3);
    assert_eq!(s.radius, 10.0);
    assert!(s.targets.vertex && s.targets.edge && s.targets.edge_midpoint);
    assert!(s.targets.polygon_center && !s.targets.polygon && !s.targets.workplane);
    let j = s.to_json();
    assert_eq!(j["mode"], "3d");
    assert_eq!(j["targets"]["edge_midpoint"], true);
    assert_eq!(j["quantize"]["angle"], 15.0);
    assert_eq!(j["workplane"]["normal"], json!([0.0, 1.0, 0.0]));
    // a partial update
    s.apply_json(&obj(json!({
        "enabled": true, "mode": "2d", "radius": 20,
        "targets": {"vertex": false, "grid_point": true},
        "workplane": {"origin": [0, 5, 0], "normal": [0, 2, 0]},
        "quantize": {"enabled": true, "move": 5}
    })))
    .unwrap();
    assert!(s.enabled && s.mode == SnapMode::D2 && s.radius == 20.0);
    assert!(!s.targets.vertex && s.targets.grid_point && s.targets.edge);
    assert_eq!(s.workplane.normal, DVec3::Y);
    assert_eq!(s.workplane.origin, DVec3::new(0.0, 5.0, 0.0));
    assert!(s.quantize.enabled && s.quantize.move_step == 5.0 && s.quantize.angle == 15.0);
    // round trip
    let mut t = SnapSettings::default();
    t.apply_json(s.to_json().as_object().unwrap()).unwrap();
    assert_eq!(t, s);
}

#[test]
fn bad_settings_change_nothing() {
    let before = SnapSettings::default();
    for bad in [
        json!({"radius": 0}),
        json!({"radius": 101}),
        json!({"mode": "4d"}),
        json!({"enabled": 1}),
        json!({"grid_spacing": -1}),
        json!({"nope": 1}),
        json!({"targets": {"corner": true}}),
        json!({"targets": {"vertex": "yes"}}),
        json!({"workplane": {"normal": [0, 0, 0]}}),
        json!({"workplane": {"origin": [0, 0]}}),
        json!({"quantize": {"move": 0}}),
        json!({"quantize": {"angle": -5}}),
        json!({"quantize": {"step": 1}}),
        json!({"enabled": true, "radius": 0}),
    ] {
        let mut s = before.clone();
        assert!(s.apply_json(&obj(bad.clone())).is_err(), "{bad}");
        assert_eq!(s, before, "{bad}");
    }
}

#[test]
fn each_target_type_in_world_space() {
    let src = source();
    let hit = |kinds: &[SnapKind], p: DVec3, r: f64| snap_point(&src, p, r, &only(kinds));
    // vertex
    let h = hit(&[SnapKind::Vertex], DVec3::new(0.9, 1.1, -1.0), 0.5).unwrap();
    assert_eq!(
        (h.kind, h.index, h.source),
        (SnapKind::Vertex, Some(3), Some(0))
    );
    close(h.position, DVec3::new(1.0, 1.0, -1.0));
    // radius is a hard limit
    assert!(hit(&[SnapKind::Vertex], DVec3::new(0.0, 1.5, -1.0), 0.4).is_none());
    // edge: nearest point on the edge from (-1,-1,-1) to (1,-1,-1)
    let h = hit(&[SnapKind::Edge], DVec3::new(0.25, -1.2, -1.2), 1.0).unwrap();
    assert_eq!(h.kind, SnapKind::Edge);
    close(h.position, DVec3::new(0.25, -1.0, -1.0));
    // edge midpoint
    let h = hit(&[SnapKind::EdgeMidpoint], DVec3::new(0.1, -1.1, -1.1), 0.5).unwrap();
    assert_eq!(h.kind, SnapKind::EdgeMidpoint);
    close(h.position, DVec3::new(0.0, -1.0, -1.0));
    // polygon centre
    let h = hit(&[SnapKind::PolygonCenter], DVec3::new(0.1, 1.2, 0.2), 0.5).unwrap();
    assert_eq!((h.kind, h.index), (SnapKind::PolygonCenter, Some(1)));
    close(h.position, DVec3::new(0.0, 1.0, 0.0));
    // polygon surface
    let h = hit(&[SnapKind::Polygon], DVec3::new(0.3, 1.4, -0.2), 1.0).unwrap();
    assert_eq!((h.kind, h.index), (SnapKind::Polygon, Some(1)));
    close(h.position, DVec3::new(0.3, 1.0, -0.2));
    // object axis
    let h = hit(&[SnapKind::ObjectAxis], DVec3::new(0.1, 0.0, 0.1), 0.5).unwrap();
    assert_eq!((h.kind, h.index), (SnapKind::ObjectAxis, None));
    close(h.position, DVec3::ZERO);
}

#[test]
fn grid_and_workplane_targets() {
    let mut s = only(&[SnapKind::GridPoint]);
    s.grid_spacing = 0.5;
    let p = DVec3::new(0.62, 0.1, -0.95);
    let h = snap_point(&[], p, 1.0, &s).unwrap();
    assert_eq!(h.kind, SnapKind::GridPoint);
    close(h.position, DVec3::new(0.5, 0.0, -1.0));
    // grid line: the nearer coordinate is rounded, the other one kept
    let mut s = only(&[SnapKind::GridLine]);
    s.grid_spacing = 0.5;
    let h = snap_point(&[], p, 1.0, &s).unwrap();
    assert_eq!(h.kind, SnapKind::GridLine);
    close(h.position, DVec3::new(0.62, 0.0, -1.0));
    // workplane
    let s = only(&[SnapKind::Workplane]);
    let h = snap_point(&[], p, 1.0, &s).unwrap();
    assert_eq!(h.kind, SnapKind::Workplane);
    close(h.position, DVec3::new(0.62, 0.0, -0.95));
    // outside the radius
    assert!(snap_point(&[], DVec3::new(0.0, 3.0, 0.0), 1.0, &s).is_none());
    // a tilted, shifted workplane: normal +X through x = 4
    let mut s = only(&[SnapKind::Workplane, SnapKind::GridPoint]);
    s.workplane = Workplane {
        origin: DVec3::new(4.0, 0.0, 0.0),
        normal: DVec3::X,
    };
    s.grid_spacing = 1.0;
    let h = snap_point(&[], DVec3::new(3.7, 1.2, 2.4), 1.0, &s).unwrap();
    assert_eq!(h.kind, SnapKind::GridPoint, "grid point beats workplane");
    close(h.position, DVec3::new(4.0, 1.0, 2.0));
}

#[test]
fn priority_point_over_edge_over_polygon_over_grid_over_workplane() {
    let src = source();
    let all = [
        SnapKind::Vertex,
        SnapKind::Edge,
        SnapKind::Polygon,
        SnapKind::GridPoint,
        SnapKind::Workplane,
    ];
    let mut s = only(&all);
    s.grid_spacing = 1.0;
    // 0.45 from a vertex, much closer to an edge and a face: the vertex still wins
    let p = DVec3::new(1.45, 1.0, 1.0);
    assert_eq!(snap_point(&src, p, 1.0, &s).unwrap().kind, SnapKind::Vertex);
    // vertices out of reach: the edge beats the polygon
    let p = DVec3::new(0.0, 1.05, 1.05);
    assert_eq!(snap_point(&src, p, 0.2, &s).unwrap().kind, SnapKind::Edge);
    // off the edges: the polygon beats the grid and the workplane
    let p = DVec3::new(0.3, 1.1, -0.3);
    assert_eq!(
        snap_point(&src, p, 0.5, &s).unwrap().kind,
        SnapKind::Polygon
    );
    // away from the mesh: grid point beats workplane
    let p = DVec3::new(5.1, 0.1, 5.05);
    let h = snap_point(&src, p, 0.5, &s).unwrap();
    assert_eq!(h.kind, SnapKind::GridPoint);
    close(h.position, DVec3::new(5.0, 0.0, 5.0));
    // grid point disabled: the workplane
    s.targets.grid_point = false;
    assert_eq!(
        snap_point(&src, p, 0.5, &s).unwrap().kind,
        SnapKind::Workplane
    );
}

#[test]
fn equal_rank_prefers_the_nearer_then_the_kind_order() {
    let src = source();
    let s = only(&[
        SnapKind::Vertex,
        SnapKind::EdgeMidpoint,
        SnapKind::PolygonCenter,
    ]);
    // nearer midpoint wins over a farther vertex
    let p = DVec3::new(0.2, -1.0, -1.0);
    assert_eq!(
        snap_point(&src, p, 2.0, &s).unwrap().kind,
        SnapKind::EdgeMidpoint
    );
    // an exact tie (vertex and axis at the same place) goes to the kind order
    let mut src2 = source();
    src2[0].origin = Some(DVec3::splat(-1.0));
    let mut s2 = only(&[SnapKind::Vertex, SnapKind::ObjectAxis]);
    s2.radius = 10.0;
    let h = snap_point(&src2, DVec3::splat(-1.0), 0.1, &s2).unwrap();
    assert_eq!(h.kind, SnapKind::Vertex);
}

#[test]
fn skipped_points_are_not_targets() {
    let mut src = source();
    src[0].skip_points = vec![3];
    let s = only(&[
        SnapKind::Vertex,
        SnapKind::EdgeMidpoint,
        SnapKind::PolygonCenter,
        SnapKind::Edge,
        SnapKind::Polygon,
    ]);
    // point 3 (1,1,-1) is skipped; the next vertex within reach is not it
    let p = DVec3::new(1.0, 1.0, -1.0);
    let h = snap_point(&src, p, 2.05, &s).unwrap();
    assert_ne!(h.index, Some(3));
    // every edge, polygon and midpoint touching point 3 is skipped as well
    for kind in [
        SnapKind::Edge,
        SnapKind::EdgeMidpoint,
        SnapKind::PolygonCenter,
        SnapKind::Polygon,
    ] {
        let one = only(&[kind]);
        if let Some(h) = snap_point(&src, p, 0.01, &one) {
            panic!("{kind:?} touching a skipped point was a target: {h:?}");
        }
    }
}

#[test]
fn two_d_mode_projects_onto_the_workplane() {
    let src = source();
    let mut s = only(&[SnapKind::Vertex]);
    s.mode = SnapMode::D2;
    let h = snap_point(&src, DVec3::new(1.0, 0.7, 1.0), 0.5, &s).unwrap();
    assert_eq!(h.kind, SnapKind::Vertex);
    close(h.raw, DVec3::new(1.0, 1.0, 1.0));
    close(h.position, DVec3::new(1.0, 0.0, 1.0));
    s.workplane = Workplane {
        origin: DVec3::new(0.0, 3.0, 0.0),
        normal: DVec3::Y,
    };
    let h = snap_point(&src, DVec3::new(1.0, 0.7, 1.0), 0.5, &s).unwrap();
    close(h.position, DVec3::new(1.0, 3.0, 1.0));
    s.mode = SnapMode::D3;
    let h = snap_point(&src, DVec3::new(1.0, 0.7, 1.0), 0.5, &s).unwrap();
    assert_eq!(h.position, h.raw);
}

#[test]
fn disabled_targets_give_nothing_and_the_flag_is_not_consulted() {
    let src = source();
    let none = only(&[]);
    assert!(snap_point(&src, DVec3::ONE, 5.0, &none).is_none());
    let mut s = only(&[SnapKind::Vertex]);
    s.enabled = false;
    assert!(snap_point(&src, DVec3::ONE, 0.1, &s).is_some());
}

#[test]
fn quantize_steps_follow_the_setting_xor_ctrl() {
    let mut s = SnapSettings::default();
    assert!(s.quantize_steps(false).is_none());
    let st = s.quantize_steps(true).unwrap();
    assert_eq!(st.translate, 10.0);
    assert!((st.rotate - 15f64.to_radians()).abs() < 1e-15);
    assert!((st.scale - 0.1).abs() < 1e-15);
    s.apply_json(&obj(
        json!({"quantize": {"enabled": true, "move": 2.5, "angle": 45, "percent": 25}}),
    ))
    .unwrap();
    let st = s.quantize_steps(false).unwrap();
    assert_eq!(st.translate, 2.5);
    assert!((st.rotate - std::f64::consts::FRAC_PI_4).abs() < 1e-15);
    assert_eq!(st.scale, 0.25);
    assert!(s.quantize_steps(true).is_none(), "Ctrl toggles it off");
}

#[test]
fn workplane_basis_and_triangle_helpers() {
    let w = Workplane::default();
    assert_eq!(w.basis(), (DVec3::X, DVec3::Z));
    let w = Workplane {
        origin: DVec3::ZERO,
        normal: DVec3::Z,
    };
    let (u, v) = w.basis();
    close(u, DVec3::X);
    close(u.cross(w.normal), v);
    close(w.at(2.0, 3.0), 2.0 * u + 3.0 * v);
    let (a, b, c) = (DVec3::ZERO, DVec3::X, DVec3::Y);
    close(
        closest_on_triangle(DVec3::new(0.2, 0.2, 1.0), a, b, c),
        DVec3::new(0.2, 0.2, 0.0),
    );
    close(
        closest_on_triangle(DVec3::new(2.0, 2.0, 0.0), a, b, c),
        DVec3::new(0.5, 0.5, 0.0),
    );
    close(closest_on_triangle(DVec3::new(-1.0, -1.0, 0.0), a, b, c), a);
    close(
        closest_on_triangle(DVec3::new(0.5, -1.0, 0.0), a, b, c),
        DVec3::new(0.5, 0.0, 0.0),
    );
}

#[test]
fn normal_basis_is_right_handed_with_z_on_the_normal() {
    for (n, hint) in [
        (DVec3::Z, DVec3::X),
        (DVec3::new(1.0, 2.0, 3.0), DVec3::new(0.0, 1.0, 0.0)),
        (DVec3::Y, DVec3::Y),
        (DVec3::NEG_Y, DVec3::ZERO),
    ] {
        let b = normal_basis(n, hint);
        close(b.z_axis, n.normalize());
        close(b.x_axis.cross(b.y_axis), b.z_axis);
        assert!((b.determinant() - 1.0).abs() < 1e-12);
        assert!(b.x_axis.dot(b.z_axis).abs() < 1e-12);
    }
    // the hint sets X
    let b = normal_basis(DVec3::Z, DVec3::new(1.0, 1.0, 5.0));
    close(b.x_axis, DVec3::new(1.0, 1.0, 0.0).normalize());
}
