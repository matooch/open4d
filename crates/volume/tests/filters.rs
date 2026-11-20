//! Filters on distance fields (docs/specs/VOLUME.md section 8).

mod common;

use common::{cube, sphere};
use forge_volume::filters::{self, Band, FilterKind, SmoothOp};
use forge_volume::{SdfParams, SparseGrid, mesh_to_sdf};
use glam::{DVec3, IVec3};

fn band() -> Band {
    Band {
        exterior: 20.0,
        interior: 20.0,
    }
}

fn sphere_grid(r: f64) -> SparseGrid<f32> {
    mesh_to_sdf(&sphere(r, 48, 64, DVec3::ZERO), &SdfParams::default())
}

#[test]
fn the_gaussian_kernel_is_normalised_and_symmetric() {
    let k = filters::gaussian_kernel(3, 1.4);
    assert!((k.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    assert_eq!(k.len(), 7);
    for i in 0..3 {
        assert!((k[i] - k[6 - i]).abs() < 1e-15);
    }
    assert!(k[3] > k[2] && k[2] > k[1]);
}

#[test]
fn smoothing_keeps_a_linear_field_and_the_active_set() {
    let mut g = SparseGrid::new(100.0f32, 1.0, DVec3::ZERO);
    for x in -10..=10 {
        for y in -10..=10 {
            for z in -10..=10 {
                g.set(IVec3::new(x, y, z), x as f32);
            }
        }
    }
    for f in [
        filters::mean(&g, 2, 1),
        filters::gaussian(&g, 2, 1.0, 2),
        filters::median(&g, 1, 1),
    ] {
        assert_eq!(f.active_count(), g.active_count());
        // away from the border a linear field is its own average
        assert!((f.get(IVec3::new(2, 1, -3)) - 2.0).abs() < 1e-4);
    }
}

#[test]
fn the_median_removes_a_spike() {
    let mut g = SparseGrid::new(0.0f32, 1.0, DVec3::ZERO);
    for x in -5..=5 {
        for y in -5..=5 {
            for z in -5..=5 {
                g.set(IVec3::new(x, y, z), 1.0);
            }
        }
    }
    g.set(IVec3::ZERO, 50.0);
    assert_eq!(filters::median(&g, 1, 1).get(IVec3::ZERO), 1.0);
    assert!(filters::mean(&g, 1, 1).get(IVec3::ZERO) > 2.0);
}

#[test]
fn redistance_restores_a_distance_field() {
    let g = sphere_grid(100.0);
    // scale the values: the zero level set stays, the field is no longer a distance
    let bent = g.map(g.background() * 1.5, |v| v * 1.5);
    let fixed = filters::redistance(&bent, band());
    let mut worst: f64 = 0.0;
    for (i, v) in fixed.active_voxels() {
        let exact = g.index_to_world(i).length() - 100.0;
        if exact.abs() < 15.0 {
            worst = worst.max((f64::from(v) - exact).abs());
        }
    }
    assert!(worst < 2.0, "{worst}");
    assert!(fixed.active_count() > 500);
    assert_eq!(fixed.get(IVec3::ZERO), -20.0);
}

#[test]
fn offset_moves_the_surface() {
    let g = sphere_grid(100.0);
    let out = filters::offset(&g, 10.0, band());
    let at = |g: &SparseGrid<f32>| f64::from(g.get(IVec3::new(11, 0, 0)));
    // x = 110: outside the sphere by about 10, so about 0 after a 10 offset
    assert!(at(&out).abs() < 1.5, "{}", at(&out));
    assert!(at(&g) > 8.0);
}

#[test]
fn reshape_closes_a_notch_and_leaves_a_ball_alone() {
    let ball = sphere_grid(100.0);
    let closed = filters::reshape(&ball, 10.0, band());
    let opened = filters::reshape(&ball, -10.0, band());
    for r in [&closed, &opened] {
        let v =
            f64::from(r.get(IVec3::new(-10, 0, 0))) - f64::from(ball.get(IVec3::new(-10, 0, 0)));
        assert!(v.abs() < 1.5, "{v}");
    }
    // a cube opened by radius 10 loses its corner: the corner-adjacent sample turns outside
    let c = mesh_to_sdf(
        &cube(DVec3::splat(100.0), DVec3::ZERO),
        &SdfParams::default(),
    );
    let o = filters::reshape(&c, -10.0, band());
    assert!(c.get(IVec3::new(-9, -9, -8)) < 0.0);
    assert!(o.get(IVec3::new(-9, -9, -9)) > c.get(IVec3::new(-9, -9, -9)));
    // the radius is limited to one voxel below the band: a huge radius equals the limit
    assert_eq!(
        filters::reshape(&c, -500.0, band()).active_count(),
        filters::reshape(&c, -10.0, band()).active_count()
    );
}

#[test]
fn apply_runs_iterations_and_blends_by_strength() {
    let c = mesh_to_sdf(
        &cube(DVec3::splat(100.0), DVec3::ZERO),
        &SdfParams::default(),
    );
    let kind = |strength| FilterKind::Smooth {
        op: SmoothOp::Mean,
        width: 2,
        strength,
    };
    let none = filters::apply(&c, kind(0.0), 1, band());
    let one = filters::apply(&c, kind(1.0), 1, band());
    let many = filters::apply(&c, kind(1.0), 3, band());
    // strength 0 keeps the field (up to the re-distance of an exact distance field)
    assert!((none.get(IVec3::new(-9, -9, -9)) - c.get(IVec3::new(-9, -9, -9))).abs() < 1.0);
    assert!(one.get(IVec3::new(-9, -9, -9)) > c.get(IVec3::new(-9, -9, -9)) + 1.0);
    assert!(many.get(IVec3::new(-9, -9, -9)) > one.get(IVec3::new(-9, -9, -9)));
}
