//! Every golden case committed to the real repo must validate (docs/specs/INFRA.md §3).

use std::path::Path;

use forge_parity_harness::{cases_without_output, discover_cases, validate_case};

#[test]
fn all_repo_golden_cases_are_valid() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut problems = Vec::new();
    for case in discover_cases(&repo.join("tests/golden")).expect("tests/golden exists") {
        for p in validate_case(&repo, &case) {
            problems.push(format!("{}/{}: {p}", case.feature, case.name));
        }
    }
    assert!(
        problems.is_empty(),
        "invalid golden cases:\n{}",
        problems.join("\n")
    );
}

/// Cases with neither mesh nor samples are allowed (tree or form only) but pinned, so a
/// new output-less case is a deliberate test change (docs/specs/INFRA.md §5.2).
#[test]
fn cases_without_output_are_pinned() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases = discover_cases(&repo.join("tests/golden")).unwrap();
    assert_eq!(cases_without_output(&cases), ["make-editable/spline"]);
}
