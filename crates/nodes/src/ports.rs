//! Port types, values and conversions (docs/specs/NODES.md section 2).

use forge_core::NodeId;
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortType {
    Float,
    Int,
    Bool,
    Vec3,
    /// Linear RGB.
    Colour,
    /// 4x4, column major.
    Matrix,
    String,
    /// A link to a scene node.
    Node,
    /// Accepts and produces any concrete type; checked when the graph runs.
    Any,
}

impl PortType {
    pub const ALL: [PortType; 9] = [
        PortType::Float,
        PortType::Int,
        PortType::Bool,
        PortType::Vec3,
        PortType::Colour,
        PortType::Matrix,
        PortType::String,
        PortType::Node,
        PortType::Any,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PortType::Float => "float",
            PortType::Int => "int",
            PortType::Bool => "bool",
            PortType::Vec3 => "vec3",
            PortType::Colour => "colour",
            PortType::Matrix => "matrix",
            PortType::String => "string",
            PortType::Node => "node",
            PortType::Any => "any",
        }
    }
}

impl std::fmt::Display for PortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// A concrete value on a port. Never "any".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PortValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    Vec3([f64; 3]),
    Colour([f64; 3]),
    Matrix([f64; 16]),
    String(String),
    Node(Option<NodeId>),
}

pub const IDENTITY: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConvertError {
    #[error("cannot convert {from} to {to}")]
    Unsupported { from: PortType, to: PortType },
    #[error("{0}")]
    Invalid(String),
}

impl PortValue {
    /// The concrete type of the value.
    pub fn ty(&self) -> PortType {
        match self {
            PortValue::Float(_) => PortType::Float,
            PortValue::Int(_) => PortType::Int,
            PortValue::Bool(_) => PortType::Bool,
            PortValue::Vec3(_) => PortType::Vec3,
            PortValue::Colour(_) => PortType::Colour,
            PortValue::Matrix(_) => PortType::Matrix,
            PortValue::String(_) => PortType::String,
            PortValue::Node(_) => PortType::Node,
        }
    }

    /// The zero value of a concrete type (`Any` gives a float).
    pub fn zero(ty: PortType) -> PortValue {
        match ty {
            PortType::Float | PortType::Any => PortValue::Float(0.0),
            PortType::Int => PortValue::Int(0),
            PortType::Bool => PortValue::Bool(false),
            PortType::Vec3 => PortValue::Vec3([0.0; 3]),
            PortType::Colour => PortValue::Colour([0.0; 3]),
            PortType::Matrix => PortValue::Matrix(IDENTITY),
            PortType::String => PortValue::String(String::new()),
            PortType::Node => PortValue::Node(None),
        }
    }

    /// True when every number in the value is finite.
    pub fn is_finite(&self) -> bool {
        match self {
            PortValue::Float(f) => f.is_finite(),
            PortValue::Vec3(v) | PortValue::Colour(v) => v.iter().all(|c| c.is_finite()),
            PortValue::Matrix(m) => m.iter().all(|c| c.is_finite()),
            _ => true,
        }
    }

    /// The value as JSON (`graph.get`, `graph.evaluate`): plain numbers, arrays, strings.
    pub fn to_json(&self) -> Json {
        match self {
            PortValue::Float(f) => json!(f),
            PortValue::Int(i) => json!(i),
            PortValue::Bool(b) => json!(b),
            PortValue::Vec3(v) | PortValue::Colour(v) => json!(v),
            PortValue::Matrix(m) => json!(m.to_vec()),
            PortValue::String(s) => json!(s),
            PortValue::Node(n) => json!(n.map(|n| n.0)),
        }
    }

    /// Parses JSON for a port of type `ty` (docs/specs/NODES.md section 6).
    pub fn from_json(ty: PortType, v: &Json) -> Result<PortValue, ConvertError> {
        let bad = |what: &str| ConvertError::Invalid(format!("expected {what}, got {v}"));
        let number = |x: &Json| {
            x.as_f64()
                .filter(|f| f.is_finite())
                .ok_or_else(|| bad("a number"))
        };
        let triple = |v: &Json| -> Result<[f64; 3], ConvertError> {
            match v.as_array().map(Vec::as_slice) {
                Some([x, y, z]) => Ok([number(x)?, number(y)?, number(z)?]),
                _ => Err(bad("[x, y, z]")),
            }
        };
        Ok(match ty {
            PortType::Float => PortValue::Float(number(v)?),
            PortType::Int => PortValue::Int(v.as_i64().ok_or_else(|| bad("an integer"))?),
            PortType::Bool => PortValue::Bool(v.as_bool().ok_or_else(|| bad("a boolean"))?),
            PortType::Vec3 => PortValue::Vec3(triple(v)?),
            PortType::Colour => PortValue::Colour(triple(v)?),
            PortType::Matrix => {
                let items = v.as_array().filter(|a| a.len() == 16);
                let items = items.ok_or_else(|| bad("16 numbers"))?;
                let mut m = [0.0; 16];
                for (o, x) in m.iter_mut().zip(items) {
                    *o = number(x)?;
                }
                PortValue::Matrix(m)
            }
            PortType::String => {
                PortValue::String(v.as_str().ok_or_else(|| bad("a string"))?.to_string())
            }
            PortType::Node => match v {
                Json::Null => PortValue::Node(None),
                _ => PortValue::Node(Some(NodeId(
                    v.as_u64().ok_or_else(|| bad("a node id or null"))?,
                ))),
            },
            PortType::Any => match v {
                Json::Bool(b) => PortValue::Bool(*b),
                Json::String(s) => PortValue::String(s.clone()),
                Json::Null => PortValue::Node(None),
                Json::Number(n) => match n.as_i64() {
                    Some(i) => PortValue::Int(i),
                    None => PortValue::Float(number(v)?),
                },
                Json::Array(a) if a.len() == 3 => PortValue::Vec3(triple(v)?),
                Json::Array(a) if a.len() == 16 => {
                    return PortValue::from_json(PortType::Matrix, v);
                }
                _ => return Err(bad("a bool, number, string, null, [x,y,z] or 16 numbers")),
            },
        })
    }
}

/// True if a value of type `from` may feed an input of type `to` (implicit conversions,
/// NODES.md section 2). `Any` on either side is accepted; a concrete check happens when the
/// graph runs.
pub fn can_convert(from: PortType, to: PortType) -> bool {
    use PortType::*;
    from == to
        || from == Any
        || to == Any
        || matches!(
            (from, to),
            (Int, Float)
                | (Bool, Int)
                | (Bool, Float)
                | (Float | Int, Vec3)
                | (Float | Int, Colour)
                | (Vec3, Colour)
                | (Colour, Vec3)
        )
}

/// Implicit conversion of a concrete value to `to` (identity for `Any`).
pub fn convert(value: &PortValue, to: PortType) -> Result<PortValue, ConvertError> {
    use PortValue as V;
    let from = value.ty();
    if to == PortType::Any || from == to {
        return Ok(value.clone());
    }
    Ok(match (value, to) {
        (V::Int(i), PortType::Float) => V::Float(*i as f64),
        (V::Bool(b), PortType::Int) => V::Int(i64::from(*b)),
        (V::Bool(b), PortType::Float) => V::Float(if *b { 1.0 } else { 0.0 }),
        (V::Float(f), PortType::Vec3) => V::Vec3([*f; 3]),
        (V::Int(i), PortType::Vec3) => V::Vec3([*i as f64; 3]),
        (V::Float(f), PortType::Colour) => V::Colour([*f; 3]),
        (V::Int(i), PortType::Colour) => V::Colour([*i as f64; 3]),
        (V::Vec3(v), PortType::Colour) => V::Colour(*v),
        (V::Colour(c), PortType::Vec3) => V::Vec3(*c),
        _ => return Err(ConvertError::Unsupported { from, to }),
    })
}

/// Implicit conversions plus the explicit ones of NODES.md section 2.
pub fn convert_explicit(value: &PortValue, to: PortType) -> Result<PortValue, ConvertError> {
    use PortValue as V;
    if let Ok(v) = convert(value, to) {
        return Ok(v);
    }
    let from = value.ty();
    let invalid = |m: String| ConvertError::Invalid(m);
    Ok(match (value, to) {
        (V::Float(f), PortType::Int) => {
            if !f.is_finite() {
                return Err(invalid(format!("{f} is not finite")));
            }
            V::Int(f.trunc().clamp(i64::MIN as f64, i64::MAX as f64) as i64)
        }
        (V::Float(f), PortType::Bool) => V::Bool(*f != 0.0),
        (V::Int(i), PortType::Bool) => V::Bool(*i != 0),
        (V::Vec3(v), PortType::Float) => V::Float((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()),
        (V::Colour(c), PortType::Float) => V::Float(0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]),
        (V::Vec3(v), PortType::Matrix) => {
            let mut m = IDENTITY;
            m[12..15].copy_from_slice(v);
            V::Matrix(m)
        }
        (V::Matrix(m), PortType::Vec3) => V::Vec3([m[12], m[13], m[14]]),
        (V::String(s), PortType::Float) => V::Float(
            s.trim()
                .parse::<f64>()
                .ok()
                .filter(|f| f.is_finite())
                .ok_or_else(|| invalid(format!("{s:?} is not a number")))?,
        ),
        (V::String(s), PortType::Int) => V::Int(
            s.trim()
                .parse::<i64>()
                .map_err(|_| invalid(format!("{s:?} is not an integer")))?,
        ),
        (V::String(s), PortType::Bool) => match s.trim() {
            "true" => V::Bool(true),
            "false" => V::Bool(false),
            _ => return Err(invalid(format!("{s:?} is not true or false"))),
        },
        (_, PortType::String) => V::String(match value {
            V::Float(f) => f.to_string(),
            V::Int(i) => i.to_string(),
            V::Bool(b) => b.to_string(),
            V::Vec3(v) | V::Colour(v) => format!("[{}, {}, {}]", v[0], v[1], v[2]),
            V::Matrix(m) => format!("{m:?}"),
            V::String(s) => s.clone(),
            V::Node(Some(n)) => n.to_string(),
            V::Node(None) => String::new(),
        }),
        _ => return Err(ConvertError::Unsupported { from, to }),
    })
}
