//! Level-C trajectory comparison for simulated motion (docs/specs/DYNAMICS.md §7).
//!
//! A physics engine cannot match another one sample by sample, so a trajectory (one value per
//! frame, from a `track` samples file) is checked through derived quantities, each with its own
//! threshold: the resting value, the frame of first contact, the bounce height and the slide
//! distance. A threshold that is `None` is not checked.

use crate::Samples;

/// The per-case thresholds (`[tolerance]` of a case file, or built in code by the test).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Thresholds {
    /// Absolute difference of the last values (scene units).
    pub rest_position: Option<f64>,
    /// Allowed difference of the first-contact frames (frames).
    pub first_contact_frames: Option<f64>,
    /// Relative difference of the rebound heights (`0.1` = 10 %).
    pub bounce_ratio: Option<f64>,
    /// Relative difference of the distances travelled between the first and the last value.
    pub slide_distance: Option<f64>,
}

impl Thresholds {
    /// Reads the `[tolerance]` table of a case file: keys `rest_position`,
    /// `first_contact_frames`, `bounce_ratio`, `slide_distance`. `None` without the table.
    pub fn from_case(case: &toml::Table) -> Option<Thresholds> {
        let t = case.get("tolerance")?.as_table()?;
        let num = |k: &str| {
            t.get(k)
                .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        };
        Some(Thresholds {
            rest_position: num("rest_position"),
            first_contact_frames: num("first_contact_frames"),
            bounce_ratio: num("bounce_ratio"),
            slide_distance: num("slide_distance"),
        })
    }
}

/// One checked quantity.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub name: &'static str,
    pub expected: f64,
    pub ours: f64,
    /// The threshold applied (absolute or relative as named by [`Thresholds`]).
    pub allowed: f64,
    /// The measured difference in the same measure as `allowed`.
    pub difference: f64,
}

impl Check {
    pub fn passed(&self) -> bool {
        self.difference <= self.allowed
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrajectoryReport {
    pub checks: Vec<Check>,
    /// Quantities that could not be derived from one of the tracks.
    pub missing: Vec<&'static str>,
}

impl TrajectoryReport {
    pub fn passed(&self) -> bool {
        self.missing.is_empty() && self.checks.iter().all(Check::passed)
    }

    /// One line per failed check.
    pub fn failures(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .checks
            .iter()
            .filter(|c| !c.passed())
            .map(|c| {
                format!(
                    "{}: expected {:.4}, got {:.4} (difference {:.4}, allowed {:.4})",
                    c.name, c.expected, c.ours, c.difference, c.allowed
                )
            })
            .collect();
        out.extend(self.missing.iter().map(|m| format!("{m}: not derivable")));
        out
    }
}

/// The values of a `track` samples file, one per frame in file order (frames must be
/// 0, 1, 2, …); `None` for other kinds or gaps.
pub fn track_values(s: &Samples) -> Option<Vec<f64>> {
    let Samples::Track { samples, .. } = s else {
        return None;
    };
    samples
        .iter()
        .enumerate()
        .map(|(i, (f, v))| ((f - i as f64).abs() < 1e-9).then_some(*v))
        .collect()
}

/// The first frame `f >= 2` whose step `v[f] - v[f-1]` is larger than the step before: a body
/// falling under gravity steps down by more each frame, a contact stops that. `None` if the
/// motion never decelerates.
pub fn first_contact_frame(v: &[f64]) -> Option<usize> {
    (2..v.len()).find(|&f| v[f] - v[f - 1] > v[f - 1] - v[f - 2] + 1e-9)
}

/// The rebound height: the highest value after the first contact minus the last value.
pub fn rebound_height(v: &[f64]) -> Option<f64> {
    let f = first_contact_frame(v)?;
    let peak = v[f + 1..].iter().copied().fold(f64::MIN, f64::max);
    let last = *v.last()?;
    (peak > f64::MIN).then_some(peak - last)
}

fn relative(expected: f64, ours: f64) -> f64 {
    if expected.abs() < 1e-9 {
        ours.abs()
    } else {
        (ours / expected - 1.0).abs()
    }
}

/// Compares two per-frame series through the derived quantities `th` names.
pub fn compare_trajectory(expected: &[f64], ours: &[f64], th: &Thresholds) -> TrajectoryReport {
    let mut r = TrajectoryReport::default();
    if let Some(tol) = th.rest_position {
        match (expected.last(), ours.last()) {
            (Some(e), Some(o)) => r.checks.push(Check {
                name: "rest position",
                expected: *e,
                ours: *o,
                allowed: tol,
                difference: (e - o).abs(),
            }),
            _ => r.missing.push("rest position"),
        }
    }
    if let Some(tol) = th.first_contact_frames {
        match (first_contact_frame(expected), first_contact_frame(ours)) {
            (Some(e), Some(o)) => r.checks.push(Check {
                name: "first contact frame",
                expected: e as f64,
                ours: o as f64,
                allowed: tol,
                difference: (e as f64 - o as f64).abs(),
            }),
            _ => r.missing.push("first contact frame"),
        }
    }
    if let Some(tol) = th.bounce_ratio {
        match (rebound_height(expected), rebound_height(ours)) {
            (Some(e), Some(o)) => r.checks.push(Check {
                name: "rebound height",
                expected: e,
                ours: o,
                allowed: tol,
                difference: relative(e, o),
            }),
            _ => r.missing.push("rebound height"),
        }
    }
    if let Some(tol) = th.slide_distance {
        let dist = |v: &[f64]| Some((v.last()? - v.first()?).abs());
        match (dist(expected), dist(ours)) {
            (Some(e), Some(o)) => r.checks.push(Check {
                name: "slide distance",
                expected: e,
                ours: o,
                allowed: tol,
                difference: relative(e, o),
            }),
            _ => r.missing.push("slide distance"),
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fall(rest: f64) -> Vec<f64> {
        let mut v: Vec<f64> = (0..8).map(|i| 100.0 - 2.0 * (i * i) as f64).collect();
        v.extend([rest + 3.0, rest + 1.0, rest, rest]);
        v
    }

    #[test]
    fn contact_is_the_first_deceleration() {
        // 100, 98, 92, 82, 68, 50, 28, 2 -> the step is -26 at 7, then +1 at 8.
        assert_eq!(first_contact_frame(&fall(0.0)), Some(8));
        assert_eq!(
            first_contact_frame(&[0.0, 1.0, 2.0]),
            None.or(Some(2)).filter(|_| false)
        );
    }

    #[test]
    fn identical_series_pass_and_a_shift_fails_the_rest_check() {
        let a = fall(0.0);
        let th = Thresholds {
            rest_position: Some(0.01),
            first_contact_frames: Some(0.0),
            bounce_ratio: Some(0.1),
            slide_distance: None,
        };
        assert!(compare_trajectory(&a, &a, &th).passed());
        let b = fall(0.5);
        let rep = compare_trajectory(&a, &b, &th);
        assert!(!rep.passed());
        assert_eq!(rep.failures().len(), 1, "{:?}", rep.failures());
    }

    #[test]
    fn thresholds_come_from_the_tolerance_table() {
        let t: toml::Table = "[tolerance]\nrest_position = 0.05\nbounce_ratio = 0.1\n"
            .parse()
            .unwrap();
        let th = Thresholds::from_case(&t).unwrap();
        assert_eq!(th.rest_position, Some(0.05));
        assert_eq!(th.slide_distance, None);
        assert!(Thresholds::from_case(&toml::Table::new()).is_none());
    }
}
