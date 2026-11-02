//! forge-mcp: a minimal Model Context Protocol server over stdio (docs/specs/RPC.md section 8).
//!
//! [`Server::handle_line`] turns one input line into at most one output line. Tools are
//! generated from the command list of a [`Backend`]: the running editor ([`EditorBackend`],
//! through `forge_rpc::Client`) or an in-process app ([`HeadlessBackend`]).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use forge_app::App;
use forge_rpc::{Client, ClientError, RpcSettings, dispatch};
use serde_json::{Value, json};

/// Protocol revisions this server speaks; the newest is offered when the client asks for
/// another one.
pub const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// A failed backend call.
#[derive(Debug, Clone, PartialEq)]
pub struct CallError {
    pub code: i64,
    pub message: String,
}

/// Where the RPC methods (RPC.md section 4) are answered.
pub trait Backend {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, CallError>;

    /// Whether the command list can change while the server runs (the editor can).
    fn supports_list_changed(&self) -> bool {
        false
    }
}

/// An in-process app, no editor and no port.
pub struct HeadlessBackend {
    app: App,
}

impl HeadlessBackend {
    pub fn new(app: App) -> HeadlessBackend {
        HeadlessBackend { app }
    }
}

impl Backend for HeadlessBackend {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, CallError> {
        dispatch(&mut self.app, method, params).map_err(|e| CallError {
            code: e.code,
            message: e.message,
        })
    }
}

/// The running editor over RPC. Connects on first use and again after an I/O failure.
pub struct EditorBackend {
    port: u16,
    token: Option<String>,
    client: Option<Client>,
}

impl EditorBackend {
    pub fn new(port: u16, token: Option<String>) -> EditorBackend {
        EditorBackend {
            port,
            token,
            client: None,
        }
    }

    /// Reads port and token from the environment and the settings file (RPC.md section 2).
    pub fn from_environment() -> EditorBackend {
        let s = RpcSettings::from_environment();
        EditorBackend::new(s.port, s.token)
    }
}

impl Backend for EditorBackend {
    fn supports_list_changed(&self) -> bool {
        true
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, CallError> {
        if self.client.is_none() {
            let client = Client::connect(self.port, self.token.as_deref()).map_err(|e| {
                CallError {
                    code: -32603,
                    message: format!(
                        "cannot connect to the editor on port {}: {e} (is it running with RPC enabled, and are FORGE_RPC_PORT and FORGE_RPC_TOKEN right?)",
                        self.port
                    ),
                }
            })?;
            self.client = Some(client);
        }
        let client = self.client.as_mut().expect("connected above");
        match client.call(method, params) {
            Ok(v) => Ok(v),
            Err(ClientError::Rpc { code, message, .. }) => {
                if code == -32001 || code == -32003 || code == -32004 {
                    // The server closed the connection after this reply.
                    self.client = None;
                }
                Err(CallError { code, message })
            }
            Err(e) => {
                self.client = None;
                Err(CallError {
                    code: -32603,
                    message: format!("editor connection failed: {e}"),
                })
            }
        }
    }
}

/// What a tool does when called.
#[derive(Debug, Clone, PartialEq)]
enum Target {
    Scene,
    Node,
    MeshSummary,
    Screenshot,
    /// An alias for a command, or a command itself.
    Command(String),
}

/// The tool name of a command id: dots become underscores.
pub fn tool_name(command_id: &str) -> String {
    command_id.replace('.', "_")
}

/// A schema that is an object schema, wrapping anything else.
fn object_schema(schema: &Value) -> Value {
    if schema.get("type").and_then(Value::as_str) == Some("object") {
        schema.clone()
    } else {
        json!({"type": "object", "properties": {"value": schema}, "required": ["value"]})
    }
}

/// An MCP image content block from a `viewport.screenshot` result.
fn image_result(shot: &Value) -> Value {
    json!({"content": [{"type": "image", "data": shot["data"], "mimeType": "image/png"}],
        "isError": false})
}

/// Counts and the bounding box of an `eval.mesh` result.
pub fn mesh_summary(mesh: &Value) -> Value {
    let points = mesh["points"].as_array().cloned().unwrap_or_default();
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in &points {
        for (axis, (lo, hi)) in min.iter_mut().zip(max.iter_mut()).enumerate() {
            let v = p[axis].as_f64().unwrap_or(0.0);
            *lo = lo.min(v);
            *hi = hi.max(v);
        }
    }
    let bbox = if points.is_empty() {
        Value::Null
    } else {
        json!({"min": min, "max": max})
    };
    json!({"node": mesh["node"], "frame": mesh["frame"],
        "points": points.len(),
        "polys": mesh["polys"].as_array().map_or(0, Vec::len),
        "bbox": bbox})
}

struct ToolDef {
    name: String,
    description: String,
    schema: Value,
    target: Target,
}

fn builtin_tools() -> Vec<ToolDef> {
    let def = |name: &str, description: &str, schema: Value, target: Target| ToolDef {
        name: name.to_string(),
        description: description.to_string(),
        schema,
        target,
    };
    vec![
        def(
            "scene_get",
            "The whole scene: frame range, selection and the node tree with parameters in user units.",
            json!({"type": "object", "properties": {}, "additionalProperties": false}),
            Target::Scene,
        ),
        def(
            "node_get",
            "One node: type, parameters, tags, children.",
            json!({"type": "object", "properties": {"id": {"type": "integer", "description": "node id"}},
                "required": ["id"], "additionalProperties": false}),
            Target::Node,
        ),
        def(
            "eval_mesh_summary",
            "Evaluate a node's mesh (animation and deformers applied, world space) and return only the point and polygon counts and the bounding box.",
            json!({"type": "object", "properties": {
                "node": {"type": "integer", "description": "node id"},
                "frame": {"type": "integer", "description": "frame; default the current one"}},
                "required": ["node"], "additionalProperties": false}),
            Target::MeshSummary,
        ),
        def(
            "viewport_screenshot",
            "Render the editor's viewport (current camera, shading and frame) and return it as a PNG image. Needs the editor window's machine to have a GPU; headless servers report an error.",
            json!({"type": "object", "properties": {
                "width": {"type": "integer", "minimum": 1, "maximum": 4096, "description": "pixels, default 800"},
                "height": {"type": "integer", "minimum": 1, "maximum": 4096, "description": "pixels, default 600"}},
                "additionalProperties": false}),
            Target::Screenshot,
        ),
        def(
            "export_obj",
            "Write the scene evaluated at the current frame as a Wavefront OBJ file (alias of file_export_obj).",
            json!({"type": "object", "properties": {
                "path": {"type": "string", "description": "output file"},
                "selected": {"type": "array", "items": {"type": "integer"},
                    "description": "only these nodes and their descendants"}},
                "required": ["path"], "additionalProperties": false}),
            Target::Command("file.export_obj".into()),
        ),
        def(
            "frame_set",
            "Move the timeline to a frame, clamped to the project range (alias of timeline_set_frame).",
            json!({"type": "object", "properties": {"frame": {"type": "integer"}},
                "required": ["frame"], "additionalProperties": false}),
            Target::Command("timeline.set_frame".into()),
        ),
    ]
}

/// The MCP server state.
pub struct Server<B: Backend> {
    backend: B,
    tools: Option<Vec<ToolDef>>,
    index: HashMap<String, usize>,
    /// Set by the thread that watches the editor: the cached tool list is stale.
    stale: Arc<AtomicBool>,
}

fn error_reply(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message.into()}})
}

fn tool_result(text: String, is_error: bool) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

impl<B: Backend> Server<B> {
    pub fn new(backend: B) -> Server<B> {
        Server {
            backend,
            tools: None,
            index: HashMap::new(),
            stale: Arc::new(AtomicBool::new(false)),
        }
    }

    /// The flag that makes the next `tools/list` or `tools/call` reload the command list.
    pub fn stale_flag(&self) -> Arc<AtomicBool> {
        self.stale.clone()
    }

    /// Handles one input line; the reply line, or `None` for notifications and blank input.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let message: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                return Some(
                    error_reply(Value::Null, -32700, format!("parse error: {e}")).to_string(),
                );
            }
        };
        self.handle(&message).map(|v| v.to_string())
    }

    pub fn handle(&mut self, message: &Value) -> Option<Value> {
        let Some(object) = message.as_object() else {
            return Some(error_reply(
                Value::Null,
                -32600,
                "invalid request: not an object",
            ));
        };
        let id = object.get("id").cloned();
        let method = match object.get("method").and_then(Value::as_str) {
            Some(m) => m,
            None if id.is_some()
                && (object.contains_key("result") || object.contains_key("error")) =>
            {
                return None; // a response to something we never sent
            }
            None => {
                return Some(error_reply(
                    id.unwrap_or(Value::Null),
                    -32600,
                    "invalid request: method must be a string",
                ));
            }
        };
        let params = object.get("params").cloned().unwrap_or_else(|| json!({}));
        // No id: a notification, never answered.
        let id = id?;
        let outcome = match method {
            "initialize" => Ok(self.initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => self.tools_list(),
            "tools/call" => self.tools_call(&params),
            other => Err((-32601, format!("method not found: {other}"))),
        };
        Some(match outcome {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err((code, message)) => error_reply(id, code, message),
        })
    }

    fn initialize(&self, params: &Value) -> Value {
        let tools_capability = if self.backend.supports_list_changed() {
            json!({"listChanged": true})
        } else {
            json!({})
        };
        let version = params["protocolVersion"]
            .as_str()
            .filter(|v| PROTOCOL_VERSIONS.contains(v))
            .unwrap_or(PROTOCOL_VERSIONS[0]);
        json!({
            "protocolVersion": version,
            "capabilities": {"tools": tools_capability},
            "serverInfo": {"name": "forge-mcp", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Forge3D scene tools. One tool per editor command (object_create, param_set, ...) plus scene_get, node_get, eval_mesh_summary, export_obj and frame_set. Node ids come from scene_get and from command results. Every editing command is one undo step (edit_undo).",
        })
    }

    /// Builds the tool list from the backend once; later calls reuse it.
    fn load_tools(&mut self) -> Result<(), (i64, String)> {
        if self.stale.swap(false, Ordering::SeqCst) {
            self.tools = None;
        }
        if self.tools.is_some() {
            return Ok(());
        }
        let list = self
            .backend
            .call("command.list", json!({}))
            .map_err(|e| (-32603, e.message))?;
        let mut tools = builtin_tools();
        for c in list["commands"].as_array().into_iter().flatten() {
            let (Some(id), Some(label)) = (c["id"].as_str(), c["label"].as_str()) else {
                continue;
            };
            let name = tool_name(id);
            if tools.iter().any(|t| t.name == name) {
                continue;
            }
            let undo = if c["undoable"].as_bool() == Some(true) {
                "Undoable editing command."
            } else {
                "Not an undoable edit."
            };
            tools.push(ToolDef {
                name,
                description: format!("{label}. {undo} Runs command `{id}`."),
                schema: object_schema(&c["args_schema"]),
                target: Target::Command(id.to_string()),
            });
        }
        self.index = tools
            .iter()
            .enumerate()
            .map(|(i, t)| (t.name.clone(), i))
            .collect();
        self.tools = Some(tools);
        Ok(())
    }

    fn tools_list(&mut self) -> Result<Value, (i64, String)> {
        self.load_tools()?;
        let tools: Vec<Value> = self
            .tools
            .iter()
            .flatten()
            .map(|t| json!({"name": t.name, "description": t.description, "inputSchema": t.schema}))
            .collect();
        Ok(json!({ "tools": tools }))
    }

    fn tools_call(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params["name"]
            .as_str()
            .ok_or((-32602, "tools/call: name must be a string".to_string()))?;
        let arguments = match params.get("arguments") {
            None | Some(Value::Null) => json!({}),
            Some(a) if a.is_object() => a.clone(),
            Some(_) => return Err((-32602, "tools/call: arguments must be an object".into())),
        };
        // A builtin needs no command list, so it works even if listing would fail.
        let target = match builtin_tools().into_iter().find(|t| t.name == name) {
            Some(t) => t.target,
            None => {
                self.load_tools()?;
                let i = *self
                    .index
                    .get(name)
                    .ok_or((-32602, format!("unknown tool {name}")))?;
                self.tools.as_ref().expect("loaded")[i].target.clone()
            }
        };
        let outcome = match target {
            Target::Scene => self.backend.call("scene.get", arguments),
            Target::Node => self.backend.call("node.get", arguments),
            Target::MeshSummary => self
                .backend
                .call("eval.mesh", arguments)
                .map(|m| mesh_summary(&m)),
            Target::Screenshot => self
                .backend
                .call("viewport.screenshot", arguments)
                .map(|r| image_result(&r)),
            Target::Command(id) => self
                .backend
                .call("command.run", json!({"id": id, "args": arguments}))
                .map(|r| r["result"].clone()),
        };
        Ok(match outcome {
            Ok(v) if v.get("content").is_some() && v.get("isError").is_some() => v,
            Ok(v) => tool_result(v.to_string(), false),
            Err(e) => tool_result(e.message, true),
        })
    }
}

/// Follows the editor's command list on a second RPC connection. Calls `on_change` when
/// the editor reports `commands.changed`, and after reconnecting to an editor it had lost
/// (it may have restarted with other commands). Retries every second until `stop`. Meant
/// to run on its own thread.
pub fn watch_command_changes(
    port: u16,
    token: Option<String>,
    stop: &AtomicBool,
    on_change: impl Fn(),
) {
    let mut connected_before = false;
    while !stop.load(Ordering::SeqCst) {
        let connected = Client::connect(port, token.as_deref())
            .and_then(|mut c| c.subscribe(&["commands"]).map(|_| c));
        let Ok(mut client) = connected else {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        };
        if connected_before {
            on_change();
        }
        connected_before = true;
        while !stop.load(Ordering::SeqCst) {
            match client.next_notification(Duration::from_millis(250)) {
                Ok(Some(n)) if n.method == "commands.changed" => on_change(),
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_replace_dots() {
        assert_eq!(tool_name("object.create"), "object_create");
        assert_eq!(tool_name("view.frame_all"), "view_frame_all");
    }

    #[test]
    fn a_screenshot_becomes_an_image_block() {
        let r = image_result(&json!({"width": 1, "height": 1, "format": "png", "data": "AAAA"}));
        assert_eq!(r["isError"], false);
        assert_eq!(
            r["content"][0],
            json!({"type": "image", "data": "AAAA", "mimeType": "image/png"})
        );
    }

    #[test]
    fn non_object_schemas_are_wrapped() {
        let obj = json!({"type": "object", "properties": {}});
        assert_eq!(object_schema(&obj), obj);
        assert_eq!(object_schema(&json!({}))["type"], "object");
    }

    #[test]
    fn summary_has_counts_and_bbox_only() {
        let mesh = json!({"node": 1, "frame": 0,
            "points": [[0.0, 1.0, 2.0], [-1.0, 3.0, 0.5]], "polys": [[0, 1]]});
        let s = mesh_summary(&mesh);
        assert_eq!(s["points"], 2);
        assert_eq!(s["polys"], 1);
        assert_eq!(s["bbox"]["min"], json!([-1.0, 1.0, 0.5]));
        assert_eq!(s["bbox"]["max"], json!([0.0, 3.0, 2.0]));
        let empty = mesh_summary(&json!({"node": 1, "frame": 0, "points": [], "polys": []}));
        assert!(empty["bbox"].is_null());
    }

    #[test]
    fn registry_commands_all_become_valid_unique_tools() {
        let mut server = Server::new(HeadlessBackend::new(App::new_default()));
        let list = server.tools_list().unwrap();
        let tools = list["tools"].as_array().unwrap();
        let commands = App::new_default().commands.len();
        assert_eq!(
            tools.len(),
            commands + 6,
            "every command plus the 6 builtins"
        );
        let mut names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), tools.len());
    }
}
