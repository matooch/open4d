//! Parity: docs/specs/PRIMITIVES3.md §6 against tests/golden/landscape (O18).
//! Topology and x/z positions are exact; heights are compared statistically (L4).

use std::path::Path;

mod common;

use common::{resolved_params, typed_params};
use forge_core::Registry;
use forge_generators::{Primitives, build};
use forge_geometry::PolyMesh;
use forge_parity_harness::discover_cases;

/// Cases whose heights fall outside the statistical tolerances (level C: the reference
/// noise is not derived, PRIMITIVES3.md L2/L4). Pinned so a model change shows up; the
/// topology and horizontal positions of every case are still exact.
const STAT_MISMATCH: &[&str] = &[
    // The unmasked field's mean/std/histogram are further off than the tolerance, and the
    // two noises are uncorrelated without the shared border mask.
    "no-borders",
    // Spherical has no border mask either (corr 0.01); its topology and radius rule are exact
    // in structure (PRIMITIVES3.md L5) but the heights are statistical.
    "spherical",
    // The reference's sea-level law is not derived: sea 50 % is not a function of the
    // default height (correlation 0.2), and plateau 50 % on top is exactly 0.005 x sea 50 %.
    "sea-level-50",
    "sea-level-50-plateau-50",
];

/// Height axis (0 = x, 1 = y, 2 = z) and sign of a case orientation.
fn height_axis(orientation: &str) -> (usize, f64) {
    let axis = match orientation
        .trim_start_matches("plus_")
        .trim_start_matches("minus_")
    {
        "x" => 0,
        "z" => 2,
        _ => 1,
    };
    (
        axis,
        if orientation.starts_with("minus_") {
            -1.0
        } else {
            1.0
        },
    )
}

/// A case's heights over Size Y, from a position list (`spherical`: the radius above the
/// sphere of radius `min(size_x, size_z) / 2`).
fn heights_of(positions: &[[f64; 3]], params: &toml::map::Map<String, toml::Value>) -> Vec<f64> {
    let get = |k: &str| params[k].as_float().unwrap();
    let size_y = get("size_y");
    if params["spherical"].as_bool().unwrap() {
        let r = get("size_x").min(get("size_z")) / 2.0;
        return positions
            .iter()
            .map(|p| ((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() - r) / size_y)
            .collect();
    }
    let (axis, sign) = height_axis(params["orientation"].as_str().unwrap());
    positions.iter().map(|p| sign * p[axis] / size_y).collect()
}

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
        .filter(|c| match c.data_not_fetched() {
            Some(note) => {
                println!("note: {note}");
                false
            }
            None => true,
        })
        .collect()
}

#[test]
fn landscape_matches_the_reference_topology() {
    let reg = registry();
    let cases = landscape_cases();
    assert!(cases.len() >= 20, "{} landscape cases", cases.len());
    for case in &cases {
        let manifest = case.manifest().unwrap();
        let params = resolved_params(case, &manifest);
        let want = case.expected_mesh().unwrap().unwrap();
        let got = ours(&reg, case);
        assert_eq!(got.points.len(), want.positions.len(), "{}", case.name);
        assert_eq!(got.polys, want.faces, "{}: face lists", case.name);
        if params["spherical"].as_bool().unwrap() {
            // Positions follow the heights, which are statistical.
            continue;
        }
        let (axis, _) = height_axis(params["orientation"].as_str().unwrap());
        for (i, (a, b)) in got.points.iter().zip(&want.positions).enumerate() {
            let a = a.to_array();
            for c in (0..3).filter(|c| *c != axis) {
                assert!(
                    (a[c] - b[c]).abs() < 1e-4,
                    "{}: vertex {i}: ours {a:?} expected {b:?}",
                    case.name
                );
            }
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
        let params = resolved_params(case, &case.manifest().unwrap());
        let want = case.expected_mesh().unwrap().unwrap();
        let theirs = heights_of(&want.positions, &params);
        let mesh = ours(&reg, case);
        let pts: Vec<[f64; 3]> = mesh.points.iter().map(|p| p.to_array()).collect();
        let mine = heights_of(&pts, &params);
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
        let within = (s.mean - t.mean).abs() <= 0.07
            && (s.std - t.std).abs() <= 0.07
            && (0.45..=1.0).contains(&s.max)
            && overlap >= 0.60
            && corr >= 0.25;
        if STAT_MISMATCH.contains(&case.name.as_str()) {
            assert!(!within, "{}: now within tolerance, unpin it", case.name);
            continue;
        }
        assert!(within, "{}: statistics out of tolerance", case.name);
        if params["spherical"].as_bool().unwrap()
            || params["orientation"].as_str() != Some("plus_y")
        {
            continue;
        }
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
