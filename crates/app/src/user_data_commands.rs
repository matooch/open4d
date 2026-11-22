//! `user_data.*`: add custom parameters to any node (brief 110).

use forge_command::{
    CommandContext, CommandError, CommandRegistry, CommandSpec, param_value_from_json,
};
use forge_core::{Category, NodeId, ParamDesc, Value};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AddArgs {
    /// Node to receive the user-data field.
    node: u64,
    /// User-visible name (label) of the field.
    name: String,
    /// Parameter kind: float, int, bool, vec3, link, string.
    kind: String,
    /// Default value. For vec3 use `[x, y, z]`; for link use a node id or `null`;
    /// for bool use `true`/`false`; for string use a string; for int/float use a number.
    #[serde(default)]
    default: Option<Json>,
    /// Hard minimum (int/float/vec3 only).
    #[serde(default)]
    min: Option<f64>,
    /// Hard maximum (int/float/vec3 only).
    #[serde(default)]
    max: Option<f64>,
    /// For link kinds: accepted target categories (default any).
    #[serde(default)]
    accepts: Option<Vec<String>>,
}

pub fn register(r: &mut CommandRegistry) -> Result<(), CommandError> {
    r.register_fn(
        CommandSpec {
            id: "user_data.add",
            label: "Add User Data",
            undoable: true,
            sample: || {
                json!({
                    "node": 1,
                    "name": "amount",
                    "kind": "float",
                    "default": 1.0,
                    "min": 0.0,
                    "max": 10.0,
                })
            },
        },
        add,
    )
}

fn add(ctx: &mut dyn CommandContext, a: AddArgs) -> Result<Json, CommandError> {
    let id = NodeId(a.node);
    let desc = build_desc(&a)?;
    ctx.doc_mut().transact("Add user data", |tx| {
        tx.add_user_data(id, desc)?;
        Ok(())
    })?;
    let index = ctx
        .doc()
        .scene()
        .node(id)
        .map(|n| n.user_data.len() - 1)
        .unwrap_or(0);
    let param = forge_core::user_data::id_for_index(index);
    Ok(json!({"param": param, "index": index}))
}

fn build_desc(a: &AddArgs) -> Result<ParamDesc, CommandError> {
    let label = if a.name.trim().is_empty() {
        return Err(CommandError::InvalidArgs("name cannot be empty".into()));
    } else {
        a.name.trim().to_string()
    };
    let id = label.to_ascii_lowercase().replace(' ', "_");
    let id = if id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        id
    } else {
        "field".to_string()
    };
    let mut desc = match a.kind.as_str() {
        "float" => ParamDesc::float(&id, &label, default_f64(a)?),
        "int" => ParamDesc::int(&id, &label, default_i64(a)?),
        "bool" => ParamDesc::bool(&id, &label, default_bool(a)?),
        "vec3" => ParamDesc::vec3(&id, &label, default_vec3(a)?),
        "link" => {
            let mut d = ParamDesc::link(&id, &label, &parse_accepts(a.accepts.as_deref()));
            if let Some(v) = &a.default {
                d.default = param_value_from_json(&d, v)?;
            }
            d
        }
        "string" => ParamDesc::string(&id, &label, &default_string(a)),
        other => {
            return Err(CommandError::InvalidArgs(format!(
                "unknown user-data kind {other:?}"
            )));
        }
    };
    if let (Some(lo), Some(hi)) = (a.min, a.max) {
        if lo > hi {
            return Err(CommandError::InvalidArgs(format!(
                "min {lo} is greater than max {hi}"
            )));
        }
        match &desc.kind {
            forge_core::ParamKind::Float { .. }
            | forge_core::ParamKind::Vec3 { .. }
            | forge_core::ParamKind::Int { .. } => {
                desc = desc.range(lo, hi);
            }
            _ => {}
        }
    }
    Ok(desc)
}

fn default_f64(a: &AddArgs) -> Result<f64, CommandError> {
    match &a.default {
        None => Ok(0.0),
        Some(v) => v
            .as_f64()
            .or_else(|| v.as_i64().map(|i| i as f64))
            .ok_or_else(|| CommandError::InvalidArgs("default must be a number".into())),
    }
}

fn default_i64(a: &AddArgs) -> Result<i64, CommandError> {
    match &a.default {
        None => Ok(0),
        Some(v) => v
            .as_i64()
            .ok_or_else(|| CommandError::InvalidArgs("default must be an integer".into())),
    }
}

fn default_bool(a: &AddArgs) -> Result<bool, CommandError> {
    match &a.default {
        None => Ok(false),
        Some(v) => v
            .as_bool()
            .ok_or_else(|| CommandError::InvalidArgs("default must be a boolean".into())),
    }
}

fn default_vec3(a: &AddArgs) -> Result<[f64; 3], CommandError> {
    match &a.default {
        None => Ok([0.0; 3]),
        Some(v) => {
            param_value_from_json(&ParamDesc::vec3("", "", [0.0; 3]), v).and_then(|p| match p {
                Value::Vec3(x) => Ok(x),
                _ => Err(CommandError::InvalidArgs(
                    "default must be [x, y, z]".into(),
                )),
            })
        }
    }
}

fn default_string(a: &AddArgs) -> String {
    a.default
        .as_ref()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn parse_accepts(list: Option<&[String]>) -> Vec<Category> {
    let Some(list) = list else { return Vec::new() };
    list.iter()
        .filter_map(|s| match s.as_str() {
            "object" => Some(Category::Object),
            "generator" => Some(Category::Generator),
            "deformer" => Some(Category::Deformer),
            "effector" => Some(Category::Effector),
            "field" => Some(Category::Field),
            "tag" => Some(Category::Tag),
            "material" => Some(Category::Material),
            "shader" => Some(Category::Shader),
            _ => None,
        })
        .collect()
}
