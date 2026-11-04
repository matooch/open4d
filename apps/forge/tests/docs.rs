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
