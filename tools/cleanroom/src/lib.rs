//! Clean-room gate: finds forbidden trademarks, product names and reference-app file
//! types in the repository. Behaviour is specified in docs/specs/INFRA.md §2.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

/// Directory names never descended into.
const SKIP_DIRS: [&str; 2] = ["target", ".git"];

/// Parsed `forbidden.txt`. All entries are stored ASCII-lowercased.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
    pub terms: Vec<String>,
    pub words: Vec<String>,
    pub extensions: Vec<String>,
    pub excludes: Vec<String>,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Term,
    Word,
    Extension,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Term => "term",
            Kind::Word => "word",
            Kind::Extension => "extension",
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
    if cfg.is_excluded(rel) {
        return Ok(());
    }
    for (_, kind, entry) in line_matches(rel, cfg) {
        out.push(violation(0, 0, kind, &entry));
    }
    let bytes = fs::read(path)?;
    for (i, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
        for (col, kind, entry) in line_matches(line, cfg) {
            out.push(violation(i + 1, col, kind, &entry));
        }
    }
    Ok(())
}
