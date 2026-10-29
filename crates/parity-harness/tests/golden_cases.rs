//! Golden-data discovery and validation (docs/specs/INFRA.md §3).

use std::fs;
use std::path::{Path, PathBuf};

use forge_parity_harness::{
    Mesh, cases_without_output, compare_meshes, discover_cases, validate_case,
};

fn fixture_repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/repo")
}

#[test]
fn fixture_case_is_valid_and_matches_our_mesh() {
    let repo = fixture_repo();
    let cases = discover_cases(&repo.join("tests/golden")).unwrap();
    assert_eq!(cases.len(), 1);
    let case = &cases[0];
    assert_eq!(
        (case.feature.as_str(), case.name.as_str()),
        ("selftest", "two-quads")
    );
    let problems = validate_case(&repo, case);
    assert!(problems.is_empty(), "{problems:?}");

    let manifest = case.manifest().unwrap();
    assert_eq!(manifest.params["segments"].as_integer(), Some(2));
    assert_eq!(manifest.tolerance.position, 1e-6);
    assert_eq!(manifest.tolerance.psnr, 40.0);

    // What "our" generator would produce for this case: meters, +Z, wound the other way.
    let ours = Mesh {
        positions: vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [2., 0., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [2., 0., 1.],
        ],
        faces: vec![vec![3, 4, 1, 0], vec![4, 5, 2, 1]],
    };
    let expected = case.expected_mesh().unwrap().expect("case has a mesh");
    let report = compare_meshes(&expected, &ours, &manifest.tolerance);
    assert!(report.passed(), "{:?}", report.failures);
}

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

fn problems_for(manifest: Option<&str>) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    write(repo, "docs/parity/cube.md", "obs\n");
    write(repo, "notes/cube.md", "obs\n");
    fs::create_dir_all(repo.join("tests/golden/cube/default")).unwrap();
    if let Some(m) = manifest {
        write(repo, "tests/golden/cube/default/case.toml", m);
    }
    let cases = discover_cases(&repo.join("tests/golden")).unwrap();
    assert_eq!(cases.len(), 1);
    validate_case(repo, &cases[0])
}

fn assert_problem(manifest: Option<&str>, needle: &str) {
    let p = problems_for(manifest);
    assert!(
        p.iter().any(|s| s.contains(needle)),
        "expected {needle:?} in {p:?}"
    );
}

const OK: &str = "feature = \"cube\"\nobservation = \"docs/parity/cube.md\"\n";

#[test]
fn minimal_manifest_without_output_is_valid_but_listed() {
    assert!(problems_for(Some(OK)).is_empty());
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    write(repo, "docs/parity/cube.md", "obs\n");
    write(repo, "tests/golden/cube/default/case.toml", OK);
    write(
        repo,
        "tests/golden/cube/default/samples-a.json",
        r#"{"kind":"field","tolerance":1e-5,"samples":[[[0,0,0],1.0]]}"#,
    );
    let cases = discover_cases(&repo.join("tests/golden")).unwrap();
    assert!(validate_case(repo, &cases[0]).is_empty());
    assert_eq!(cases[0].samples_files(), vec!["samples-a.json"]);
    assert!(cases_without_output(&cases).is_empty());
    // a broken samples file is reported with its name
    write(
        repo,
        "tests/golden/cube/default/samples.json",
        r#"{"kind":"x"}"#,
    );
    let p = validate_case(repo, &cases[0]);
    assert!(p.iter().any(|s| s.starts_with("samples.json: ")), "{p:?}");
}

#[test]
fn validation_flags_each_rule() {
    assert_problem(None, "case.toml");
    assert_problem(
        Some("feature = \"sphere\"\nobservation = \"docs/parity/cube.md\"\n"),
        "feature",
    );
    assert_problem(
        Some("feature = \"cube\"\nobservation = \"docs/parity/nope.md\"\n"),
        "does not exist",
    );
    assert_problem(
        Some("feature = \"cube\"\nobservation = \"notes/cube.md\"\n"),
        "docs/parity/",
    );
    assert_problem(Some(&format!("{OK}mesh = \"m.obj\"\n")), "m.obj");
    assert_problem(Some(&format!("{OK}tolerence = 1\n")), "tolerence");
    assert_problem(Some("feature = \"cube\"\n"), "observation");
}

#[test]
fn invalid_mesh_file_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    write(repo, "docs/parity/cube.md", "obs\n");
    write(
        repo,
        "tests/golden/cube/default/case.toml",
        &format!("{OK}mesh = \"mesh.obj\"\n"),
    );
    write(repo, "tests/golden/cube/default/mesh.obj", "v 0 0\n");
    let cases = discover_cases(&repo.join("tests/golden")).unwrap();
    let p = validate_case(repo, &cases[0]);
    assert!(p.iter().any(|s| s.contains("line 1")), "{p:?}");
}

#[test]
fn missing_golden_root_is_an_error_but_empty_root_has_no_cases() {
    let dir = tempfile::tempdir().unwrap();
    assert!(discover_cases(&dir.path().join("absent")).is_err());
    assert!(discover_cases(dir.path()).unwrap().is_empty());
}
