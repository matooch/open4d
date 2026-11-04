//! The committed reference must equal what the code generates (docs/specs/CLI.md §docs).
//! On failure run: `cargo run -p forge-cli -- docs --out docs/manual/reference`.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use forge_cli::docs;

fn reference_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual/reference")
}

#[test]
fn the_committed_reference_is_current() {
    let dir = reference_dir();
    let mut stale = Vec::new();
    for (name, content) in docs::generate() {
        match std::fs::read_to_string(dir.join(name)) {
            Ok(on_disk) if on_disk == content => {}
            _ => stale.push(name),
        }
    }
    assert!(
        stale.is_empty(),
        "stale reference {stale:?}: run `cargo run -p forge-cli -- docs --out docs/manual/reference`"
    );
}

#[test]
fn no_unexpected_files_in_the_reference_folder() {
    let expected: Vec<&str> = docs::generate().iter().map(|(n, _)| *n).collect();
    for entry in std::fs::read_dir(reference_dir()).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        assert!(
            expected.contains(&name.as_str()),
            "{name} is in docs/manual/reference but is not generated"
        );
    }
}

#[test]
fn generation_is_deterministic_and_complete() {
    assert_eq!(docs::generate(), docs::generate());
    let pages = docs::generate();
    let commands = &pages.iter().find(|(n, _)| *n == "commands.md").unwrap().1;
    // Every registry command has a section, an MCP tool name and an argument table.
    let app = docs::docs_app();
    for c in app.commands.iter() {
        assert!(
            commands.contains(&format!("### `{}`", c.id())),
            "{}",
            c.id()
        );
        assert!(commands.contains(&format!("`{}`", forge_mcp::tool_name(c.id()))));
    }
    let rpc = &pages.iter().find(|(n, _)| *n == "rpc.md").unwrap().1;
    for m in forge_rpc::docs::METHODS {
        assert!(rpc.contains(&format!("`{}`", m.name)));
    }
    let cli = &pages.iter().find(|(n, _)| *n == "cli.md").unwrap().1;
    for sub in ["new", "info", "eval", "cmd", "docs", "run", "rpc"] {
        assert!(cli.contains(&format!("### `forge {sub}`")));
    }
    let scripting = &pages.iter().find(|(n, _)| *n == "scripting.md").unwrap().1;
    assert!(scripting.contains("`run(id)`") && scripting.contains("`rand(seed)`"));
}

#[test]
fn forge_docs_writes_and_checks() {
    let d = std::env::temp_dir().join(format!("forge-docs-test-{}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    let out = d.to_str().unwrap();
    let forge = || {
        let mut cmd = Command::cargo_bin("forge").unwrap();
        cmd.env("FORGE_NO_PLUGINS", "1");
        cmd
    };
    // Nothing there yet: the check fails and names the fix.
    let failed = forge()
        .args(["docs", "--check", "--out", out])
        .output()
        .unwrap();
    assert_eq!(failed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("forge docs --out"));
    forge().args(["docs", "--out", out]).assert().success();
    forge()
        .args(["docs", "--check", "--out", out])
        .assert()
        .success();
    // Editing a page makes the check fail again.
    std::fs::write(d.join("cli.md"), "edited").unwrap();
    let failed = forge()
        .args(["docs", "--check", "--out", out])
        .output()
        .unwrap();
    assert_eq!(failed.status.code(), Some(1));
    // `forge commands --markdown` prints the commands page.
    let md = forge().args(["commands", "--markdown"]).output().unwrap();
    assert_eq!(
        String::from_utf8(md.stdout).unwrap().trim_end(),
        std::fs::read_to_string(d.join("commands.md"))
            .unwrap()
            .trim_end()
    );
    std::fs::remove_dir_all(&d).ok();
}

// ---- the manual (docs/manual, brief 183) ----

fn manual_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual")
}

/// Link targets of `[text](target)` in a markdown source, fragments and queries removed;
/// external links and pure anchors are skipped.
fn local_links(md: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = md;
    while let Some(i) = rest.find("](") {
        rest = &rest[i + 2..];
        let Some(end) = rest.find(')') else { break };
        let target = rest[..end].split(['#', '?']).next().unwrap_or("");
        if !target.is_empty() && !target.contains("://") && !target.starts_with("mailto:") {
            out.push(target.to_string());
        }
        rest = &rest[end..];
    }
    out
}

fn markdown_files(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        let rel = format!("{prefix}{name}");
        if entry.file_type().unwrap().is_dir() {
            markdown_files(&entry.path(), &format!("{rel}/"), out);
        } else if name.ends_with(".md") {
            out.push(rel);
        }
    }
}

#[test]
fn every_summary_entry_resolves() {
    let summary = std::fs::read_to_string(manual_dir().join("SUMMARY.md")).unwrap();
    let links = local_links(&summary);
    assert!(links.len() >= 10, "SUMMARY.md lists too few pages");
    for link in &links {
        assert!(
            manual_dir().join(link).is_file(),
            "SUMMARY.md links to {link}, which does not exist"
        );
    }
    // The generated references are part of the book.
    for (name, _) in docs::generate() {
        assert!(
            links.contains(&format!("reference/{name}")),
            "reference/{name} is not listed in SUMMARY.md"
        );
    }
}

#[test]
fn every_page_is_in_the_summary_and_its_links_resolve() {
    let summary = std::fs::read_to_string(manual_dir().join("SUMMARY.md")).unwrap();
    let listed = local_links(&summary);
    let mut pages = Vec::new();
    markdown_files(&manual_dir(), "", &mut pages);
    for page in pages.iter().filter(|p| *p != "SUMMARY.md") {
        assert!(
            listed.contains(page),
            "{page} is in docs/manual but not in SUMMARY.md (mdBook would not show it)"
        );
        let path = manual_dir().join(page);
        let text = std::fs::read_to_string(&path).unwrap();
        let base = path.parent().unwrap();
        for link in local_links(&text) {
            if link.ends_with(".md") {
                assert!(
                    base.join(&link).is_file(),
                    "{page} links to {link}, which does not exist"
                );
            }
        }
    }
}

#[test]
fn the_book_config_points_at_this_folder() {
    let cfg = std::fs::read_to_string(manual_dir().join("book.toml")).unwrap();
    assert!(cfg.contains("src = \".\""));
    assert!(manual_dir().join("introduction.md").is_file());
}

#[test]
fn the_pages_workflow_deploys_only_from_main() {
    let wf = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/pages.yml"),
    )
    .unwrap();
    assert!(wf.contains("pull_request:"), "build on pull requests");
    assert!(wf.contains("workflow_dispatch:"));
    assert!(
        wf.contains("github.ref == 'refs/heads/main'"),
        "the deploy job must be restricted to main"
    );
    assert!(!wf.contains("secrets."), "no secrets");
    assert!(wf.contains("contents: read"), "minimal permissions");
}
