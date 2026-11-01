//! Bevel behaviour the golden cases cannot see (docs/specs/MODELING.md §3, §5).

use std::collections::HashMap;

use forge_geometry::mesh::poly_edges;
use forge_geometry::selection::{IndexSet, Selection};
use forge_geometry::{Edge, PolyMesh};
use forge_modeling::{Settings, ToolError, bevel, tool_desc, tool_fn};
use glam::DVec3;
use serde_json::json;

/// A box `x × y × z` (full sizes) in the point and polygon order of the reference cube
/// (points 0..8 as in the golden cases, polygon 4 on top).
fn block(x: f64, y: f64, z: f64) -> PolyMesh {
    let (x, y, z) = (x / 2.0, y / 2.0, z / 2.0);
    let p = |a: f64, b: f64, c: f64| DVec3::new(a * x, b * y, c * z);
    PolyMesh {
        points: vec![
            p(-1.0, -1.0, -1.0),
            p(-1.0, 1.0, -1.0),
            p(1.0, -1.0, -1.0),
            p(1.0, 1.0, -1.0),
            p(1.0, -1.0, 1.0),
            p(1.0, 1.0, 1.0),
            p(-1.0, -1.0, 1.0),
            p(-1.0, 1.0, 1.0),
        ],
        polys: vec![
            vec![0, 1, 3, 2],
            vec![2, 3, 5, 4],
            vec![4, 5, 7, 6],
            vec![6, 7, 1, 0],
            vec![1, 7, 5, 3],
            vec![6, 0, 2, 4],
        ],
    }
}

fn settings(v: serde_json::Value) -> Settings {
    Settings::from_json(&tool_desc("bevel").unwrap(), v.as_object().unwrap()).unwrap()
}

fn edges(mesh: &PolyMesh, pairs: &[(u32, u32)]) -> Selection {
    let ids = mesh.edge_ids();
    Selection {
        edges: pairs
            .iter()
            .map(|(a, b)| ids[&Edge::new(*a, *b)])
            .collect::<IndexSet>(),
        ..Selection::default()
    }
}

fn polys(items: &[u32]) -> Selection {
    Selection {
        polys: items.iter().copied().collect(),
        ..Selection::default()
    }
}

fn closed(m: &PolyMesh) -> bool {
    let mut uses: HashMap<(u32, u32), u32> = HashMap::new();
    for poly in &m.polys {
        for e in poly_edges(poly) {
            *uses.entry((e.0, e.1)).or_default() += 1;
        }
    }
    uses.values().all(|n| *n == 2)
}

fn volume(m: &PolyMesh) -> f64 {
    // divergence theorem over a fan triangulation
    let mut v = 0.0;
    for poly in &m.polys {
        let a = m.points[poly[0] as usize];
        for w in poly[1..].windows(2) {
            let (b, c) = (m.points[w[0] as usize], m.points[w[1] as usize]);
            v += a.dot(b.cross(c)) / 6.0;
        }
    }
    v
}

const TOP: [(u32, u32); 4] = [(1, 7), (1, 3), (3, 5), (5, 7)];

#[test]
fn defaults_are_the_observed_ones() {
    let s = Settings::defaults(&tool_desc("bevel").unwrap());
    assert_eq!(s.choice("bevel_mode"), "chamfer");
    assert_eq!(s.choice("offset_mode"), "fixed_distance");
    assert_eq!(s.float("offset"), 10.0);
    assert_eq!(s.int("subdivision"), 0);
    assert_eq!(s.float("depth"), 1.0);
    assert!(s.bool("limit") && s.bool("realtime_update"));
    assert_eq!(s.choice("shape"), "round");
    assert_eq!(s.float("tension"), 1.0);
    assert_eq!(s.choice("mitering"), "default");
    assert_eq!(s.float("extrusion"), 10.0);
    assert!(
        Settings::from_json(
            &tool_desc("bevel").unwrap(),
            json!({"bevel_mode": "sharp"}).as_object().unwrap()
        )
        .is_err()
    );
    assert!(tool_fn("bevel").is_some());
}

#[test]
fn beveled_meshes_stay_closed() {
    let m = block(200.0, 200.0, 200.0);
    for (sel, v) in [
        (edges(&m, &[(1, 3)]), json!({"offset": 20.0})),
        (
            edges(&m, &[(1, 3)]),
            json!({"offset": 20.0, "subdivision": 3}),
        ),
        (edges(&m, &TOP), json!({"offset": 20.0, "subdivision": 4})),
        (
            edges(&m, &[(1, 3)]),
            json!({"offset": 20.0, "bevel_mode": "solid"}),
        ),
        (polys(&[4]), json!({"offset": 20.0})),
    ] {
        let (out, picked) = bevel(&m, &sel, &settings(v.clone())).unwrap();
        assert!(closed(&out), "{v}");
        assert!(!picked.polys.is_empty());
        out.validate().unwrap();
    }
}

#[test]
fn a_chamfer_removes_the_expected_volume() {
    // a 90 degree edge of a box: the rounded corner removes (1 - pi/4) d^2 per unit length
    let m = block(200.0, 200.0, 200.0);
    let d = 20.0;
    let s = settings(json!({"offset": d, "subdivision": 31}));
    let (out, _) = bevel(&m, &edges(&m, &[(1, 3)]), &s).unwrap();
    let removed = volume(&m) - volume(&out);
    let ideal = (1.0 - std::f64::consts::FRAC_PI_4) * d * d * 200.0;
    assert!(
        (removed - ideal).abs() / ideal < 0.01,
        "{removed} vs {ideal}"
    );
    // the arc points of the x = 100 end lie on the circle of radius d around (y, z) = (80, -80)
    let arc: Vec<_> = out
        .points
        .iter()
        .filter(|p| (p.x - 100.0).abs() < 1e-9 && p.y >= 80.0 && p.z <= -80.0)
        .collect();
    assert_eq!(arc.len(), 33);
    for p in arc {
        let r = ((p.y - 80.0).powi(2) + (p.z + 80.0).powi(2)).sqrt();
        assert!((r - d).abs() < 1e-9, "{p:?}");
    }
}

#[test]
fn the_edge_order_and_sides_follow_the_half_edge_index() {
    // the same four edges selected in any order give the same mesh
    let m = block(200.0, 200.0, 200.0);
    let s = settings(json!({"offset": 20.0, "subdivision": 1}));
    let a = bevel(&m, &edges(&m, &TOP), &s).unwrap().0;
    let mut rev = TOP;
    rev.reverse();
    let b = bevel(&m, &edges(&m, &rev), &s).unwrap().0;
    assert_eq!(a, b);
    assert_eq!(a.points.len(), 16);
    assert_eq!(a.polys.len(), 14);
}

#[test]
fn proportional_is_a_fraction_of_the_edge_length() {
    let m = block(400.0, 200.0, 100.0);
    // the edge (1,3) is 400 long: 0.25 gives a distance of 100 on the top and 100 down
    let s = settings(json!({"offset": 0.25, "offset_mode": "proportional"}));
    let (out, _) = bevel(&m, &edges(&m, &[(1, 3)]), &s).unwrap();
    let fixed = settings(json!({"offset": 100.0}));
    let (same, _) = bevel(&m, &edges(&m, &[(1, 3)]), &fixed).unwrap();
    assert_eq!(out, same);
}

#[test]
fn polygon_mode_lifts_and_insets() {
    let m = block(200.0, 200.0, 200.0);
    let (out, picked) = bevel(
        &m,
        &polys(&[4]),
        &settings(json!({"offset": 30.0, "extrusion": 25.0})),
    )
    .unwrap();
    assert_eq!(picked.polys.iter().collect::<Vec<_>>(), [4]);
    let cap = &out.polys[4];
    for v in cap {
        let p = out.points[*v as usize];
        assert!(
            (p.y - 125.0).abs() < 1e-9
                && (p.x.abs() - 70.0).abs() < 1e-9
                && (p.z.abs() - 70.0).abs() < 1e-9
        );
    }
    assert_eq!(out.polys.len(), 10);
}

#[test]
fn unsupported_options_and_bad_selections_fail() {
    let m = block(200.0, 200.0, 200.0);
    let one = edges(&m, &[(1, 3)]);
    for (sel, v) in [
        (one.clone(), json!({"shape": "user"})),
        (one.clone(), json!({"shape": "profile"})),
        (one.clone(), json!({"offset_mode": "radial"})),
        (one.clone(), json!({"tension": 2.0})),
        (one.clone(), json!({"depth": 0.5})),
        (one.clone(), json!({"mitering": "radial"})),
        (one.clone(), json!({"mitering": "patch"})),
        (one.clone(), json!({"ending": "extend"})),
        (one.clone(), json!({"partial_rounding": "full"})),
        (one.clone(), json!({"corner_ngons": true})),
        (one.clone(), json!({"rounding_ngons": true})),
        (
            one.clone(),
            json!({"bevel_mode": "solid", "subdivision": 1}),
        ),
        (edges(&m, &TOP), json!({"bevel_mode": "solid"})),
        (polys(&[4]), json!({"subdivision": 1})),
        (polys(&[4]), json!({"bevel_mode": "solid"})),
        (polys(&[4]), json!({"offset_mode": "proportional"})),
        (
            Selection {
                points: [1u32].into_iter().collect(),
                ..Selection::default()
            },
            json!({}),
        ),
    ] {
        let r = bevel(&m, &sel, &settings(v.clone()));
        assert!(matches!(r, Err(ToolError::Unsupported(_))), "{v}: {r:?}");
    }
    let s = settings(json!({}));
    assert_eq!(
        bevel(&m, &Selection::default(), &s).unwrap_err(),
        ToolError::EmptySelection
    );
    let far = Selection {
        edges: [99u32].into_iter().collect(),
        ..Selection::default()
    };
    assert_eq!(bevel(&m, &far, &s).unwrap_err(), ToolError::BadEdge(99));
    assert_eq!(
        bevel(&m, &polys(&[9]), &s).unwrap_err(),
        ToolError::BadPolygon(9)
    );
    // all three edges at the corner point 1
    let corner = edges(&m, &[(1, 3), (1, 7), (0, 1)]);
    assert!(matches!(
        bevel(&m, &corner, &s),
        Err(ToolError::Unsupported(_))
    ));
    // an open mesh: drop the bottom
    let mut open = m.clone();
    open.polys.pop();
    assert!(matches!(
        bevel(&open, &edges(&m, &[(0, 1)]), &s),
        Err(ToolError::Unsupported(_))
    ));
}

#[test]
fn a_vertex_with_more_than_three_polygons_is_refused() {
    // a pyramid apex has four polygons
    let m = PolyMesh {
        points: vec![
            DVec3::new(-1.0, 0.0, -1.0),
            DVec3::new(1.0, 0.0, -1.0),
            DVec3::new(1.0, 0.0, 1.0),
            DVec3::new(-1.0, 0.0, 1.0),
            DVec3::new(0.0, 1.0, 0.0),
        ],
        polys: vec![
            vec![0, 3, 2, 1],
            vec![0, 1, 4],
            vec![1, 2, 4],
            vec![2, 3, 4],
            vec![3, 0, 4],
        ],
    };
    let r = bevel(&m, &edges(&m, &[(0, 4)]), &settings(json!({"offset": 0.1})));
    assert!(matches!(r, Err(ToolError::Unsupported(_))), "{r:?}");
}

#[test]
fn a_non_right_angle_edge_keeps_its_profile_on_the_faces() {
    // a wedge with a 45 degree roof: the chamfer end points stay on both faces
    let m = PolyMesh {
        points: vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(10.0, 0.0, 0.0),
            DVec3::new(10.0, 0.0, 10.0),
            DVec3::new(0.0, 0.0, 10.0),
            DVec3::new(0.0, 5.0, 5.0),
            DVec3::new(10.0, 5.0, 5.0),
        ],
        polys: vec![
            vec![0, 3, 2, 1],
            vec![0, 1, 5, 4],
            vec![3, 4, 5, 2],
            vec![0, 4, 3],
            vec![1, 2, 5],
        ],
    };
    let (out, _) = bevel(
        &m,
        &edges(&m, &[(4, 5)]),
        &settings(json!({"offset": 1.0, "subdivision": 2})),
    )
    .unwrap();
    assert!(closed(&out));
    out.validate().unwrap();
    assert_eq!(out.polys.len(), m.polys.len() + 3);
}
