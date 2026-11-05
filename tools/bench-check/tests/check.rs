//! The budget rule, the comparison and the checked-in `bench/budgets.toml`.

use std::collections::BTreeMap;

use forge_bench_check::{Budgets, Verdict, all_passed, compare, median_us, round_up_3, table};

const SAMPLE: &str = r#"
[meta]
headroom = 1.2
ci_factor = 4.0
floor_us = 100.0

[[budget]]
id = "g/a"
measured_us = 1000.0
budget_us = 4800.0

[[budget]]
id = "g/tiny"
measured_us = 0.5
budget_us = 100.0
"#;

fn medians(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

#[test]
fn rounds_up_to_three_significant_digits() {
    assert_eq!(round_up_3(4800.0), 4800.0);
    assert_eq!(round_up_3(4801.0), 4810.0);
    assert!((round_up_3(0.012345) - 0.0124).abs() < 1e-12);
    assert_eq!(round_up_3(0.0), 0.0);
}

#[test]
fn the_budget_rule_has_headroom_ci_factor_and_a_floor() {
    let b = Budgets::parse(SAMPLE).unwrap();
    assert_eq!(b.meta.expected_budget_us(1000.0), 4800.0);
    assert_eq!(b.meta.expected_budget_us(0.5), 100.0);
}

#[test]
fn a_budget_under_the_headroom_or_over_the_rule_is_rejected() {
    let low = SAMPLE.replace("budget_us = 4800.0", "budget_us = 1100.0");
    assert!(Budgets::parse(&low).unwrap_err().contains("headroom"));
    let loose = SAMPLE.replace("budget_us = 4800.0", "budget_us = 9000.0");
    assert!(Budgets::parse(&loose).unwrap_err().contains("looser"));
    let dup = format!("{SAMPLE}\n[[budget]]\nid = \"g/a\"\nmeasured_us = 1.0\nbudget_us = 100.0\n");
    assert!(Budgets::parse(&dup).unwrap_err().contains("twice"));
    let weak = SAMPLE.replace("headroom = 1.2", "headroom = 1.05");
    assert!(Budgets::parse(&weak).is_err());
}

#[test]
fn medians_are_compared_with_the_scaled_budget() {
    let b = Budgets::parse(SAMPLE).unwrap();
    let ok = compare(&b, &medians(&[("g/a", 4800.0), ("g/tiny", 3.0)]), 1.0);
    assert!(all_passed(&ok));
    let slow = compare(&b, &medians(&[("g/a", 4801.0), ("g/tiny", 3.0)]), 1.0);
    assert_eq!(slow[0].verdict, Verdict::Fail);
    assert_eq!(slow[1].verdict, Verdict::Pass);
    // a slow runner: the same result passes at scale 2
    assert!(all_passed(&compare(
        &b,
        &medians(&[("g/a", 4801.0), ("g/tiny", 3.0)]),
        2.0
    )));
    // a benchmark that did not run is a failure, not a pass
    let missing = compare(&b, &medians(&[("g/a", 1.0)]), 1.0);
    assert_eq!(missing[1].verdict, Verdict::Missing);
    assert!(!all_passed(&missing));
    // results nobody budgeted are ignored
    assert_eq!(compare(&b, &medians(&[("other", 1.0)]), 1.0).len(), 2);
}

#[test]
fn reads_the_median_of_a_criterion_estimates_file() {
    let json = r#"{"mean":{"point_estimate":9.0},"median":{"point_estimate":2500000.0,"standard_error":1.0}}"#;
    assert_eq!(median_us(json).unwrap(), 2500.0);
    assert!(median_us("{}").is_err());
    assert!(median_us("not json").is_err());
}

#[test]
fn the_table_marks_failures() {
    let b = Budgets::parse(SAMPLE).unwrap();
    let rows = compare(&b, &medians(&[("g/a", 6000.0)]), 1.0);
    let text = table(&rows, false);
    assert!(text.contains("FAIL") && text.contains("MISSING"), "{text}");
    assert!(table(&rows, true).starts_with("| benchmark"));
}

#[test]
fn the_checked_in_budgets_follow_the_rule() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/budgets.toml");
    let b = Budgets::load(std::path::Path::new(path)).unwrap();
    assert!(b.meta.headroom >= 1.2, "brief 181: at least 20 % headroom");
    assert!(
        b.budgets.len() >= 15,
        "S10 coverage shrank: {} budgets",
        b.budgets.len()
    );
    // every area S10 names has a budget
    for prefix in [
        "s10_generate/",
        "s10_clones/",
        "s10_file/",
        "s10_export/",
        "s10_startup/",
        "animated_cube_plus_1000_static/",
        "cloner_10k_clones/",
        "instances/",
        "mesh/",
    ] {
        assert!(
            b.budgets.iter().any(|x| x.id.starts_with(prefix)),
            "no budget for {prefix}"
        );
    }
}
