//! Random argument objects that satisfy a command's JSON Schema (the subset `schemars` emits
//! for our argument structs; docs/specs/COMMANDS.md §7), plus value mutation for the error path.

use serde_json::{Map, Number, Value, json};

use crate::rng::XorShift64;

/// What the generator may use to make arguments meaningful.
pub struct GenCtx<'a> {
    pub rng: &'a mut XorShift64,
    /// Node ids that exist now; integer fields take one most of the time.
    pub nodes: &'a [u64],
}

/// A value for `schema` (the whole schema document is `root`, for `$ref`).
pub fn generate(cx: &mut GenCtx, root: &Value, schema: &Value, depth: usize) -> Value {
    let schema = resolve(root, schema);
    if depth > 6 {
        return Value::Null;
    }
    for key in ["oneOf", "anyOf"] {
        if let Some(options) = schema.get(key).and_then(Value::as_array)
            && !options.is_empty()
        {
            let pick = &options[cx.rng.below(options.len())];
            return generate(cx, root, pick, depth + 1);
        }
    }
    if let Some(options) = schema.get("allOf").and_then(Value::as_array)
        && let Some(first) = options.first()
    {
        return generate(cx, root, first, depth + 1);
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array)
        && !options.is_empty()
    {
        return options[cx.rng.below(options.len())].clone();
    }
    if let Some(c) = schema.get("const") {
        return c.clone();
    }
    let ty = match schema.get("type") {
        Some(Value::String(s)) => s.as_str(),
        Some(Value::Array(list)) => {
            let names: Vec<&str> = list.iter().filter_map(Value::as_str).collect();
            if names.is_empty() {
                "null"
            } else {
                names[cx.rng.below(names.len())]
            }
        }
        _ if schema.get("properties").is_some() => "object",
        _ => "null",
    };
    match ty {
        "object" => {
            let mut out = Map::new();
            let required: Vec<&str> = schema
                .get("required")
                .and_then(Value::as_array)
                .map(|r| r.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            if let Some(props) = schema.get("properties").and_then(Value::as_object) {
                for (name, sub) in props {
                    if required.contains(&name.as_str()) || cx.rng.chance(40) {
                        let v = generate(cx, root, sub, depth + 1);
                        out.insert(name.clone(), v);
                    }
                }
            }
            Value::Object(out)
        }
        "array" => {
            let item = schema.get("items").cloned().unwrap_or(Value::Null);
            let min = schema.get("minItems").and_then(Value::as_u64).unwrap_or(0) as usize;
            let max = schema
                .get("maxItems")
                .and_then(Value::as_u64)
                .map_or(4, |m| m as usize);
            let n = (min + cx.rng.below(4)).min(max.max(min));
            Value::Array(
                (0..n)
                    .map(|_| generate(cx, root, &item, depth + 1))
                    .collect(),
            )
        }
        "string" => Value::String(random_string(cx.rng)),
        "boolean" => Value::Bool(cx.rng.chance(50)),
        "integer" => {
            let lo = schema.get("minimum").and_then(Value::as_i64);
            let hi = schema.get("maximum").and_then(Value::as_i64);
            if lo.is_none() && hi.is_none() && !cx.nodes.is_empty() && cx.rng.chance(60) {
                return json!(cx.nodes[cx.rng.below(cx.nodes.len())]);
            }
            let lo = lo.unwrap_or(-5);
            let hi = hi.unwrap_or(lo.saturating_add(120)).max(lo);
            let span = (hi as i128 - lo as i128 + 1).min(1 << 20) as usize;
            json!(lo + cx.rng.below(span) as i64)
        }
        "number" => {
            let lo = schema
                .get("minimum")
                .and_then(Value::as_f64)
                .unwrap_or(-100.0);
            let hi = schema
                .get("maximum")
                .and_then(Value::as_f64)
                .unwrap_or(lo + 200.0)
                .max(lo);
            Number::from_f64(lo + (hi - lo) * cx.rng.unit()).map_or(Value::Null, Value::Number)
        }
        _ => Value::Null,
    }
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

fn random_string(rng: &mut XorShift64) -> String {
    const WORDS: [&str; 8] = ["Cube", "a", "", "x y", "ünï", "0", "-1", "forge.cube"];
    if rng.chance(70) {
        WORDS[rng.below(WORDS.len())].to_string()
    } else {
        let n = rng.below(12);
        (0..n).map(|_| (b'a' + rng.byte() % 26) as char).collect()
    }
}

/// Breaks `v` in one random place: a wrong type, a huge or non-finite-looking number, a
/// dropped or extra field. Drives the invalid-argument path.
pub fn corrupt(rng: &mut XorShift64, v: &mut Value) {
    match v {
        Value::Object(map) if !map.is_empty() => {
            let keys: Vec<String> = map.keys().cloned().collect();
            let k = keys[rng.below(keys.len())].clone();
            match rng.below(3) {
                0 => {
                    map.remove(&k);
                }
                1 => {
                    if let Some(inner) = map.get_mut(&k) {
                        corrupt(rng, inner);
                    }
                }
                _ => {
                    map.insert("extra".into(), json!(1));
                }
            }
        }
        Value::Array(list) if !list.is_empty() => {
            let i = rng.below(list.len());
            corrupt(rng, &mut list[i]);
        }
        _ => {
            const BAD: [fn() -> Value; 7] = [
                || Value::Null,
                || json!("text"),
                || json!(true),
                || json!(i64::MAX),
                || json!(i64::MIN),
                || json!(1e308),
                || json!([]),
            ];
            *v = BAD[rng.below(BAD.len())]();
        }
    }
}
