//! docs/specs/GEOMETRY.md §4. Grid edge ids: q0 (0,1)=0 (1,4)=1 (3,4)=2 (0,3)=3;
//! q1 (1,2)=4 (2,5)=5 (4,5)=6; q2 (4,7)=7 (6,7)=8 (3,6)=9; q3 (5,8)=10 (7,8)=11.

mod common;

use common::grid;
use forge_geometry::selection::*;

fn set(v: &[u32]) -> IndexSet {
    v.iter().copied().collect()
}

fn ids(s: &IndexSet) -> Vec<u32> {
    s.iter().collect()
}

#[test]
fn index_set_operations() {
    let mut a = set(&[5, 1, 3, 1]);
    assert_eq!(ids(&a), [1, 3, 5]);
    assert_eq!(a.len(), 3);
    assert!(a.contains(3) && !a.contains(2));
    a.insert(2);
    a.remove(5);
    assert_eq!(ids(&a), [1, 2, 3]);
    let b = set(&[3, 4]);
    assert_eq!(ids(&a.union(&b)), [1, 2, 3, 4]);
    assert_eq!(ids(&a.intersection(&b)), [3]);
    assert_eq!(ids(&a.difference(&b)), [1, 2]);
    assert_eq!(ids(&set(&[0, 2, 7]).invert(4)), [1, 3]);
    assert!(IndexSet::new().is_empty());
}

#[test]
fn conversions() {
    let m = grid();
    assert_eq!(ids(&polys_to_points(&m, &set(&[0, 99]))), [0, 1, 3, 4]);
    assert_eq!(ids(&polys_to_edges(&m, &set(&[0]))), [0, 1, 2, 3]);
    let quad0 = set(&[0, 1, 3, 4]);
    assert_eq!(ids(&points_to_polys(&m, &quad0, Mode::All)), [0]);
    assert_eq!(ids(&points_to_polys(&m, &quad0, Mode::Any)), [0, 1, 2, 3]);
    assert_eq!(ids(&points_to_edges(&m, &set(&[0, 1]), Mode::All)), [0]);
    assert_eq!(ids(&points_to_edges(&m, &set(&[0]), Mode::Any)), [0, 3]);
    assert_eq!(ids(&edges_to_points(&m, &set(&[0, 5, 50]))), [0, 1, 2, 5]);
}

#[test]
fn grow_and_shrink() {
    let m = grid();
    let grown = grow_points(&m, &set(&[4]));
    assert_eq!(ids(&grown), [1, 3, 4, 5, 7]);
    assert_eq!(ids(&shrink_points(&m, &grown)), [4]);
    let gp = grow_polys(&m, &set(&[0]));
    assert_eq!(ids(&gp), [0, 1, 2], "shares an edge, not just a vertex");
    assert_eq!(ids(&shrink_polys(&m, &gp)), [0]);
    let all = set(&[0, 1, 2, 3]);
    assert_eq!(ids(&shrink_polys(&m, &all)), [0, 1, 2, 3]);
}

#[test]
fn selection_bundle_defaults_empty() {
    let s = Selection::default();
    assert!(s.points.is_empty() && s.edges.is_empty() && s.polys.is_empty());
}
