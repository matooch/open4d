//! A Standard material as a parameter map with the O29 defaults behind it.

use std::collections::BTreeMap;

use forge_core::{Node, Value};

use crate::params::standard_type;

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    values: BTreeMap<String, Value>,
}

impl Default for Material {
    fn default() -> Self {
        Material::new()
    }
}

impl Material {
    /// A material with every default.
    pub fn new() -> Material {
        let values = standard_type()
            .params
            .iter()
            .map(|p| (p.id.clone(), p.default.clone()))
            .collect();
        Material { values }
    }

    /// Defaults overridden by `overrides` (unknown ids are ignored).
    pub fn from_values(overrides: &BTreeMap<String, Value>) -> Material {
        let mut m = Material::new();
        for (k, v) in overrides {
            if m.values.contains_key(k) {
                m.values.insert(k.clone(), v.clone());
            }
        }
        m
    }

    pub fn from_node(node: &Node) -> Material {
        Material::from_values(&node.params)
    }

    pub fn set(&mut self, id: &str, v: Value) {
        if self.values.contains_key(id) {
            self.values.insert(id.to_string(), v);
        }
    }

    pub fn f(&self, id: &str) -> f64 {
        match self.values.get(id) {
            Some(Value::Float(v)) => *v,
            Some(Value::Int(v)) => *v as f64,
            _ => 0.0,
        }
    }
    pub fn b(&self, id: &str) -> bool {
        matches!(self.values.get(id), Some(Value::Bool(true)))
    }
    pub fn c(&self, id: &str) -> [f64; 3] {
        match self.values.get(id) {
            Some(Value::Color(c)) => *c,
            _ => [0.0; 3],
        }
    }
    pub fn e(&self, id: &str) -> &str {
        match self.values.get(id) {
            Some(Value::Enum(s)) | Some(Value::String(s)) => s,
            _ => "",
        }
    }
    pub fn enabled(&self, channel: &str) -> bool {
        self.b(&format!("enable.{channel}"))
    }
}
