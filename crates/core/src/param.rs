//! Parameter values and descriptors (docs/specs/CORE.md §1).

use serde::{Deserialize, Serialize};

use crate::{Category, NodeId};

/// The only representation of a parameter value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Vec3([f64; 3]),
    /// Linear RGB, each component finite and ≥ 0.
    Color([f64; 3]),
    /// An option id of an enum parameter.
    Enum(String),
    String(String),
    Link(Option<NodeId>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParamKind {
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    Float {
        min: f64,
        max: f64,
    },
    /// Range applies per component.
    Vec3 {
        min: f64,
        max: f64,
    },
    Color,
    Enum {
        options: Vec<EnumOption>,
    },
    String,
    /// Accepted target categories; empty accepts any.
    Link {
        accepts: Vec<Category>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Unit {
    #[default]
    None,
    Distance,
    Angle,
    Percent,
    Time,
}

/// Presentation only; never affects validation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiHint {
    pub group: String,
    pub unit: Unit,
    pub soft_min: Option<f64>,
    pub soft_max: Option<f64>,
    pub step: Option<f64>,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDesc {
    /// Stable serialization key.
    pub id: String,
    pub label: String,
    pub kind: ParamKind,
    pub default: Value,
    pub ui: UiHint,
    pub animatable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ParamError {
    #[error("value has the wrong type for this parameter")]
    TypeMismatch,
    #[error("value is not finite")]
    NotFinite,
    #[error("value is out of range")]
    OutOfRange,
    #[error("unknown enum option")]
    UnknownOption,
}

/// Numeric types accepted by [`ParamDesc::range`].
pub trait RangeValue: Copy {
    fn as_f64(self) -> f64;
    fn as_i64(self) -> i64;
}

macro_rules! range_value {
    ($($t:ty),*) => {$(
        impl RangeValue for $t {
            fn as_f64(self) -> f64 { self as f64 }
            fn as_i64(self) -> i64 { self as i64 }
        }
    )*};
}
range_value!(i32, i64, f32, f64);

impl ParamDesc {
    fn new(id: &str, label: &str, kind: ParamKind, default: Value) -> ParamDesc {
        let animatable = matches!(
            kind,
            ParamKind::Bool
                | ParamKind::Int { .. }
                | ParamKind::Float { .. }
                | ParamKind::Vec3 { .. }
                | ParamKind::Color
        );
        ParamDesc {
            id: id.to_string(),
            label: label.to_string(),
            kind,
            default,
            ui: UiHint::default(),
            animatable,
        }
    }

    pub fn bool(id: &str, label: &str, default: bool) -> ParamDesc {
        Self::new(id, label, ParamKind::Bool, Value::Bool(default))
    }

    pub fn int(id: &str, label: &str, default: i64) -> ParamDesc {
        let kind = ParamKind::Int {
            min: i64::MIN,
            max: i64::MAX,
        };
        Self::new(id, label, kind, Value::Int(default))
    }

    pub fn float(id: &str, label: &str, default: f64) -> ParamDesc {
        let kind = ParamKind::Float {
            min: f64::MIN,
            max: f64::MAX,
        };
        Self::new(id, label, kind, Value::Float(default))
    }

    pub fn vec3(id: &str, label: &str, default: [f64; 3]) -> ParamDesc {
        let kind = ParamKind::Vec3 {
            min: f64::MIN,
            max: f64::MAX,
        };
        Self::new(id, label, kind, Value::Vec3(default))
    }

    pub fn color(id: &str, label: &str, default: [f64; 3]) -> ParamDesc {
        Self::new(id, label, ParamKind::Color, Value::Color(default))
    }

    pub fn enumeration(
        id: &str,
        label: &str,
        default_id: &str,
        options: &[(&str, &str)],
    ) -> ParamDesc {
        let options = options
            .iter()
            .map(|(id, label)| EnumOption {
                id: id.to_string(),
                label: label.to_string(),
            })
            .collect();
        let kind = ParamKind::Enum { options };
        Self::new(id, label, kind, Value::Enum(default_id.to_string()))
    }

    pub fn string(id: &str, label: &str, default: &str) -> ParamDesc {
        Self::new(
            id,
            label,
            ParamKind::String,
            Value::String(default.to_string()),
        )
    }

    pub fn link(id: &str, label: &str, accepts: &[Category]) -> ParamDesc {
        let kind = ParamKind::Link {
            accepts: accepts.to_vec(),
        };
        Self::new(id, label, kind, Value::Link(None))
    }

    /// Hard range for `Int`, `Float` and `Vec3` parameters.
    ///
    /// # Panics
    /// On other kinds, or if `min > max`.
    pub fn range(mut self, min: impl RangeValue, max: impl RangeValue) -> ParamDesc {
        match &mut self.kind {
            ParamKind::Int { min: lo, max: hi } => {
                (*lo, *hi) = (min.as_i64(), max.as_i64());
                assert!(lo <= hi, "range: min > max for `{}`", self.id);
            }
            ParamKind::Float { min: lo, max: hi } | ParamKind::Vec3 { min: lo, max: hi } => {
                (*lo, *hi) = (min.as_f64(), max.as_f64());
                assert!(lo <= hi, "range: min > max for `{}`", self.id);
            }
            _ => panic!("range() on non-numeric parameter `{}`", self.id),
        }
        self
    }

    pub fn unit(mut self, unit: Unit) -> ParamDesc {
        self.ui.unit = unit;
        self
    }

    pub fn group(mut self, group: &str) -> ParamDesc {
        self.ui.group = group.to_string();
        self
    }

    pub fn soft_range(mut self, min: f64, max: f64) -> ParamDesc {
        (self.ui.soft_min, self.ui.soft_max) = (Some(min), Some(max));
        self
    }

    pub fn step(mut self, step: f64) -> ParamDesc {
        self.ui.step = Some(step);
        self
    }

    pub fn hidden(mut self) -> ParamDesc {
        self.ui.hidden = true;
        self
    }

    pub fn animatable(mut self, animatable: bool) -> ParamDesc {
        self.animatable = animatable;
        self
    }

    /// Type-checks `value` and clamps numbers into the hard range. Link targets are
    /// checked by the scene.
    pub fn validate(&self, value: Value) -> Result<Value, ParamError> {
        let finite = |f: f64| {
            if f.is_finite() {
                Ok(f)
            } else {
                Err(ParamError::NotFinite)
            }
        };
        match (&self.kind, value) {
            (ParamKind::Bool, v @ Value::Bool(_)) => Ok(v),
            (ParamKind::Int { min, max }, Value::Int(i)) => Ok(Value::Int(i.clamp(*min, *max))),
            (ParamKind::Float { .. }, Value::Int(i)) => self.validate(Value::Float(i as f64)),
            (ParamKind::Float { min, max }, Value::Float(f)) => {
                Ok(Value::Float(finite(f)?.clamp(*min, *max)))
            }
            (ParamKind::Vec3 { min, max }, Value::Vec3(v)) => {
                let mut out = [0.0; 3];
                for (o, c) in out.iter_mut().zip(v) {
                    *o = finite(c)?.clamp(*min, *max);
                }
                Ok(Value::Vec3(out))
            }
            (ParamKind::Color, Value::Color(c)) => {
                for x in c {
                    if finite(x)? < 0.0 {
                        return Err(ParamError::OutOfRange);
                    }
                }
                Ok(Value::Color(c))
            }
            (ParamKind::Enum { options }, Value::Enum(id)) => {
                if options.iter().any(|o| o.id == id) {
                    Ok(Value::Enum(id))
                } else {
                    Err(ParamError::UnknownOption)
                }
            }
            (ParamKind::String, v @ Value::String(_)) => Ok(v),
            (ParamKind::Link { .. }, v @ Value::Link(_)) => Ok(v),
            _ => Err(ParamError::TypeMismatch),
        }
    }
}
