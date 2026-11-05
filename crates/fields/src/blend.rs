//! Layer blend modes of a field list (docs/specs/FIELDS.md F7).

/// How a layer combines with the layers below it. `base` is the value accumulated from the
/// layers below, `top` the layer's own value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// The layer replaces what is below.
    #[default]
    Normal,
    Max,
    Min,
    /// `base + top`, limited to 1.
    Add,
    /// `base - top`, limited to 0.
    Subtract,
    Multiply,
    /// `2 base top` where `base < 0.5`, else `1 - 2 (1 - base)(1 - top)`.
    Overlay,
    /// `base + top - base top`.
    Screen,
    /// Observed equal to Min (**parity-pending**: the cases cannot tell them apart).
    Clip,
}

/// `(id, label)` of every blend mode: the options of a field list row, Normal first.
pub const BLEND_OPTIONS: [(&str, &str); 9] = [
    ("normal", "Normal"),
    ("max", "Max"),
    ("min", "Min"),
    ("add", "Add"),
    ("subtract", "Subtract"),
    ("multiply", "Multiply"),
    ("overlay", "Overlay"),
    ("screen", "Screen"),
    ("clip", "Clip"),
];

impl BlendMode {
    const ALL: [BlendMode; 9] = [
        BlendMode::Normal,
        BlendMode::Max,
        BlendMode::Min,
        BlendMode::Add,
        BlendMode::Subtract,
        BlendMode::Multiply,
        BlendMode::Overlay,
        BlendMode::Screen,
        BlendMode::Clip,
    ];

    /// The option id of this mode (a row's `blend`).
    pub fn id(self) -> &'static str {
        BLEND_OPTIONS[Self::ALL.iter().position(|m| *m == self).unwrap_or(0)].0
    }

    /// The mode of an option id; `None` for an unknown id.
    pub fn from_id(id: &str) -> Option<BlendMode> {
        BLEND_OPTIONS
            .iter()
            .position(|(i, _)| *i == id)
            .map(|k| Self::ALL[k])
    }

    /// The blended value of one channel, before the layer strength is applied.
    pub fn apply(self, base: f64, top: f64) -> f64 {
        match self {
            BlendMode::Normal => top,
            BlendMode::Max => base.max(top),
            BlendMode::Min | BlendMode::Clip => base.min(top),
            BlendMode::Add => (base + top).clamp(0.0, 1.0),
            BlendMode::Subtract => (base - top).clamp(0.0, 1.0),
            BlendMode::Multiply => base * top,
            BlendMode::Overlay => {
                if base < 0.5 {
                    2.0 * base * top
                } else {
                    1.0 - 2.0 * (1.0 - base) * (1.0 - top)
                }
            }
            BlendMode::Screen => base + top - base * top,
        }
    }

    /// The channel after the layer at `strength`: `base + (blend(base, top) - base) * strength`.
    pub fn mix(self, base: f64, top: f64, strength: f64) -> f64 {
        base + (self.apply(base, top) - base) * strength
    }
}
