//! Sparse grid (docs/specs/VOLUME.md section 1).

use forge_volume::{BLOCK, SparseGrid};
use glam::{DVec3, IVec3};

#[test]
fn unset_voxels_read_the_background() {
    let g = SparseGrid::new(7.0f32, 0.5, DVec3::ZERO);
    assert_eq!(g.get(IVec3::new(-100, 3, 99)), 7.0);
    assert!(!g.is_active(IVec3::ZERO));
    assert_eq!(g.active_count(), 0);
    assert_eq!(g.bounds(), None);
}

#[test]
fn set_get_across_negative_block_boundaries() {
    let mut g = SparseGrid::new(0.0f32, 1.0, DVec3::ZERO);
    let pts = [
        IVec3::new(-1, -1, -1),
        IVec3::new(-BLOCK, 0, BLOCK - 1),
        IVec3::new(BLOCK, -BLOCK - 1, 0),
        IVec3::new(0, 0, 0),
    ];
    for (n, p) in pts.iter().enumerate() {
        g.set(*p, n as f32 + 1.0);
    }
    for (n, p) in pts.iter().enumerate() {
        assert_eq!(g.get(*p), n as f32 + 1.0);
        assert!(g.is_active(*p));
    }
    assert_eq!(g.active_count(), 4);
    assert_eq!(g.block_count(), 4);
    assert_eq!(
        g.bounds(),
        Some((
            IVec3::new(-BLOCK, -BLOCK - 1, -1),
            IVec3::new(BLOCK, 0, BLOCK - 1)
        ))
    );
}

#[test]
fn active_iteration_is_sorted_and_set_inactive_keeps_the_value() {
    let mut g = SparseGrid::new(0.0f32, 1.0, DVec3::ZERO);
    g.set(IVec3::new(9, 0, 0), 1.0);
    g.set(IVec3::new(-3, 5, 2), 2.0);
    g.set(IVec3::new(-3, 5, 1), 3.0);
    g.set_inactive(IVec3::new(100, 0, 0), -4.0);
    let a: Vec<_> = g.active_voxels().into_iter().map(|(i, _)| i).collect();
    assert_eq!(
        a,
        vec![
            IVec3::new(-3, 5, 1),
            IVec3::new(-3, 5, 2),
            IVec3::new(9, 0, 0)
        ]
    );
    assert_eq!(g.get(IVec3::new(100, 0, 0)), -4.0);
    assert!(!g.is_active(IVec3::new(100, 0, 0)));
    g.clear(IVec3::new(9, 0, 0));
    assert_eq!(g.get(IVec3::new(9, 0, 0)), 0.0);
    assert_eq!(g.active_count(), 2);
}

#[test]
fn world_index_round_trip_and_trilinear() {
    let mut g = SparseGrid::new(0.0f32, 2.0, DVec3::new(1.0, 0.0, -1.0));
    assert_eq!(
        g.index_to_world(IVec3::new(1, 2, 3)),
        DVec3::new(3.0, 4.0, 5.0)
    );
    assert_eq!(
        g.world_to_index(DVec3::new(3.0, 4.0, 5.0)),
        DVec3::new(1.0, 2.0, 3.0)
    );
    // v(i, j, k) = i + 10 j + 100 k is linear, so trilinear reproduces it exactly
    for i in -2..4 {
        for j in -2..4 {
            for k in -2..4 {
                g.set(IVec3::new(i, j, k), (i + 10 * j + 100 * k) as f32);
            }
        }
    }
    let p = g.index_to_world(IVec3::ZERO) + DVec3::new(0.5, 1.0, 0.25) * 2.0;
    let want = 0.5 + 10.0 * 1.0 + 100.0 * 0.25;
    assert!((g.trilinear(p) - want as f32).abs() < 1e-4);
}

#[test]
fn flood_fill_sets_the_interior_between_negative_voxels() {
    let mut g = SparseGrid::new(2.0f32, 1.0, DVec3::ZERO);
    g.set(IVec3::new(0, 0, 0), -1.0);
    g.set(IVec3::new(5, 0, 0), -1.0);
    g.set(IVec3::new(0, 1, 0), 1.0);
    g.set(IVec3::new(9, 1, 0), 1.0); // positive voxels do not fill
    g.signed_flood_fill(2.0);
    for x in 1..5 {
        assert_eq!(g.get(IVec3::new(x, 0, 0)), -2.0);
        assert!(!g.is_active(IVec3::new(x, 0, 0)));
    }
    assert_eq!(g.get(IVec3::new(5, 1, 0)), 2.0);
    assert_eq!(g.get(IVec3::new(6, 0, 0)), 2.0);
}

#[test]
fn retain_and_map() {
    let mut g = SparseGrid::new(0.0f32, 1.0, DVec3::ZERO);
    for i in 0..10 {
        g.set(IVec3::new(i, 0, 0), i as f32);
    }
    g.retain_active(|_, v| v >= 5.0);
    assert_eq!(g.active_count(), 5);
    assert_eq!(g.get(IVec3::new(2, 0, 0)), 0.0);
    let m = g.map(1.0f32, |v| v * 2.0);
    assert_eq!(m.get(IVec3::new(7, 0, 0)), 14.0);
    assert!(m.is_active(IVec3::new(7, 0, 0)));
    assert_eq!(m.background(), 1.0);
}
