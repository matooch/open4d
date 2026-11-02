//! The RPC surface as data, for the generated reference (docs/specs/RPC.md §4, §10).
//!
//! Unit tests keep these tables honest: every method and notification name that appears in
//! the server sources must be listed here, every listed method must be answered by a real
//! server (`tests/reference.rs`), and every error code constant must be listed.

use crate::code;

/// A request method.
pub struct MethodDoc {
    pub name: &'static str,
    /// Params object, `-` for none.
    pub params: &'static str,
    /// Result object.
    pub result: &'static str,
    pub summary: &'static str,
}

/// A server-to-client notification.
pub struct NotificationDoc {
    pub name: &'static str,
    pub params: &'static str,
    pub sent_when: &'static str,
}

/// An error code.
pub struct ErrorDoc {
    pub code: i64,
    pub meaning: &'static str,
}

pub const METHODS: &[MethodDoc] = &[
    MethodDoc {
        name: "app.version",
        params: "-",
        result: r#"{"name","version","protocol"}"#,
        summary: "Server name, version and protocol revision.",
    },
    MethodDoc {
        name: "command.list",
        params: "-",
        result: r#"{"commands": [{"id","label","undoable","args_schema"}]}"#,
        summary: "The command registry with JSON Schemas (the same data as the command reference).",
    },
    MethodDoc {
        name: "command.run",
        params: r#"{"id": str, "args": object}"#,
        result: r#"{"command": id, "result": value}"#,
        summary: "Runs one command in the editor's own context; one undo step if it edits.",
    },
    MethodDoc {
        name: "scene.get",
        params: "-",
        result: r#"{"fps","start","end","frame","nodes","selected","roots"}"#,
        summary: "Project settings, selection and the node tree with parameters in user units.",
    },
    MethodDoc {
        name: "node.get",
        params: r#"{"id": int}"#,
        result: "node object",
        summary: "One node (type, parameters, tags, animated tracks, children).",
    },
    MethodDoc {
        name: "eval.mesh",
        params: r#"{"node": int, "frame"?: int}"#,
        result: r#"{"node","frame","points","polys"}"#,
        summary: "The node's evaluated mesh in world space; the timeline does not move.",
    },
    MethodDoc {
        name: "timeline.get",
        params: "-",
        result: r#"{"frame","fps","start","end","playing","autokey","preview","mode","drop_frames","display","text","markers"}"#,
        summary: "Timeline state.",
    },
    MethodDoc {
        name: "viewport.screenshot",
        params: r#"{"width"?: int, "height"?: int}"#,
        result: r#"{"width","height","format":"png","data": base64}"#,
        summary: "Renders the editor viewport offscreen (1 to 4096 per side, default 800x600). Needs a viewport and a GPU.",
    },
    MethodDoc {
        name: "events.subscribe",
        params: r#"{"kinds"?: [str]}"#,
        result: r#"{"kinds": [str]}"#,
        summary: "Subscribes this connection to notification kinds (`scene`, `frame`, `selection`, `commands`; absent means all).",
    },
    MethodDoc {
        name: "events.unsubscribe",
        params: r#"{"kinds"?: [str]}"#,
        result: r#"{"kinds": [str]}"#,
        summary: "Removes kinds from the subscription (absent means all).",
    },
    MethodDoc {
        name: "job.start",
        params: r#"{"kind": str, "args"?: object}"#,
        result: r#"{"job_id": int}"#,
        summary: "Starts a long job (kind `export.obj_sequence`: `dir`, `start`, `end`, `step`, `prefix`). Progress arrives as notifications.",
    },
    MethodDoc {
        name: "job.cancel",
        params: r#"{"id": int}"#,
        result: r#"{"id","cancelled"}"#,
        summary: "Stops a running job before its next slice.",
    },
    MethodDoc {
        name: "job.list",
        params: "-",
        result: r#"{"jobs": [{"id","kind","state","fraction","message"}]}"#,
        summary: "Running jobs and the last 32 ended ones.",
    },
];

pub const NOTIFICATIONS: &[NotificationDoc] = &[
    NotificationDoc {
        name: "scene.changed",
        params: r#"{"revision": int}"#,
        sent_when: "The scene revision changed (edit, undo, load). Kind `scene`.",
    },
    NotificationDoc {
        name: "frame.changed",
        params: r#"{"frame": int}"#,
        sent_when: "The current frame changed. Kind `frame`.",
    },
    NotificationDoc {
        name: "selection.changed",
        params: r#"{"nodes": [int]}"#,
        sent_when: "The object selection changed. Kind `selection`.",
    },
    NotificationDoc {
        name: "commands.changed",
        params: r#"{"count": int}"#,
        sent_when: "The command list changed. Kind `commands`.",
    },
    NotificationDoc {
        name: "job.progress",
        params: r#"{"id","fraction","message"}"#,
        sent_when: "After each slice of a job, to the connection that started it (no subscription needed).",
    },
    NotificationDoc {
        name: "job.finished",
        params: r#"{"id","state","result"?,"error"?}"#,
        sent_when: "A job ended (`done`, `cancelled`, `failed`), to the connection that started it.",
    },
];

pub const ERRORS: &[ErrorDoc] = &[
    ErrorDoc {
        code: code::PARSE_ERROR,
        meaning: "parse error (the line is not JSON)",
    },
    ErrorDoc {
        code: code::INVALID_REQUEST,
        meaning: "invalid request (not an object, no `id`, `method` not a string, wrong `jsonrpc`)",
    },
    ErrorDoc {
        code: code::METHOD_NOT_FOUND,
        meaning: "unknown method",
    },
    ErrorDoc {
        code: code::INVALID_PARAMS,
        meaning: "invalid params (missing or wrongly typed field)",
    },
    ErrorDoc {
        code: code::INTERNAL,
        meaning: "internal error (handler panic, editor gone)",
    },
    ErrorDoc {
        code: code::FAILED,
        meaning: "the call ran and failed (a command refused, unknown node, no viewport); `data.kind` says what",
    },
    ErrorDoc {
        code: code::UNAUTHORIZED,
        meaning: "unauthorized (token missing or wrong); the connection is closed",
    },
    ErrorDoc {
        code: code::RATE_LIMITED,
        meaning: "rate limited (the request did not run)",
    },
    ErrorDoc {
        code: code::TOO_LARGE,
        meaning: "message too large; the connection is closed",
    },
    ErrorDoc {
        code: code::TOO_MANY_CONNECTIONS,
        meaning: "too many connections; the connection is closed",
    },
    ErrorDoc {
        code: code::TIMEOUT,
        meaning: "the editor did not answer in time",
    },
];

/// The limits a client must respect: `(name, value)`.
pub fn limits() -> Vec<(&'static str, String)> {
    vec![
        ("default port", crate::DEFAULT_PORT.to_string()),
        (
            "request line",
            format!("{} bytes", crate::MAX_MESSAGE_BYTES),
        ),
        (
            "requests per second per connection",
            crate::MAX_REQUESTS_PER_SECOND.to_string(),
        ),
        (
            "simultaneous connections",
            crate::MAX_CONNECTIONS.to_string(),
        ),
        (
            "queued notifications per connection",
            crate::OUTBOX_CAPACITY.to_string(),
        ),
        ("running jobs", crate::MAX_ACTIVE_JOBS.to_string()),
        (
            "screenshot side",
            format!("1 to {} pixels", crate::MAX_SCREENSHOT_SIDE),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Quoted tokens like `"scene.get"` (lowercase letters, `_` and one dot) in `source`.
    fn tokens(source: &str, prefixes: &[&str]) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = source;
        while let Some(i) = rest.find('"') {
            rest = &rest[i + 1..];
            let end = rest
                .find(|c: char| !(c.is_ascii_lowercase() || c == '_' || c == '.'))
                .unwrap_or(rest.len());
            let word = &rest[..end];
            if rest[end..].starts_with('"')
                && word.matches('.').count() == 1
                && prefixes.iter().any(|p| word.starts_with(&format!("{p}.")))
            {
                out.push(word.to_string());
            }
        }
        out
    }

    #[test]
    fn every_method_in_the_sources_is_documented() {
        let sources = [
            include_str!("host.rs"),
            include_str!("server.rs"),
            include_str!("jobs.rs"),
        ];
        let prefixes = [
            "app", "command", "scene", "node", "eval", "timeline", "viewport", "events", "job",
        ];
        for source in sources {
            for word in tokens(source, &prefixes) {
                let known = METHODS.iter().any(|m| m.name == word)
                    || NOTIFICATIONS.iter().any(|n| n.name == word)
                    // job kinds and other names that share a prefix
                    || word == "export.obj_sequence";
                assert!(
                    known,
                    "{word} appears in the RPC sources but not in docs.rs"
                );
            }
        }
    }

    #[test]
    fn every_notification_sent_is_documented() {
        let sources = [include_str!("server.rs"), include_str!("hub.rs")];
        for source in sources {
            let mut rest = source;
            while let Some(i) = rest.find("notification(\"") {
                rest = &rest[i + "notification(\"".len()..];
                let name = &rest[..rest.find('"').unwrap()];
                assert!(
                    NOTIFICATIONS.iter().any(|n| n.name == name),
                    "notification {name} is sent but not documented"
                );
            }
        }
    }

    #[test]
    fn names_are_unique_and_codes_complete() {
        let mut names: Vec<&str> = METHODS.iter().map(|m| m.name).collect();
        names.extend(NOTIFICATIONS.iter().map(|n| n.name));
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
        let source = include_str!("protocol.rs");
        let constants = source.matches("pub const ").count();
        assert_eq!(ERRORS.len(), constants, "an error code is not documented");
    }
}
