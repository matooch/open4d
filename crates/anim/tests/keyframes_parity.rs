//! Parity: docs/specs/ANIM.md §1 against tests/golden/keyframes (level B samples).

use std::path::Path;

use forge_anim::{UserTangent, value_at, value_at_exact};
use forge_core::{Behavior, Interp, Key, Track};
use forge_parity_harness::{GoldenCase, Samples, compare_samples, discover_cases};

/// Observed cases whose options are not derived.
/// `linear-to-spline` has no machine-readable per-key interpolation (docs/specs/ANIM.md
/// §1.5a); its redo `linear-to-spline-2` is evaluated.
const UNSUPPORTED: &[&str] = &["keyframes/linear-to-spline"];

/// Cases we evaluate but that differ, pinned so a fix shows up.
const KNOWN_MISMATCH: &[&str] = &[];

/// Follow-up (redo) recordings whose half-frame samples (14.5, 29.5) hold the value of the
/// rounded whole frame (15, 30): in `linear-to-spline-2` and `broken-2` the two rows equal
/// their whole-frame neighbours to every digit, while the original recordings read the true
/// fractional time. A property of how the follow-up was sampled (ANIM.md §1.5a).
const WHOLE_FRAME_SAMPLING: &[&str] = &["keyframes/linear-to-spline-2", "keyframes/broken-2"];

/// Observation whose samples are taken at the exact frame time (ANIM.md §1.5a).
const EXACT_TIME_OBSERVATION: &str = "docs/parity/fcurves.md";

/// The user tangents of a case from `tangents.json` rows
/// `[frame, value, left_dt, left_dv, right_dt, right_dv]`, matched to keys by frame.
fn user_tangents(case: &GoldenCase, fps: f64, track: &Track) -> Vec<Option<UserTangent>> {
    let Ok(src) = std::fs::read_to_string(case.dir.join("tangents.json")) else {
        return Vec::new();
    };
    let rows: Vec<Vec<f64>> = serde_json::from_str(&src).expect("tangents.json");
    track
        .keys
        .iter()
        .map(|k| {
            rows.iter()
                .find(|r| (r[0] / fps - k.time).abs() < 1e-9)
                .map(|r| UserTangent {
                    left: (r[2] / fps, r[3]),
                    right: (r[4] / fps, r[5]),
                })
        })
        .collect()
}

fn number(v: &toml::Value) -> f64 {
    v.as_float()
        .or_else(|| v.as_integer().map(|i| i as f64))
        .expect("number")
}

/// The behaviour a case description states: "after-behaviour Offset Repeat",
/// "before-behaviour Repeat" (`case.toml` has no field for it; ANIM.md §1.9).
fn behaviour(
    params: &std::collections::BTreeMap<String, toml::Value>,
    description: &str,
    side: &str,
) -> Behavior {
    if let Some(name) = params
        .get(&format!("{side}_behavior"))
        .and_then(|v| v.as_str())
    {
        return behaviour_named(name);
    }
    let marker = format!("{side}-behaviour ");
    let Some(at) = description.find(&marker) else {
        return Behavior::Constant;
    };
    let rest = &description[at + marker.len()..];
    behaviour_named(rest.split(',').next().unwrap_or("").trim())
}

fn behaviour_named(name: &str) -> Behavior {
    match name {
        "Repeat" => Behavior::Repeat,
        "Offset Repeat" => Behavior::OffsetRepeat,
        "Oscillate" => Behavior::Oscillate,
        "Continue" => Behavior::Continue,
        "Constant" => Behavior::Constant,
        other => panic!("unknown behaviour {other:?}"),
    }
}

fn interp(name: &str) -> Interp {
    match name.to_lowercase().as_str() {
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
    let description = case.manifest().unwrap().description.unwrap_or_default();
    let mut track = Track::new(
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
    );
    track.before = behaviour(&params, &description, "before");
    track.after = behaviour(&params, &description, "after");
    track
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
    let whole = WHOLE_FRAME_SAMPLING.contains(&format!("{}/{}", case.feature, case.name).as_str());
    let exact = case.manifest().unwrap().observation == EXACT_TIME_OBSERVATION;
    let user = if exact && !angle {
        user_tangents(case, *fps, &track)
    } else {
        Vec::new()
    };
    Samples::Track {
        param: param.clone(),
        fps: *fps,
        tolerance: *tolerance,
        samples: samples
            .iter()
            .map(|(frame, _)| {
                let read = if whole { (frame + 0.5).floor() } else { *frame };
                let t = read / fps;
                let v = if exact {
                    value_at_exact(&track, &user, t)
                } else {
                    value_at(&track, t)
                };
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
    assert_eq!(matched.len() + mismatched.len() + unsupported.len(), 40);
    // the UNSUPPORTED case is not evaluated.
    assert_eq!(files, 41);
}
