//! The last commands run through the registry, for crash reports
//! (docs/specs/PLATFORM.md §8.3).

use std::collections::VecDeque;
use std::sync::{Mutex, TryLockError};

use serde_json::Value;

/// How many runs the process-wide history keeps.
pub const CAPACITY: usize = 50;
/// Arguments longer than this many characters are cut.
pub const MAX_ARGS_CHARS: usize = 2000;

/// One recorded run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub seq: u64,
    pub id: String,
    pub args: String,
}

impl Entry {
    /// `<seq> <id> <args>`.
    pub fn line(&self) -> String {
        format!("{} {} {}", self.seq, self.id, self.args)
    }
}

/// A bounded ring of recent runs.
#[derive(Debug)]
pub struct CommandHistory {
    capacity: usize,
    next_seq: u64,
    entries: VecDeque<Entry>,
}

impl CommandHistory {
    pub fn new(capacity: usize) -> CommandHistory {
        CommandHistory {
            capacity: capacity.max(1),
            next_seq: 1,
            entries: VecDeque::new(),
        }
    }

    /// Records a run; the oldest entry goes when the ring is full.
    pub fn record(&mut self, id: &str, args: &Value) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(Entry {
            seq: self.next_seq,
            id: id.to_string(),
            args: render_args(args),
        });
        self.next_seq += 1;
    }

    /// Oldest first.
    pub fn entries(&self) -> Vec<Entry> {
        self.entries.iter().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// True for keys that may hold a credential (`token`, `rpc_token`, `Token`...).
fn secret_key(key: &str) -> bool {
    key.to_ascii_lowercase().contains("token")
}

fn redact(v: &Value) -> Value {
    match v {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let v = if secret_key(k) {
                        Value::String("<redacted>".into())
                    } else {
                        redact(v)
                    };
                    (k.clone(), v)
                })
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(redact).collect()),
        other => other.clone(),
    }
}

/// Compact JSON of `args` with secrets redacted, cut at [`MAX_ARGS_CHARS`] characters.
pub fn render_args(args: &Value) -> String {
    let text = redact(args).to_string();
    if text.chars().count() > MAX_ARGS_CHARS {
        let cut: String = text.chars().take(MAX_ARGS_CHARS).collect();
        format!("{cut}...")
    } else {
        text
    }
}

static GLOBAL: Mutex<Option<CommandHistory>> = Mutex::new(None);

/// Records a run in the process-wide history.
pub fn record(id: &str, args: &Value) {
    let mut guard = GLOBAL.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_or_insert_with(|| CommandHistory::new(CAPACITY))
        .record(id, args);
}

/// The process-wide history, oldest first, as report lines. Never blocks: when another thread
/// holds the lock (a panic hook runs at any time) the history is reported as unavailable.
pub fn snapshot() -> Vec<String> {
    let guard = match GLOBAL.try_lock() {
        Ok(g) => g,
        Err(TryLockError::Poisoned(e)) => e.into_inner(),
        Err(TryLockError::WouldBlock) => return vec!["(history busy)".to_string()],
    };
    guard
        .as_ref()
        .map(|h| h.entries().iter().map(Entry::line).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keeps_only_the_newest_entries() {
        let mut h = CommandHistory::new(3);
        for i in 0..5 {
            h.record("object.create", &json!({ "n": i }));
        }
        let e = h.entries();
        assert_eq!(e.len(), 3);
        assert_eq!(e[0].seq, 3);
        assert_eq!(e[2].seq, 5);
        assert_eq!(e[2].line(), r#"5 object.create {"n":4}"#);
    }

    #[test]
    fn default_capacity_is_fifty() {
        let mut h = CommandHistory::new(CAPACITY);
        for _ in 0..80 {
            h.record("a.b", &json!({}));
        }
        assert_eq!(h.len(), 50);
        assert_eq!(h.entries()[0].seq, 31);
    }

    #[test]
    fn token_values_are_redacted_at_any_depth() {
        let text = render_args(&json!({
            "name": "cube",
            "rpc_token": "s3cret",
            "nested": [{ "Token": "s3cret", "keep": 1 }],
        }));
        assert!(!text.contains("s3cret"), "{text}");
        assert!(text.contains("<redacted>"));
        assert!(text.contains("cube") && text.contains("\"keep\":1"));
    }

    #[test]
    fn long_arguments_are_cut() {
        let text = render_args(&json!({ "blob": "x".repeat(5000) }));
        assert!(text.chars().count() <= MAX_ARGS_CHARS + 3);
        assert!(text.ends_with("..."));
    }

    #[test]
    fn global_history_records_runs() {
        // Other tests run commands in parallel, so look for our own marker only.
        record("history.marker_test", &json!({ "type": "forge.null" }));
        let lines = snapshot();
        assert!(lines.len() <= CAPACITY + 1);
        assert!(
            lines.iter().any(|l| l.contains("history.marker_test")),
            "{lines:?}"
        );
    }
}
