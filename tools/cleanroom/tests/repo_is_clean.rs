//! The real CI gate, also run by `cargo test` (docs/specs/INFRA.md §2).

use std::path::Path;

use forge_cleanroom::{Config, scan};

#[test]
fn repository_has_no_forbidden_material() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cfg = Config::load(&root.join("tools/cleanroom/forbidden.txt")).expect("config loads");
    assert!(
        !cfg.terms.is_empty() && !cfg.extensions.is_empty(),
        "config looks empty"
    );
    let violations = scan(&root, &cfg).expect("scan runs");
    let report: Vec<String> = violations.iter().map(ToString::to_string).collect();
    assert!(
        report.is_empty(),
        "clean-room violations:\n{}",
        report.join("\n")
    );
}
