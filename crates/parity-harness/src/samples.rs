//! Level-B golden cases: numeric `samples.json` files (docs/specs/INFRA.md §5).

use serde_json::Value;

/// One clone as recorded by the observer (cloner space).
#[derive(Debug, Clone, PartialEq)]
pub struct Clone {
    pub pos: [f64; 3],
    pub x: [f64; 3],
    pub y: [f64; 3],
    pub z: [f64; 3],
    pub color: [f64; 3],
    pub weight: f64,
    pub visible: bool,
}

/// The content of one samples file.
#[derive(Debug, Clone, PartialEq)]
pub enum Samples {
    /// `(frame, value)` pairs of one animated parameter.
    Track {
        param: String,
        fps: f64,
        tolerance: f64,
        samples: Vec<(f64, f64)>,
    },
    /// `(point, value)` pairs of a field.
    Field {
        tolerance: f64,
        samples: Vec<([f64; 3], f64)>,
    },
    /// Clone data of a cloner.
    Clones { tolerance: f64, clones: Vec<Clone> },
}

impl Samples {
    /// The `kind` string of the file.
    pub fn kind(&self) -> &'static str {
        match self {
            Samples::Track { .. } => "track",
            Samples::Field { .. } => "field",
            Samples::Clones { .. } => "clones",
        }
    }

    /// The tolerance recorded in the file.
    pub fn tolerance(&self) -> f64 {
        match self {
            Samples::Track { tolerance, .. }
            | Samples::Field { tolerance, .. }
            | Samples::Clones { tolerance, .. } => *tolerance,
        }
    }

    /// Number of samples (track and field) or clones.
    pub fn count(&self) -> usize {
        match self {
            Samples::Track { samples, .. } => samples.len(),
            Samples::Field { samples, .. } => samples.len(),
            Samples::Clones { clones, .. } => clones.len(),
        }
    }
}

fn field<'a>(obj: &'a Value, name: &str) -> Result<&'a Value, String> {
    obj.get(name)
        .ok_or_else(|| format!("missing field {name:?}"))
}

fn number(v: &Value, what: &str) -> Result<f64, String> {
    v.as_f64()
        .ok_or_else(|| format!("{what}: expected a number"))
}

fn num_field(obj: &Value, name: &str) -> Result<f64, String> {
    number(field(obj, name)?, name)
}

fn array<'a>(v: &'a Value, what: &str) -> Result<&'a Vec<Value>, String> {
    v.as_array()
        .ok_or_else(|| format!("{what}: expected an array"))
}

fn vec3(v: &Value, what: &str) -> Result<[f64; 3], String> {
    let a = array(v, what)?;
    if a.len() != 3 {
        return Err(format!("{what}: expected 3 numbers, got {}", a.len()));
    }
    Ok([
        number(&a[0], what)?,
        number(&a[1], what)?,
        number(&a[2], what)?,
    ])
}

fn pair<'a>(v: &'a Value, what: &str) -> Result<&'a [Value], String> {
    let a = array(v, what)?;
    if a.len() != 2 {
        return Err(format!("{what}: expected 2 items, got {}", a.len()));
    }
    Ok(a)
}

fn parse_clone(v: &Value, what: &str) -> Result<Clone, String> {
    let get = |name: &str| -> Result<&Value, String> {
        v.get(name)
            .ok_or_else(|| format!("{what}: missing field {name:?}"))
    };
    let vec = |name: &str| vec3(get(name)?, &format!("{what}.{name}"));
    Ok(Clone {
        pos: vec("pos")?,
        x: vec("x")?,
        y: vec("y")?,
        z: vec("z")?,
        color: vec("color")?,
        weight: number(get("weight")?, &format!("{what}.weight"))?,
        visible: get("visible")?
            .as_bool()
            .ok_or_else(|| format!("{what}.visible: expected a boolean"))?,
    })
}

/// Parses a samples file; dispatches on `kind`. Errors name the offending field.
/// Semantic rules (order, finiteness, tolerance) are in [`validate_samples`].
pub fn parse_samples(src: &str) -> Result<Samples, String> {
    let root: Value = serde_json::from_str(src).map_err(|e| format!("invalid JSON: {e}"))?;
    let kind = field(&root, "kind")?
        .as_str()
        .ok_or("kind: expected a string")?;
    match kind {
        "track" => {
            let param = field(&root, "param")?
                .as_str()
                .ok_or("param: expected a string")?
                .to_string();
            let fps = num_field(&root, "fps")?;
            let tolerance = num_field(&root, "tolerance")?;
            let mut samples = Vec::new();
            for (i, s) in array(field(&root, "samples")?, "samples")?
                .iter()
                .enumerate()
            {
                let what = format!("samples[{i}]");
                let p = pair(s, &what)?;
                samples.push((number(&p[0], &what)?, number(&p[1], &what)?));
            }
            Ok(Samples::Track {
                param,
                fps,
                tolerance,
                samples,
            })
        }
        "field" => {
            let tolerance = num_field(&root, "tolerance")?;
            let mut samples = Vec::new();
            for (i, s) in array(field(&root, "samples")?, "samples")?
                .iter()
                .enumerate()
            {
                let what = format!("samples[{i}]");
                let p = pair(s, &what)?;
                samples.push((vec3(&p[0], &what)?, number(&p[1], &what)?));
            }
            Ok(Samples::Field { tolerance, samples })
        }
        "clones" => {
            let tolerance = num_field(&root, "tolerance")?;
            let mut clones = Vec::new();
            for (i, c) in array(field(&root, "clones")?, "clones")?.iter().enumerate() {
                clones.push(parse_clone(c, &format!("clones[{i}]"))?);
            }
            Ok(Samples::Clones { tolerance, clones })
        }
        other => Err(format!(
            "kind: unknown kind {other:?} (expected track, field or clones)"
        )),
    }
}

fn finite(problems: &mut Vec<String>, what: &str, values: &[f64]) {
    if values.iter().any(|v| !v.is_finite()) {
        problems.push(format!("{what}: not finite"));
    }
}

/// The rules `golden check` applies to a parsed file (docs/specs/INFRA.md §5.2).
/// Empty means valid.
pub fn validate_samples(s: &Samples) -> Vec<String> {
    let mut problems = Vec::new();
    match s {
        Samples::Track {
            fps,
            tolerance,
            samples,
            ..
        } => {
            finite(&mut problems, "fps", &[*fps]);
            finite(&mut problems, "tolerance", &[*tolerance]);
            for (i, (f, v)) in samples.iter().enumerate() {
                finite(&mut problems, &format!("samples[{i}]"), &[*f, *v]);
                if i > 0 && f.partial_cmp(&samples[i - 1].0) != Some(std::cmp::Ordering::Greater) {
                    problems.push(format!(
                        "samples[{i}]: frame {f} does not follow frame {} (frames must strictly increase)",
                        samples[i - 1].0
                    ));
                }
            }
        }
        Samples::Field { tolerance, samples } => {
            finite(&mut problems, "tolerance", &[*tolerance]);
            if tolerance.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
                problems.push(format!("tolerance {tolerance} must be > 0"));
            }
            for (i, (p, v)) in samples.iter().enumerate() {
                finite(
                    &mut problems,
                    &format!("samples[{i}]"),
                    &[p[0], p[1], p[2], *v],
                );
            }
        }
        Samples::Clones { tolerance, clones } => {
            finite(&mut problems, "tolerance", &[*tolerance]);
            if tolerance.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
                problems.push(format!("tolerance {tolerance} must be > 0"));
            }
            for (i, c) in clones.iter().enumerate() {
                let mut all = Vec::new();
                for v in [&c.pos, &c.x, &c.y, &c.z, &c.color] {
                    all.extend_from_slice(v);
                }
                all.push(c.weight);
                finite(&mut problems, &format!("clones[{i}]"), &all);
            }
        }
    }
    problems
}

fn fmt3(v: &[f64; 3]) -> String {
    format!("({}, {}, {})", v[0], v[1], v[2])
}

fn min_max(values: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    values.fold(None, |acc, v| match acc {
        None => Some((v, v)),
        Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
    })
}

/// Text printed by `golden info` for a samples file.
pub fn samples_info(s: &Samples) -> String {
    let mut out = format!("kind: {}\n", s.kind());
    match s {
        Samples::Track {
            param,
            fps,
            samples,
            ..
        } => {
            out += &format!("param: {param}\nfps: {fps}\nsamples: {}\n", samples.len());
            if let (Some(first), Some(last)) = (samples.first(), samples.last()) {
                out += &format!("frames: {} .. {}\n", first.0, last.0);
            }
            if let Some((lo, hi)) = min_max(samples.iter().map(|s| s.1)) {
                out += &format!("value min: {lo}\nvalue max: {hi}\n");
            }
        }
        Samples::Field { samples, .. } => {
            out += &format!("samples: {}\n", samples.len());
            if let Some((lo, hi)) = min_max(samples.iter().map(|s| s.1)) {
                out += &format!("value min: {lo}\nvalue max: {hi}\n");
            }
        }
        Samples::Clones { clones, .. } => {
            out += &format!("clones: {}\n", clones.len());
            let invisible = clones.iter().filter(|c| !c.visible).count();
            out += &format!("invisible: {invisible}\n");
            if !clones.is_empty() {
                let mut lo = [f64::INFINITY; 3];
                let mut hi = [f64::NEG_INFINITY; 3];
                for c in clones {
                    for k in 0..3 {
                        lo[k] = lo[k].min(c.pos[k]);
                        hi[k] = hi[k].max(c.pos[k]);
                    }
                }
                out += &format!("position min: {}\nposition max: {}\n", fmt3(&lo), fmt3(&hi));
            }
        }
    }
    out += &format!("tolerance: {}\n", s.tolerance());
    out
}

/// At most this many failures are listed in a [`SamplesReport`].
pub const MAX_FAILURES: usize = 10;

/// Slack for sample positions (frames, field points).
pub const POSITION_EPSILON: f64 = 1e-9;

/// Result of [`compare_samples`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SamplesReport {
    /// Number of failures found (may exceed `failures.len()`).
    pub failure_count: usize,
    /// The first [`MAX_FAILURES`] failures, human-readable.
    pub failures: Vec<String>,
    /// Largest value error among compared samples.
    pub max_error: f64,
}

impl SamplesReport {
    pub fn passed(&self) -> bool {
        self.failure_count == 0
    }

    fn fail(&mut self, msg: String) {
        self.failure_count += 1;
        if self.failures.len() < MAX_FAILURES {
            self.failures.push(msg);
        }
    }
}

fn max_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// Compares an observed file against ours. Same kind and same sample positions are
/// required; values compare within the **expected** file's tolerance (absolute);
/// clone visibility and the clone count compare exactly. (docs/specs/INFRA.md §5.4)
pub fn compare_samples(expected: &Samples, actual: &Samples) -> SamplesReport {
    let mut r = SamplesReport::default();
    let tol = expected.tolerance();
    // Err is NaN-safe: a NaN error never passes.
    let ok = |e: f64| e <= tol;
    match (expected, actual) {
        (
            Samples::Track {
                param: ep,
                fps: ef,
                samples: e,
                ..
            },
            Samples::Track {
                param: ap,
                fps: af,
                samples: a,
                ..
            },
        ) => {
            if ep != ap {
                r.fail(format!("param: expected {ep:?}, got {ap:?}"));
            }
            if (ef - af)
                .abs()
                .partial_cmp(&POSITION_EPSILON)
                .is_none_or(|o| o == std::cmp::Ordering::Greater)
            {
                r.fail(format!("fps: expected {ef}, got {af}"));
            }
            if e.len() != a.len() {
                r.fail(format!(
                    "sample count: expected {}, got {}",
                    e.len(),
                    a.len()
                ));
                return r;
            }
            for (i, (es, as_)) in e.iter().zip(a).enumerate() {
                if (es.0 - as_.0)
                    .abs()
                    .partial_cmp(&POSITION_EPSILON)
                    .is_none_or(|o| o == std::cmp::Ordering::Greater)
                {
                    r.fail(format!(
                        "sample {i}: expected frame {}, got frame {}",
                        es.0, as_.0
                    ));
                    continue;
                }
                let err = (es.1 - as_.1).abs();
                r.max_error = r.max_error.max(err);
                if !ok(err) {
                    r.fail(format!(
                        "frame {}: expected {}, got {} (error {err:e} > tolerance {tol:e})",
                        es.0, es.1, as_.1
                    ));
                }
            }
        }
        (Samples::Field { samples: e, .. }, Samples::Field { samples: a, .. }) => {
            if e.len() != a.len() {
                r.fail(format!(
                    "sample count: expected {}, got {}",
                    e.len(),
                    a.len()
                ));
                return r;
            }
            for (i, (es, as_)) in e.iter().zip(a).enumerate() {
                if max_diff(&es.0, &as_.0)
                    .partial_cmp(&POSITION_EPSILON)
                    .is_none_or(|o| o == std::cmp::Ordering::Greater)
                {
                    r.fail(format!(
                        "sample {i}: expected point {}, got point {}",
                        fmt3(&es.0),
                        fmt3(&as_.0)
                    ));
                    continue;
                }
                let err = (es.1 - as_.1).abs();
                r.max_error = r.max_error.max(err);
                if !ok(err) {
                    r.fail(format!(
                        "point {i} {}: expected {}, got {} (error {err:e} > tolerance {tol:e})",
                        fmt3(&es.0),
                        es.1,
                        as_.1
                    ));
                }
            }
        }
        (Samples::Clones { clones: e, .. }, Samples::Clones { clones: a, .. }) => {
            if e.len() != a.len() {
                r.fail(format!(
                    "clone count: expected {}, got {}",
                    e.len(),
                    a.len()
                ));
                return r;
            }
            for (i, (ec, ac)) in e.iter().zip(a).enumerate() {
                let parts: [(&str, f64); 6] = [
                    ("pos", max_diff(&ec.pos, &ac.pos)),
                    ("x", max_diff(&ec.x, &ac.x)),
                    ("y", max_diff(&ec.y, &ac.y)),
                    ("z", max_diff(&ec.z, &ac.z)),
                    ("color", max_diff(&ec.color, &ac.color)),
                    ("weight", (ec.weight - ac.weight).abs()),
                ];
                for (name, err) in parts {
                    r.max_error = r.max_error.max(err);
                    if !ok(err) {
                        let (ev, av) = match name {
                            "pos" => (fmt3(&ec.pos), fmt3(&ac.pos)),
                            "x" => (fmt3(&ec.x), fmt3(&ac.x)),
                            "y" => (fmt3(&ec.y), fmt3(&ac.y)),
                            "z" => (fmt3(&ec.z), fmt3(&ac.z)),
                            "color" => (fmt3(&ec.color), fmt3(&ac.color)),
                            _ => (ec.weight.to_string(), ac.weight.to_string()),
                        };
                        r.fail(format!(
                            "clone {i} {name}: expected {ev}, got {av} (error {err:e} > tolerance {tol:e})"
                        ));
                    }
                }
                if ec.visible != ac.visible {
                    r.fail(format!(
                        "clone {i} visible: expected {}, got {}",
                        ec.visible, ac.visible
                    ));
                }
            }
        }
        (e, a) => r.fail(format!("kind: expected {}, got {}", e.kind(), a.kind())),
    }
    r
}
