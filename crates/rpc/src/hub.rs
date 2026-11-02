//! Connections that can receive notifications (docs/specs/RPC.md §10.1).

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

/// Notifications waiting for one connection to read them.
pub const OUTBOX_CAPACITY: usize = 1024;

/// What a subscription can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Scene,
    Frame,
    Selection,
    Commands,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Scene, Kind::Frame, Kind::Selection, Kind::Commands];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Scene => "scene",
            Kind::Frame => "frame",
            Kind::Selection => "selection",
            Kind::Commands => "commands",
        }
    }

    pub fn parse(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.name() == name)
    }
}

/// A JSON-RPC notification line (no `id`).
pub fn notification(method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "method": method, "params": params})
}

struct Sub {
    tx: SyncSender<Value>,
    kinds: BTreeSet<Kind>,
    /// Set when the outbox overflowed: the connection thread closes the socket.
    kill: Arc<AtomicBool>,
}

/// What a connection thread keeps: its id, the notifications to write, the kill flag.
pub struct Registration {
    pub id: u64,
    pub outbox: Receiver<Value>,
    pub kill: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct Hub {
    subs: Mutex<HashMap<u64, Sub>>,
    next: AtomicU64,
}

impl Hub {
    fn subs(&self) -> std::sync::MutexGuard<'_, HashMap<u64, Sub>> {
        self.subs.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn register(&self) -> Registration {
        let id = self.next.fetch_add(1, Ordering::SeqCst) + 1;
        let (tx, outbox) = mpsc::sync_channel(OUTBOX_CAPACITY);
        let kill = Arc::new(AtomicBool::new(false));
        self.subs().insert(
            id,
            Sub {
                tx,
                kinds: BTreeSet::new(),
                kill: kill.clone(),
            },
        );
        Registration { id, outbox, kill }
    }

    pub fn unregister(&self, id: u64) {
        self.subs().remove(&id);
    }

    /// Adds `kinds`; returns the subscription afterwards.
    pub fn subscribe(&self, id: u64, kinds: &[Kind]) -> Vec<Kind> {
        let mut subs = self.subs();
        let Some(sub) = subs.get_mut(&id) else {
            return Vec::new();
        };
        sub.kinds.extend(kinds.iter().copied());
        sub.kinds.iter().copied().collect()
    }

    pub fn unsubscribe(&self, id: u64, kinds: &[Kind]) -> Vec<Kind> {
        let mut subs = self.subs();
        let Some(sub) = subs.get_mut(&id) else {
            return Vec::new();
        };
        for k in kinds {
            sub.kinds.remove(k);
        }
        sub.kinds.iter().copied().collect()
    }

    fn push(sub: &Sub, message: &Value) -> bool {
        match sub.tx.try_send(message.clone()) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                sub.kill.store(true, Ordering::SeqCst);
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    /// Queues `message` for every connection subscribed to `kind`.
    pub fn publish(&self, kind: Kind, message: &Value) {
        let mut subs = self.subs();
        subs.retain(|_, sub| !sub.kinds.contains(&kind) || Hub::push(sub, message));
    }

    /// Queues `message` for one connection (job notifications).
    pub fn send_to(&self, id: u64, message: &Value) {
        let mut subs = self.subs();
        if subs.get(&id).is_some_and(|sub| !Hub::push(sub, message)) {
            subs.remove(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_reaches_subscribers_of_that_kind_only() {
        let hub = Hub::default();
        let (a, b) = (hub.register(), hub.register());
        hub.subscribe(a.id, &[Kind::Frame]);
        hub.subscribe(b.id, &[Kind::Scene]);
        hub.publish(Kind::Frame, &json!({"n": 1}));
        assert_eq!(a.outbox.try_recv().unwrap(), json!({"n": 1}));
        assert!(b.outbox.try_recv().is_err());
        assert_eq!(hub.unsubscribe(a.id, &[Kind::Frame]), vec![]);
        hub.publish(Kind::Frame, &json!({"n": 2}));
        assert!(a.outbox.try_recv().is_err());
    }

    #[test]
    fn a_reader_that_falls_behind_is_killed_and_never_blocks_the_publisher() {
        let hub = Hub::default();
        let slow = hub.register();
        hub.subscribe(slow.id, &[Kind::Frame]);
        for i in 0..=OUTBOX_CAPACITY {
            hub.publish(Kind::Frame, &json!({ "n": i }));
        }
        assert!(slow.kill.load(Ordering::SeqCst));
        assert_eq!(slow.outbox.try_iter().count(), OUTBOX_CAPACITY);
        // Gone from the hub: further publishes do nothing.
        hub.publish(Kind::Frame, &json!({}));
        assert!(slow.outbox.try_recv().is_err());
    }
}
