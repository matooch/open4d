//! forge-rpc: JSON-RPC 2.0 over newline-delimited TCP on 127.0.0.1, with a token.
//! Spec: docs/specs/RPC.md.
//!
//! [`Server`] runs on its own threads and posts every request to an [`Inbox`]; the thread
//! that owns the document (the editor's UI thread, or a test) drains it and answers through
//! the [`Host`] trait. [`Client`] is the matching blocking client.

mod client;
mod config;
mod host;
mod protocol;
mod server;

pub use client::{Client, ClientError, MeshData};
pub use config::{DEFAULT_PORT, RpcSettings, ct_eq, settings_path};
pub use host::{Host, dispatch};
pub use protocol::{RpcError, code};
pub use server::{
    Inbox, MAX_CONNECTIONS, MAX_MESSAGE_BYTES, MAX_REQUESTS_PER_SECOND, Server, ServerConfig,
    StartError,
};
