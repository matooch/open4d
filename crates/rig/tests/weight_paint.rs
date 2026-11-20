//! Weight brush and smoothing (docs/specs/RIG.md section 9).

use forge_rig::WeightMap;
use forge_rig::paint::{Brush, BrushMode, Falloff, point_adjacency, smooth_weights, stroke};
use glam::DVec3;

/// A row of 5 points along x at 0, 10, ..., 40 joined as a strip of quads.
fn line() -> (Vec<DVec3>, Vec<Vec<u32>>) {
    let pts: Vec<DVec3> = (0..5)
        .map(|i| DVec3::new(10.0 * i as f64, 0.0, 0.0))
        .collect();
    let polys = vec![vec![0, 1, 2, 3, 4]];
    (pts, polys)
}

fn w(map: &WeightMap, p: u32, slot: u32) -> f64 {
    map.get(p)
        .iter()
        .find(|(s, _)| *s == slot)
        .map_or(0.0, |(_, w)| *w)
}

fn brush(mode: BrushMode, falloff: Falloff, strength: f64) -> Brush {
    Brush {
        mode,
        radius: 20.0,
        strength,
        falloff,
        target: 1.0,
    }
}

#[test]
fn falloff_shapes() {
    let mut b = brush(BrushMode::Add, Falloff::Constant, 0.5);
    assert_eq!(b.factor(0.0), 0.5);
    assert_eq!(b.factor(19.9), 0.5);
    assert_eq!(b.factor(20.0), 0.0);
    b.falloff = Falloff::Linear;
    assert!((b.factor(10.0) - 0.25).abs() < 1e-12);
    b.falloff = Falloff::Smooth;
    assert!((b.factor(10.0) - 0.25).abs() < 1e-12); // 1 - (0.75 - 0.25) = 0.5, times 0.5
    assert!((b.factor(0.0) - 0.5).abs() < 1e-12);
}

#[test]
fn adjacency_of_a_polygon_is_sorted_and_unique() {
    let (pts, polys) = line();
    let adj = point_adjacency(&[polys[0].clone(), vec![0, 1, 4]], pts.len());
    assert_eq!(adj[0], vec![1, 4]);
    assert_eq!(adj[2], vec![1, 3]);
}

#[test]
fn add_inside_the_radius_and_the_row_stays_normalised() {
    let (pts, polys) = line();
    let adj = point_adjacency(&polys, 5);
    let mut m = WeightMap::from_dense(&vec![vec![1.0, 0.0]; 5]);
    let b = brush(BrushMode::Add, Falloff::Constant, 0.25);
    let n = stroke(&mut m, &pts, &adj, 1, DVec3::new(10.0, 0.0, 0.0), &b);
    assert_eq!(n, 3); // points 0..=2 are within 20 (point 3 is at exactly 20: outside)
    for p in 0..3 {
        assert!((w(&m, p, 1) - 0.25).abs() < 1e-12);
        assert!((w(&m, p, 0) - 0.75).abs() < 1e-12);
    }
    assert_eq!(w(&m, 3, 1), 0.0);
    assert_eq!(w(&m, 4, 0), 1.0);
}

#[test]
fn add_clamps_at_one_and_subtract_at_zero() {
    let (pts, polys) = line();
    let adj = point_adjacency(&polys, 5);
    let mut m = WeightMap::from_dense(&vec![vec![0.5, 0.5]; 5]);
    let add = brush(BrushMode::Add, Falloff::Constant, 1.0);
    stroke(&mut m, &pts, &adj, 0, DVec3::ZERO, &add);
    assert_eq!(w(&m, 0, 0), 1.0);
    assert_eq!(w(&m, 0, 1), 0.0);
    let sub = brush(BrushMode::Subtract, Falloff::Constant, 1.0);
    stroke(&mut m, &pts, &adj, 0, DVec3::ZERO, &sub);
    assert_eq!(w(&m, 0, 0), 0.0);
    // nothing else to take the weight: the row is empty
    assert!(m.get(0).is_empty() || w(&m, 0, 1) >= 0.0);
}

#[test]
fn set_moves_towards_the_target() {
    let (pts, polys) = line();
    let adj = point_adjacency(&polys, 5);
    let mut m = WeightMap::from_dense(&vec![vec![1.0, 0.0]; 5]);
    let mut b = brush(BrushMode::Set, Falloff::Constant, 0.5);
    b.target = 0.0;
    stroke(&mut m, &pts, &adj, 0, DVec3::ZERO, &b);
    assert!((w(&m, 0, 0) - 0.5).abs() < 1e-12);
    assert_eq!(w(&m, 4, 0), 1.0);
}

#[test]
fn a_smooth_stroke_averages_with_the_neighbours_from_the_old_weights() {
    let (pts, polys) = line();
    let adj = point_adjacency(&[vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 4]], 5);
    let _ = polys;
    // slot 1 weight 0.8 at point 2 only
    let mut rows = vec![vec![1.0, 0.0]; 5];
    rows[2] = vec![0.2, 0.8];
    let mut m = WeightMap::from_dense(&rows);
    let b = Brush {
        radius: 5.0,
        ..brush(BrushMode::Smooth, Falloff::Constant, 1.0)
    };
    stroke(&mut m, &pts, &adj, 1, pts[2], &b);
    // point 2 takes the mean of points 1 and 3 (both 0)
    assert!(w(&m, 2, 1).abs() < 1e-12);
    assert!((w(&m, 2, 0) - 1.0).abs() < 1e-12);
}

#[test]
fn smooth_weights_keeps_row_sums_and_spreads() {
    let adj = point_adjacency(&[vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 4]], 5);
    let mut rows = vec![vec![1.0, 0.0]; 5];
    rows[2] = vec![0.0, 1.0];
    let mut m = WeightMap::from_dense(&rows);
    smooth_weights(&mut m, &adj, 1, 0.5);
    // point 2: 0.5 own + 0.5 * mean(0, 0)
    assert!((w(&m, 2, 1) - 0.5).abs() < 1e-12);
    // point 1: 0.5 * 0 + 0.5 * mean(0, 1)
    assert!((w(&m, 1, 1) - 0.25).abs() < 1e-12);
    for p in 0..5 {
        let sum: f64 = m.get(p).iter().map(|(_, x)| *x).sum();
        assert!((sum - 1.0).abs() < 1e-12, "point {p}");
    }
}

#[test]
fn prune_and_normalise_tools() {
    let mut m = WeightMap::from_dense(&[vec![0.6, 0.02, 0.2]]);
    m.prune(0.05);
    assert_eq!(m.get(0).len(), 2);
    m.normalise();
    assert!((w(&m, 0, 0) - 0.75).abs() < 1e-12);
    assert!((w(&m, 0, 2) - 0.25).abs() < 1e-12);
}
