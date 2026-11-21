//! Parity: docs/specs/SHADERS.md against tests/golden/shaders (level B, 1024 samples per channel,
//! tolerance 1e-4). Gradient, Checkerboard, Tiles and Fresnel match; the noise based cases are
//! pinned (the reference noise basis is not public) and checked statistically.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use forge_core::Value;
use forge_material::ShadeCtx;
use forge_parity_harness::{Samples, compare_samples, discover_cases};
use forge_shaders::{Inputs, Shader, ShaderError, Values, build};

/// Cases that match exactly.
const EXACT: &[&str] = &[
    "gradient-u",
    "gradient-v",
    "gradient-circular",
    "checkerboard",
    "tiles",
    "fresnel",
];

/// Built but different: noise values of the reference cannot be reproduced (SHADERS.md S12).
const KNOWN_MISMATCH: &[&str] = &[
    "noise-defaults",
    "noise-fbm",
    "noise-turbulence",
    "noise-voronoi1",
    "noise-cell",
    "noise-octaves",
    "noise-scale",
    "noise-time",
    "gradient-turbulence",
    "colorizer",
];

/// Options not derived: building must fail with `Unsupported`.
const UNSUPPORTED: &[&str] = &["brick"];

fn slug(s: &str) -> String {
    s.to_lowercase()
        .replace(" - ", "_")
        .replace(['(', ')'], "")
        .replace([' ', '-'], "_")
}

fn f(v: &toml::Value) -> f64 {
    v.as_float()
        .or_else(|| v.as_integer().map(|i| i as f64))
        .unwrap()
}

fn color(v: &toml::Value) -> Value {
    let a = v.as_array().unwrap();
    Value::Color([f(&a[0]), f(&a[1]), f(&a[2])])
}

/// The case's shader table as parameter values (percent keys become fractions).
fn values(shader: &toml::Table) -> Values {
    let mut out = Values::new();
    for (k, v) in shader {
        if k == "type" {
            continue;
        }
        let (key, val): (String, Value) = if let Some(a) = v.as_array() {
            let key = k.strip_suffix("_percent").unwrap_or(k);
            if key.starts_with("color") || key.ends_with("_color") || key.starts_with("tiles_color")
            {
                (key.to_string(), color(v))
            } else {
                // percent vectors are ratios (100 % = 1.0) in the table's spelling already
                (key.to_string(), Value::Vec3([f(&a[0]), f(&a[1]), f(&a[2])]))
            }
        } else if let Some(base) = k.strip_suffix("_percent") {
            (base.to_string(), Value::Float(f(v) / 100.0))
        } else if let Some(base) = k.strip_suffix("_deg") {
            (base.to_string(), Value::Float(f(v).to_radians()))
        } else if let Some(s) = v.as_str() {
            let key = match k.as_str() {
                "gradient_type" => "type",
                other => other,
            };
            let id = slug(s);
            let id = id
                .strip_prefix("2d_")
                .or(id.strip_prefix("3d_"))
                .map(str::to_string)
                .unwrap_or(id);
            (key.to_string(), Value::Enum(id))
        } else if let Some(b) = v.as_bool() {
            (k.clone(), Value::Bool(b))
        } else if v.is_integer() && (k == "seed" || k == "octaves") {
            (k.clone(), Value::Int(v.as_integer().unwrap()))
        } else {
            (k.clone(), Value::Float(f(v)))
        };
        out.insert(key, val);
    }
    // colour keys that are plain floats in the table
    out
}

fn make(case: &forge_parity_harness::GoldenCase) -> Result<(Shader, f64), ShaderError> {
    let m = case.manifest().unwrap();
    let shader = m.params["shader"].as_table().unwrap();
    let ty = shader["type"].as_str().unwrap();
    let time = m.params.get("sample_time_s").map(f).unwrap_or(0.0);
    let mut inputs = Inputs::default();
    if ty == "colorizer" {
        inputs.shaders.push(build(
            "forge.shader.noise",
            &Values::new(),
            &Inputs::default(),
        )?);
    }
    let mut v = values(shader);
    if ty == "colorizer" {
        v.remove("texture");
    }
    Ok((build(&format!("forge.shader.{ty}"), &v, &inputs)?, time))
}

/// Our samples in the layout of one expected file (`samples.json` red, `-g`, `-b`).
fn ours(shader: &Shader, time: f64, expected: &Samples, channel: usize) -> Samples {
    let Samples::Field { tolerance, samples } = expected else {
        panic!("field samples expected");
    };
    Samples::Field {
        tolerance: *tolerance,
        samples: samples
            .iter()
            .map(|(p, _)| {
                let ctx = ShadeCtx {
                    uvw: *p,
                    time,
                    ..ShadeCtx::default()
                };
                (*p, shader.sample(&ctx)[channel])
            })
            .collect(),
    }
}

fn channel_of(file: &str) -> usize {
    match file {
        "samples-g.json" => 1,
        "samples-b.json" => 2,
        _ => 0,
    }
}

fn values_of(s: &Samples) -> Vec<f64> {
    let Samples::Field { samples, .. } = s else {
        unreachable!()
    };
    samples.iter().map(|(_, v)| *v).collect()
}

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

fn cases() -> Vec<forge_parity_harness::GoldenCase> {
    discover_cases(&root())
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "shaders")
        .collect()
}

#[test]
fn every_case_is_listed_once() {
    let names: BTreeSet<String> = cases().iter().map(|c| c.name.clone()).collect();
    let mut listed: Vec<&str> = EXACT
        .iter()
        .chain(KNOWN_MISMATCH)
        .chain(UNSUPPORTED)
        .copied()
        .collect();
    listed.sort();
    assert_eq!(names.iter().map(String::as_str).collect::<Vec<_>>(), listed);
}

#[test]
fn exact_cases_match_every_channel() {
    for case in cases().iter().filter(|c| EXACT.contains(&c.name.as_str())) {
        let (shader, time) = make(case).unwrap_or_else(|e| panic!("{}: {e}", case.name));
        for (file, expected) in case.samples().unwrap() {
            let r = compare_samples(
                &expected,
                &ours(&shader, time, &expected, channel_of(&file)),
            );
            assert!(
                r.passed(),
                "{}/{file}: {:?} max {}",
                case.name,
                r.failures,
                r.max_error
            );
            assert_eq!(values_of(&expected).len(), 1024);
        }
    }
}

#[test]
fn known_mismatch_cases_still_differ() {
    for case in cases()
        .iter()
        .filter(|c| KNOWN_MISMATCH.contains(&c.name.as_str()))
    {
        let (shader, time) = make(case).unwrap_or_else(|e| panic!("{}: {e}", case.name));
        let (file, expected) = case.samples().unwrap().remove(0);
        let r = compare_samples(
            &expected,
            &ours(&shader, time, &expected, channel_of(&file)),
        );
        assert!(!r.passed(), "{} now matches: move it to EXACT", case.name);
    }
}

#[test]
fn unsupported_cases_report_unsupported() {
    for case in cases()
        .iter()
        .filter(|c| UNSUPPORTED.contains(&c.name.as_str()))
    {
        assert!(
            matches!(make(case), Err(ShaderError::Unsupported(_))),
            "{}",
            case.name
        );
    }
}

/// The noise cases are compared by distribution: values inside 0..1 and a mean and spread in the
/// range of the reference (it varies a lot between types, so the bounds are loose).
#[test]
fn noise_cases_are_statistically_plausible() {
    let mut seen = BTreeMap::new();
    for case in cases().iter().filter(|c| c.name.starts_with("noise-")) {
        let (shader, time) = make(case).unwrap();
        let (file, expected) = case.samples().unwrap().remove(0);
        let theirs = values_of(&expected);
        let mine = values_of(&ours(&shader, time, &expected, channel_of(&file)));
        let stats = |v: &[f64]| {
            let m = v.iter().sum::<f64>() / v.len() as f64;
            let s = (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt();
            (m, s)
        };
        let ((tm, ts), (om, os)) = (stats(&theirs), stats(&mine));
        assert!(
            mine.iter().all(|v| (0.0..=1.0).contains(v)),
            "{}",
            case.name
        );
        // our Turbulence is a sum of |noise| remapped to -1..1, so its mean is far below the
        // reference's 0.5 (SHADERS.md S12); only the range is checked for it
        if case.name != "noise-turbulence" {
            assert!((tm - om).abs() < 0.25, "{}: mean {tm} vs {om}", case.name);
        }
        assert!(
            os > 0.0 && ts > 0.0 && os < 0.5,
            "{}: spread {ts} vs {os}",
            case.name
        );
        seen.insert(case.name.clone(), (tm, om));
    }
    assert_eq!(seen.len(), 8);
}
