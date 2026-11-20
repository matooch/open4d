//! Behaviour tests for docs/specs/INFRA.md §2. Synthetic terms only: real forbidden
//! terms would trip the repo scan.

use std::fs;
use std::path::Path;

use forge_cleanroom::{Config, Kind, api_matches, line_matches, scan};

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

// ---- Scoped api-* rules (docs/specs/INFRA.md §2) ----

const API_CFG: &str = "\
exclude docs/
api-scope docs/parity/
api-scope tests/golden/
api-prefix zz.
api-prefix CallThing
api-prefix FMT_
api-call Get
api-call Set
api-tick Oxyz
api-tick-prefix FL
api-id id
api-id parameter
api-bignum 44000
";

fn api_cfg() -> Config {
    Config::parse(API_CFG).expect("valid config")
}

fn api(line: &str) -> Vec<String> {
    api_matches(line, &api_cfg())
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

#[test]
fn api_directives_parse_and_validate() {
    let c = api_cfg();
    assert_eq!(c.api_scopes, vec!["docs/parity/", "tests/golden/"]);
    assert_eq!(c.api_prefixes, vec!["zz.", "CallThing", "FMT_"]);
    assert_eq!(c.api_calls, vec!["Get", "Set"]);
    assert_eq!(c.api_ticks, vec!["Oxyz"]);
    assert_eq!(c.api_tick_prefixes, vec!["FL"]);
    assert_eq!(c.api_ids, vec!["id", "parameter"]);
    assert_eq!(c.api_bignums, vec!["44000"]);
    assert!(Config::parse("api-scope").is_err());
    assert!(Config::parse("api-bignum abc").is_err());
    assert!(Config::parse("api-bignum 123456789").is_err());
}

#[test]
fn api_prefix_matches_tokens_case_sensitively() {
    assert_eq!(api("calls zz.Thing(1) here"), vec!["zz.Thing"]);
    assert_eq!(api("`CallThing` with the id"), vec!["CallThing"]);
    assert_eq!(api("flag FMT_BIN_EXPORT set"), vec!["FMT_BIN_EXPORT"]);
    // Not at a token start, or wrong case: no hit.
    assert!(api("REFMT_X and xzz.y").is_empty());
    assert!(api("fmt_bin and callthing").is_empty());
    // Our own ALL_CAPS words are not flagged.
    assert!(api("status KNOWN_MISMATCH, CURRENT_CASE, TEXCOORD_0").is_empty());
}

#[test]
fn api_call_needs_prefix_upper_and_open_paren() {
    assert_eq!(api("then GetThing() returns"), vec!["GetThing("]);
    assert_eq!(api("SetWeightMap(a)"), vec!["SetWeightMap("]);
    assert!(api("GetThing without parens").is_empty());
    assert!(api("getThing() and Getting() and Get()").is_empty());
    assert!(api("forgetMe(x) is ours").is_empty());
}

#[test]
fn api_tick_only_in_backticks() {
    assert_eq!(api("resolves `Oxyz` only"), vec!["Oxyz"]);
    assert!(api("the Oxyz object").is_empty());
    assert!(api("`Oxyzzy` and `oxyz`").is_empty());
}

#[test]
fn api_tick_prefix_needs_lowercase_after_prefix() {
    assert_eq!(api("the layer `FLcolorize` drops"), vec!["FLcolorize"]);
    assert_eq!(api("`FLinvert2`"), vec!["FLinvert2"]);
    assert!(api("the layer FLcolorize unquoted").is_empty());
    assert!(api("`FLAT` and `FL` and `Flat` and `Flower`").is_empty());
    assert!(api("`FLcolor-x`").is_empty());
}

#[test]
fn api_id_needs_keyword_and_four_digits() {
    assert_eq!(api("### Spline (id 1008982)"), vec!["id 1008982"]);
    assert_eq!(api("Bone (ID 2008) = Null"), vec!["ID 2008"]);
    assert_eq!(api("hidden parameter 6019 read"), vec!["parameter 6019"]);
    assert_eq!(api("Parameter 1003."), vec!["Parameter 1003"]);
    // Too short, wrong shape, or not a whole number: no hit.
    assert!(api("id 100 and id 999").is_empty());
    assert!(api("id: 1234 and id1234 and valid 1234").is_empty());
    assert!(api("id 1234.5 and id 1234x").is_empty());
    assert!(api("(id / count) with 1000 points").is_empty());
}

#[test]
fn api_bignum_is_nine_digits_not_a_fraction() {
    assert_eq!(api("Enabled 440000265 shown"), vec!["440000265"]);
    assert_eq!(api("(440000301)"), vec!["440000301"]);
    assert!(api("440000265.5 and 0.440000265").is_empty());
    assert_eq!(api("it was 440000265."), vec!["440000265"]);
    assert!(api("0.440000265").is_empty());
    assert!(api("4400002650 and 44000026 and 999999999").is_empty());
    assert!(api("x 340000265 y").is_empty());
}

fn api_scan(files: &[(&str, &[u8])]) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    for (rel, body) in files {
        write(dir.path(), rel, body);
    }
    scan(dir.path(), &api_cfg())
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn api_rules_apply_in_excluded_scope_for_md_toml_json_only() {
    let leak = b"line\nSpline (id 1008982)\n";
    let got = api_scan(&[
        ("docs/parity/a.md", leak),
        ("tests/golden/x/case.toml", leak),
        ("tests/golden/x/samples.json", leak),
        ("tests/golden/x/mesh.obj", leak),
        ("tests/golden/x/render.png", leak),
        ("docs/parity/a.txt", leak),
    ]);
    assert_eq!(
        got,
        vec![
            "docs/parity/a.md:2:9: forbidden api \"id 1008982\"",
            "tests/golden/x/case.toml:2:9: forbidden api \"id 1008982\"",
            "tests/golden/x/samples.json:2:9: forbidden api \"id 1008982\"",
        ]
    );
}

#[test]
fn api_rules_do_not_apply_outside_scope() {
    let leak = b"zz.thing id 1008982 GetThing(\n";
    let got = api_scan(&[
        ("docs/specs/a.md", leak),
        ("docs/notes.md", leak),
        ("src/lib.rs", leak),
        ("crates/x/README.md", leak),
    ]);
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn api_scope_does_not_enable_term_checks_in_excluded_paths() {
    // `xyz` is a forbidden word, but docs/ stays excluded from term checks.
    let cfg = Config::parse(&format!("{CFG}api-scope docs/parity/\napi-id id\n")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "docs/parity/a.md", b"acme corp xyz\nid 12345\n");
    let got: Vec<String> = scan(dir.path(), &cfg)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        got,
        vec!["docs/parity/a.md:2:1: forbidden api \"id 12345\""]
    );
}

#[test]
fn api_scope_in_non_excluded_path_still_gets_term_checks() {
    let cfg = Config::parse("word xyz\napi-scope tests/golden/\napi-id id\n").unwrap();
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "tests/golden/a/case.toml", b"xyz id 4321\n");
    let got: Vec<String> = scan(dir.path(), &cfg)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        got,
        vec![
            "tests/golden/a/case.toml:1:1: forbidden word \"xyz\"",
            "tests/golden/a/case.toml:1:5: forbidden api \"id 4321\"",
        ]
    );
}
