//! Clean-room gate: finds forbidden trademarks, product names and reference-app file
//! types in the repository. Behaviour is specified in docs/specs/INFRA.md §2.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

/// Directory names never descended into.
const SKIP_DIRS: [&str; 3] = ["target", ".git", ".claude"];

/// File extensions the scoped `api-*` rules apply to (never `.obj` or images).
const API_EXTENSIONS: [&str; 3] = ["md", "toml", "json"];

/// Parsed `forbidden.txt`. `term`/`word`/`ext` entries are stored ASCII-lowercased; the
/// `api-*` entries keep their case (except `api_ids`, which match case-insensitively).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
    pub terms: Vec<String>,
    pub words: Vec<String>,
    pub extensions: Vec<String>,
    pub excludes: Vec<String>,
    /// `api-scope`: path prefixes whose md/toml/json files get the `api-*` rules.
    pub api_scopes: Vec<String>,
    /// `api-prefix`: case-sensitive token-start prefixes.
    pub api_prefixes: Vec<String>,
    /// `api-call`: accessor-call prefixes (`Get` + `Upper...` + `(`).
    pub api_calls: Vec<String>,
    /// `api-tick`: exact names forbidden as a whole backtick-quoted token.
    pub api_ticks: Vec<String>,
    /// `api-tick-prefix`: backtick-quoted tokens `<P>` + lowercase letter + alphanumerics.
    pub api_tick_prefixes: Vec<String>,
    /// `api-id`: keywords (stored lowercased) followed by a space and 4+ digits.
    pub api_ids: Vec<String>,
    /// `api-bignum`: digit prefixes of forbidden 9-digit numbers.
    pub api_bignums: Vec<String>,
    /// `api-key-suffix`: minimum digit count of a trailing `_<digits>` on a `.toml` key.
    pub api_key_suffix: Option<usize>,
    /// `api-key-field`: minimum digit count of a `.toml` key that is `p` plus digits only.
    pub api_key_field: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "config line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn parse(src: &str) -> Result<Config, ConfigError> {
        let mut cfg = Config::default();
        for (i, raw) in src.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (directive, arg) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
            let arg = arg.trim();
            let err = |message: String| ConfigError {
                line: i + 1,
                message,
            };
            if arg.is_empty() {
                return Err(err(format!("`{directive}` needs an argument")));
            }
            let value = arg.to_ascii_lowercase();
            match directive {
                "term" => cfg.terms.push(value),
                "word" => cfg.words.push(value),
                "ext" => cfg
                    .extensions
                    .push(value.trim_start_matches('.').to_string()),
                // Paths keep their case; they are matched against real file names.
                "exclude" => cfg.excludes.push(arg.to_string()),
                "api-scope" => cfg.api_scopes.push(arg.to_string()),
                "api-prefix" => cfg.api_prefixes.push(arg.to_string()),
                "api-call" => cfg.api_calls.push(arg.to_string()),
                "api-tick" => cfg.api_ticks.push(arg.to_string()),
                "api-tick-prefix" => cfg.api_tick_prefixes.push(arg.to_string()),
                "api-id" => cfg.api_ids.push(value),
                "api-bignum" => {
                    if !arg.chars().all(|c| c.is_ascii_digit()) || arg.len() >= 9 {
                        return Err(err("`api-bignum` needs fewer than 9 digits".into()));
                    }
                    cfg.api_bignums.push(arg.to_string());
                }
                "api-key-suffix" => match arg.parse::<usize>() {
                    Ok(n) if n >= 1 => cfg.api_key_suffix = Some(n),
                    _ => {
                        return Err(err(
                            "`api-key-suffix` needs a digit count of 1 or more".into()
                        ));
                    }
                },
                "api-key-field" => match arg.parse::<usize>() {
                    Ok(n) if n >= 1 => cfg.api_key_field = Some(n),
                    _ => {
                        return Err(err(
                            "`api-key-field` needs a digit count of 1 or more".into()
                        ));
                    }
                },
                other => return Err(err(format!("unknown directive `{other}`"))),
            }
        }
        Ok(cfg)
    }

    pub fn load(path: &Path) -> io::Result<Config> {
        let src = fs::read_to_string(path)?;
        Config::parse(&src).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn is_excluded(&self, rel: &str) -> bool {
        self.excludes.iter().any(|p| rel.starts_with(p.as_str()))
    }

    /// True when `rel` is an md/toml/json file under an `api-scope` prefix.
    fn is_api_checked(&self, rel: &str) -> bool {
        let ext = rel.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
        ext.is_some_and(|e| API_EXTENSIONS.contains(&e.as_str()))
            && self.api_scopes.iter().any(|p| rel.starts_with(p.as_str()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Term,
    Word,
    Extension,
    Api,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Term => "term",
            Kind::Word => "word",
            Kind::Extension => "extension",
            Kind::Api => "api",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    /// Repo-relative, `/`-separated.
    pub path: String,
    /// 1-based; 0 for path and extension hits.
    pub line: usize,
    /// 1-based char index; 0 for path and extension hits.
    pub col: usize,
    pub kind: Kind,
    pub entry: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: forbidden {} \"{}\"",
            self.path, self.line, self.col, self.kind, self.entry
        )
    }
}

/// All `term`/`word` hits in `line` as `(1-based column, kind, entry)`, ordered by column.
pub fn line_matches(line: &str, cfg: &Config) -> Vec<(usize, Kind, String)> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    for (kind, entries) in [(Kind::Term, &cfg.terms), (Kind::Word, &cfg.words)] {
        for entry in entries {
            let pat: Vec<char> = entry.chars().collect();
            if pat.is_empty() || pat.len() > chars.len() {
                continue;
            }
            for s in 0..=chars.len() - pat.len() {
                let e = s + pat.len();
                let hit = chars[s..e]
                    .iter()
                    .zip(&pat)
                    .all(|(c, p)| c.to_ascii_lowercase() == *p);
                if hit && (kind == Kind::Term || at_word_boundary(&chars, s, e)) {
                    out.push((s + 1, kind, entry.clone()));
                }
            }
        }
    }
    out.sort();
    out
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// True when `needle` occurs in `chars` at `at`.
fn starts_with_at(chars: &[char], at: usize, needle: &str) -> bool {
    needle
        .chars()
        .enumerate()
        .all(|(k, n)| chars.get(at + k) == Some(&n))
}

/// Scoped API/ID hits in `line` (INFRA.md §2) as `(1-based column, matched text)`,
/// ordered by column.
pub fn api_matches(line: &str, cfg: &Config) -> Vec<(usize, String)> {
    let chars: Vec<char> = line.chars().collect();
    let text = |s: usize, e: usize| chars[s..e].iter().collect::<String>();
    let token_start = |s: usize| s == 0 || !is_ident(chars[s - 1]);
    let mut out = Vec::new();
    for s in 0..chars.len() {
        // api-prefix: the whole identifier-ish run after the prefix is reported.
        if token_start(s) {
            for p in &cfg.api_prefixes {
                if starts_with_at(&chars, s, p) {
                    let mut e = s + p.chars().count();
                    while e < chars.len() && is_ident(chars[e]) {
                        e += 1;
                    }
                    out.push((s + 1, text(s, e)));
                }
            }
            // api-call: Prefix + Upper + ident chars + '('.
            for p in &cfg.api_calls {
                if starts_with_at(&chars, s, p) {
                    let mut e = s + p.chars().count();
                    if chars.get(e).is_some_and(char::is_ascii_uppercase) {
                        while e < chars.len() && is_ident(chars[e]) {
                            e += 1;
                        }
                        if chars.get(e) == Some(&'(') {
                            out.push((s + 1, text(s, e + 1)));
                        }
                    }
                }
            }
            // api-id: keyword, one space, 4+ digits not followed by digit . or letter.
            for k in &cfg.api_ids {
                let n = k.chars().count();
                let kw_hit = chars.len() >= s + n
                    && chars[s..s + n]
                        .iter()
                        .zip(k.chars())
                        .all(|(c, p)| c.to_ascii_lowercase() == p);
                if kw_hit && chars.get(s + n) == Some(&' ') {
                    let mut e = s + n + 1;
                    while chars.get(e).is_some_and(char::is_ascii_digit) {
                        e += 1;
                    }
                    let digits = e - (s + n + 1);
                    let next_ok = chars.get(e).is_none_or(|c| {
                        !(c.is_ascii_alphanumeric()
                            || (*c == '.' && chars.get(e + 1).is_some_and(char::is_ascii_digit)))
                    });
                    if digits >= 4 && next_ok {
                        out.push((s + 1, text(s, e)));
                    }
                }
            }
        }
        // api-tick / api-tick-prefix: a whole backtick-quoted token.
        if chars[s] == '`' && (s == 0 || chars[s - 1] != '`') {
            let mut e = s + 1;
            while e < chars.len() && is_ident(chars[e]) {
                e += 1;
            }
            if chars.get(e) == Some(&'`') && e > s + 1 {
                let tok = text(s + 1, e);
                let prefixed = cfg.api_tick_prefixes.iter().any(|p| {
                    tok.strip_prefix(p.as_str()).is_some_and(|rest| {
                        rest.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                            && rest.chars().all(|c| c.is_ascii_alphanumeric())
                    })
                });
                if prefixed || cfg.api_ticks.contains(&tok) {
                    out.push((s + 2, tok));
                }
            }
        }
        // api-bignum: exactly 9 digits, not inside a longer number or a fraction.
        if chars[s].is_ascii_digit()
            && (s == 0 || !(chars[s - 1].is_ascii_digit() || chars[s - 1] == '.'))
        {
            let mut e = s;
            while chars.get(e).is_some_and(char::is_ascii_digit) {
                e += 1;
            }
            let next_ok = chars
                .get(e)
                .is_none_or(|c| *c != '.' || !chars.get(e + 1).is_some_and(char::is_ascii_digit));
            if e - s == 9 && next_ok && cfg.api_bignums.iter().any(|p| starts_with_at(&chars, s, p))
            {
                out.push((s + 1, text(s, e)));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// `api-key-suffix` / `api-key-field` hit on a `.toml` line: a key (bare or quoted, before
/// `=`) that ends in `_<digits>` with at least the configured number of digits, or that is
/// `p` followed only by at least the configured number of digits. Returns
/// `(1-based column, key)`. Table headers and values are not looked at.
pub fn api_key_matches(line: &str, cfg: &Config) -> Option<(usize, String)> {
    if cfg.api_key_suffix.is_none() && cfg.api_key_field.is_none() {
        return None;
    }
    let trimmed = line.trim_start();
    let indent = line.chars().count() - trimmed.chars().count();
    let (key_part, _) = trimmed.split_once('=')?;
    let key = key_part.trim_end();
    let bare = key
        .strip_prefix('"')
        .and_then(|k| k.strip_suffix('"'))
        .unwrap_or(key);
    if bare.is_empty() || !bare.chars().all(|c| is_ident(c) || c == '-' || c == '.') {
        return None;
    }
    let quote = usize::from(key.starts_with('"'));
    let hit = (indent + 1 + quote, bare.to_string());
    if let Some(min) = cfg.api_key_field
        && let Some(digits) = bare.strip_prefix('p')
        && digits.len() >= min
        && digits.chars().all(|c| c.is_ascii_digit())
    {
        return Some(hit);
    }
    let min = cfg.api_key_suffix?;
    let (stem, digits) = bare.rsplit_once('_')?;
    if !stem.is_empty() && digits.len() >= min && digits.chars().all(|c| c.is_ascii_digit()) {
        return Some(hit);
    }
    None
}

fn at_word_boundary(chars: &[char], s: usize, e: usize) -> bool {
    let start_ok = s == 0 || {
        let prev = chars[s - 1];
        !prev.is_ascii_alphanumeric()
            || ((prev.is_ascii_lowercase() || prev.is_ascii_digit())
                && chars[s].is_ascii_uppercase())
    };
    let end_ok = e == chars.len() || {
        let next = chars[e];
        !next.is_ascii_alphanumeric() || next.is_ascii_uppercase()
    };
    start_ok && end_ok
}

/// Scans every file under `root` (see module docs). Results are sorted.
pub fn scan(root: &Path, cfg: &Config) -> io::Result<Vec<Violation>> {
    let mut out = Vec::new();
    walk(root, "", cfg, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, prefix: &str, cfg: &Config, out: &mut Vec<Violation>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = format!("{prefix}{name}");
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !SKIP_DIRS.contains(&name.as_str()) {
                walk(&entry.path(), &format!("{rel}/"), cfg, out)?;
            }
        } else if file_type.is_file() && !(prefix.is_empty() && name == ".git") {
            // A root `.git` *file* is a git worktree's pointer to its metadata.
            check_file(&entry.path(), &rel, &name, cfg, out)?;
        }
    }
    Ok(())
}

fn check_file(
    path: &Path,
    rel: &str,
    name: &str,
    cfg: &Config,
    out: &mut Vec<Violation>,
) -> io::Result<()> {
    let violation = |line, col, kind, entry: &str| Violation {
        path: rel.to_string(),
        line,
        col,
        kind,
        entry: entry.to_string(),
    };
    if let Some((_, ext)) = name.rsplit_once('.') {
        let ext = ext.to_ascii_lowercase();
        if let Some(bad) = cfg.extensions.iter().find(|e| **e == ext) {
            out.push(violation(0, 0, Kind::Extension, bad));
        }
    }
    let excluded = cfg.is_excluded(rel);
    let api = cfg.is_api_checked(rel);
    if excluded && !api {
        return Ok(());
    }
    if !excluded {
        for (_, kind, entry) in line_matches(rel, cfg) {
            out.push(violation(0, 0, kind, &entry));
        }
    }
    let bytes = fs::read(path)?;
    for (i, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
        if !excluded {
            for (col, kind, entry) in line_matches(line, cfg) {
                out.push(violation(i + 1, col, kind, &entry));
            }
        }
        if api {
            for (col, entry) in api_matches(line, cfg) {
                out.push(violation(i + 1, col, Kind::Api, &entry));
            }
            if rel.ends_with(".toml")
                && let Some((col, key)) = api_key_matches(line, cfg)
            {
                out.push(violation(i + 1, col, Kind::Api, &key));
            }
        }
    }
    Ok(())
}
