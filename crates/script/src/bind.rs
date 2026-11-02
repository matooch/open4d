//! The functions and types scripts see (docs/specs/SCRIPTING.md §3).

use std::rc::Rc;

use forge_app::query::display_value;
use forge_core::{NodeId, Scene, Value};
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};
use serde_json::{Value as Json, json};

use crate::convert::{from_json, to_json, vec3_from_json};
use crate::host::{Host, error};
use crate::maths;

type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

/// Handle to the live document.
#[derive(Clone)]
struct SceneView;

/// Handle to one node.
#[derive(Clone)]
struct NodeRef {
    id: u64,
}

fn args_json(map: &Map) -> RhaiResult<Json> {
    to_json(&Dynamic::from(map.clone())).map_err(error)
}

fn arg_json(value: &Dynamic) -> RhaiResult<Json> {
    to_json(value).map_err(error)
}

fn ids(list: impl IntoIterator<Item = NodeId>) -> Array {
    list.into_iter()
        .map(|n| Dynamic::from(n.0 as i64))
        .collect()
}

fn node_id(raw: i64) -> RhaiResult<NodeId> {
    u64::try_from(raw)
        .map(NodeId)
        .map_err(|_| error(format!("invalid node id {raw}")))
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

fn named(scene: &Scene, name: &str) -> Vec<NodeId> {
    all_nodes(scene)
        .into_iter()
        .filter(|id| scene.node(*id).is_some_and(|n| n.name == name))
        .collect()
}

/// A parameter in display units; `vec3` parameters become `vec3`.
fn param_value(scene: &Scene, id: NodeId, name: &str) -> RhaiResult<Dynamic> {
    let node = scene
        .node(id)
        .ok_or_else(|| error(format!("unknown node {}", id.0)))?;
    let value = node
        .params
        .get(name)
        .ok_or_else(|| error(format!("node {} has no parameter {name}", id.0)))?;
    let unit = scene
        .node_type(id)
        .and_then(|t| t.params.iter().find(|d| d.id == name))
        .map(|d| d.ui.unit)
        .unwrap_or_default();
    let json = display_value(value, unit);
    if matches!(value, Value::Vec3(_))
        && let Some(v) = vec3_from_json(&json)
    {
        return Ok(Dynamic::from(v));
    }
    Ok(from_json(&json))
}

pub(crate) fn register(engine: &mut Engine, host: &Rc<Host>) {
    maths::register(engine);
    commands(engine, host);
    scene(engine, host);
    time(engine, host);
}

fn commands(engine: &mut Engine, host: &Rc<Host>) {
    let h = host.clone();
    engine.register_fn("run", move |id: &str| -> RhaiResult<Dynamic> {
        Ok(from_json(&h.command(id, json!({}))?))
    });
    let h = host.clone();
    engine.register_fn("run", move |id: &str, args: Map| -> RhaiResult<Dynamic> {
        Ok(from_json(&h.command(id, args_json(&args)?)?))
    });
    let h = host.clone();
    engine.register_fn("commands", move || -> Array {
        h.command_ids().into_iter().map(Dynamic::from).collect()
    });

    let h = host.clone();
    engine.register_fn("create", move |ty: &str| -> RhaiResult<i64> {
        create(&h, ty, None)
    });
    let h = host.clone();
    engine.register_fn("create", move |ty: &str, name: &str| -> RhaiResult<i64> {
        create(&h, ty, Some(name))
    });
    let h = host.clone();
    engine.register_fn(
        "set_param",
        move |node: i64, name: &str, value: Dynamic| -> RhaiResult<()> {
            let args = json!({"node": node_id(node)?.0, "param": name, "value": arg_json(&value)?});
            h.command("param.set", args)?;
            Ok(())
        },
    );
    let h = host.clone();
    engine.register_fn(
        "set_transform",
        move |node: i64, parts: Map| -> RhaiResult<()> {
            let mut args = args_json(&parts)?;
            args["node"] = json!(node_id(node)?.0);
            h.command("transform.set", args)?;
            Ok(())
        },
    );
}

fn create(host: &Host, ty: &str, name: Option<&str>) -> RhaiResult<i64> {
    let out = host.command("object.create", json!({"type": ty}))?;
    let id = out["node"]
        .as_u64()
        .ok_or_else(|| error("object.create returned no node"))?;
    if let Some(name) = name {
        host.command("object.rename", json!({"node": id, "name": name}))?;
    }
    Ok(id as i64)
}

fn scene(engine: &mut Engine, host: &Rc<Host>) {
    engine.register_type_with_name::<SceneView>("scene");
    engine.register_type_with_name::<NodeRef>("node");
    engine.register_fn("scene", || SceneView);

    let h = host.clone();
    engine.register_fn("nodes", move |_: &mut SceneView| {
        h.with(|c| ids(all_nodes(c.doc().scene())))
    });
    let h = host.clone();
    engine.register_fn("roots", move |_: &mut SceneView| {
        h.with(|c| ids(c.doc().scene().roots().iter().copied()))
    });
    let h = host.clone();
    engine.register_fn("find", move |_: &mut SceneView, name: &str| -> Dynamic {
        h.with(|c| match named(c.doc().scene(), name).first() {
            Some(id) => Dynamic::from(id.0 as i64),
            None => Dynamic::UNIT,
        })
    });
    let h = host.clone();
    engine.register_fn("find_all", move |_: &mut SceneView, name: &str| {
        h.with(|c| ids(named(c.doc().scene(), name)))
    });
    let h = host.clone();
    engine.register_fn("selection", move |_: &mut SceneView| {
        h.with(|c| ids(c.selection().nodes.iter().copied()))
    });
    let h = host.clone();
    engine.register_fn("len", move |_: &mut SceneView| {
        h.with(|c| c.doc().scene().len() as i64)
    });
    let h = host.clone();
    engine.register_fn(
        "node",
        move |_: &mut SceneView, id: i64| -> RhaiResult<NodeRef> {
            let id = node_id(id)?;
            h.with(|c| {
                if c.doc().scene().node(id).is_some() {
                    Ok(NodeRef { id: id.0 })
                } else {
                    Err(error(format!("unknown node {}", id.0)))
                }
            })
        },
    );
    let h = host.clone();
    engine.register_get("fps", move |_: &mut SceneView| {
        h.with(|c| i64::from(c.doc().scene().settings().fps))
    });
    let h = host.clone();
    engine.register_get("start", move |_: &mut SceneView| {
        h.with(|c| c.doc().scene().settings().start)
    });
    let h = host.clone();
    engine.register_get("end", move |_: &mut SceneView| {
        h.with(|c| c.doc().scene().settings().end)
    });

    engine.register_get("id", |n: &mut NodeRef| n.id as i64);
    let h = host.clone();
    engine.register_get("name", move |n: &mut NodeRef| -> RhaiResult<String> {
        node_field(&h, n, |node| node.name.clone())
    });
    let h = host.clone();
    engine.register_get("type", move |n: &mut NodeRef| -> RhaiResult<String> {
        node_field(&h, n, |node| node.type_id.clone())
    });
    let h = host.clone();
    engine.register_fn("parent", move |n: &mut NodeRef| -> RhaiResult<Dynamic> {
        node_field(&h, n, |node| match node.parent {
            Some(p) => Dynamic::from(p.0 as i64),
            None => Dynamic::UNIT,
        })
    });
    let h = host.clone();
    engine.register_fn("children", move |n: &mut NodeRef| -> RhaiResult<Array> {
        node_field(&h, n, |node| ids(node.children.iter().copied()))
    });
    let h = host.clone();
    engine.register_fn(
        "param",
        move |n: &mut NodeRef, name: &str| -> RhaiResult<Dynamic> {
            h.with(|c| param_value(c.doc().scene(), NodeId(n.id), name))
        },
    );
    let h = host.clone();
    engine.register_fn("params", move |n: &mut NodeRef| -> RhaiResult<Map> {
        h.with(|c| {
            let scene = c.doc().scene();
            let node = scene
                .node(NodeId(n.id))
                .ok_or_else(|| error(format!("unknown node {}", n.id)))?;
            let mut map = Map::new();
            for name in node.params.keys() {
                map.insert(
                    name.as_str().into(),
                    param_value(scene, NodeId(n.id), name)?,
                );
            }
            Ok(map)
        })
    });
}

fn node_field<R>(
    host: &Host,
    n: &NodeRef,
    f: impl FnOnce(&forge_core::Node) -> R,
) -> RhaiResult<R> {
    host.with(|c| {
        c.doc()
            .scene()
            .node(NodeId(n.id))
            .map(f)
            .ok_or_else(|| error(format!("unknown node {}", n.id)))
    })
}

fn time(engine: &mut Engine, host: &Rc<Host>) {
    let h = host.clone();
    engine.register_fn("frame", move || h.with(|c| c.timeline().current));
    let h = host.clone();
    engine.register_fn("frame", move |n: i64| -> RhaiResult<i64> {
        let out = h.command("timeline.set_frame", json!({"frame": n}))?;
        Ok(out["frame"].as_i64().unwrap_or(n))
    });
}
