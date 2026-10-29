//! Parity: docs/specs/ANIM.md §1 against tests/golden/keyframes (level B samples).

use std::path::Path;

use forge_anim::value_at;
use forge_core::{Interp, Key, Track};
use forge_parity_harness::{GoldenCase, Samples, compare_samples, discover_cases};

/// Observed cases whose options are not derived.
const UNSUPPORTED: &[&str] = &[];

/// Cases we evaluate but that differ, pinned so a fix shows up.
const KNOWN_MISMATCH: &[&str] = &[
    // O16 (docs/parity/keyframes-3.md) broke two parity-pending rules of ANIM.md §1:
    // the millisecond time rounding (2.7e-4 at quarter frames and at 24 fps) and the
    // tangent at a monotone key between uneven spans. Rework brief 09b fixes them.
    "keyframes/dense-quarter",
    "keyframes/fps-24",
    "keyframes/mono-uneven",
];

fn number(v: &toml::Value) -> f64 {
    v.as_float()
        .or_else(|| v.as_integer().map(|i| i as f64))
        .expect("number")
}

fn interp(name: &str) -> Interp {
    match name {
        "spline" => Interp::Spline,
        "linear" => Interp::Linear,
        "step" => Interp::Step,
        other => panic!("unknown interpolation {other:?}"),
    }
}

/// The track of a case: `keys = [[frame, value], ...]` at `fps`, values in degrees for
/// `rot_*` params (stored as radians), `interpolation` for all keys or
/// `interpolation_by_key` per key.
fn track_of(case: &GoldenCase, fps: f64, angle: bool) -> Track {
    let params = case.manifest().unwrap().params;
    let keys = params["keys"].as_array().unwrap();
    let all = params
        .get("interpolation")
        .map_or(Interp::Spline, |v| interp(v.as_str().unwrap()));
    let per_key: Option<Vec<Interp>> = params.get("interpolation_by_key").map(|v| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|i| interp(i.as_str().unwrap()))
            .collect()
    });
    Track::new(
        keys.iter()
            .enumerate()
            .map(|(n, k)| {
                let k = k.as_array().unwrap();
                let value = number(&k[1]);
                let value = if angle { value.to_radians() } else { value };
                let i = per_key.as_ref().map_or(all, |p| p[n]);
                Key::new(number(&k[0]) / fps, value).with_interp(i)
            })
            .collect(),
    )
}

/// Evaluates the case's track at every sample frame of `expected`.
fn evaluate(case: &GoldenCase, expected: &Samples) -> Samples {
    let Samples::Track {
        param,
        fps,
        tolerance,
        samples,
    } = expected
    else {
        panic!("keyframe case with a non-track samples file");
    };
    let angle = param.starts_with("rot_");
    let track = track_of(case, *fps, angle);
    Samples::Track {
        param: param.clone(),
        fps: *fps,
        tolerance: *tolerance,
        samples: samples
            .iter()
            .map(|(frame, _)| {
                let v = value_at(&track, frame / fps);
                (*frame, if angle { v.to_degrees() } else { v })
            })
            .collect(),
    }
}

#[test]
fn keyframes_match_the_reference() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut mismatched, mut unsupported) = (Vec::new(), Vec::new(), Vec::new());
    let mut files = 0;
    let mut worst: Vec<(String, f64)> = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "keyframes" {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        if UNSUPPORTED.contains(&key.as_str()) {
            unsupported.push(key);
            continue;
        }
        let mut ok = true;
        for (name, expected) in case.samples().unwrap() {
            files += 1;
            let report = compare_samples(&expected, &evaluate(&case, &expected));
            worst.push((format!("{key}/{name}"), report.max_error));
            if !report.passed() {
                println!("{key}/{name}: {:?}", report.failures);
                ok = false;
            }
        }
        if ok {
            matched.push(key);
        } else {
            mismatched.push(key);
        }
    }
    for (name, err) in &worst {
        println!("max error {err:.3e}  {name}");
    }
    println!(
        "matched {}/{} cases, {files} files",
        matched.len(),
        matched.len() + mismatched.len() + unsupported.len()
    );
    let expected_mismatch: Vec<String> = KNOWN_MISMATCH.iter().map(|s| s.to_string()).collect();
    assert_eq!(mismatched, expected_mismatch, "mismatching cases");
    let expected_unsupported: Vec<String> = UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, expected_unsupported);
    assert_eq!(matched.len() + mismatched.len() + unsupported.len(), 24);
    assert_eq!(files, 26);
}
