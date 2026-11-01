//! A small JSON Schema checker for argument objects (docs/specs/COMMANDS.md §7). It covers
//! what `schemars` emits for our argument structs: `type`, `properties`, `required`,
//! `additionalProperties: false`, `enum`, `oneOf`/`anyOf`, `items`, `$ref` into `$defs`.

use serde_json::Value;

/// Why `value` does not fit `schema`.
pub fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    check(schema, schema, value, "$")
}

fn resolve<'a>(root: &'a Value, schema: &'a Value) -> &'a Value {
    match schema.get("$ref").and_then(Value::as_str) {
        Some(r) => r
            .strip_prefix('#')
            .and_then(|p| root.pointer(p))
            .unwrap_or(schema),
        None => schema,
    }
}

fn type_ok(ty: &str, v: &Value) -> bool {
    match ty {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        _ => true,
    }
}

fn check(root: &Value, schema: &Value, v: &Value, at: &str) -> Result<(), String> {
    let schema = resolve(root, schema);
    for key in ["oneOf", "anyOf"] {
        if let Some(options) = schema.get(key).and_then(Value::as_array) {
            return if options.iter().any(|o| check(root, o, v, at).is_ok()) {
                Ok(())
            } else {
                Err(format!("{at}: matches none of the {key} alternatives"))
            };
        }
    }
    if let Some(t) = schema.get("type") {
        let ok = match t {
            Value::String(s) => type_ok(s, v),
            Value::Array(ts) => ts.iter().filter_map(Value::as_str).any(|s| type_ok(s, v)),
            _ => true,
        };
        if !ok {
            return Err(format!("{at}: expected type {t}, got {v}"));
        }
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array)
        && !options.contains(v)
    {
        return Err(format!("{at}: {v} is not one of {options:?}"));
    }
    if let Some(c) = schema.get("const")
        && c != v
    {
        return Err(format!("{at}: expected {c}"));
    }
    if let (Some(obj), Some(props)) = (v.as_object(), schema.get("properties")) {
        let props = props.as_object().ok_or("bad properties")?;
        for req in schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !obj.contains_key(req) {
                return Err(format!("{at}: missing required key {req}"));
            }
        }
        for (k, val) in obj {
            match props.get(k) {
                Some(s) => check(root, s, val, &format!("{at}.{k}"))?,
                None if schema.get("additionalProperties") == Some(&Value::Bool(false)) => {
                    return Err(format!("{at}: unknown key {k}"));
                }
                None => {}
            }
        }
    }
    if let (Some(items), Some(schema_items)) = (v.as_array(), schema.get("items")) {
        for (i, item) in items.iter().enumerate() {
            check(root, schema_items, item, &format!("{at}[{i}]"))?;
        }
    }
    if let (Some(items), Some(prefix)) = (
        v.as_array(),
        schema.get("prefixItems").and_then(Value::as_array),
    ) {
        if items.len() != prefix.len() {
            return Err(format!("{at}: expected {} items", prefix.len()));
        }
        for (i, (item, s)) in items.iter().zip(prefix).enumerate() {
            check(root, s, item, &format!("{at}[{i}]"))?;
        }
    }
    Ok(())
}
