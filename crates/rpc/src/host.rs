//! The methods (docs/specs/RPC.md §4), run on the thread that owns the document.

use forge_app::query::node_json;
use forge_app::{App, AppError};
use forge_command::CommandError;
use forge_core::NodeId;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::protocol::{RpcError, code};

/// What the methods need from the application that owns the document.
pub trait Host {
    fn app(&self) -> &App;
    /// Runs a registry command with the host's own context (the editor adds view hooks).
    fn run_command(&mut self, id: &str, args: Value) -> Result<Value, CommandError>;
}

impl Host for App {
    fn app(&self) -> &App {
        self
    }

    fn run_command(&mut self, id: &str, args: Value) -> Result<Value, CommandError> {
        self.run(id, args)
    }
}

fn command_error(e: CommandError) -> RpcError {
    match e {
        CommandError::InvalidArgs(m) => RpcError::new(code::INVALID_PARAMS, m),
        other => RpcError::failed("command", other.to_string()),
    }
}

fn app_error(e: AppError) -> RpcError {
    let kind = match &e {
        AppError::UnknownNode(_) | AppError::NoGeometry(_) => "node",
        AppError::Build(_) => "build",
        AppError::Command(_) => "command",
        AppError::NothingToExport | AppError::Io(_) => "io",
    };
    RpcError::failed(kind, e.to_string())
}

fn parse<T: for<'de> Deserialize<'de>>(method: &str, params: Value) -> Result<T, RpcError> {
    serde_json::from_value(params).map_err(|e| RpcError::invalid_params(format!("{method}: {e}")))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RunParams {
    id: String,
    #[serde(default)]
    args: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeParams {
    id: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeshParams {
    node: u64,
    frame: Option<i64>,
}

/// Runs `method` with `params` (an object) on `host`.
pub fn dispatch(host: &mut dyn Host, method: &str, params: Value) -> Result<Value, RpcError> {
    match method {
        "app.version" => {
            parse::<Empty>(method, params)?;
            Ok(json!({"name": "forge3d", "version": env!("CARGO_PKG_VERSION"), "protocol": 1}))
        }
        "command.list" => {
            parse::<Empty>(method, params)?;
            let list: Vec<Value> = host
                .app()
                .commands
                .iter()
                .map(|c| {
                    json!({"id": c.id(), "label": c.label(), "undoable": c.undoable(),
                        "args_schema": c.args_schema()})
                })
                .collect();
            Ok(json!({ "commands": list }))
        }
        "command.run" => {
            let p: RunParams = parse(method, params)?;
            let result = host.run_command(&p.id, p.args).map_err(command_error)?;
            Ok(json!({"command": p.id, "result": result}))
        }
        "scene.get" => {
            parse::<Empty>(method, params)?;
            let app = host.app();
            let scene = app.doc.scene();
            let settings = scene.settings();
            let roots: Vec<Value> = scene.roots().iter().map(|r| node_json(scene, *r)).collect();
            let selected: Vec<u64> = app.selection.nodes.iter().map(|n| n.0).collect();
            Ok(
                json!({"fps": settings.fps, "start": settings.start, "end": settings.end,
                "frame": app.timeline.current, "nodes": scene.len(),
                "selected": selected, "roots": roots}),
            )
        }
        "node.get" => {
            let p: NodeParams = parse(method, params)?;
            let scene = host.app().doc.scene();
            if !scene.contains(NodeId(p.id)) {
                return Err(app_error(AppError::UnknownNode(p.id)));
            }
            Ok(node_json(scene, NodeId(p.id)))
        }
        "eval.mesh" => {
            let p: MeshParams = parse(method, params)?;
            let app = host.app();
            let frame = p.frame.unwrap_or(app.timeline.current);
            let mesh = app.eval_mesh(NodeId(p.node), frame).map_err(app_error)?;
            let points: Vec<[f64; 3]> = mesh.points.iter().map(|q| q.to_array()).collect();
            Ok(json!({"node": p.node, "frame": frame, "points": points, "polys": mesh.polys}))
        }
        "timeline.get" => {
            parse::<Empty>(method, params)?;
            let app = host.app();
            let settings = app.settings();
            Ok(json!({"frame": app.timeline.current, "fps": settings.fps,
                "start": settings.start, "end": settings.end,
                "playing": app.timeline.playing, "autokey": app.autokey}))
        }
        other => Err(RpcError::new(
            code::METHOD_NOT_FOUND,
            format!("unknown method {other}"),
        )),
    }
}
