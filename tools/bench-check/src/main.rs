//! `forge-bench-check`: run the S10 benchmarks in quick mode and compare their medians with
//! `bench/budgets.toml`. Exit 0 all within budget, 1 a regression or a missing result,
//! 2 usage or I/O error. Run from the repository root:
//!
//! ```text
//! cargo run --release -p forge-bench-check            # run + check (Linux only)
//! cargo run --release -p forge-bench-check -- --no-run --markdown   # re-check target/criterion
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use forge_bench_check::{Budgets, Verdict, all_passed, compare, read_median_us, suggest, table};

/// (package, bench target, criterion name filter). The filter keeps the viewport suite to
/// its CPU rows: the GPU rows run on whatever software adapter the runner has.
const SUITES: &[(&str, &str, Option<&str>)] = &[
    ("forge-app", "schedule", None),
    ("forge-app", "s10", None),
    ("forge-viewport", "performance", Some("^(instances|mesh)/")),
];

/// Criterion's quick mode: short warm-up and measurement; the median of the samples is
/// what is compared (the benches fix their own sample sizes, 10 for the slow ones).
const QUICK: &[&str] = &["--warm-up-time", "1", "--measurement-time", "2"];

struct Args {
    budgets: PathBuf,
    scale: f64,
    run: bool,
    force: bool,
    markdown: bool,
    suggest: bool,
    retries: u32,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        budgets: PathBuf::from("bench/budgets.toml"),
        scale: std::env::var("FORGE_BENCH_SCALE")
            .ok()
            .map(|s| {
                s.parse::<f64>()
                    .map_err(|e| format!("FORGE_BENCH_SCALE: {e}"))
            })
            .transpose()?
            .unwrap_or(1.0),
        run: true,
        force: false,
        markdown: false,
        suggest: false,
        retries: 1,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--budgets" => a.budgets = it.next().ok_or("--budgets needs a path")?.into(),
            "--scale" => {
                a.scale = it
                    .next()
                    .ok_or("--scale needs a number")?
                    .parse()
                    .map_err(|e| format!("--scale: {e}"))?
            }
            "--retries" => {
                a.retries = it
                    .next()
                    .ok_or("--retries needs a number")?
                    .parse()
                    .map_err(|e| format!("--retries: {e}"))?
            }
            "--no-run" => a.run = false,
            "--force" => a.force = true,
            "--markdown" => a.markdown = true,
            "--suggest" => a.suggest = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if !(a.scale > 0.0 && a.scale.is_finite()) {
        return Err("scale must be positive".into());
    }
    Ok(a)
}

fn criterion_dir() -> PathBuf {
    let target =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| PathBuf::from("target"), PathBuf::from);
    target.join("criterion")
}

fn run_suites() -> Result<(), String> {
    for (package, bench, filter) in SUITES {
        eprintln!("bench-check: cargo bench -p {package} --bench {bench}");
        let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.args(["bench", "-p", package, "--bench", bench, "--"]);
        cmd.args(QUICK);
        if let Some(f) = filter {
            cmd.arg(f);
        }
        let status = cmd.status().map_err(|e| format!("cargo: {e}"))?;
        if !status.success() {
            return Err(format!("cargo bench -p {package} --bench {bench} failed"));
        }
    }
    Ok(())
}

/// The lowest median seen for each budgeted id across attempts (noise only adds time).
fn collect(budgets: &Budgets, into: &mut BTreeMap<String, f64>) {
    let dir = criterion_dir();
    for b in &budgets.budgets {
        if let Ok(m) = read_median_us(&dir, &b.id) {
            let slot = into.entry(b.id.clone()).or_insert(m);
            *slot = slot.min(m);
        }
    }
}

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("bench-check: {e}");
            ExitCode::from(2)
        }
    }
}

fn real_main() -> Result<ExitCode, String> {
    let args = parse_args()?;
    let budgets = Budgets::load(&args.budgets)?;
    if args.run && !cfg!(target_os = "linux") && !args.force {
        println!(
            "bench-check: budgets are enforced on Linux only; skipped (use --force to run anyway)"
        );
        return Ok(ExitCode::SUCCESS);
    }
    let mut medians = BTreeMap::new();
    if args.run {
        // stale results from an earlier run must not satisfy a missing benchmark
        let _ = std::fs::remove_dir_all(criterion_dir());
        run_suites()?;
    }
    collect(&budgets, &mut medians);
    if args.suggest {
        let ids: Vec<_> = budgets
            .budgets
            .iter()
            .map(|b| (b.id.clone(), b.note.clone()))
            .collect();
        print!("{}", suggest(&budgets.meta, &ids, &medians));
        return Ok(ExitCode::SUCCESS);
    }
    let mut rows = compare(&budgets, &medians, args.scale);
    // a slow neighbour or a cold cache can spoil one pass: rerun the failing suites and
    // keep the best median of each benchmark
    let mut attempt = 0;
    while args.run && attempt < args.retries && rows.iter().any(|r| r.verdict == Verdict::Fail) {
        attempt += 1;
        eprintln!("bench-check: over budget, retry {attempt}/{}", args.retries);
        run_suites()?;
        collect(&budgets, &mut medians);
        rows = compare(&budgets, &medians, args.scale);
    }
    print!("{}", table(&rows, args.markdown));
    if all_passed(&rows) {
        println!(
            "bench-check: all {} benchmarks within budget (scale {})",
            rows.len(),
            args.scale
        );
        Ok(ExitCode::SUCCESS)
    } else {
        let bad = rows.iter().filter(|r| r.verdict != Verdict::Pass).count();
        println!("bench-check: {bad} benchmark(s) over budget or missing");
        Ok(ExitCode::from(1))
    }
}
