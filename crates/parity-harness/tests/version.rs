//! App version from the observation header and the R20 defaults overlay
//! (docs/specs/INFRA.md §3.1).

use std::fs;
use std::path::Path;

use forge_parity_harness::{AppVersion, CaseManifest, app_version, defaults_overlay};

fn manifest(observation: &str) -> CaseManifest {
    toml::from_str(&format!(
        "feature = \"cube\"\nobservation = \"{observation}\"\n"
    ))
    .unwrap()
}

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

fn version_of(header: &str) -> Result<AppVersion, String> {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "docs/parity/x.md",
        &format!("# Title\n\n{header}\nBody R25 mentioned here\n"),
    );
    app_version(dir.path(), &manifest("docs/parity/x.md"))
}

#[test]
fn reads_r20_and_r25_from_the_status_header() {
    assert_eq!(
        version_of("Status: observed · App version: Tool R20.059 (Build 1) · OS: x"),
        Ok(AppVersion::R20)
    );
    assert_eq!(
        version_of("Status: observed · App version: Tool R25 (version number 25010) · OS: x"),
        Ok(AppVersion::R25)
    );
}

#[test]
fn only_the_token_after_app_version_counts() {
    // an earlier R25 in the line, or a later one, must not win
    assert_eq!(
        version_of("Status: R25 draft · App version: Tool R20.059 · OS: R25"),
        Ok(AppVersion::R20)
    );
}

#[test]
fn anything_else_is_an_error_naming_the_file() {
    for header in [
        "Status: observed · App version: Tool R21 · OS: x",
        "Status: observed · App version: TODO · OS: x",
        "Status: observed · OS: x",
        "Not a status line R20",
    ] {
        let e = version_of(header).unwrap_err();
        assert!(e.contains("docs/parity/x.md"), "{e}");
    }
    let dir = tempfile::tempdir().unwrap();
    let e = app_version(dir.path(), &manifest("docs/parity/none.md")).unwrap_err();
    assert!(e.contains("docs/parity/none.md"), "{e}");
}

#[test]
fn the_repo_observations_have_the_right_version() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(
        app_version(&root, &manifest("docs/parity/sphere.md")),
        Ok(AppVersion::R20)
    );
    for o in ["r25-defaults", "r25-drift"] {
        assert_eq!(
            app_version(&root, &manifest(&format!("docs/parity/{o}.md"))),
            Ok(AppVersion::R25)
        );
    }
}

#[test]
fn overlay_is_the_r20_table_for_r20_and_empty_for_r25() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "tests/defaults/r20.toml",
        "[sphere]\nsegments = 24\n\n[cylinder]\nslice_to = 360.0\nrotation_segments = 36\n",
    );
    let o = defaults_overlay(dir.path(), AppVersion::R20, "cylinder");
    assert_eq!(o.len(), 2);
    assert_eq!(o["slice_to"].as_float(), Some(360.0));
    assert_eq!(o["rotation_segments"].as_integer(), Some(36));
    assert!(defaults_overlay(dir.path(), AppVersion::R20, "torus").is_empty());
    assert!(defaults_overlay(dir.path(), AppVersion::R25, "cylinder").is_empty());
}

#[test]
fn the_repo_overlay_parses_and_holds_the_documented_rows() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let o = defaults_overlay(&root, AppVersion::R20, "sphere");
    assert_eq!(o["segments"].as_integer(), Some(24));
    let o = defaults_overlay(&root, AppVersion::R20, "boole");
    assert_eq!(o["hide_new_edges"].as_bool(), Some(false));
}
