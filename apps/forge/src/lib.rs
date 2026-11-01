//! The `forge` command-line tool. Spec: docs/specs/CLI.md.
//!
//! A thin layer over [`forge_app::App`]: `run` parses the arguments, executes one
//! subcommand and returns the exit code (0 ok, 1 user error, 2 scene load error,
//! 3 internal error). The editor forwards `--headless <args>` to the same function.

use std::ffi::OsString;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use forge_app::{App, AppError};
use forge_core::Unit;
use forge_core::{NodeId, Scene, Value};
use forge_generators::is_consumed;
use serde_json::{Map, json};

type Json = serde_json::Value;

pub const EXIT_USER: u8 = 1;
pub const EXIT_LOAD: u8 = 2;
pub const EXIT_INTERNAL: u8 = 3;

#[derive(Parser)]
#[command(
    name = "forge",
    about = "Headless Forge3D: scenes, commands and export"
)]
struct Cli {
    /// Machine output: one JSON object on stdout, messages on stderr.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Sub,
}

#[derive(Subcommand)]
enum Sub {
    /// Write an empty scene.
    New {
        path: String,
        /// Overwrite an existing file.
        #[arg(long)]
        force: bool,
    },
    /// Show the node tree.
    Info { scene: String },
    /// Load the scene and build every node.
    Validate { scene: String },
    /// Evaluate a frame, report geometry, optionally export it.
    Eval {
        scene: String,
        #[arg(long, default_value_t = 0, allow_negative_numbers = true)]
        frame: i64,
        #[arg(long)]
        node: Option<u64>,
        #[arg(long)]
        export: Option<String>,
    },
    /// Export the scene evaluated at a frame.
    Export {
        scene: String,
        #[arg(long, default_value = "obj")]
        format: String,
        #[arg(long)]
        out: String,
        /// Only these node ids (and their descendants), comma separated.
        #[arg(long, value_delimiter = ',')]
        selected: Option<Vec<u64>>,
        #[arg(long, default_value_t = 0, allow_negative_numbers = true)]
        frame: i64,
    },
    /// Run one command on a scene file.
    Cmd {
        scene: String,
        command: String,
        /// JSON object with the command arguments.
        #[arg(long, default_value = "{}")]
        args: String,
        /// Write the changed scene back to the file.
        #[arg(long)]
        save: bool,
    },
    /// List the command registry with argument schemas.
    Commands,
    /// Run a script (not available yet).
    Run {
        script: String,
        #[arg(long)]
        scene: Option<String>,
    },
    /// Call a running editor (not available yet).
    Rpc {
        method: String,
        #[arg(long)]
        params: Option<String>,
    },
    /// Render an image (not available yet).
    Render,
}

/// A failed subcommand: exit code plus message.
struct Fail {
    code: u8,
    message: String,
}

fn user(message: impl Into<String>) -> Fail {
    Fail {
        code: EXIT_USER,
        message: message.into(),
    }
}

fn load_fail(message: impl Into<String>) -> Fail {
    Fail {
        code: EXIT_LOAD,
        message: message.into(),
    }
}

impl From<AppError> for Fail {
    fn from(e: AppError) -> Fail {
        user(e.to_string())
    }
}

/// A successful subcommand: the JSON fields and the human text.
struct Out {
    json: Json,
    text: String,
}

type Outcome = Result<Out, Fail>;

/// Runs the tool with `args` (without the program name) and returns the exit code.
pub fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let argv: Vec<OsString> = std::iter::once(OsString::from("forge"))
        .chain(args.into_iter().map(Into::into))
        .collect();
    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            return ExitCode::from(if e.use_stderr() { EXIT_USER } else { 0 });
        }
    };
    let json = cli.json;
    let outcome = catch_unwind(AssertUnwindSafe(|| execute(cli.command)));
    let outcome = outcome.unwrap_or_else(|_| {
        Err(Fail {
            code: EXIT_INTERNAL,
            message: "internal error".into(),
        })
    });
    match outcome {
        Ok(out) => {
            if json {
                let mut object = Map::new();
                object.insert("ok".into(), Json::Bool(true));
                if let Json::Object(fields) = out.json {
                    object.extend(fields);
                }
                println!("{}", Json::Object(object));
            } else if !out.text.is_empty() {
                println!("{}", out.text.trim_end());
            }
            ExitCode::SUCCESS
        }
        Err(fail) => {
            eprintln!("error: {}", fail.message);
            if json {
                println!(
                    "{}",
                    json!({"ok": false, "error": {"code": fail.code, "message": fail.message}})
                );
            }
            ExitCode::from(fail.code)
        }
    }
}

fn execute(command: Sub) -> Outcome {
    match command {
        Sub::New { path, force } => new(&path, force),
        Sub::Info { scene } => info(&scene),
        Sub::Validate { scene } => validate(&scene),
        Sub::Eval {
            scene,
            frame,
            node,
            export,
        } => eval(&scene, frame, node, export.as_deref()),
        Sub::Export {
            scene,
            format,
            out,
            selected,
            frame,
        } => {
            if format != "obj" {
                return Err(user(format!("unsupported format {format:?} (only obj)")));
            }
            export(&scene, &out, selected.as_deref(), frame)
        }
        Sub::Cmd {
            scene,
            command,
            args,
            save,
        } => cmd(&scene, &command, &args, save),
        Sub::Commands => commands(),
        Sub::Run { .. } => Err(stub("run", "brief 016")),
        Sub::Rpc { .. } => Err(stub("rpc", "brief 014")),
        Sub::Render => Err(stub("render", "area K")),
    }
}

fn stub(name: &str, when: &str) -> Fail {
    user(format!("{name} is not available yet ({when})"))
}

/// Loads a scene file; load warnings go to stderr.
fn open(path: &str) -> Result<(App, Vec<String>), Fail> {
    let mut app = App::new_default();
    let result = app.open(path).map_err(|e| load_fail(e.to_string()))?;
    let warnings: Vec<String> = result["warnings"]
        .as_array()
        .map(|w| {
            w.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    for w in &warnings {
        eprintln!("warning: {w}");
    }
    Ok((app, warnings))
}

fn new(path: &str, force: bool) -> Outcome {
    if !force && std::path::Path::new(path).exists() {
        return Err(user(format!("{path} exists (use --force to overwrite)")));
    }
    let mut app = App::new_default();
    app.save(path).map_err(|e| user(e.to_string()))?;
    Ok(Out {
        json: json!({"path": path}),
        text: format!("created {path}"),
    })
}

/// A stored value in the units the commands use (degrees, percent).
fn display_value(value: &Value, unit: Unit) -> Json {
    let scale = |v: f64| match unit {
        Unit::Angle => v.to_degrees(),
        Unit::Percent => v * 100.0,
        _ => v,
    };
    match value {
        Value::Bool(b) => json!(b),
        Value::Int(i) => json!(i),
        Value::Float(f) => json!(scale(*f)),
        Value::Vec3(v) => json!(v.map(scale)),
        Value::Color(c) => json!(c),
        Value::Enum(s) | Value::String(s) => json!(s),
        Value::Link(l) => json!(l.map(|n| n.0)),
        Value::Mesh(m) => json!({"points": m.points.len(), "polys": m.polys.len()}),
        Value::IndexList(i) => json!(i),
    }
}

fn node_json(scene: &Scene, id: NodeId) -> Json {
    let Some(node) = scene.node(id) else {
        return Json::Null;
    };
    let descs = scene
        .node_type(id)
        .map(|t| t.params.as_slice())
        .unwrap_or(&[]);
    let mut params = Map::new();
    for (name, value) in &node.params {
        let unit = descs
            .iter()
            .find(|d| &d.id == name)
            .map(|d| d.ui.unit)
            .unwrap_or_default();
        params.insert(name.clone(), display_value(value, unit));
    }
    let mut out = Map::new();
    out.insert("id".into(), json!(id.0));
    out.insert("type".into(), json!(node.type_id));
    out.insert("name".into(), json!(node.name));
    out.insert("params".into(), Json::Object(params));
    if !node.tags.is_empty() {
        let tags: Vec<Json> = node.tags.iter().map(|t| node_json(scene, *t)).collect();
        out.insert("tags".into(), json!(tags));
    }
    if !node.tracks.is_empty() {
        out.insert(
            "animated".into(),
            json!(node.tracks.keys().collect::<Vec<_>>()),
        );
    }
    if !node.is_tag() {
        let children: Vec<Json> = node.children.iter().map(|c| node_json(scene, *c)).collect();
        out.insert("children".into(), json!(children));
    }
    Json::Object(out)
}

fn tree_text(scene: &Scene, id: NodeId, depth: usize, out: &mut String) {
    let Some(node) = scene.node(id) else { return };
    let tags: Vec<String> = node
        .tags
        .iter()
        .filter_map(|t| scene.node(*t))
        .map(|t| format!("[{}]", t.type_id))
        .collect();
    out.push_str(&format!(
        "{}{} {} {:?} {}\n",
        "  ".repeat(depth),
        id.0,
        node.type_id,
        node.name,
        tags.join(" ")
    ));
    for c in &node.children {
        tree_text(scene, *c, depth + 1, out);
    }
}

fn info(path: &str) -> Outcome {
    let (app, _) = open(path)?;
    let scene = app.doc.scene();
    let settings = scene.settings();
    let roots: Vec<Json> = scene.roots().iter().map(|r| node_json(scene, *r)).collect();
    let mut text = format!(
        "{path}: {} nodes, {} fps, frames {}..{}\n",
        scene.len(),
        settings.fps,
        settings.start,
        settings.end
    );
    for r in scene.roots() {
        tree_text(scene, *r, 0, &mut text);
    }
    Ok(Out {
        json: json!({"path": path, "fps": settings.fps, "start": settings.start,
            "end": settings.end, "nodes": scene.len(), "roots": roots}),
        text,
    })
}

/// Every non-tag node in tree order.
fn all_nodes(scene: &Scene) -> Vec<NodeId> {
    let mut out = Vec::new();
    for r in scene.roots() {
        out.push(*r);
        out.extend(scene.descendants(*r));
    }
    out.retain(|id| scene.node(*id).is_some_and(|n| !n.is_tag()));
    out
}

fn validate(path: &str) -> Outcome {
    let (app, warnings) = open(path)?;
    let frame = app.settings().start;
    let mut errors = Vec::new();
    for id in all_nodes(app.doc.scene()) {
        if let Err(AppError::Build(message)) = app.eval_mesh(id, frame) {
            errors.push(json!({"node": id.0, "message": message}));
        }
    }
    let ok = errors.is_empty();
    let mut text = format!(
        "{path}: {} warnings, {} errors\n",
        warnings.len(),
        errors.len()
    );
    for e in &errors {
        text.push_str(&format!("node {}: {}\n", e["node"], e["message"]));
    }
    if ok {
        Ok(Out {
            json: json!({"warnings": warnings, "errors": errors}),
            text,
        })
    } else {
        // The error list is the payload: print it before failing.
        eprint!("{text}");
        Err(user(format!("{} node(s) failed to build", errors.len())))
    }
}

fn bbox(points: &[[f64; 3]]) -> Json {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in points {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    json!([lo, hi])
}

fn eval(path: &str, frame: i64, node: Option<u64>, export_to: Option<&str>) -> Outcome {
    let (mut app, _) = open(path)?;
    let scene = app.frame(frame);
    let frame = app.timeline.current;
    let ids: Vec<NodeId> = match node {
        Some(n) => vec![NodeId(n)],
        None => all_nodes(&scene)
            .into_iter()
            .filter(|id| !is_consumed(&scene, *id))
            .collect(),
    };
    let mut nodes = Vec::new();
    let mut text = format!("frame {frame}\n");
    for id in ids {
        let mesh = match app.eval_mesh(id, frame) {
            Ok(m) => m,
            Err(AppError::NoGeometry(_)) if node.is_none() => continue,
            Err(e) => return Err(e.into()),
        };
        let points: Vec<[f64; 3]> = mesh.points.iter().map(|p| p.to_array()).collect();
        let name = scene.node(id).map(|n| n.name.clone()).unwrap_or_default();
        text.push_str(&format!(
            "{} {:?}: {} points, {} polys\n",
            id.0,
            name,
            points.len(),
            mesh.polys.len()
        ));
        nodes.push(json!({"id": id.0, "name": name, "points": points.len(),
            "polys": mesh.polys.len(), "bbox": bbox(&points)}));
    }
    let mut result = json!({"frame": frame, "nodes": nodes});
    if let Some(out) = export_to {
        let roots = node.map(|n| vec![NodeId(n)]);
        let objects = app.export_obj(out, roots.as_deref())?;
        result["export"] = json!({"path": out, "objects": objects});
        text.push_str(&format!("exported {objects} object(s) to {out}\n"));
    }
    Ok(Out { json: result, text })
}

fn export(path: &str, out: &str, selected: Option<&[u64]>, frame: i64) -> Outcome {
    let (mut app, _) = open(path)?;
    app.frame(frame);
    let roots: Option<Vec<NodeId>> = selected.map(|s| s.iter().map(|n| NodeId(*n)).collect());
    let objects = app.export_obj(out, roots.as_deref())?;
    Ok(Out {
        json: json!({"path": out, "objects": objects, "frame": app.timeline.current}),
        text: format!("exported {objects} object(s) to {out}"),
    })
}

fn cmd(path: &str, id: &str, args: &str, save: bool) -> Outcome {
    let args: Json =
        serde_json::from_str(args).map_err(|e| user(format!("--args is not valid JSON: {e}")))?;
    let (mut app, _) = open(path)?;
    let result = app.run(id, args).map_err(|e| user(e.to_string()))?;
    if save {
        app.save(path).map_err(|e| user(e.to_string()))?;
    }
    Ok(Out {
        text: result.to_string(),
        json: json!({"command": id, "result": result, "saved": save}),
    })
}

fn commands() -> Outcome {
    let app = App::new_default();
    let mut list = Vec::new();
    let mut text = String::new();
    for c in app.commands.iter() {
        text.push_str(&format!("{}  {}\n", c.id(), c.label()));
        list.push(
            json!({"id": c.id(), "label": c.label(), "undoable": c.undoable(),
            "args_schema": c.args_schema()}),
        );
    }
    Ok(Out {
        json: json!({"commands": list}),
        text,
    })
}
