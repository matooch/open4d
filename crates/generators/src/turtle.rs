//! The Turtle mode of the clone-toolset spline generator (spec S1): an L-system string is
//! rewritten and then walked by a 3D turtle that leaves open polylines (chains).

use glam::DVec3;

/// The most characters a rewritten string may have.
const MAX_LEN: usize = 2_000_000;

/// The Turtle tab's settings (angles in degrees, percentages as fractions).
#[derive(Debug, Clone)]
pub struct TurtleSettings {
    pub premise: String,
    pub rules: String,
    pub iterations: usize,
    pub angle_deg: f64,
    pub movement: f64,
    pub tropism: f64,
    pub move_multiplier: f64,
    pub randomness: f64,
}

/// Parses `X=string` lines into (predecessor, successor); the first line of a character wins.
fn parse_rules(text: &str) -> Result<Vec<(char, String)>, &'static str> {
    let mut rules: Vec<(char, String)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((lhs, rhs)) = line.split_once('=') else {
            return Err("Turtle rule without '='");
        };
        let mut chars = lhs.trim().chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return Err("Turtle context rule");
        };
        if !rules.iter().any(|(p, _)| *p == c) {
            rules.push((c, rhs.trim().to_string()));
        }
    }
    Ok(rules)
}

/// The premise rewritten `iterations` times.
pub fn rewrite(premise: &str, rules_text: &str, iterations: usize) -> Result<String, &'static str> {
    let rules = parse_rules(rules_text)?;
    let mut current = premise.trim().to_string();
    for _ in 0..iterations {
        let mut next = String::new();
        for c in current.chars() {
            match rules.iter().find(|(p, _)| *p == c) {
                Some((_, s)) => next.push_str(s),
                None => next.push(c),
            }
            if next.len() > MAX_LEN {
                return Err("Turtle string too long");
            }
        }
        current = next;
    }
    Ok(current)
}

#[derive(Clone)]
struct State {
    pos: DVec3,
    heading: DVec3,
    up: DVec3,
    movement: f64,
    /// The chain the turtle extends (its last point is `pos`), if any.
    chain: Option<usize>,
}

/// Rotates `v` about the unit `axis` by `angle` (right-handed).
fn rotate(v: DVec3, axis: DVec3, angle: f64) -> DVec3 {
    let (s, c) = angle.sin_cos();
    v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0 - c))
}

/// Reads an optional `(x)` after position `i` of `chars`; returns the value and the next index.
fn argument(chars: &[char], i: usize) -> Result<(Option<f64>, usize), &'static str> {
    if chars.get(i) != Some(&'(') {
        return Ok((None, i));
    }
    let end = chars[i..]
        .iter()
        .position(|c| *c == ')')
        .ok_or("Turtle unclosed (")?
        + i;
    let text: String = chars[i + 1..end].iter().collect();
    let v = text
        .trim()
        .parse::<f64>()
        .map_err(|_| "Turtle bad number")?;
    Ok((Some(v), end + 1))
}

/// Walks the string and returns the chains (open polylines, in creation order).
pub fn walk(s: &TurtleSettings, string: &str) -> Result<Vec<Vec<DVec3>>, &'static str> {
    let chars: Vec<char> = string.chars().collect();
    let mut chains: Vec<Vec<DVec3>> = Vec::new();
    let mut st = State {
        pos: DVec3::ZERO,
        heading: DVec3::Z,
        up: DVec3::Y,
        movement: s.movement,
        chain: None,
    };
    let mut stack: Vec<State> = Vec::new();
    let tropism_dir = DVec3::new(0.0, -1.0, 0.0);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let (arg, next) = argument(&chars, i + 1)?;
        i = next;
        let angle = arg.unwrap_or(s.angle_deg).to_radians();
        let left = st.up.cross(st.heading);
        match c {
            'F' | 'f' => {
                let step = arg.unwrap_or(st.movement);
                let from = st.pos;
                st.pos += st.heading * step;
                if c == 'F' {
                    let id = *st.chain.get_or_insert_with(|| {
                        chains.push(vec![from]);
                        chains.len() - 1
                    });
                    chains[id].push(st.pos);
                } else {
                    st.chain = None;
                }
            }
            '+' | '-' => {
                let a = if c == '+' { angle } else { -angle };
                st.heading = rotate(st.heading, st.up, a);
            }
            '&' | '^' => {
                let a = if c == '&' { angle } else { -angle };
                st.heading = rotate(st.heading, left, a);
                st.up = rotate(st.up, left, a);
            }
            '/' | '\\' => {
                let a = if c == '/' { angle } else { -angle };
                st.up = rotate(st.up, st.heading, a);
            }
            '|' => st.heading = rotate(st.heading, st.up, std::f64::consts::PI),
            '[' => {
                stack.push(st.clone());
                st.chain = None;
            }
            ']' => st = stack.pop().ok_or("Turtle unbalanced ]")?,
            '"' => st.movement *= s.move_multiplier,
            // The scale (width) has no effect on the polyline.
            '!' => {}
            'T' => {
                let target = (st.heading + tropism_dir * (0.2 * s.tropism)).normalize();
                let axis = st.heading.cross(target);
                if axis.length() > 1e-12 {
                    let axis = axis.normalize();
                    let a = st.heading.dot(target).clamp(-1.0, 1.0).acos();
                    st.heading = target;
                    st.up = rotate(st.up, axis, a);
                }
            }
            c if c.is_ascii_alphabetic() => {
                if arg.is_some() {
                    return Err("Turtle argument after placeholder");
                }
            }
            c if c.is_whitespace() => {}
            _ => return Err("Turtle unknown symbol"),
        }
    }
    Ok(chains)
}

/// Rewrites and walks: the chains of the settings.
pub fn chains(s: &TurtleSettings) -> Result<Vec<Vec<DVec3>>, &'static str> {
    if s.randomness > 0.0 {
        return Err("Turtle randomness");
    }
    let string = rewrite(&s.premise, &s.rules, s.iterations)?;
    walk(s, &string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(premise: &str, rules: &str, iterations: usize) -> TurtleSettings {
        TurtleSettings {
            premise: premise.into(),
            rules: rules.into(),
            iterations,
            angle_deg: 90.0,
            movement: 10.0,
            tropism: 0.0,
            move_multiplier: 0.5,
            randomness: 0.0,
        }
    }

    #[test]
    fn rewriting_replaces_every_character_at_once() {
        assert_eq!(rewrite("A", "A=AB\nB=A", 3).unwrap(), "ABAAB");
        assert_eq!(rewrite("F", "", 5).unwrap(), "F");
    }

    #[test]
    fn a_straight_run_is_one_chain_and_f_lowercase_breaks_it() {
        let s = settings("FFfFF", "", 0);
        let chains = chains(&s).unwrap();
        assert_eq!(chains.len(), 2);
        assert_eq!(chains[0].len(), 3);
        assert!((chains[1][0] - DVec3::new(0.0, 0.0, 30.0)).length() < 1e-12);
    }

    #[test]
    fn brackets_start_new_chains_and_restore_the_parent() {
        // Up the Y axis, a branch to the side, then the trunk goes on.
        let s = settings("^F[&F]F", "", 0);
        let chains = chains(&s).unwrap();
        assert_eq!(chains.len(), 2);
        assert_eq!(chains[0].len(), 3, "trunk continues after the bracket");
        assert_eq!(chains[1].len(), 2);
        let up = DVec3::new(0.0, 10.0, 0.0);
        assert!((chains[1][0] - up).length() < 1e-9);
        // `&` 90 degrees tilts the heading from +Y towards +Z.
        assert!((chains[1][1] - (up + DVec3::new(0.0, 0.0, 10.0))).length() < 1e-9);
    }

    #[test]
    fn the_quote_scales_the_movement_and_pop_restores_it() {
        let s = settings("F\"F[\"F]F", "", 0);
        let c = &chains(&s).unwrap();
        let steps: Vec<f64> = c[0].windows(2).map(|w| (w[1] - w[0]).length()).collect();
        assert!((steps[0] - 10.0).abs() < 1e-12);
        assert!((steps[1] - 5.0).abs() < 1e-12);
        assert!(
            (steps[2] - 5.0).abs() < 1e-12,
            "the bracket's own scaling is popped"
        );
        assert!(((c[1][1] - c[1][0]).length() - 2.5).abs() < 1e-12);
    }

    #[test]
    fn tropism_bends_the_heading_towards_minus_y() {
        let mut s = settings("^(45)FTF", "", 0);
        s.tropism = 1.0;
        let c = &chains(&s).unwrap()[0];
        assert!(c[2].y - c[1].y < c[1].y - c[0].y - 1e-6);
        assert!(((c[2] - c[1]).length() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn unsupported_input_is_an_error() {
        assert!(chains(&settings("F?", "", 0)).is_err());
        assert!(chains(&settings("F]", "", 0)).is_err());
        assert!(chains(&settings("A", "A<B>C=F", 1)).is_err());
        assert!(chains(&settings("A", "A=AA", 40)).is_err());
        let mut s = settings("F", "", 0);
        s.randomness = 0.1;
        assert!(chains(&s).is_err());
    }
}
