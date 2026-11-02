//! Observation helper (docs/specs/INFRA.md §4.1).
//!
//! `golden info <file.obj | samples.json | case-dir>` — mesh counts, face sizes, bounding
//! box, normals/UVs; samples kind, count and value range.
//! `golden check [repo_root]` — validates every case under `tests/golden/`.

use std::path::Path;
use std::process::ExitCode;

use forge_parity_harness::{
    cases_without_output, discover_cases, mesh_info, obj_extras, parse_obj, parse_samples,
    samples_info, validate_case,
};

/// Writes to stdout; a reader that closed the pipe early (`golden check | head -1`) is a
/// quiet exit, like apps/forge, not a panic.
fn emit(args: std::fmt::Arguments<'_>, newline: bool) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let result = out.write_fmt(args).and_then(|()| {
        if newline {
            out.write_all(b"\n")
        } else {
            Ok(())
        }
    });
    match result {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => std::process::exit(0),
        Err(e) => {
            eprintln!("error: cannot write to stdout: {e}");
            std::process::exit(2);
        }
    }
}

macro_rules! out {
    ($($arg:tt)*) => { emit(format_args!($($arg)*), false) };
}

macro_rules! outln {
    () => { emit(format_args!(""), true) };
    ($($arg:tt)*) => { emit(format_args!($($arg)*), true) };
}

const USAGE: &str =
    "usage: golden info <file.obj | samples.json | case-dir> | golden check [repo_root]";

fn info(path: &str) -> ExitCode {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(2);
        }
    };
    match parse_obj(&src) {
        Ok(mesh) => {
            let extras = obj_extras(&src);
            out!("{}", mesh_info(&mesh));
            outln!("normals: {}", extras.normals);
            outln!("uvs: {}", extras.uvs);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{path}: {e}");
            ExitCode::from(2)
        }
    }
}

fn info_samples(path: &str) -> ExitCode {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(2);
        }
    };
    match parse_samples(&src) {
        Ok(s) => {
            out!("{}", samples_info(&s));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{path}: {e}");
            ExitCode::from(2)
        }
    }
}

/// Info for every mesh and samples file of a case directory.
fn info_dir(dir: &str) -> ExitCode {
    let dir = Path::new(dir);
    let mut files: Vec<String> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".obj") || n == "samples.json" || n.starts_with("samples-"))
            .filter(|n| n.ends_with(".obj") || n.ends_with(".json"))
            .collect(),
        Err(e) => {
            eprintln!("{}: {e}", dir.display());
            return ExitCode::from(2);
        }
    };
    files.sort();
    let mut code = ExitCode::SUCCESS;
    for (i, name) in files.iter().enumerate() {
        if i > 0 {
            outln!();
        }
        outln!("== {name}");
        let path = dir.join(name).to_string_lossy().into_owned();
        let one = if name.ends_with(".obj") {
            info(&path)
        } else {
            info_samples(&path)
        };
        if one != ExitCode::SUCCESS {
            code = one;
        }
    }
    code
}

fn info_any(path: &str) -> ExitCode {
    if Path::new(path).is_dir() {
        info_dir(path)
    } else if path.ends_with(".json") {
        info_samples(path)
    } else {
        info(path)
    }
}

fn check(root: &str) -> ExitCode {
    let root = Path::new(root);
    let cases = match discover_cases(&root.join("tests/golden")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: {e}", root.join("tests/golden").display());
            return ExitCode::from(2);
        }
    };
    let mut bad = false;
    for case in &cases {
        for p in validate_case(root, case) {
            outln!("{}/{}: {p}", case.feature, case.name);
            bad = true;
        }
    }
    if bad {
        ExitCode::from(1)
    } else {
        outln!("ok: {} cases", cases.len());
        for name in cases_without_output(&cases) {
            outln!("note: {name} has no mesh or samples file (tree or form only)");
        }
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["info", path] => info_any(path),
        ["check"] => check("."),
        ["check", root] => check(root),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
