//! Golden-free tests for forge-noise (docs/specs/NOISE.md N6).

use forge_noise::*;
use glam::DVec3;

/// Deterministic point cloud (LCG), `n` points in `[0, extent)^3`.
fn points(n: usize, extent: f64) -> Vec<DVec3> {
    let mut s = 12345u64;
    let mut r = move || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
        .map(|_| DVec3::new(r() * extent, r() * extent, r() * extent))
        .collect()
}

type Case = (&'static str, Box<dyn Fn(DVec3) -> f64>, f64, f64);

fn stats(v: &[f64]) -> (f64, f64) {
    let m = v.iter().sum::<f64>() / v.len() as f64;
    let var = v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64;
    (m, var.sqrt())
}

fn noise(kind: NoiseKind, seed: u32) -> Noise {
    Noise {
        kind,
        seed,
        ..Noise::default()
    }
}

#[test]
fn determinism_same_seed_same_values() {
    let ps = points(500, 40.0);
    for kind in NoiseKind::ALL {
        let a = noise(kind, 9);
        let b = noise(kind, 9);
        for &p in &ps {
            assert_eq!(a.sample3(p).to_bits(), b.sample3(p).to_bits(), "{kind:?}");
            assert_eq!(
                a.sample4(p, 0.7).to_bits(),
                b.sample4(p, 0.7).to_bits(),
                "{kind:?}"
            );
        }
    }
}

#[test]
fn pinned_values() {
    // Regression pins of this implementation (not of any reference app).
    let p = DVec3::new(1.37, -2.61, 3.19);
    let pins = PINS;
    for (kind, want) in pins {
        let got = noise(kind, 42).sample3(p);
        assert!((got - want).abs() < 1e-12, "{kind:?}: {got:.15} vs {want}");
    }
}

#[test]
fn range_is_signed_unit() {
    let ps = points(20_000, 60.0);
    for kind in NoiseKind::ALL {
        let n = noise(kind, 3);
        for &p in &ps {
            for v in [n.sample3(p), n.sample4(p, 2.5)] {
                assert!((-1.0..=1.0).contains(&v), "{kind:?} {v}");
            }
            let u = n.unit3(p);
            assert!((0.0..=1.0).contains(&u));
        }
    }
}

#[test]
fn base_noises_stay_in_range_before_clamp() {
    let ps = points(100_000, 80.0);
    for &p in &ps {
        let a = [p.x, p.y, p.z];
        let b = [p.x, p.y, p.z, p.x * 0.37];
        for v in [
            perlin::<3>(5, a),
            perlin::<4>(5, b),
            value::<3>(5, a),
            value::<4>(5, b),
            simplex::<3>(5, a),
            simplex::<4>(5, b),
        ] {
            assert!(v.abs() <= 1.0, "{v}");
        }
    }
}

#[test]
fn continuity_of_smooth_kinds() {
    let ps = points(2_000, 30.0);
    let e = 1e-4;
    for kind in NoiseKind::ALL {
        if !kind.is_continuous() {
            continue;
        }
        let n = noise(kind, 11);
        let mut worst: f64 = 0.0;
        for &p in &ps {
            for d in [DVec3::X, DVec3::Y, DVec3::Z] {
                worst = worst.max((n.sample3(p + d * e) - n.sample3(p)).abs() / e);
            }
            worst = worst.max((n.sample4(p, 1.0 + e) - n.sample4(p, 1.0)).abs() / e);
        }
        // Slope bound: octaves multiply frequency by up to 2^3 and the wavy/warp kinds add more.
        assert!(worst < 200.0, "{kind:?} slope {worst}");
    }
}

#[test]
fn box_and_cell_are_piecewise_constant() {
    let p = DVec3::new(3.2, 4.4, 5.6);
    for kind in [NoiseKind::Box, NoiseKind::Cell] {
        let n = noise(kind, 1);
        assert_eq!(n.sample3(p), n.sample3(p + DVec3::splat(0.001)));
    }
    let n = noise(NoiseKind::Box, 1);
    assert_ne!(n.sample3(p), n.sample3(p + DVec3::X * 3.0));
}

#[test]
fn moments_of_base_noises() {
    let ps = points(200_000, 100.0);
    let cases: [Case; 6] = [
        (
            "perlin3",
            Box::new(|p| perlin::<3>(1, [p.x, p.y, p.z])),
            0.2,
            0.35,
        ),
        (
            "value3",
            Box::new(|p| value::<3>(1, [p.x, p.y, p.z])),
            0.3,
            0.5,
        ),
        (
            "simplex3",
            Box::new(|p| simplex::<3>(1, [p.x, p.y, p.z])),
            0.3,
            0.5,
        ),
        (
            "perlin4",
            Box::new(|p| perlin::<4>(1, [p.x, p.y, p.z, p.z * 0.5])),
            0.2,
            0.35,
        ),
        (
            "simplex4",
            Box::new(|p| simplex::<4>(1, [p.x, p.y, p.z, p.z * 0.5])),
            0.15,
            0.35,
        ),
        (
            "value4",
            Box::new(|p| value::<4>(1, [p.x, p.y, p.z, p.z * 0.5])),
            0.25,
            0.45,
        ),
    ];
    for (name, f, lo, hi) in cases {
        let v: Vec<f64> = ps.iter().map(|&p| f(p)).collect();
        let (m, sd) = stats(&v);
        assert!(m.abs() < 0.01, "{name} mean {m}");
        assert!((lo..hi).contains(&sd), "{name} std {sd}");
    }
}

#[test]
fn moments_of_composite_kinds() {
    let ps = points(100_000, 100.0);
    // (kind, mean band, std band): turbulence-like kinds are biased low by construction.
    let bands = [
        (NoiseKind::Fbm, -0.01, 0.01, 0.10, 0.25),
        (NoiseKind::Gaseous, -0.01, 0.01, 0.10, 0.25),
        (NoiseKind::Turbulence, -0.7, -0.4, 0.1, 0.3),
        (NoiseKind::RidgedMultifractal, 0.0, 0.4, 0.2, 0.45),
        (NoiseKind::Voronoi1, -0.1, 0.15, 0.25, 0.45),
        (NoiseKind::Voronoi2, -0.1, 0.1, 0.1, 0.3),
        (NoiseKind::Cell, -0.02, 0.02, 0.5, 0.65),
    ];
    for (kind, mlo, mhi, slo, shi) in bands {
        let n = noise(kind, 5);
        let v: Vec<f64> = ps.iter().map(|&p| n.sample3(p)).collect();
        let (m, sd) = stats(&v);
        assert!((mlo..mhi).contains(&m), "{kind:?} mean {m}");
        assert!((slo..shi).contains(&sd), "{kind:?} std {sd}");
    }
}

#[test]
fn seed_independence() {
    let ps = points(50_000, 100.0);
    for kind in [
        NoiseKind::Perlin,
        NoiseKind::Simplex,
        NoiseKind::Value,
        NoiseKind::Fbm,
    ] {
        let a: Vec<f64> = ps.iter().map(|&p| noise(kind, 1).sample3(p)).collect();
        let b: Vec<f64> = ps.iter().map(|&p| noise(kind, 2).sample3(p)).collect();
        let c: Vec<f64> = ps.iter().map(|&p| noise(kind, 1).sample3(p)).collect();
        assert_eq!(a, c);
        let (ma, sa) = stats(&a);
        let (mb, sb) = stats(&b);
        let cov = a
            .iter()
            .zip(&b)
            .map(|(x, y)| (x - ma) * (y - mb))
            .sum::<f64>()
            / a.len() as f64;
        assert!(
            (cov / (sa * sb)).abs() < 0.05,
            "{kind:?} corr {}",
            cov / (sa * sb)
        );
    }
}

#[test]
fn scale_stretches_features() {
    let p = DVec3::new(2.5, 1.5, 0.5);
    let a = Noise {
        scale: 1.0,
        ..noise(NoiseKind::Perlin, 4)
    };
    let b = Noise {
        scale: 10.0,
        ..noise(NoiseKind::Perlin, 4)
    };
    assert_eq!(a.sample3(p), b.sample3(p * 10.0));
}

#[test]
fn time_axis() {
    let n = Noise {
        time: 0.25,
        ..noise(NoiseKind::Perlin, 8)
    };
    let p = DVec3::new(0.3, 0.6, 0.9);
    assert_eq!(n.sample(p), n.sample4(p, 0.25));
    assert_ne!(n.sample4(p, 0.25), n.sample4(p, 1.25));
    assert_ne!(n.sample3(p), n.sample4(p, 0.25));
}

#[test]
fn lattice_properties() {
    // Gradient noise is zero at lattice points.
    for c in [[0.0, 0.0, 0.0], [3.0, -2.0, 7.0]] {
        assert_eq!(perlin::<3>(1, c), 0.0);
    }
    // Worley: f1 <= f2, ids in range, f1 is 0 at... distances bounded by sqrt(N).
    for p in points(5_000, 50.0) {
        let w = worley::<3>(2, [p.x, p.y, p.z]);
        assert!(w.f1 <= w.f2 && w.f1 >= 0.0 && w.f1 < 3f64.sqrt());
        assert!((-1.0..=1.0).contains(&w.id));
    }
}

#[test]
fn domain_warp_moves_points_boundedly() {
    let q = domain_warp(1, [1.0, 2.0, 3.0, 0.0], 0.5);
    assert_eq!(q, domain_warp(1, [1.0, 2.0, 3.0, 0.0], 0.5));
    for i in 0..3 {
        assert!((q[i] - [1.0, 2.0, 3.0][i]).abs() <= 0.5);
    }
    assert_eq!(q[3], 0.0);
    assert_eq!(
        domain_warp(1, [1.0, 2.0, 3.0, 0.0], 0.0),
        [1.0, 2.0, 3.0, 0.0]
    );
}

#[test]
fn octaves_add_detail_but_one_octave_equals_base() {
    let p = DVec3::new(0.37, 1.71, 2.9);
    let one = Noise {
        octaves: 1,
        ..noise(NoiseKind::Fbm, 6)
    };
    let four = noise(NoiseKind::Fbm, 6);
    assert_ne!(one.sample3(p), four.sample3(p));
    // Single octave is exactly the base noise with the octave-0 seed.
    let want = perlin::<3>(octave_seed_for_test(6, 0), [p.x, p.y, p.z]);
    assert_eq!(one.sample3(p), want);
}

fn octave_seed_for_test(seed: u32, i: u32) -> u32 {
    forge_noise::octave_seed(seed, i)
}
const PINS: [(NoiseKind, f64); 17] = [
    (NoiseKind::Perlin, -0.3683852364664),
    (NoiseKind::Value, -0.2732905619609488),
    (NoiseKind::Simplex, -0.3262021993161481),
    (NoiseKind::Box, -0.9056874132329884),
    (NoiseKind::Cell, -0.9353644349895804),
    (NoiseKind::Voronoi1, -0.016489573114657263),
    (NoiseKind::Voronoi2, 0.15255717104289168),
    (NoiseKind::Voronoi3, -0.3370001825383546),
    (NoiseKind::Fbm, -0.03726769980142457),
    (NoiseKind::Turbulence, -0.7475280220223852),
    (NoiseKind::RidgedMultifractal, 0.5478356647284632),
    (NoiseKind::WavyTurbulence, -0.5612208074866163),
    (NoiseKind::Electric, 0.6139031219943649),
    (NoiseKind::Gaseous, -0.03669510252169586),
    (NoiseKind::BlisteredTurbulence, -0.5752875895287101),
    (NoiseKind::DisplacedTurbulence, -0.6129540648009777),
    (NoiseKind::DisplacedVoronoi, 0.3080742587726675),
];
