//! docs/specs/GEOMETRY.md §5.

mod common;

use std::f64::consts::{FRAC_PI_2, PI};

use common::assert_vec_eq;
use forge_geometry::spline::{Spline, SplineError, SplineKind};
use glam::DVec3;

const KINDS: [SplineKind; 4] = [
    SplineKind::Linear,
    SplineKind::CatmullRom,
    SplineKind::BSpline,
    SplineKind::Akima,
];

fn pts(v: &[[f64; 3]]) -> Vec<DVec3> {
    v.iter().map(|p| DVec3::from_array(*p)).collect()
}

fn zigzag() -> Vec<DVec3> {
    pts(&[
        [0., 0., 0.],
        [1., 2., 0.],
        [2., -1., 1.],
        [3., 1., 0.],
        [4., 0., 2.],
    ])
}

/// Planar step data: x = index, y = [0,0,0,1,1,1].
fn step() -> Vec<DVec3> {
    [0., 0., 0., 1., 1., 1.]
        .iter()
        .enumerate()
        .map(|(i, y)| DVec3::new(i as f64, *y, 0.0))
        .collect()
}

#[test]
fn constructors_validate() {
    let one = pts(&[[0.; 3]]);
    assert_eq!(
        Spline::new(SplineKind::Linear, one.clone(), false).unwrap_err(),
        SplineError::TooFewPoints
    );
    assert_eq!(
        Spline::new(SplineKind::Bezier, zigzag(), false).unwrap_err(),
        SplineError::IsBezier
    );
    assert_eq!(
        Spline::bezier(zigzag(), vec![[DVec3::ZERO; 2]; 2], false).unwrap_err(),
        SplineError::HandleCount
    );
    let s = Spline::new(SplineKind::Akima, zigzag(), true).unwrap();
    assert_eq!(s.kind, SplineKind::Akima);
    assert!(s.closed);
}

#[test]
fn segment_counts() {
    for kind in KINDS {
        assert_eq!(
            Spline::new(kind, zigzag(), false).unwrap().segment_count(),
            4
        );
        assert_eq!(
            Spline::new(kind, zigzag(), true).unwrap().segment_count(),
            5
        );
    }
}

#[test]
fn interpolating_kinds_pass_through_points() {
    for kind in [
        SplineKind::Linear,
        SplineKind::CatmullRom,
        SplineKind::Akima,
    ] {
        for closed in [false, true] {
            let p = zigzag();
            let s = Spline::new(kind, p.clone(), closed).unwrap();
            for i in 0..s.segment_count() {
                assert_vec_eq(s.eval(i, 0.0), p[i]);
                assert_vec_eq(s.eval(i, 1.0), p[(i + 1) % p.len()]);
            }
        }
    }
}

#[test]
fn linear_is_lerp() {
    let s = Spline::new(SplineKind::Linear, zigzag(), false).unwrap();
    assert_vec_eq(s.eval(1, 0.25), DVec3::new(1.25, 1.25, 0.25));
    assert_vec_eq(s.derivative(1, 0.7), DVec3::new(1., -3., 1.));
}

#[test]
fn bezier_matches_bernstein_form() {
    let p = pts(&[[0., 0., 0.], [3., 0., 0.]]);
    let h = vec![
        [DVec3::ZERO, DVec3::new(1., 2., 0.)],
        [DVec3::new(-1., 2., 0.), DVec3::ZERO],
    ];
    let s = Spline::bezier(p, h, false).unwrap();
    // P0=(0,0) P1=(1,2) P2=(2,2) P3=(3,0): B(0.5) = (P0 + 3P1 + 3P2 + P3) / 8
    assert_vec_eq(s.eval(0, 0.5), DVec3::new(1.5, 1.5, 0.));
    assert_vec_eq(s.derivative(0, 0.0), DVec3::new(3., 6., 0.));

    let flat = Spline::bezier(zigzag(), vec![[DVec3::ZERO; 2]; 5], false).unwrap();
    let lin = Spline::new(SplineKind::Linear, zigzag(), false).unwrap();
    for i in 0..4 {
        for t in [0.0, 0.3, 1.0] {
            // zero handles: same path as linear (different speed along it)
            let b = flat.eval(i, t);
            let a = lin.eval(i, 0.0);
            let c = lin.eval(i, 1.0);
            assert!(((b - a).cross(c - a)).length() < 1e-9);
        }
    }
}

#[test]
fn catmull_rom_tangent_is_half_the_chord() {
    let p = zigzag();
    let s = Spline::new(SplineKind::CatmullRom, p.clone(), false).unwrap();
    assert_vec_eq(s.derivative(2, 0.0), (p[3] - p[1]) / 2.0);
    assert_vec_eq(s.derivative(1, 1.0), (p[3] - p[1]) / 2.0);
    // open end uses the phantom point 2p0 - p1, so the end tangent is the first chord
    assert_vec_eq(s.derivative(0, 0.0), p[1] - p[0]);
}

#[test]
fn bspline_uses_uniform_cubic_basis_and_clamped_ends() {
    let p = zigzag();
    let s = Spline::new(SplineKind::BSpline, p.clone(), false).unwrap();
    assert_vec_eq(s.eval(2, 0.0), (p[1] + 4.0 * p[2] + p[3]) / 6.0);
    assert_vec_eq(s.eval(0, 0.0), p[0]);
    assert_vec_eq(s.eval(3, 1.0), p[4]);
    let closed = Spline::new(SplineKind::BSpline, p.clone(), true).unwrap();
    assert_vec_eq(closed.eval(0, 0.0), (p[4] + 4.0 * p[0] + p[1]) / 6.0);
}

#[test]
fn closed_curves_are_continuous_at_every_join() {
    for kind in KINDS {
        let s = Spline::new(kind, zigzag(), true).unwrap();
        let n = s.segment_count();
        for i in 0..n {
            let j = (i + 1) % n;
            assert_vec_eq(s.eval(i, 1.0), s.eval(j, 0.0));
            if kind != SplineKind::Linear {
                assert_vec_eq(s.derivative(i, 1.0), s.derivative(j, 0.0));
            }
        }
    }
}

#[test]
fn derivative_matches_finite_difference() {
    let h = 1e-6;
    for kind in KINDS {
        let s = Spline::new(kind, zigzag(), false).unwrap();
        let fd = (s.eval(1, 0.4 + h) - s.eval(1, 0.4 - h)) / (2.0 * h);
        assert!((fd - s.derivative(1, 0.4)).length() < 1e-5, "{kind:?}");
    }
}

#[test]
fn akima_does_not_overshoot_flat_data_but_catmull_rom_does() {
    let max_y = |kind| {
        let s = Spline::new(kind, step(), false).unwrap();
        s.tessellate(50)
            .iter()
            .map(|p| p.y)
            .fold(f64::MIN, f64::max)
    };
    assert!(max_y(SplineKind::Akima) <= 1.0 + 1e-12);
    assert!(max_y(SplineKind::CatmullRom) > 1.0 + 1e-3);
}

#[test]
fn akima_reproduces_straight_lines_and_degrades_to_linear() {
    let line = pts(&[[0., 0., 0.], [1., 1., 0.], [2., 2., 0.], [3., 3., 0.]]);
    let s = Spline::new(SplineKind::Akima, line, false).unwrap();
    assert_vec_eq(s.eval(1, 0.3), DVec3::new(1.3, 1.3, 0.));
    let two = Spline::new(SplineKind::Akima, pts(&[[0.; 3], [2., 0., 0.]]), false).unwrap();
    assert_vec_eq(two.eval(0, 0.25), DVec3::new(0.5, 0., 0.));
}

#[test]
fn tessellate_counts_and_endpoints() {
    let open = Spline::new(SplineKind::CatmullRom, zigzag(), false).unwrap();
    let t = open.tessellate(4);
    assert_eq!(t.len(), 4 * 4 + 1);
    assert_vec_eq(t[0], zigzag()[0]);
    assert_vec_eq(*t.last().unwrap(), zigzag()[4]);
    let closed = Spline::new(SplineKind::CatmullRom, zigzag(), true).unwrap();
    assert_eq!(closed.tessellate(4).len(), 5 * 4);
}

/// Quarter circle of radius r as one Bézier segment (k = 4/3 (√2 − 1)).
fn quarter_circle(r: f64) -> Spline {
    let k = 4.0 / 3.0 * (2f64.sqrt() - 1.0) * r;
    Spline::bezier(
        pts(&[[r, 0., 0.], [0., r, 0.]]),
        vec![
            [DVec3::ZERO, DVec3::new(0., k, 0.)],
            [DVec3::new(k, 0., 0.), DVec3::ZERO],
        ],
        false,
    )
    .unwrap()
}

#[test]
fn arc_length() {
    let line = Spline::new(
        SplineKind::Linear,
        pts(&[[0.; 3], [3., 4., 0.], [3., 4., 2.]]),
        false,
    )
    .unwrap();
    assert!((line.length() - 7.0).abs() < 1e-12);
    let q = quarter_circle(2.0);
    // the cubic approximation is within 0.03% of the true radius
    assert!(
        (q.length() - FRAC_PI_2 * 2.0).abs() < 1e-3,
        "{}",
        q.length()
    );
    let sq = Spline::new(
        SplineKind::Linear,
        pts(&[[0.; 3], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]]),
        true,
    )
    .unwrap();
    assert!((sq.length() - 4.0).abs() < 1e-12);
    let _ = PI;
}

#[test]
fn sample_by_length_is_evenly_spaced() {
    let s = Spline::new(SplineKind::CatmullRom, zigzag(), false).unwrap();
    let samples = s.sample_by_length(21);
    assert_eq!(samples.len(), 21);
    assert_vec_eq(samples[0], zigzag()[0]);
    assert_vec_eq(samples[20], zigzag()[4]);
    let step = s.length() / 20.0;
    // Chords undershoot arcs at tight bends, so measure arc position on a fine polyline.
    let fine = s.tessellate(2000);
    let mut cum = vec![0.0];
    for w in fine.windows(2) {
        cum.push(cum.last().unwrap() + (w[1] - w[0]).length());
    }
    let arc_pos = |p: DVec3| {
        let (i, _) = fine
            .iter()
            .enumerate()
            .map(|(i, q)| (i, (*q - p).length()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        cum[i]
    };
    let positions: Vec<f64> = samples.iter().map(|p| arc_pos(*p)).collect();
    for w in positions.windows(2) {
        assert!(
            (w[1] - w[0] - step).abs() < step * 0.01,
            "{} vs {step}",
            w[1] - w[0]
        );
    }

    // On a gentle arc, chord ≈ arc, so spacing must be tight.
    let q = quarter_circle(2.0);
    let arc = q.sample_by_length(21);
    let chord = 2.0 * 2.0 * (FRAC_PI_2 / 20.0 / 2.0).sin();
    for w in arc.windows(2) {
        let d = (w[1] - w[0]).length();
        assert!((d - chord).abs() < 1e-3, "{d} vs {chord}");
    }

    let sq = Spline::new(
        SplineKind::Linear,
        pts(&[[0.; 3], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]]),
        true,
    )
    .unwrap();
    let ring = sq.sample_by_length(8);
    assert_eq!(ring.len(), 8);
    assert_vec_eq(ring[1], DVec3::new(0.5, 0., 0.));
    assert_vec_eq(ring[7], DVec3::new(0., 0.5, 0.));
}
