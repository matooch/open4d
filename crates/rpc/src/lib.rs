//! forge-rpc: JSON-RPC 2.0 over newline-delimited TCP on 127.0.0.1, with a token.
//! Spec: docs/specs/RPC.md.
//!
//! [`Server`] runs on its own threads and posts every request to an [`Inbox`]; the thread
//! that owns the document (the editor's UI thread, or a test) drains it and answers through
//! the [`Host`] trait. [`Client`] is the matching blocking client.

pub mod base64;
mod client;
mod config;
pub mod docs;
mod host;
mod hub;
mod jobs;
mod protocol;
mod server;

pub use client::{Client, ClientError, MeshData, Notification};
pub use config::{DEFAULT_PORT, RpcSettings, ct_eq, settings_path};
pub use host::{Host, MAX_SCREENSHOT_SIDE, dispatch};
pub use hub::{Kind, OUTBOX_CAPACITY};
pub use jobs::MAX_ACTIVE_JOBS;
pub use protocol::{RpcError, code};
pub use server::{
    Inbox, MAX_CONNECTIONS, MAX_MESSAGE_BYTES, MAX_REQUESTS_PER_SECOND, Server, ServerConfig,
    StartError, handle_request_line,
};
