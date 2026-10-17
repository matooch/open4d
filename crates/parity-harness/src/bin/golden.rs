//! Observation helper (docs/specs/INFRA.md §4.1).
//!
//! `golden info <file.obj>` — counts, face sizes, bounding box, normals/UVs.
//! `golden check [repo_root]` — validates every case under `tests/golden/`.

use std::path::Path;
use std::process::ExitCode;

use forge_parity_harness::{discover_cases, mesh_info, obj_extras, parse_obj, validate_case};

const USAGE: &str = "usage: golden info <file.obj> | golden check [repo_root]";

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
            print!("{}", mesh_info(&mesh));
            println!("normals: {}", extras.normals);
            println!("uvs: {}", extras.uvs);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{path}: {e}");
            ExitCode::from(2)
        }
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
            println!("{}/{}: {p}", case.feature, case.name);
            bad = true;
        }
    }
    if bad {
        ExitCode::from(1)
    } else {
        println!("ok: {} cases", cases.len());
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
        ["info", path] => info(path),
        ["check"] => check("."),
        ["check", root] => check(root),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
