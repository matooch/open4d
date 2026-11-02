//! The blocking Rust client (docs/specs/RPC.md §6).

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

#[derive(Debug)]
pub enum ClientError {
    Io(std::io::Error),
    /// The server's reply is not a JSON-RPC response for this request.
    Protocol(String),
    /// The server answered with an error object.
    Rpc {
        code: i64,
        message: String,
        data: Option<Value>,
    },
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Io(e) => write!(f, "{e}"),
            ClientError::Protocol(m) => write!(f, "protocol error: {m}"),
            ClientError::Rpc { code, message, .. } => write!(f, "{message} (code {code})"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> ClientError {
        ClientError::Io(e)
    }
}

/// A mesh from `eval.mesh`.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshData {
    pub points: Vec<[f64; 3]>,
    pub polys: Vec<Vec<u32>>,
}

/// A server notification (docs/specs/RPC.md §10.1).
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub method: String,
    pub params: Value,
}

pub struct Client {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    token: Option<String>,
    next_id: u64,
    /// Bytes of a line not yet complete.
    pending: Vec<u8>,
    /// Notifications read while waiting for a response.
    notifications: VecDeque<Notification>,
}

impl Client {
    /// Connects to `127.0.0.1:port`. `token` goes into every request.
    pub fn connect(port: u16, token: Option<&str>) -> Result<Client, ClientError> {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let writer = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
        writer.set_read_timeout(Some(Duration::from_secs(60)))?;
        writer.set_nodelay(true)?;
        let reader = BufReader::new(writer.try_clone()?);
        Ok(Client {
            reader,
            writer,
            token: token.filter(|t| !t.is_empty()).map(String::from),
            next_id: 1,
            pending: Vec::new(),
            notifications: VecDeque::new(),
        })
    }

    /// Sends one request and waits for its response.
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, ClientError> {
        let id = self.next_id;
        self.next_id += 1;
        let mut request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        if let Some(token) = &self.token {
            request["token"] = json!(token);
        }
        let mut text = request.to_string();
        text.push('\n');
        self.writer.write_all(text.as_bytes())?;
        self.writer.flush()?;
        self.read_response(id)
    }

    /// The next complete line. `Ok(None)` when `deadline` passes first; partial data stays
    /// buffered for the next call.
    fn read_line(&mut self, deadline: Option<Instant>) -> Result<Option<String>, ClientError> {
        loop {
            if let Some(pos) = self.pending.iter().position(|b| *b == b'\n') {
                let rest = self.pending.split_off(pos + 1);
                let line = std::mem::replace(&mut self.pending, rest);
                return String::from_utf8(line)
                    .map(Some)
                    .map_err(|_| ClientError::Protocol("not UTF-8".into()));
            }
            let wait = match deadline {
                Some(d) => {
                    let left = d.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return Ok(None);
                    }
                    left
                }
                None => Duration::from_secs(60),
            };
            self.writer.set_read_timeout(Some(wait))?;
            match self.reader.fill_buf() {
                Ok([]) => return Err(ClientError::Protocol("connection closed".into())),
                Ok(chunk) => {
                    let n = chunk.len();
                    self.pending.extend_from_slice(chunk);
                    self.reader.consume(n);
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    if deadline.is_some() {
                        return Ok(None);
                    }
                    return Err(ClientError::Io(e));
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// The next notification, from the queue or the socket; `None` after `timeout`.
    pub fn next_notification(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Notification>, ClientError> {
        if let Some(n) = self.notifications.pop_front() {
            return Ok(Some(n));
        }
        let deadline = Instant::now() + timeout;
        let Some(line) = self.read_line(Some(deadline))? else {
            return Ok(None);
        };
        let value: Value = serde_json::from_str(&line)
            .map_err(|e| ClientError::Protocol(format!("not JSON: {e}")))?;
        match as_notification(&value) {
            Some(n) => Ok(Some(n)),
            None => Err(ClientError::Protocol(format!(
                "unexpected response while waiting for a notification: {value}"
            ))),
        }
    }

    fn read_response(&mut self, id: u64) -> Result<Value, ClientError> {
        let response: Value = loop {
            let line = self
                .read_line(None)?
                .ok_or_else(|| ClientError::Protocol("connection closed".into()))?;
            let value: Value = serde_json::from_str(&line)
                .map_err(|e| ClientError::Protocol(format!("not JSON: {e}")))?;
            match as_notification(&value) {
                Some(n) => self.notifications.push_back(n),
                None => break value,
            }
        };
        if let Some(error) = response.get("error") {
            // A null id is a connection-level refusal (too many connections, too large).
            return Err(ClientError::Rpc {
                code: error["code"].as_i64().unwrap_or(0),
                message: error["message"].as_str().unwrap_or("").to_string(),
                data: error.get("data").cloned(),
            });
        }
        if response["id"] != json!(id) {
            return Err(ClientError::Protocol(format!(
                "response id {} does not match request {id}",
                response["id"]
            )));
        }
        response
            .get("result")
            .cloned()
            .ok_or_else(|| ClientError::Protocol("response has neither result nor error".into()))
    }

    pub fn version(&mut self) -> Result<Value, ClientError> {
        self.call("app.version", json!({}))
    }

    pub fn command_list(&mut self) -> Result<Value, ClientError> {
        self.call("command.list", json!({}))
    }

    /// Runs a command; returns the command's own result.
    pub fn command_run(&mut self, id: &str, args: Value) -> Result<Value, ClientError> {
        let mut reply = self.call("command.run", json!({"id": id, "args": args}))?;
        Ok(reply["result"].take())
    }

    pub fn scene_get(&mut self) -> Result<Value, ClientError> {
        self.call("scene.get", json!({}))
    }

    pub fn node_get(&mut self, id: u64) -> Result<Value, ClientError> {
        self.call("node.get", json!({"id": id}))
    }

    /// The node's world-space mesh at `frame` (`None`: the current frame).
    pub fn eval_mesh(&mut self, node: u64, frame: Option<i64>) -> Result<MeshData, ClientError> {
        let mut params = json!({"node": node});
        if let Some(f) = frame {
            params["frame"] = json!(f);
        }
        let reply = self.call("eval.mesh", params)?;
        let bad = |what: &str| ClientError::Protocol(format!("eval.mesh: bad {what}"));
        let points = serde_json::from_value(reply["points"].clone()).map_err(|_| bad("points"))?;
        let polys = serde_json::from_value(reply["polys"].clone()).map_err(|_| bad("polys"))?;
        Ok(MeshData { points, polys })
    }

    /// Adds `kinds` to this connection's subscription; returns the subscription.
    pub fn subscribe(&mut self, kinds: &[&str]) -> Result<Vec<String>, ClientError> {
        self.events("events.subscribe", kinds)
    }

    pub fn unsubscribe(&mut self, kinds: &[&str]) -> Result<Vec<String>, ClientError> {
        self.events("events.unsubscribe", kinds)
    }

    fn events(&mut self, method: &str, kinds: &[&str]) -> Result<Vec<String>, ClientError> {
        let r = self.call(method, json!({ "kinds": kinds }))?;
        Ok(r["kinds"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|k| k.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default())
    }

    /// A PNG of the editor's viewport (`viewport.screenshot`).
    pub fn screenshot(&mut self, width: u32, height: u32) -> Result<Vec<u8>, ClientError> {
        let r = self.call(
            "viewport.screenshot",
            json!({"width": width, "height": height}),
        )?;
        r["data"]
            .as_str()
            .and_then(crate::base64::decode)
            .ok_or_else(|| ClientError::Protocol("viewport.screenshot: bad data".into()))
    }

    /// Starts a long job; returns its id.
    pub fn job_start(&mut self, kind: &str, args: Value) -> Result<u64, ClientError> {
        let r = self.call("job.start", json!({"kind": kind, "args": args}))?;
        r["job_id"]
            .as_u64()
            .ok_or_else(|| ClientError::Protocol("job.start: no job_id".into()))
    }

    pub fn job_cancel(&mut self, id: u64) -> Result<Value, ClientError> {
        self.call("job.cancel", json!({"id": id}))
    }

    pub fn timeline_get(&mut self) -> Result<Value, ClientError> {
        self.call("timeline.get", json!({}))
    }
}

/// A line with a `method` and no `id` is a notification.
fn as_notification(value: &Value) -> Option<Notification> {
    if value.get("id").is_some() {
        return None;
    }
    Some(Notification {
        method: value.get("method")?.as_str()?.to_string(),
        params: value.get("params").cloned().unwrap_or(Value::Null),
    })
}
