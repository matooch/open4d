//! The blocking Rust client (docs/specs/RPC.md §6).

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

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

pub struct Client {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    token: Option<String>,
    next_id: u64,
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

    fn read_response(&mut self, id: u64) -> Result<Value, ClientError> {
        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            return Err(ClientError::Protocol("connection closed".into()));
        }
        let response: Value = serde_json::from_str(&line)
            .map_err(|e| ClientError::Protocol(format!("not JSON: {e}")))?;
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

    pub fn timeline_get(&mut self) -> Result<Value, ClientError> {
        self.call("timeline.get", json!({}))
    }
}
