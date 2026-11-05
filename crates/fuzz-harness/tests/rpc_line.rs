//! `forge_rpc::handle_request_line`, the entry point of the `rpc_line` fuzz target
//! (docs/specs/PLATFORM.md §9.1): error codes of the validation path, and that a valid call
//! without an editor answers at once.

use std::time::{Duration, Instant};

use forge_rpc::{code, handle_request_line};
use serde_json::{Value, json};

fn code_of(line: &str, token: Option<&str>) -> i64 {
    let v: Value = handle_request_line(line.as_bytes(), token);
    v["error"]["code"].as_i64().unwrap_or(0)
}

#[test]
fn validation_path_codes() {
    assert_eq!(code_of("{", None), code::PARSE_ERROR);
    assert_eq!(code_of("[1]", None), code::INVALID_REQUEST);
    assert_eq!(
        code_of(r#"{"jsonrpc":"2.0","method":"x"}"#, None),
        code::INVALID_REQUEST
    );
    assert_eq!(
        code_of(r#"{"jsonrpc":"1.0","id":1,"method":"x"}"#, None),
        code::INVALID_REQUEST
    );
    assert_eq!(
        code_of(r#"{"jsonrpc":"2.0","id":1,"method":"x","params":[]}"#, None),
        code::INVALID_PARAMS
    );
    assert_eq!(
        code_of(
            r#"{"jsonrpc":"2.0","id":1,"method":"app.version"}"#,
            Some("t")
        ),
        code::UNAUTHORIZED
    );
}

#[test]
fn a_call_that_needs_the_editor_fails_fast() {
    let started = Instant::now();
    let line = json!({"jsonrpc":"2.0","id":1,"method":"app.version"}).to_string();
    assert_eq!(code_of(&line, None), code::INTERNAL);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn event_subscription_is_answered_without_the_editor() {
    let line =
        json!({"jsonrpc":"2.0","id":1,"method":"events.subscribe","params":{"kinds":["scene"]}})
            .to_string();
    let v = handle_request_line(line.as_bytes(), None);
    assert!(v.get("result").is_some(), "{v}");
}
