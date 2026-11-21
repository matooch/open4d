//! Blend modes of the Layer and Fusion shaders (SHADERS.md S5).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Add,
    Subtract,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    Difference,
}

impl BlendMode {
    pub fn parse(s: &str) -> Option<BlendMode> {
        Some(match s {
            "normal" => BlendMode::Normal,
            "add" => BlendMode::Add,
            "subtract" => BlendMode::Subtract,
            "multiply" => BlendMode::Multiply,
            "screen" => BlendMode::Screen,
            "overlay" => BlendMode::Overlay,
            "darken" => BlendMode::Darken,
            "lighten" => BlendMode::Lighten,
            "difference" => BlendMode::Difference,
            _ => return None,
        })
    }

    /// `base + (blend(base, top) - base) * opacity` per component.
    pub fn apply(self, base: [f64; 3], top: [f64; 3], opacity: f64) -> [f64; 3] {
        std::array::from_fn(|i| {
            let (a, b) = (base[i], top[i]);
            let r = match self {
                BlendMode::Normal => b,
                BlendMode::Add => a + b,
                BlendMode::Subtract => a - b,
                BlendMode::Multiply => a * b,
                BlendMode::Screen => a + b - a * b,
                BlendMode::Overlay => {
                    if a < 0.5 {
                        2.0 * a * b
                    } else {
                        1.0 - 2.0 * (1.0 - a) * (1.0 - b)
                    }
                }
                BlendMode::Darken => a.min(b),
                BlendMode::Lighten => a.max(b),
                BlendMode::Difference => (a - b).abs(),
            };
            a + (r - a) * opacity
        })
    }
}
