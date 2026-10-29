//! Level-B samples: docs/specs/INFRA.md §5.

use std::path::Path;

use forge_parity_harness::{
    Samples, compare_samples, discover_cases, parse_samples, samples_info, validate_samples,
};

const TRACK: &str = r#"{"kind":"track","param":"pos_x","fps":30,"tolerance":1e-4,
  "samples":[[0.0,0.0],[0.5,1.0],[1.0,2.0]]}"#;
const FIELD: &str = r#"{"kind":"field","tolerance":1e-5,
  "samples":[[[0,0,0],1.0],[[75,0,0],0.5]]}"#;
const CLONES: &str = r#"{"kind":"clones","tolerance":1e-4,"clones":[
  {"pos":[1,2,3],"x":[1,0,0],"y":[0,1,0],"z":[0,0,1],"color":[1,1,1],"weight":0.5,"visible":true},
  {"pos":[4,5,6],"x":[1,0,0],"y":[0,1,0],"z":[0,0,1],"color":[0,0,0],"weight":0.0,"visible":false}]}"#;

fn parse(s: &str) -> Samples {
    parse_samples(s).unwrap()
}

#[test]
fn parses_track() {
    let Samples::Track {
        param,
        fps,
        tolerance,
        samples,
    } = parse(TRACK)
    else {
        panic!("not a track")
    };
    assert_eq!((param.as_str(), fps, tolerance), ("pos_x", 30.0, 1e-4));
    assert_eq!(samples, vec![(0.0, 0.0), (0.5, 1.0), (1.0, 2.0)]);
}

#[test]
fn parses_field() {
    let Samples::Field { tolerance, samples } = parse(FIELD) else {
        panic!("not a field")
    };
    assert_eq!(tolerance, 1e-5);
    assert_eq!(samples, vec![([0.0; 3], 1.0), ([75.0, 0.0, 0.0], 0.5)]);
}

#[test]
fn parses_clones() {
    let Samples::Clones { tolerance, clones } = parse(CLONES) else {
        panic!("not clones")
    };
    assert_eq!(tolerance, 1e-4);
    assert_eq!(clones.len(), 2);
    assert_eq!(clones[0].pos, [1.0, 2.0, 3.0]);
    assert_eq!(clones[0].weight, 0.5);
    assert!(clones[0].visible && !clones[1].visible);
}

#[test]
fn parse_errors_name_the_field() {
    let e = parse_samples(r#"{"kind":"sphere"}"#).unwrap_err();
    assert!(e.contains("unknown kind") && e.contains("sphere"), "{e}");
    let e = parse_samples(r#"{"kind":"field","samples":[]}"#).unwrap_err();
    assert_eq!(e, "missing field \"tolerance\"");
    let e = parse_samples(r#"{"samples":[]}"#).unwrap_err();
    assert_eq!(e, "missing field \"kind\"");
    let e =
        parse_samples(r#"{"kind":"track","param":"a","fps":30,"tolerance":1,"samples":[[0,1,2]]}"#)
            .unwrap_err();
    assert!(e.contains("samples[0]"), "{e}");
    let e = parse_samples(
        r#"{"kind":"clones","tolerance":1,"clones":[{"pos":[0,0,0],"x":[1,0,0],"y":[0,1,0],"z":[0,0,1],"color":[1,1,1],"visible":true}]}"#,
    )
    .unwrap_err();
    assert!(e.contains("clones[0]") && e.contains("weight"), "{e}");
    assert!(parse_samples("{").unwrap_err().contains("invalid JSON"));
}

#[test]
fn validate_rejects_unsorted_track_and_bad_tolerance() {
    assert!(validate_samples(&parse(TRACK)).is_empty());
    let unsorted = parse(
        r#"{"kind":"track","param":"a","fps":30,"tolerance":1e-4,"samples":[[0,0],[2,1],[1,2]]}"#,
    );
    let p = validate_samples(&unsorted);
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(
        p[0].contains("samples[2]") && p[0].contains("strictly increase"),
        "{p:?}"
    );
    let dup =
        parse(r#"{"kind":"track","param":"a","fps":30,"tolerance":1e-4,"samples":[[0,0],[0,1]]}"#);
    assert_eq!(validate_samples(&dup).len(), 1);
    let zero = parse(r#"{"kind":"field","tolerance":0,"samples":[]}"#);
    assert!(validate_samples(&zero)[0].contains("tolerance"));
    let zero = parse(r#"{"kind":"clones","tolerance":0,"clones":[]}"#);
    assert!(validate_samples(&zero)[0].contains("tolerance"));
}

#[test]
fn info_summarises() {
    let t = samples_info(&parse(TRACK));
    assert!(
        t.starts_with("kind: track\nparam: pos_x\nfps: 30\nsamples: 3\n"),
        "{t}"
    );
    assert!(
        t.contains("frames: 0 .. 1\n") && t.contains("value max: 2\n"),
        "{t}"
    );
    let c = samples_info(&parse(CLONES));
    assert!(
        c.contains("clones: 2\n") && c.contains("invisible: 1\n"),
        "{c}"
    );
    assert!(
        c.contains("position min: (1, 2, 3)\nposition max: (4, 5, 6)\n"),
        "{c}"
    );
}

#[test]
fn compare_passes_within_expected_tolerance() {
    let e = parse(TRACK);
    let a = parse(&TRACK.replace("[1.0,2.0]", "[1.0,2.00005]"));
    assert!(compare_samples(&e, &a).passed());
    assert!(compare_samples(&parse(CLONES), &parse(CLONES)).passed());
    assert!(compare_samples(&parse(FIELD), &parse(FIELD)).passed());
}

#[test]
fn compare_fails_on_value_and_uses_expected_tolerance() {
    let e = parse(TRACK);
    let a = parse(&TRACK.replace("[1.0,2.0]", "[1.0,2.001]"));
    let r = compare_samples(&e, &a);
    assert!(!r.passed());
    assert_eq!(r.failure_count, 1);
    assert!(
        r.failures[0].contains("frame 1") && r.failures[0].contains("2.001"),
        "{:?}",
        r.failures
    );
    // a looser tolerance in the actual file does not help
    let a = parse(
        &TRACK
            .replace("[1.0,2.0]", "[1.0,2.001]")
            .replace("1e-4", "1"),
    );
    assert!(!compare_samples(&e, &a).passed());
}

#[test]
fn compare_fails_on_sample_position() {
    let e = parse(TRACK);
    let a = parse(&TRACK.replace("[0.5,1.0]", "[0.6,1.0]"));
    let r = compare_samples(&e, &a);
    assert!(!r.passed());
    assert!(
        r.failures[0].contains("sample 1") && r.failures[0].contains("0.6"),
        "{:?}",
        r.failures
    );
    let f = parse(&FIELD.replace("[75,0,0]", "[76,0,0]"));
    assert!(!compare_samples(&parse(FIELD), &f).passed());
    let short =
        parse(r#"{"kind":"track","param":"pos_x","fps":30,"tolerance":1e-4,"samples":[[0,0]]}"#);
    assert!(compare_samples(&e, &short).failures[0].contains("sample count"));
}

#[test]
fn compare_fails_on_kind_mismatch() {
    let r = compare_samples(&parse(TRACK), &parse(FIELD));
    assert!(!r.passed());
    assert_eq!(
        r.failures,
        vec!["kind: expected track, got field".to_string()]
    );
}

#[test]
fn compare_clones_checks_visibility_count_and_fields() {
    let e = parse(CLONES);
    let a = parse(&CLONES.replace("\"visible\":false", "\"visible\":true"));
    let r = compare_samples(&e, &a);
    assert!(
        r.failures[0].contains("clone 1 visible"),
        "{:?}",
        r.failures
    );
    let a = parse(&CLONES.replace("\"weight\":0.5", "\"weight\":0.6"));
    assert!(compare_samples(&e, &a).failures[0].contains("clone 0 weight"));
    let a = parse(&CLONES.replace("[4,5,6]", "[4,5,7]"));
    assert!(compare_samples(&e, &a).failures[0].contains("clone 1 pos"));
    let one = parse(
        r#"{"kind":"clones","tolerance":1e-4,"clones":[{"pos":[1,2,3],"x":[1,0,0],"y":[0,1,0],"z":[0,0,1],"color":[1,1,1],"weight":0.5,"visible":true}]}"#,
    );
    assert!(compare_samples(&e, &one).failures[0].contains("clone count"));
}

#[test]
fn failures_are_capped_at_ten() {
    let e = parse(&FIELD.replace(
        "[[[0,0,0],1.0],[[75,0,0],0.5]]",
        &format!("[{}]", vec!["[[0,0,0],1.0]"; 25].join(",")),
    ));
    let a = parse(&FIELD.replace(
        "[[[0,0,0],1.0],[[75,0,0],0.5]]",
        &format!("[{}]", vec!["[[0,0,0],2.0]"; 25].join(",")),
    ));
    let r = compare_samples(&e, &a);
    assert_eq!((r.failure_count, r.failures.len()), (25, 10));
}

/// Every real samples file parses, validates and has the pinned count per kind.
#[test]
fn all_repo_samples_files_parse() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (mut tracks, mut fields, mut clones) = (0, 0, 0);
    for case in discover_cases(&repo.join("tests/golden")).unwrap() {
        for (name, s) in case
            .samples()
            .unwrap_or_else(|e| panic!("{}: {e}", case.name))
        {
            let p = validate_samples(&s);
            assert!(p.is_empty(), "{}/{}/{name}: {p:?}", case.feature, case.name);
            match s {
                Samples::Track { .. } => tracks += 1,
                Samples::Field { .. } => fields += 1,
                Samples::Clones { .. } => clones += 1,
            }
        }
    }
    // Tracks: 11 files from O2 (keyframes.md) + 8 from O12 (keyframes-2.md) + 7 from O16.
    // Fields: 16 from O7 + 8 from O11 + 11 from O13 (fields-2.md).
    // Clones: 12 from O8 + 4 from O11 + 12 from O14 (effectors-2.md).
    assert_eq!((tracks, fields, clones), (26, 35, 28));
}

#[test]
fn compare_track_requires_same_param_and_fps() {
    let e = parse(TRACK);
    let r = compare_samples(&e, &parse(&TRACK.replace("pos_x", "pos_y")));
    assert!(r.failures[0].starts_with("param:"), "{:?}", r.failures);
    let r = compare_samples(&e, &parse(&TRACK.replace("\"fps\":30", "\"fps\":24")));
    assert!(r.failures[0].starts_with("fps:"), "{:?}", r.failures);
}
