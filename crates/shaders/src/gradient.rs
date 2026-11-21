//! Colour gradients (knots with an interpolation) used by several shaders (S2).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interp {
    None,
    Linear,
    /// Smoothstep between neighbouring knots (the observed default, `gradient-u`).
    Cubic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    /// Sorted by position; colours linear.
    pub knots: Vec<(f64, [f64; 3])>,
    pub interp: Interp,
}

impl Default for Gradient {
    /// Black at 0 to white at 1, cubic.
    fn default() -> Self {
        Gradient {
            knots: vec![(0.0, [0.0; 3]), (1.0, [1.0; 3])],
            interp: Interp::Cubic,
        }
    }
}

impl Gradient {
    pub fn eval(&self, t: f64) -> [f64; 3] {
        let k = &self.knots;
        match k.len() {
            0 => return [0.0; 3],
            1 => return k[0].1,
            _ => {}
        }
        if t <= k[0].0 {
            return k[0].1;
        }
        let last = k[k.len() - 1];
        if t >= last.0 {
            return last.1;
        }
        let i = k.iter().rposition(|(p, _)| *p <= t).unwrap_or(0);
        let (p0, c0) = k[i];
        let (p1, c1) = k[i + 1];
        let mut x = if p1 > p0 { (t - p0) / (p1 - p0) } else { 1.0 };
        match self.interp {
            Interp::None => x = 0.0,
            Interp::Linear => {}
            Interp::Cubic => x = x * x * (3.0 - 2.0 * x),
        }
        std::array::from_fn(|c| c0[c] + (c1[c] - c0[c]) * x)
    }
}
