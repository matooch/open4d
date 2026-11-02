//! The Rhai node: a pure, sandboxed script with typed inputs and outputs
//! (docs/specs/NODES.md section 4.1; sandbox rules of docs/specs/SCRIPTING.md sections 4, 5).

use std::time::{Duration, Instant};

use rhai::{Dynamic, Engine, EvalAltResult, Scope};

use crate::ports::PortValue;
use crate::registry::{EvalError, NodeCtx};

/// Termination token of the wall-clock limit.
const TIMEOUT: &str = "timeout";

/// The sandbox limits of the Rhai node; the defaults equal `forge_script::ScriptOptions`
/// (a test compares them).
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxLimits {
    pub max_operations: u64,
    /// Wall clock, checked every 1024 operations.
    pub timeout: Duration,
    pub max_call_levels: usize,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        SandboxLimits {
            max_operations: 5_000_000,
            timeout: Duration::from_secs(10),
            max_call_levels: 48,
        }
    }
}

fn engine(limits: &SandboxLimits) -> Engine {
    // `no_module` and `no_time` are crate features (Cargo.toml): no import, no clock.
    let mut engine = Engine::new();
    engine.set_max_operations(limits.max_operations);
    engine.set_max_call_levels(limits.max_call_levels);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_string_size(1 << 20);
    engine.set_max_array_size(100_000);
    engine.set_max_map_size(100_000);
    engine.set_strict_variables(true);
    engine.disable_symbol("eval");
    let started = Instant::now();
    let timeout = limits.timeout;
    engine.on_progress(move |n| {
        if n % 1024 == 0 && started.elapsed() > timeout {
            return Some(TIMEOUT.into());
        }
        None
    });
    // `print` and `debug` are silently dropped: a node has no output channel.
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    engine
}

fn describe(e: &EvalAltResult, limits: &SandboxLimits) -> String {
    match e {
        EvalAltResult::ErrorTooManyOperations(_) => format!(
            "script exceeded the operation limit ({})",
            limits.max_operations
        ),
        EvalAltResult::ErrorStackOverflow(_) => format!(
            "script exceeded the call depth limit ({})",
            limits.max_call_levels
        ),
        EvalAltResult::ErrorDataTooLarge(what, _) => format!("script data too large ({what})"),
        EvalAltResult::ErrorTerminated(token, _) => match token.clone().into_string().as_deref() {
            Ok(TIMEOUT) => format!("script exceeded the time limit ({:?})", limits.timeout),
            _ => "script terminated".to_string(),
        },
        EvalAltResult::ErrorInFunctionCall(_, _, inner, _) => describe(inner, limits),
        EvalAltResult::ErrorRuntime(v, _) => v.to_string(),
        other => other.to_string(),
    }
}

fn number(d: &Dynamic) -> Option<f64> {
    d.as_float()
        .ok()
        .or_else(|| d.as_int().ok().map(|i| i as f64))
}

fn triple(d: &Dynamic) -> Option<[f64; 3]> {
    let a = d.read_lock::<rhai::Array>()?;
    match a.as_slice() {
        [x, y, z] => Some([number(x)?, number(y)?, number(z)?]),
        _ => None,
    }
}

/// `script.rhai`.
pub(crate) fn eval_script(ctx: &mut NodeCtx<'_>) -> Result<Vec<PortValue>, EvalError> {
    let limits = ctx.env.limits.clone();
    let engine = engine(&limits);
    let mut scope = Scope::new();
    for name in ["a", "b", "c", "d"] {
        scope.push_constant(name, ctx.float(name));
    }
    let v = ctx.vec3("v");
    scope.push_constant(
        "v",
        vec![
            Dynamic::from(v[0]),
            Dynamic::from(v[1]),
            Dynamic::from(v[2]),
        ],
    );
    scope.push_constant("flag", ctx.bool("flag"));
    scope.push_constant("time", ctx.env.seconds);
    scope.push_constant("frame", ctx.env.frame());

    let ast = engine
        .compile_with_scope(&scope, ctx.string("source"))
        .map_err(|e| EvalError(format!("script syntax error: {e}")))?;

    let value: Dynamic = engine
        .eval_ast_with_scope(&mut scope, &ast)
        .map_err(|e| EvalError(describe(&e, &limits)))?;

    let mut result = 0.0;
    let mut vec = [0.0; 3];
    let mut flag = false;
    if value.is_unit() {
        return Err(EvalError("script returned nothing".into()));
    } else if let Ok(b) = value.as_bool() {
        flag = b;
        result = f64::from(u8::from(b));
    } else if let Some(n) = number(&value) {
        result = n;
    } else if let Some(t) = triple(&value) {
        vec = t;
    } else if let Some(map) = value.read_lock::<rhai::Map>() {
        for (key, item) in map.iter() {
            match key.as_str() {
                "result" => {
                    result = number(item)
                        .ok_or_else(|| EvalError("`result` must be a number".into()))?;
                }
                "vec" => {
                    vec = triple(item)
                        .ok_or_else(|| EvalError("`vec` must be an array of 3 numbers".into()))?;
                }
                "flag" => {
                    flag = item
                        .as_bool()
                        .map_err(|_| EvalError("`flag` must be a bool".into()))?;
                }
                other => return Err(EvalError(format!("unknown output key {other:?}"))),
            }
        }
    } else {
        return Err(EvalError(format!(
            "script returned a {}, expected a number, bool, array or map",
            value.type_name()
        )));
    }
    Ok(vec![
        PortValue::Float(result),
        PortValue::Vec3(vec),
        PortValue::Bool(flag),
    ])
}
