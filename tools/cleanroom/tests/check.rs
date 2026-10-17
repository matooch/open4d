//! Behaviour tests for docs/specs/INFRA.md §2. Synthetic terms only: real forbidden
//! terms would trip the repo scan.

use std::fs;
use std::path::Path;

use forge_cleanroom::{Config, Kind, line_matches, scan};

const CFG: &str = "\
# synthetic config
term acme corp
word xyz
ext badext
exclude docs/
";

fn cfg() -> Config {
    Config::parse(CFG).expect("valid config")
}

fn hits(line: &str) -> Vec<(usize, Kind, String)> {
    line_matches(line, &cfg())
}

#[test]
fn parses_all_directives() {
    let c = cfg();
    assert_eq!(c.terms, vec!["acme corp"]);
    assert_eq!(c.words, vec!["xyz"]);
    assert_eq!(c.extensions, vec!["badext"]);
    assert_eq!(c.excludes, vec!["docs/"]);
}

#[test]
fn rejects_unknown_directive_and_missing_argument() {
    assert!(Config::parse("forbid foo").is_err());
    assert!(Config::parse("term").is_err());
    assert!(Config::parse("word   ").is_err());
}

#[test]
fn term_is_case_insensitive_substring() {
    assert_eq!(
        hits("// made by ACME Corp."),
        vec![(12, Kind::Term, "acme corp".into())]
    );
    assert_eq!(hits("let s = \"theacme corpx\";").len(), 1);
    assert!(hits("acme  corp").is_empty(), "spacing is literal");
}

#[test]
fn word_matches_identifiers() {
    for line in [
        "xyz",
        "xyz_tool",
        "load_xyz()",
        "XyzTool",
        "myXyzThing",
        "XYZ",
        "a-xyz-b",
    ] {
        assert_eq!(hits(line).len(), 1, "expected hit in {line:?}");
    }
}

#[test]
fn word_ignores_embedded_lowercase_runs() {
    for line in ["9a1xyzf0", "xyza", "axyz", "xyz1", "1xyz"] {
        assert!(hits(line).is_empty(), "unexpected hit in {line:?}");
    }
}

#[test]
fn word_column_is_one_based_char_index() {
    assert_eq!(hits("é xyz")[0].0, 3);
}

#[test]
fn reports_every_occurrence() {
    assert_eq!(hits("xyz xyz acme corp").len(), 3);
}

fn write(root: &Path, rel: &str, body: &[u8]) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

#[test]
fn scan_finds_content_path_and_extension_violations() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "src/ok.rs", b"fn main() {}\n");
    write(
        root,
        "src/lib.rs",
        b"// line one\nconst N: &str = \"Xyz\";\n",
    );
    write(root, "src/xyz_import.rs", b"\n");
    write(root, "assets/scene.BadExt", b"\0\x01binary");
    write(root, "docs/notes.md", b"acme corp is fine in docs\n");
    write(root, "docs/raw.badext", b"");
    write(root, "target/debug/xyz.rs", b"xyz\n");
    write(root, ".git/xyz", b"xyz\n");

    let v = scan(root, &cfg()).unwrap();
    let rendered: Vec<String> = v.iter().map(ToString::to_string).collect();
    assert_eq!(
        rendered,
        vec![
            "assets/scene.BadExt:0:0: forbidden extension \"badext\"",
            "docs/raw.badext:0:0: forbidden extension \"badext\"",
            "src/lib.rs:2:18: forbidden word \"xyz\"",
            "src/xyz_import.rs:0:0: forbidden word \"xyz\"",
        ]
    );
}

#[test]
fn worktree_git_file_at_the_root_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        ".git",
        b"gitdir: /home/xyz/repo/.git/worktrees/w\n",
    );
    write(dir.path(), "sub/.git", b"xyz\n");
    let v = scan(dir.path(), &cfg()).unwrap();
    let rendered: Vec<String> = v.iter().map(ToString::to_string).collect();
    assert_eq!(rendered, vec!["sub/.git:1:1: forbidden word \"xyz\""]);
}

#[test]
fn scan_of_clean_tree_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a/b/c.rs", b"fn ok() {}\n");
    assert!(scan(dir.path(), &cfg()).unwrap().is_empty());
}

#[test]
fn scan_skips_local_tool_state() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        ".claude/worktrees/copy/CLAUDE.md",
        b"acme corp xyz\n",
    );
    write(dir.path(), "src/ok.rs", b"fn ok() {}\n");
    assert!(scan(dir.path(), &cfg()).unwrap().is_empty());
}
