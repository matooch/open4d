//! Benchmark budget check (brief 181, docs/specs/PERFORMANCE.md).
//!
//! The budget file lists, per criterion benchmark id, the median measured on the reference
//! machine and the budget a median may not exceed. This crate holds the pure parts: parsing,
//! the budget rule, reading criterion's `estimates.json` and comparing. `main.rs` runs cargo.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

/// The parsed `bench/budgets.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct Budgets {
    pub meta: Meta,
    #[serde(default, rename = "budget")]
    pub budgets: Vec<Budget>,
}

/// The rule that turns a measurement into a budget (see [`Meta::expected_budget_us`]).
#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    /// Slack over the reference measurement a budget must keep at least (brief 181: 20 %).
    pub headroom: f64,
    /// Extra factor for CI runners that are slower and noisier than the reference machine.
    pub ci_factor: f64,
    /// Budgets are never lower than this (microseconds): below it timer and scheduler noise
    /// dominate the median.
    pub floor_us: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Budget {
    /// Criterion id: `group/function` or `group/function/parameter`.
    pub id: String,
    /// Median on the reference machine, microseconds.
    pub measured_us: f64,
    /// The most a median may be, microseconds.
    pub budget_us: f64,
    /// What S10 requirement or spec number this guards.
    #[serde(default)]
    pub note: String,
}

impl Meta {
    /// `max(measured * headroom * ci_factor, floor_us)`, rounded up to three significant
    /// digits. The checked-in budgets must equal this (see the tests): loosening a budget
    /// means changing `ci_factor` visibly, not editing one number.
    pub fn expected_budget_us(&self, measured_us: f64) -> f64 {
        round_up_3(f64::max(
            measured_us * self.headroom * self.ci_factor,
            self.floor_us,
        ))
    }
}

/// Rounds up to three significant digits.
pub fn round_up_3(x: f64) -> f64 {
    if x <= 0.0 || !x.is_finite() {
        return x;
    }
    let mag = 10f64.powi(x.log10().floor() as i32 - 2);
    (x / mag).ceil() * mag
}

impl Budgets {
    pub fn parse(text: &str) -> Result<Budgets, String> {
        let b: Budgets = toml::from_str(text).map_err(|e| e.to_string())?;
        b.validate()?;
        Ok(b)
    }

    pub fn load(path: &Path) -> Result<Budgets, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Budgets::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Checks the file against its own rule: unique ids, finite positive numbers, at least
    /// `headroom` over the measurement, and no looser than the rule allows.
    pub fn validate(&self) -> Result<(), String> {
        let m = &self.meta;
        if !(m.headroom >= 1.2 && m.ci_factor >= 1.0 && m.floor_us > 0.0) {
            return Err(
                "meta: headroom must be at least 1.2, ci_factor at least 1, floor_us positive"
                    .into(),
            );
        }
        let mut seen = std::collections::BTreeSet::new();
        for b in &self.budgets {
            if !seen.insert(b.id.as_str()) {
                return Err(format!("{}: listed twice", b.id));
            }
            if !(b.measured_us > 0.0
                && b.budget_us > 0.0
                && b.measured_us.is_finite()
                && b.budget_us.is_finite())
            {
                return Err(format!("{}: numbers must be positive and finite", b.id));
            }
            if b.budget_us < b.measured_us * m.headroom {
                return Err(format!(
                    "{}: budget {} us is under measured {} us x headroom {}",
                    b.id, b.budget_us, b.measured_us, m.headroom
                ));
            }
            let allowed = m.expected_budget_us(b.measured_us);
            if b.budget_us > allowed {
                return Err(format!(
                    "{}: budget {} us is looser than the rule allows ({} us)",
                    b.id, b.budget_us, allowed
                ));
            }
        }
        Ok(())
    }
}

/// The median point estimate of a criterion `estimates.json`, in microseconds.
pub fn median_us(estimates_json: &str) -> Result<f64, String> {
    let v: serde_json::Value = serde_json::from_str(estimates_json).map_err(|e| e.to_string())?;
    let ns = v
        .pointer("/median/point_estimate")
        .and_then(serde_json::Value::as_f64)
        .ok_or("no median.point_estimate")?;
    Ok(ns / 1000.0)
}

/// Reads `<criterion_dir>/<id>/new/estimates.json`.
pub fn read_median_us(criterion_dir: &Path, id: &str) -> Result<f64, String> {
    let path = criterion_dir.join(id).join("new").join("estimates.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    median_us(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    /// The median exceeds the (scaled) budget.
    Fail,
    /// No result for this id: the benchmark did not run or was renamed.
    Missing,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub id: String,
    pub median_us: Option<f64>,
    pub measured_us: f64,
    pub budget_us: f64,
    pub verdict: Verdict,
}

impl Row {
    /// Median over budget (1.0 is exactly at the limit).
    pub fn used(&self) -> Option<f64> {
        self.median_us.map(|m| m / self.budget_us)
    }
}

/// Compares medians with budgets. `scale` multiplies every budget (a slow runner); results
/// for ids not in the file are ignored, ids in the file without a result are `Missing`.
pub fn compare(budgets: &Budgets, medians: &BTreeMap<String, f64>, scale: f64) -> Vec<Row> {
    budgets
        .budgets
        .iter()
        .map(|b| {
            let median = medians.get(&b.id).copied();
            let limit = b.budget_us * scale;
            let verdict = match median {
                None => Verdict::Missing,
                Some(m) if m > limit => Verdict::Fail,
                Some(_) => Verdict::Pass,
            };
            Row {
                id: b.id.clone(),
                median_us: median,
                measured_us: b.measured_us,
                budget_us: limit,
                verdict,
            }
        })
        .collect()
}

pub fn all_passed(rows: &[Row]) -> bool {
    rows.iter().all(|r| r.verdict == Verdict::Pass)
}

/// Human readable time: `1.23 s`, `4.56 ms`, `7.89 us`.
pub fn fmt_us(us: f64) -> String {
    if us >= 1_000_000.0 {
        format!("{:.2} s", us / 1_000_000.0)
    } else if us >= 1000.0 {
        format!("{:.2} ms", us / 1000.0)
    } else {
        format!("{us:.2} us")
    }
}

/// A plain text table (or markdown with `markdown`).
pub fn table(rows: &[Row], markdown: bool) -> String {
    let mut out = String::new();
    let head = [
        "benchmark",
        "median",
        "reference",
        "budget",
        "used",
        "result",
    ];
    let cells: Vec<[String; 6]> = rows
        .iter()
        .map(|r| {
            [
                r.id.clone(),
                r.median_us.map_or("-".into(), fmt_us),
                fmt_us(r.measured_us),
                fmt_us(r.budget_us),
                r.used()
                    .map_or("-".into(), |u| format!("{:.0} %", u * 100.0)),
                match r.verdict {
                    Verdict::Pass => "ok",
                    Verdict::Fail => "FAIL",
                    Verdict::Missing => "MISSING",
                }
                .into(),
            ]
        })
        .collect();
    if markdown {
        out.push_str(&format!(
            "| {} |\n|---|---|---|---|---|---|\n",
            head.join(" | ")
        ));
        for c in &cells {
            out.push_str(&format!("| `{}` | {} |\n", c[0], c[1..].join(" | ")));
        }
        return out;
    }
    let mut w = head.map(str::len);
    for c in &cells {
        for (i, s) in c.iter().enumerate() {
            w[i] = w[i].max(s.len());
        }
    }
    let line = |cols: [&str; 6]| {
        let mut s = format!("{:<w0$}", cols[0], w0 = w[0]);
        for i in 1..6 {
            s.push_str(&format!("  {:>wi$}", cols[i], wi = w[i]));
        }
        s.push('\n');
        s
    };
    out.push_str(&line(head));
    for c in &cells {
        out.push_str(&line([&c[0], &c[1], &c[2], &c[3], &c[4], &c[5]]));
    }
    out
}

/// Budget-file text for fresh medians, following the rule in `meta`: what to paste after a
/// deliberate re-measurement.
pub fn suggest(meta: &Meta, ids: &[(String, String)], medians: &BTreeMap<String, f64>) -> String {
    let mut out = String::new();
    for (id, note) in ids {
        let Some(m) = medians.get(id) else { continue };
        out.push_str(&format!(
            "[[budget]]\nid = \"{id}\"\nmeasured_us = {:.3}\nbudget_us = {}\nnote = \"{note}\"\n\n",
            m,
            meta.expected_budget_us(*m)
        ));
    }
    out
}
