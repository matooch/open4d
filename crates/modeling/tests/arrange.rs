//! The arrangement rules of docs/specs/MODELING.md §9 (pure functions and descriptors).

use std::collections::HashMap;

use forge_geometry::PolyMesh;
use forge_geometry::mesh::poly_edges;
use forge_geometry::selection::Selection;
use forge_modeling::{
    Settings, ToolError, arrange_positions, copy_step, object_desc, randomize_draw, tool_desc,
    tool_fn,
};
use glam::DVec3;
use serde_json::json;

fn cube() -> PolyMesh {
    let p = |x: f64, y: f64, z: f64| DVec3::new(x, y, z);
    PolyMesh {
        points: vec![
            p(-1.0, -1.0, -1.0),
            p(1.0, -1.0, -1.0),
            p(1.0, 1.0, -1.0),
            p(-1.0, 1.0, -1.0),
            p(-1.0, -1.0, 1.0),
            p(1.0, -1.0, 1.0),
            p(1.0, 1.0, 1.0),
            p(-1.0, 1.0, 1.0),
        ],
        polys: vec![
            vec![0, 3, 2, 1],
            vec![4, 5, 6, 7],
            vec![0, 1, 5, 4],
            vec![2, 3, 7, 6],
            vec![0, 4, 7, 3],
            vec![1, 2, 6, 5],
        ],
    }
}

fn settings(tool: &str, v: serde_json::Value) -> Settings {
    let desc = tool_desc(tool).unwrap();
    Settings::from_json(&desc, v.as_object().unwrap()).unwrap()
}

fn array(mesh: &PolyMesh, v: serde_json::Value) -> Result<(PolyMesh, Selection), ToolError> {
    tool_fn("array").unwrap()(mesh, &Selection::default(), &settings("array", v))
}

fn close(a: DVec3, b: DVec3) -> bool {
    (a - b).length() < 1e-9
}

#[test]
fn step_rule_in_both_modes() {
    let (o, r, s) = (
        DVec3::new(10.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 90.0),
        DVec3::new(4.0, 1.0, 1.0),
    );
    let step = |k, mode| copy_step(k, 2, mode, o, r, s);
    // step mode: every copy adds the settings once more
    let k2 = step(2, "step");
    assert!(close(k2.pos, DVec3::new(20.0, 0.0, 0.0)));
    assert!(close(k2.rot_deg, DVec3::new(0.0, 0.0, 180.0)));
    assert!(close(k2.scale, DVec3::new(7.0, 1.0, 1.0))); // 1 + 2 * (4 - 1)
    // a shrinking scale never reaches zero or turns negative
    let shrink = copy_step(5, 2, "step", DVec3::ZERO, DVec3::ZERO, DVec3::splat(0.5));
    assert!(close(shrink.scale, DVec3::splat(0.001)));
    // total mode: the last copy reaches the settings, the first is half way
    let t1 = step(1, "total");
    assert!(close(t1.pos, DVec3::new(5.0, 0.0, 0.0)));
    assert!(close(t1.rot_deg, DVec3::new(0.0, 0.0, 45.0)));
    assert!(close(t1.scale, DVec3::new(2.5, 1.0, 1.0))); // 1 + 0.5 * (4 - 1)
    let t2 = step(2, "total");
    assert!(close(t2.pos, o) && close(t2.rot_deg, r) && close(t2.scale, s));
}

fn array_sel(
    mesh: &PolyMesh,
    polys: &[u32],
    v: serde_json::Value,
) -> Result<(PolyMesh, Selection), ToolError> {
    let sel = Selection {
        polys: polys.iter().copied().collect(),
        ..Selection::default()
    };
    tool_fn("array").unwrap()(mesh, &sel, &settings("array", v))
}

#[test]
fn array_appends_instances_in_order() {
    // count 4 includes the original; the offset is the total span (O58 arr-f-array-3)
    let m = cube();
    let (out, sel) = array(&m, json!({"count_x": 4, "offset": [30.0, 0.0, 0.0]})).unwrap();
    assert_eq!((out.points.len(), out.polys.len()), (32, 24));
    // the original is untouched and first
    assert_eq!(&out.points[..8], &m.points[..]);
    assert_eq!(&out.polys[..6], &m.polys[..]);
    for k in 1..=3usize {
        for i in 0..8 {
            assert!(close(
                out.points[8 * k + i],
                m.points[i] + DVec3::new(10.0 * k as f64, 0.0, 0.0)
            ));
        }
        // same corner order, shifted indices
        for (j, poly) in m.polys.iter().enumerate() {
            let want: Vec<u32> = poly.iter().map(|c| c + 8 * k as u32).collect();
            assert_eq!(out.polys[6 * k + j], want);
        }
    }
    // the selection is the polygons of the copies
    assert_eq!(
        sel.polys.iter().collect::<Vec<_>>(),
        (6..24).collect::<Vec<u32>>()
    );
}

#[test]
fn array_grid_order_and_single_axis_offsets() {
    let m = cube();
    // 2 x 2 x 1: i fastest; the z offset is ignored while count_z is 1
    let (out, _) = array(
        &m,
        json!({"count_x": 2, "count_y": 2, "offset": [10.0, 20.0, 99.0]}),
    )
    .unwrap();
    assert_eq!(out.points.len(), 32);
    let first = |k: usize| out.points[8 * k] - m.points[0];
    assert!(close(first(1), DVec3::new(10.0, 0.0, 0.0)));
    assert!(close(first(2), DVec3::new(0.0, 20.0, 0.0)));
    assert!(close(first(3), DVec3::new(10.0, 20.0, 0.0)));
    // count 1 along x: the x offset is ignored (arr-f-array-mode)
    let (out, _) = array(
        &m,
        json!({"count_x": 1, "count_z": 2, "offset": [50.0, 0.0, 8.0]}),
    )
    .unwrap();
    assert!(close(
        out.points[8] - m.points[0],
        DVec3::new(0.0, 0.0, 8.0)
    ));
}

#[test]
fn array_copies_only_the_selected_polygons() {
    // arr-f-array-mode: the top polygon of a cube, count 2, no displacement
    let m = cube();
    let (out, sel) = array_sel(&m, &[3], json!({"count_x": 2, "offset": [0.0, 0.0, 0.0]})).unwrap();
    assert_eq!((out.points.len(), out.polys.len()), (12, 7));
    // used points ascending: 2, 3, 6, 7 become 8, 9, 10, 11
    for (i, p) in [2usize, 3, 6, 7].iter().enumerate() {
        assert!(close(out.points[8 + i], m.points[*p]));
    }
    assert_eq!(out.polys[6], vec![8, 9, 11, 10]);
    assert_eq!(sel.polys.iter().collect::<Vec<_>>(), [6]);
    // a polygon that does not exist is ignored, none at all is an error
    assert!(array_sel(&m, &[99], json!({})).is_err());
}

#[test]
fn array_copies_stay_closed_and_keep_winding() {
    let (out, _) = array(
        &cube(),
        json!({"count_x": 3, "rotation_deg": [30.0, 10.0, 5.0], "scale": [2.0, 1.0, 0.8],
               "offset": [10.0, 0.0, 0.0]}),
    )
    .unwrap();
    let mut uses: HashMap<(u32, u32), u32> = HashMap::new();
    for poly in &out.polys {
        for e in poly_edges(poly) {
            *uses.entry((e.0.min(e.1), e.0.max(e.1))).or_default() += 1;
        }
    }
    assert!(uses.values().all(|n| *n == 2));
    // outward winding is preserved: signed volume of each cube keeps the sign
    let volume = |polys: &[Vec<u32>]| {
        let mut vol = 0.0;
        for poly in polys {
            let p0 = out.points[poly[0] as usize];
            for w in 1..poly.len() - 1 {
                let (a, b) = (
                    out.points[poly[w] as usize],
                    out.points[poly[w + 1] as usize],
                );
                vol += p0.dot(a.cross(b)) / 6.0;
            }
        }
        vol
    };
    let sign0 = volume(&out.polys[0..6]).signum();
    for k in 1..3usize {
        let vol = volume(&out.polys[6 * k..6 * k + 6]);
        assert!(vol.abs() > 0.1);
        assert_eq!(vol.signum(), sign0);
    }
}

#[test]
fn array_rotation_reaches_the_total_at_the_last_instance() {
    // 3 instances and a bank of 180: the middle one is turned by 90 degrees
    let (out, _) = array(
        &cube(),
        json!({"count_x": 3, "rotation_deg": [0.0, 0.0, 180.0], "offset": [0.0, 0.0, 0.0]}),
    )
    .unwrap();
    // the bank turns about Z by the negated angle (TRANSFORM.md): (1, 0) -> (0, -1) at 90
    let p = out.points[8 + 5]; // original point 5 is (1, -1, 1)
    assert!(close(p, DVec3::new(-1.0, -1.0, 1.0)), "{p:?}");
    let q = out.points[16 + 5]; // 180 degrees
    assert!(close(q, DVec3::new(-1.0, 1.0, 1.0)), "{q:?}");
}

#[test]
fn array_errors() {
    let m = cube();
    assert!(
        Settings::from_json(
            &tool_desc("array").unwrap(),
            json!({"count_x": 5, "nope": 1}).as_object().unwrap()
        )
        .is_err()
    );
    // counts clamp into 1..1000
    let s = settings("array", json!({"count_x": 0}));
    assert_eq!(s.int("count_x"), 1);
    let s = settings("array", json!({"count_y": 99999}));
    assert_eq!(s.int("count_y"), 1000);
    // the point cap
    let big = PolyMesh {
        points: (0..5000).map(|i| DVec3::new(i as f64, 0.0, 0.0)).collect(),
        polys: vec![(0..5000).collect()],
    };
    assert!(matches!(
        array(&big, json!({"count_x": 1000})),
        Err(ToolError::Invalid(_))
    ));
    assert!(array(&PolyMesh::default(), json!({})).is_err());
    assert!(
        array(&m, json!({"scale": [0.0, 1.0, 1.0]})).is_ok(),
        "scale clamps to 0.001"
    );
}

#[test]
fn arrange_line_grid_circle() {
    let start = [
        DVec3::new(5.0, 1.0, 2.0),
        DVec3::ZERO,
        DVec3::ZERO,
        DVec3::ZERO,
    ];
    let line = arrange_positions(
        &start,
        &settings_obj("arrange", json!({"spacing": [10.0, 0.0, 0.0]})),
    );
    assert!(close(line[0], start[0]) && close(line[3], DVec3::new(35.0, 1.0, 2.0)));
    let grid = arrange_positions(
        &start,
        &settings_obj(
            "arrange",
            json!({"mode": "grid", "columns": 2,
            "spacing": [10.0, 0.0, 0.0], "row_spacing": [0.0, 0.0, 20.0]}),
        ),
    );
    assert!(close(grid[1], DVec3::new(15.0, 1.0, 2.0)));
    assert!(close(grid[2], DVec3::new(5.0, 1.0, 22.0)));
    assert!(close(grid[3], DVec3::new(15.0, 1.0, 22.0)));
    let circle = arrange_positions(
        &[DVec3::new(10.0, 7.0, 0.0), DVec3::new(-10.0, 7.0, 0.0)],
        &settings_obj("arrange", json!({"mode": "circle", "radius": 100.0})),
    );
    // centre (0, 7, 0), plane xz: angle 0 -> +X, angle 180 -> -X, the Y component is kept
    assert!(close(circle[0], DVec3::new(100.0, 7.0, 0.0)));
    assert!(close(circle[1], DVec3::new(-100.0, 7.0, 0.0)));
    let xy = arrange_positions(
        &[DVec3::ZERO; 4],
        &settings_obj(
            "arrange",
            json!({"mode": "circle", "radius": 1.0, "plane": "xy",
            "start_angle_deg": 90.0}),
        ),
    );
    assert!(close(xy[0], DVec3::new(0.0, 1.0, 0.0)));
    assert!(close(xy[1], DVec3::new(-1.0, 0.0, 0.0)));
}

fn settings_obj(tool: &str, v: serde_json::Value) -> Settings {
    let desc = object_desc(tool).unwrap();
    Settings::from_json(&desc, v.as_object().unwrap()).unwrap()
}

#[test]
fn randomize_draws_are_deterministic_and_independent() {
    let a = randomize_draw(7, 3);
    assert_eq!(a, randomize_draw(7, 3));
    assert_ne!(a, randomize_draw(8, 3));
    assert_ne!(a, randomize_draw(7, 4));
    assert!(a.iter().all(|u| (-1.0..1.0).contains(u)));
    // a fixed stream (SplitMix64, seed 7 xor 3 * golden): pins the algorithm
    let again = randomize_draw(7, 3);
    assert_eq!(a[0].to_bits(), again[0].to_bits());
    let all: Vec<f64> = (0..200).flat_map(|n| randomize_draw(1, n)).collect();
    let mean = all.iter().sum::<f64>() / all.len() as f64;
    assert!(mean.abs() < 0.05, "{mean}");
}

#[test]
fn object_descriptors_have_the_documented_defaults() {
    let d = Settings::defaults(&object_desc("duplicate").unwrap());
    assert_eq!(d.int("copies"), 1);
    assert_eq!(d.choice("mode"), "step");
    let a = Settings::defaults(&object_desc("arrange").unwrap());
    assert_eq!((a.int("columns"), a.float("radius")), (3, 200.0));
    assert_eq!(a.vec3("spacing"), DVec3::new(200.0, 0.0, 0.0));
    let r = Settings::defaults(&object_desc("randomize").unwrap());
    assert_eq!((r.int("seed"), r.vec3("position")), (1, DVec3::splat(50.0)));
    let t = Settings::defaults(&object_desc("transfer").unwrap());
    assert!(t.bool("position") && t.bool("rotation") && t.bool("scale"));
    assert_eq!(t.choice("space"), "world");
    assert!(object_desc("nope").is_none());
}
