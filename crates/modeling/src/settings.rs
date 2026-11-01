//! Tool settings: a map from parameter id to value, checked against the descriptors
//! (docs/specs/MODELING.md §0).

use std::collections::BTreeMap;

use forge_core::{ParamDesc, Value};
use serde_json::{Map, Number, Value as Json};

use crate::desc::ToolDesc;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SettingsError {
    #[error("unknown setting {0:?}")]
    Unknown(String),
    #[error("setting {0:?}: {1}")]
    Invalid(String, String),
}

/// The values of one tool run. Every descriptor id has a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    values: BTreeMap<String, Value>,
}

impl Settings {
    /// The factory defaults of the descriptor ("Reset Values").
    pub fn defaults(desc: &ToolDesc) -> Settings {
        Settings {
            values: desc
                .params
                .iter()
                .map(|p| (p.id.clone(), p.default.clone()))
                .collect(),
        }
    }

    /// Defaults overridden by the keys of `object`; numbers are clamped into the declared
    /// range, an unknown key or a value of the wrong type is an error.
    pub fn from_json(
        desc: &ToolDesc,
        object: &Map<String, Json>,
    ) -> Result<Settings, SettingsError> {
        let mut s = Settings::defaults(desc);
        for (key, json) in object {
            let p = desc
                .params
                .iter()
                .find(|p| &p.id == key)
                .ok_or_else(|| SettingsError::Unknown(key.clone()))?;
            let bad = |why: &str| SettingsError::Invalid(key.clone(), why.to_string());
            let value = match (&p.default, json) {
                (Value::Bool(_), Json::Bool(b)) => Value::Bool(*b),
                (Value::Int(_), Json::Number(n)) => Value::Int(
                    n.as_i64()
                        .or_else(|| n.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
                        .ok_or_else(|| bad("expected an integer"))?,
                ),
                (Value::Float(_), Json::Number(n)) => {
                    Value::Float(n.as_f64().ok_or_else(|| bad("expected a number"))?)
                }
                (Value::Enum(_), Json::String(s)) => Value::Enum(s.clone()),
                _ => return Err(bad("wrong type")),
            };
            let value = p
                .validate(value)
                .map_err(|e| SettingsError::Invalid(key.clone(), e.to_string()))?;
            s.values.insert(key.clone(), value);
        }
        Ok(s)
    }

    /// Every setting as JSON (floats stay floats, integers integers).
    pub fn to_json(&self) -> Map<String, Json> {
        self.values
            .iter()
            .filter_map(|(k, v)| {
                let j = match v {
                    Value::Bool(b) => Json::Bool(*b),
                    Value::Int(i) => Json::from(*i),
                    Value::Float(f) => Number::from_f64(*f).map(Json::Number)?,
                    Value::Enum(e) => Json::String(e.clone()),
                    _ => return None,
                };
                Some((k.clone(), j))
            })
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<&Value> {
        self.values.get(id)
    }

    /// Sets one value, validated by its descriptor (the UI uses this).
    pub fn set(&mut self, desc: &ToolDesc, id: &str, value: Value) -> Result<(), SettingsError> {
        let p: &ParamDesc = desc
            .params
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| SettingsError::Unknown(id.to_string()))?;
        let v = p
            .validate(value)
            .map_err(|e| SettingsError::Invalid(id.to_string(), e.to_string()))?;
        self.values.insert(id.to_string(), v);
        Ok(())
    }

    pub fn float(&self, id: &str) -> f64 {
        match self.values.get(id) {
            Some(Value::Float(f)) => *f,
            Some(Value::Int(i)) => *i as f64,
            _ => 0.0,
        }
    }

    pub fn int(&self, id: &str) -> i64 {
        match self.values.get(id) {
            Some(Value::Int(i)) => *i,
            _ => 0,
        }
    }

    /// The option id of an enum setting ("" when absent).
    pub fn choice(&self, id: &str) -> &str {
        match self.values.get(id) {
            Some(Value::Enum(e)) => e,
            _ => "",
        }
    }

    pub fn bool(&self, id: &str) -> bool {
        matches!(self.values.get(id), Some(Value::Bool(true)))
    }
}
