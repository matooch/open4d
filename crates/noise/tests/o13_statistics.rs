//! Statistical plausibility check against the O13 / O23 field samples (docs/specs/NOISE.md N7).
//!
//! Not a parity test: the reference feature size, octaves and seed hashing are unknown, so only
//! range and loose moment relations are asserted. The statistics are printed with `--nocapture`.

use forge_noise::{Noise, NoiseKind};
use glam::DVec3;
use std::path::PathBuf;

struct Grid {
    pts: Vec<DVec3>,
    vals: Vec<f64>,
}

fn load(case: &str) -> Grid {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "../../tests/golden/fields",
        case,
        "samples.json",
    ]
    .iter()
    .collect();
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut pts = vec![];
    let mut vals = vec![];
    for s in v["samples"].as_array().unwrap() {
        let p = s[0].as_array().unwrap();
        pts.push(DVec3::new(
            p[0].as_f64().unwrap(),
            p[1].as_f64().unwrap(),
            p[2].as_f64().unwrap(),
        ));
        vals.push(s[1].as_f64().unwrap());
    }
    Grid { pts, vals }
}

#[derive(Debug)]
struct Stats {
    mean: f64,
    std: f64,
    min: f64,
    max: f64,
    /// Correlation of neighbours one grid step apart along x (x is the fastest index).
    lag1: f64,
}

fn stats(v: &[f64]) -> Stats {
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let mut cov = 0.0;
    let mut cnt = 0.0;
    for row in v.chunks(9) {
        for w in row.windows(2) {
            cov += (w[0] - mean) * (w[1] - mean);
            cnt += 1.0;
        }
    }
    Stats {
        mean,
        std: var.sqrt(),
        min: v.iter().cloned().fold(f64::MAX, f64::min),
        max: v.iter().cloned().fold(f64::MIN, f64::max),
        lag1: cov / cnt / var,
    }
}

fn ours(grid: &Grid, scale: f64, seed: u32) -> Stats {
    let n = Noise {
        kind: NoiseKind::Perlin,
        seed,
        scale,
        ..Noise::default()
    };
    let v: Vec<f64> = grid.pts.iter().map(|&p| n.unit3(p)).collect();
    stats(&v)
}

#[test]
fn random_field_perlin_is_statistically_plausible() {
    for (case, lo, hi) in [
        ("random-defaults", 0.10056, 0.95476),
        ("random-seed", 0.10959, 0.88282),
    ] {
        let g = load(case);
        assert_eq!(g.vals.len(), 729);
        let r = stats(&g.vals);
        println!("{case} reference: {r:?}");
        assert!(
            (r.min - lo).abs() < 1e-4 && (r.max - hi).abs() < 1e-4,
            "{r:?}"
        );
        // The reference is centred near 0.5 and strictly inside (0, 1) like our unit mapping.
        assert!((r.mean - 0.5).abs() < 0.1, "{r:?}");
        // Neighbours 75 units apart are uncorrelated in the reference (feature size well under 75).
        assert!(r.lag1.abs() < 0.1, "{r:?}");
        // Search over plausible feature sizes (grid step is 75): at least one gives
        // a range, mean and spread of the same order as the reference.
        let mut best = f64::MAX;
        for scale in [50.0, 60.0, 100.0, 130.0, 200.0] {
            let o = ours(&g, scale, 123456);
            println!("  ours scale {scale}: {o:?}");
            assert!(o.min >= 0.0 && o.max <= 1.0);
            best = best.min((o.std - r.std).abs());
            if scale <= 100.0 {
                assert!(o.lag1.abs() < 0.15, "{o:?}");
            }
        }
        assert!(
            best < 0.5 * r.std,
            "no feature size reproduces spread (best diff {best})"
        );
    }
}

#[test]
fn shader_noise_sample_is_inside_unit_range() {
    let g = load("shader-noise");
    let r = stats(&g.vals);
    println!("shader-noise reference: {r:?}");
    assert!(r.min >= 0.0 && r.max <= 1.0);
    assert!((r.mean - 0.5).abs() < 0.1);
    assert!(r.lag1.abs() < 0.1);
    let o = ours(&g, 100.0, 665);
    assert!((o.mean - 0.5).abs() < 0.15, "{o:?}");
}
