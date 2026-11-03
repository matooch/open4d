//! Parity: docs/specs/PRIMITIVES3.md §6 against tests/golden/landscape (O18).
//! Topology and x/z positions are exact; heights are compared statistically (L4).

use std::path::Path;

mod common;

use common::{resolved_params, typed_params};
use forge_core::Registry;
use forge_generators::{Primitives, build};
use forge_geometry::PolyMesh;
use forge_parity_harness::discover_cases;

fn registry() -> Registry {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r
}

/// Our mesh for a case (percent fields are stored as fractions, orientation spelled `pos_y`).
fn ours(reg: &Registry, case: &forge_parity_harness::GoldenCase) -> PolyMesh {
    let manifest = case.manifest().unwrap();
    let raw = resolved_params(case, &manifest)
        .into_iter()
        .map(|(k, v)| {
            let v = match (k.as_str(), &v) {
                ("rough_furrows" | "fine_furrows" | "sea_level" | "plateau_level", _) => {
                    toml::Value::Float(v.as_float().unwrap() / 100.0)
                }
                ("orientation", toml::Value::String(s)) => {
                    toml::Value::String(s.replace("plus_", "pos_").replace("minus_", "neg_"))
                }
                _ => v,
            };
            (k, v)
        })
        .collect();
    let p = typed_params(reg, "forge.landscape", &raw);
    let m = build("forge.landscape", &p).unwrap();
    m.validate().unwrap();
    m
}

fn landscape_cases() -> Vec<forge_parity_harness::GoldenCase> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "landscape")
        .collect()
}

#[test]
fn landscape_matches_the_reference_topology() {
    let reg = registry();
    let cases = landscape_cases();
    assert_eq!(cases.len(), 4);
    for case in &cases {
        let want = case.expected_mesh().unwrap().unwrap();
        let got = ours(&reg, case);
        assert_eq!(got.points.len(), want.positions.len(), "{}", case.name);
        assert_eq!(got.polys, want.faces, "{}: face lists", case.name);
        for (i, (a, b)) in got.points.iter().zip(&want.positions).enumerate() {
            assert!(
                (a.x - b[0]).abs() < 1e-4 && (a.z - b[2]).abs() < 1e-4,
                "{}: vertex {i}: ours {a:?} expected {b:?}",
                case.name
            );
        }
    }
}

struct Stats {
    mean: f64,
    std: f64,
    max: f64,
}

fn stats(h: &[f64]) -> Stats {
    let mean = h.iter().sum::<f64>() / h.len() as f64;
    let var = h.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / h.len() as f64;
    Stats {
        mean,
        std: var.sqrt(),
        max: h.iter().cloned().fold(f64::MIN, f64::max),
    }
}

fn histogram(h: &[f64]) -> [f64; 10] {
    let mut bins = [0.0; 10];
    for x in h {
        bins[((x * 10.0) as usize).min(9)] += 1.0 / h.len() as f64;
    }
    bins
}

fn correlation(a: &[f64], b: &[f64]) -> f64 {
    let (ma, mb) = (stats(a).mean, stats(b).mean);
    let cov: f64 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let (va, vb): (f64, f64) = (
        a.iter().map(|x| (x - ma).powi(2)).sum(),
        b.iter().map(|y| (y - mb).powi(2)).sum(),
    );
    cov / (va * vb).sqrt()
}

#[test]
fn landscape_heights_are_statistically_close() {
    let reg = registry();
    for case in &landscape_cases() {
        let want = case.expected_mesh().unwrap().unwrap();
        let theirs: Vec<f64> = want.positions.iter().map(|p| p[1] / 100.0).collect();
        let mesh = ours(&reg, case);
        let mine: Vec<f64> = mesh.points.iter().map(|p| p.y / 100.0).collect();
        let (s, t) = (stats(&mine), stats(&theirs));
        let overlap: f64 = histogram(&mine)
            .iter()
            .zip(histogram(&theirs))
            .map(|(a, b)| a.min(b))
            .sum();
        let corr = correlation(&mine, &theirs);
        println!(
            "{}: mean {:.3}/{:.3} std {:.3}/{:.3} max {:.3}/{:.3} overlap {overlap:.3} corr {corr:.3}",
            case.name, s.mean, t.mean, s.std, t.std, s.max, t.max
        );
        assert!((s.mean - t.mean).abs() <= 0.07, "{}: mean", case.name);
        assert!((s.std - t.std).abs() <= 0.07, "{}: std", case.name);
        assert!((0.45..=1.0).contains(&s.max), "{}: max", case.name);
        assert!(
            overlap >= 0.60,
            "{}: histogram overlap {overlap}",
            case.name
        );
        assert!(corr >= 0.25, "{}: correlation {corr}", case.name);
        let n = (mesh.points.len() as f64).sqrt() as usize;
        for (i, p) in mesh.points.iter().enumerate() {
            let (c, r) = (i % n, i / n);
            if c == 0 || r == 0 || c == n - 1 || r == n - 1 {
                assert_eq!(p.y, 0.0, "{}: border vertex {i}", case.name);
            }
        }
    }
}

/// F1: the field is a function of the position; a coarser grid equals every 5th point of
/// the finer one up to one global factor (ours: the grid-extreme normalisation).
#[test]
fn landscape_field_is_continuous_in_position() {
    let reg = registry();
    let cases = landscape_cases();
    let get = |n: &str| ours(&reg, cases.iter().find(|c| c.name == n).unwrap());
    let (fine, coarse) = (get("defaults"), get("segments"));
    let ratios: Vec<f64> = (1..20)
        .flat_map(|k| (1..20).map(move |i| (k, i)))
        .map(|(k, i)| coarse.points[k * 21 + i].y / fine.points[5 * k * 101 + 5 * i].y)
        .collect();
    let (lo, hi) = ratios
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), &r| (l.min(r), h.max(r)));
    assert!(hi / lo < 1.0 + 1e-9, "ratio spread {lo} {hi}");
    assert!((0.9..1.15).contains(&lo), "global factor {lo}");
}
