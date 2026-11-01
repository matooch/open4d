//! Behaviour the golden cases cannot see (docs/specs/MODELING.md §4).

use std::collections::HashMap;

use forge_geometry::PolyMesh;
use forge_geometry::mesh::poly_edges;
use forge_geometry::selection::IndexSet;
use forge_modeling::{Settings, ToolError, extrude, extrude_inner, tool_desc, tool_descs};
use glam::DVec3;
use serde_json::json;

/// A cube with 8 points; polygon 0 is the top (+y), wound counter-clockwise from above.
fn cube() -> PolyMesh {
    let p = |x: f64, y: f64, z: f64| DVec3::new(x, y, z);
    PolyMesh {
        points: vec![
            p(-1.0, 1.0, -1.0),
            p(-1.0, 1.0, 1.0),
            p(1.0, 1.0, 1.0),
            p(1.0, 1.0, -1.0),
            p(-1.0, -1.0, -1.0),
            p(-1.0, -1.0, 1.0),
            p(1.0, -1.0, 1.0),
            p(1.0, -1.0, -1.0),
        ],
        polys: vec![
            vec![0, 1, 2, 3],
            vec![4, 7, 6, 5],
            vec![0, 3, 7, 4],
            vec![3, 2, 6, 7],
            vec![2, 1, 5, 6],
            vec![1, 0, 4, 5],
        ],
    }
}

fn set(items: &[u32]) -> IndexSet {
    items.iter().copied().collect()
}

fn settings(tool: &str, v: serde_json::Value) -> Settings {
    Settings::from_json(&tool_desc(tool).unwrap(), v.as_object().unwrap()).unwrap()
}

fn edge_uses(m: &PolyMesh) -> HashMap<(u32, u32), u32> {
    let mut uses = HashMap::new();
    for poly in &m.polys {
        for e in poly_edges(poly) {
            *uses.entry((e.0, e.1)).or_default() += 1;
        }
    }
    uses
}

fn closed(m: &PolyMesh) -> bool {
    edge_uses(m).values().all(|n| *n == 2)
}

#[test]
fn defaults_are_the_observed_ones() {
    let d = tool_desc("extrude").unwrap();
    let s = Settings::defaults(&d);
    assert_eq!(s.float("offset"), 5.0);
    assert_eq!(s.float("maximum_angle_deg"), 91.0);
    assert_eq!(s.float("bevel"), 5.0);
    assert!(s.bool("preserve_groups") && s.bool("quantize_rotation") && s.bool("realtime_update"));
    assert!(!s.bool("create_caps") && !s.bool("create_ngons"));
    let i = Settings::defaults(&tool_desc("extrude_inner").unwrap());
    assert_eq!(i.float("maximum_angle_deg"), 89.0);
    assert_eq!(tool_desc("inset").unwrap().id, "extrude_inner");
    assert_eq!(tool_descs().len(), 3);
}

#[test]
fn settings_are_validated_and_clamped() {
    let d = tool_desc("extrude").unwrap();
    let s = settings(
        "extrude",
        json!({"maximum_angle_deg": 500.0, "subdivision": 2.0}),
    );
    assert_eq!(s.float("maximum_angle_deg"), 180.0);
    assert_eq!(s.int("subdivision"), 2);
    for bad in [
        json!({"nope": 1}),
        json!({"offset": "far"}),
        json!({"subdivision": 1.5}),
        json!({"preserve_groups": 1}),
    ] {
        assert!(
            Settings::from_json(&d, bad.as_object().unwrap()).is_err(),
            "{bad}"
        );
    }
    // round trip
    let back = Settings::from_json(&d, &s.to_json()).unwrap();
    assert_eq!(back, s);
}

#[test]
fn unsupported_options_and_bad_selections_fail() {
    let m = cube();
    for (tool, v) in [
        ("extrude", json!({"offset_variation": 1.0})),
        ("extrude", json!({"rotation_deg": 10.0})),
        ("extrude", json!({"create_caps": true})),
        ("extrude_inner", json!({"offset_variation": 1.0})),
    ] {
        let s = settings(tool, v);
        let r = if tool == "extrude" {
            extrude(&m, &set(&[0]), &s)
        } else {
            extrude_inner(&m, &set(&[0]), &s)
        };
        assert!(matches!(r, Err(ToolError::Unsupported(_))), "{tool}");
    }
    let s = settings("extrude", json!({}));
    assert_eq!(
        extrude(&m, &set(&[]), &s).unwrap_err(),
        ToolError::EmptySelection
    );
    assert_eq!(
        extrude(&m, &set(&[9]), &s).unwrap_err(),
        ToolError::BadPolygon(9)
    );
}

#[test]
fn a_closed_mesh_stays_closed() {
    let m = cube();
    assert!(closed(&m));
    for sub in [0, 1, 3] {
        for off in [2.0_f64, -0.5] {
            let s = settings("extrude", json!({"offset": off, "subdivision": sub}));
            let (out, sel) = extrude(&m, &set(&[0, 3]), &s).unwrap();
            assert!(closed(&out), "extrude sub {sub} offset {off}");
            assert_eq!(sel.polys, set(&[0, 3]));
            out.validate().unwrap();
            let s = settings(
                "extrude_inner",
                json!({"offset": off.abs() / 4.0, "subdivision": sub}),
            );
            let (out, _) = extrude_inner(&m, &set(&[0, 3]), &s).unwrap();
            assert!(closed(&out), "inset sub {sub}");
            out.validate().unwrap();
        }
    }
}

#[test]
fn subdivision_adds_rows_and_columns() {
    let m = cube();
    let s = settings("extrude", json!({"subdivision": 3}));
    let (out, _) = extrude(&m, &set(&[0]), &s).unwrap();
    // 4 cap points + 4 columns of 3 points; 6 original polygons + 4 sides of 4 rows
    assert_eq!(out.points.len(), 8 + 4 + 12);
    assert_eq!(out.polys.len(), 6 + 16);
}

#[test]
fn preserve_groups_splits_at_the_maximum_angle() {
    // top (0) and the +z side (4) meet at 90 degrees
    let m = cube();
    let one = settings("extrude", json!({"maximum_angle_deg": 91.0}));
    let (a, _) = extrude(&m, &set(&[0, 4]), &one).unwrap();
    let split = settings("extrude", json!({"maximum_angle_deg": 45.0}));
    let (b, _) = extrude(&m, &set(&[0, 4]), &split).unwrap();
    // one group: 6 distinct points, 6 boundary edges; two groups: 4 + 4 points, 4 + 4 sides
    assert_eq!(a.points.len(), 8 + 6);
    assert_eq!(a.polys.len(), 6 + 6);
    assert_eq!(b.points.len(), 8 + 8);
    assert_eq!(b.polys.len(), 6 + 8);
    // without Preserve Groups the angle does not matter
    let off = settings("extrude", json!({"preserve_groups": false}));
    let (c, _) = extrude(&m, &set(&[0, 4]), &off).unwrap();
    assert_eq!(c.points.len(), b.points.len());
}

#[test]
fn a_group_around_a_bend_moves_along_the_mean_normal() {
    let m = cube();
    let s = settings("extrude", json!({"offset": 1.0, "maximum_angle_deg": 91.0}));
    let (out, _) = extrude(&m, &set(&[0, 4]), &s).unwrap();
    // the shared point 1 (-1, 1, 1) is used by both polygons: normals +y and +z
    let new1 = out.points.iter().skip(8).find(|p| {
        (**p - DVec3::new(-1.0, 1.0, 1.0)).length() > 1e-9
            && (**p - DVec3::new(-1.0, 1.0, 1.0) - DVec3::new(0.0, 1.0, 1.0).normalize()).length()
                < 1e-9
    });
    assert!(new1.is_some());
}

#[test]
fn inset_of_a_rectangle_keeps_the_offset_to_every_edge() {
    let m = PolyMesh {
        points: vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 4.0),
            DVec3::new(10.0, 0.0, 4.0),
            DVec3::new(10.0, 0.0, 0.0),
        ],
        polys: vec![vec![0, 1, 2, 3]],
    };
    // winding gives +y; the inside is 10 x 4
    let s = settings("extrude_inner", json!({"offset": 1.0}));
    let (out, _) = extrude_inner(&m, &set(&[0]), &s).unwrap();
    let inner: Vec<DVec3> = out.points[4..].to_vec();
    let want = [
        DVec3::new(1.0, 0.0, 1.0),
        DVec3::new(1.0, 0.0, 3.0),
        DVec3::new(9.0, 0.0, 3.0),
        DVec3::new(9.0, 0.0, 1.0),
    ];
    for (g, w) in inner.iter().zip(want) {
        assert!((*g - w).length() < 1e-9, "{g} != {w}");
    }
    // outward with a negative offset
    let s = settings("extrude_inner", json!({"offset": -1.0}));
    let (out, _) = extrude_inner(&m, &set(&[0]), &s).unwrap();
    assert!((out.points[4] - DVec3::new(-1.0, 0.0, -1.0)).length() < 1e-9);
}

#[test]
fn inset_leaves_interior_group_points_in_place() {
    // a 2 x 2 patch: the centre point is not on the group's boundary
    let mut pts = Vec::new();
    for z in 0..3 {
        for x in 0..3 {
            pts.push(DVec3::new(f64::from(x), 0.0, f64::from(z)));
        }
    }
    let m = PolyMesh {
        points: pts,
        polys: vec![
            vec![0, 3, 4, 1],
            vec![1, 4, 5, 2],
            vec![3, 6, 7, 4],
            vec![4, 7, 8, 5],
        ],
    };
    let s = settings("extrude_inner", json!({"offset": 0.25}));
    let (out, _) = extrude_inner(&m, &set(&[0, 1, 2, 3]), &s).unwrap();
    // 8 boundary points get copies; the centre (4) is shared by the caps
    assert_eq!(out.points.len(), 9 + 8);
    assert!(out.polys[..4].iter().all(|p| p.contains(&4)));
    assert_eq!(out.polys.len(), 4 + 8);
}
