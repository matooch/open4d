//! JSON <-> Rhai values and error mapping.

use rhai::{Dynamic, EvalAltResult, ParseError};
use serde_json::{Map as JsonMap, Number, Value as Json};

use crate::maths::Vec3;
use crate::{FailureKind, ScriptFailure, ScriptOptions};

/// Termination tokens of the limits that are not Rhai's own.
pub(crate) const TIMEOUT: &str = "timeout";
pub(crate) const COMMAND_LIMIT: &str = "commands";

/// A script value as a command argument. `vec3` becomes `[x, y, z]`.
pub(crate) fn to_json(d: &Dynamic) -> Result<Json, String> {
    if d.is_unit() {
        return Ok(Json::Null);
    }
    if let Ok(b) = d.as_bool() {
        return Ok(Json::Bool(b));
    }
    if let Ok(i) = d.as_int() {
        return Ok(Json::from(i));
    }
    if let Ok(f) = d.as_float() {
        return Number::from_f64(f)
            .map(Json::Number)
            .ok_or_else(|| format!("{f} is not a finite number"));
    }
    if d.is_string() || d.is_char() {
        return Ok(Json::String(d.to_string()));
    }
    if let Some(v) = d.read_lock::<Vec3>() {
        return Ok(Json::from(v.to_array().to_vec()));
    }
    if let Some(array) = d.read_lock::<rhai::Array>() {
        return array
            .iter()
            .map(to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array);
    }
    if let Some(map) = d.read_lock::<rhai::Map>() {
        let mut out = JsonMap::new();
        for (k, v) in map.iter() {
            out.insert(k.to_string(), to_json(v)?);
        }
        return Ok(Json::Object(out));
    }
    Err(format!("cannot pass a {} to a command", d.type_name()))
}

/// A command result as a script value.
pub(crate) fn from_json(v: &Json) -> Dynamic {
    match v {
        Json::Null => Dynamic::UNIT,
        Json::Bool(b) => Dynamic::from(*b),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Dynamic::from(i),
            None => Dynamic::from(n.as_f64().unwrap_or(f64::NAN)),
        },
        Json::String(s) => Dynamic::from(s.clone()),
        Json::Array(items) => Dynamic::from(items.iter().map(from_json).collect::<rhai::Array>()),
        Json::Object(fields) => {
            let mut map = rhai::Map::new();
            for (k, v) in fields {
                map.insert(k.as_str().into(), from_json(v));
            }
            Dynamic::from(map)
        }
    }
}

/// Like [`from_json`], with three-number arrays of `vec3` parameters as `vec3`.
pub(crate) fn vec3_from_json(v: &Json) -> Option<Vec3> {
    let a = v.as_array()?;
    match a.as_slice() {
        [x, y, z] => Some(Vec3::new(x.as_f64()?, y.as_f64()?, z.as_f64()?)),
        _ => None,
    }
}

pub(crate) fn syntax_failure(e: &ParseError) -> ScriptFailure {
    ScriptFailure {
        kind: FailureKind::Syntax,
        message: e.0.to_string(),
        line: e.1.line(),
        output: Vec::new(),
    }
}

/// Strips Rhai's trailing ` (line N, position M)`.
fn bare(message: String) -> String {
    match message.rfind(" (line ") {
        Some(i) if message.ends_with(')') => message[..i].to_string(),
        _ => message,
    }
}

pub(crate) fn failure(e: &EvalAltResult, o: &ScriptOptions) -> ScriptFailure {
    let (kind, message) = match e {
        EvalAltResult::ErrorTooManyOperations(_) => (
            FailureKind::Limit,
            format!("script exceeded the operation limit ({})", o.max_operations),
        ),
        EvalAltResult::ErrorStackOverflow(_) => (
            FailureKind::Limit,
            format!(
                "script exceeded the call depth limit ({})",
                o.max_call_levels
            ),
        ),
        EvalAltResult::ErrorDataTooLarge(what, _) => (
            FailureKind::Limit,
            format!("script data too large ({what})"),
        ),
        EvalAltResult::ErrorTerminated(token, _) => {
            let message = match token.clone().into_string().as_deref() {
                Ok(TIMEOUT) => format!("script exceeded the time limit ({:?})", o.timeout),
                Ok(COMMAND_LIMIT) => {
                    format!("script exceeded the command limit ({})", o.max_commands)
                }
                _ => "script terminated".to_string(),
            };
            (FailureKind::Limit, message)
        }
        EvalAltResult::ErrorInFunctionCall(_, _, inner, pos) => {
            let mut f = failure(inner, o);
            f.line = f.line.or(pos.line());
            return f;
        }
        EvalAltResult::ErrorRuntime(value, _) => (FailureKind::Runtime, value.to_string()),
        other => (FailureKind::Runtime, bare(other.to_string())),
    };
    ScriptFailure {
        kind,
        message,
        line: e.position().line(),
        output: Vec::new(),
    }
}
