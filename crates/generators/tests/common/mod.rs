//! Shared by the parity tests of this crate: the R20 defaults overlay
//! (docs/specs/INFRA.md §3.1). Our descriptors default to R25; a case observed in R20 is
//! built from `tests/defaults/r20.toml` plus its own params, the case winning.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use forge_core::{NodeId, ParamKind, Registry, Value};
use forge_parity_harness::{CaseManifest, GoldenCase, app_version, defaults_overlay};

pub type Table = toml::map::Map<String, toml::Value>;

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Our node type name (without `forge.`) for a `type` key of a case file.
pub fn node_type(key: &str) -> &str {
    match key {
        "subdivision_surface" => "subdivision",
        other => other,
    }
}

/// Our param id for a case-file key: the case files of the O20 batch spell the fillet
/// segment count `fillet_subdivision`.
pub fn param_key(key: &str) -> &str {
    match key {
        "fillet_subdivision" => "fillet_segments",
        other => other,
    }
}

/// The param of `ty` a case key stands for. The scripted case form spells keys as the
/// labels read in the app, with the unit in the key (`end_angle_degrees`, `radial_bias_percent`)
/// and numbered duplicates (`2_rail`); our ids drop the unit.
pub fn resolve_key(ty: &forge_core::NodeType, key: &str) -> String {
    let key = param_key(key);
    // labels of the scripted form that differ from our ids beyond units and numbering
    const ALIASES: &[(&str, &str)] = &[
        ("2_rail", "two_rail"),
        ("intermediate_points", "intermediate"),
        ("number", "intermediate_number"),
        ("angle_degrees", "intermediate_angle"),
        ("maximum_length", "intermediate_max_length"),
        ("radius", "rounding_radius"),
        ("boolean_type", "mode"),
        ("create_single_object", "single_object"),
        ("create_phong_breaks_at_intersections", "phong_breaks"),
        ("caps_type", "cap_type"),
    ];
    // the key itself, the units dropped (`end_angle_degrees`), then the aliases
    let mut candidates = vec![
        key.to_string(),
        key.replace("_degrees", ""),
        key.replace("_percent", ""),
        key.replace("_degrees", "").replace("_percent", ""),
    ];
    candidates.extend(
        ALIASES
            .iter()
            .filter(|(from, _)| *from == key)
            .map(|(_, to)| to.to_string()),
    );
    candidates
        .into_iter()
        .find(|c| ty.find_param(c).is_some())
        .unwrap_or_else(|| key.to_string())
}

/// Case keys that carry the scene time of a scripted case, not a param.
pub fn is_time_key(key: &str) -> bool {
    matches!(key, "frame" | "fps")
}

/// The Caps / Bevel group of the scripted form of Extrude, Lathe, Loft and Sweep: bevels are
/// off in every observed case (`size` 0), so no node type stores their values.
pub fn is_bevel_key(key: &str) -> bool {
    matches!(
        key,
        "separate_bevel_controls"
            | "bevel_shape"
            | "size"
            | "extended_shape"
            | "height"
            | "shape_depth_percent"
            | "segments"
            | "bevel_shape_secondary"
            | "size_secondary"
            | "extended_shape_secondary"
            | "height_secondary"
            | "shape_depth_secondary_percent"
            | "extended_shape_3"
            | "use_absolute_value"
            | "max_shape_height"
            | "segments_secondary"
            | "bevel_outside"
            | "avoid_self_intersections"
            | "break_phong_rounding"
            | "shell"
            | "start_cap_secondary"
            | "end_cap_secondary"
            | "start_bevel"
            | "end_bevel"
            | "start_shell"
            | "end_shell"
            | "start_cap_3"
            | "end_cap_3"
            | "start_step"
            | "end_step"
            // Sweep's Rotation range (-180 to 180, the default in every observed case)
            | "rotation_from_degrees"
            | "rotation_to_degrees"
    )
}

/// Scripted-form display settings no node type stores (icon, display color).
pub fn is_ui_key(key: &str) -> bool {
    matches!(key, "icon_file_id" | "icon_color" | "color")
}

/// An enum option id for a case value: the id itself, or the label read in the app (case
/// and punctuation ignored: `N-gon` is `ngon`).
pub fn enum_id(desc: &forge_core::ParamDesc, s: &str) -> String {
    let norm = |t: &str| {
        t.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    };
    if let ParamKind::Enum { options } = &desc.kind {
        if options.iter().any(|o| o.id == s) {
            return s.to_string();
        }
        let n = norm(s);
        for o in options {
            // `N-gon` for the option `ngons`
            let plural = |t: String| t.trim_end_matches('s').to_string();
            if norm(&o.id) == n
                || norm(&o.label) == n
                || plural(norm(&o.label)) == plural(n.clone())
            {
                return o.id.clone();
            }
        }
    }
    s.to_string()
}

/// A case table in our param ids: renamed keys (`param_key`), and a 3-array `size` / `scale`
/// style key spelled for a type that stores `<key>_x`, `<key>_y`, `<key>_z` split up.
pub fn normalize(ty: &forge_core::NodeType, table: &Table) -> Table {
    let mut out = Table::new();
    for (k, v) in table {
        let k = &resolve_key(ty, k);
        let split = ty.find_param(&format!("{k}_x")).is_some();
        if ty.find_param(k).is_none()
            && !split
            && (is_time_key(k) || is_ui_key(k) || is_bevel_key(k))
        {
            continue;
        }
        // Start / End cap checkboxes of the scripted form are our `cap_start` / `cap_end`.
        if (k == "start_cap" || k == "end_cap") && ty.find_param(k).is_none() {
            let id = if k == "start_cap" {
                "cap_start"
            } else {
                "cap_end"
            };
            if ty.find_param(id).is_some() {
                let on = v.as_integer().is_some_and(|i| i != 0) || v.as_bool() == Some(true);
                out.insert(
                    id.into(),
                    toml::Value::String(if on { "cap" } else { "none" }.into()),
                );
                continue;
            }
        }
        // The symmetry cases spell its weld tolerance `weld_tolerance`.
        let k = if k == "weld_tolerance" && ty.find_param(k).is_none() {
            "tolerance"
        } else {
            k
        };
        match v {
            toml::Value::Array(a)
                if a.len() == 3
                    && ty.find_param(k).is_none()
                    && ty.find_param(&format!("{k}_x")).is_some() =>
            {
                for (axis, x) in ["x", "y", "z"].iter().zip(a) {
                    out.insert(format!("{k}_{axis}"), x.clone());
                }
            }
            _ => {
                out.insert(k.to_string(), v.clone());
            }
        }
    }
    out
}

/// Case keys that are editor display settings (isoparm lines drawn in the viewport); they
/// do not change the exported mesh, so no node type stores them.
pub fn is_display_only(key: &str) -> bool {
    key.starts_with("isoparm_subdivision")
}

/// `overlay(version of the case, node type) ∪ table`; the table's keys win.
pub fn with_defaults(case: &GoldenCase, manifest: &CaseManifest, ty: &str, table: &Table) -> Table {
    let version = app_version(&repo_root(), manifest)
        .unwrap_or_else(|e| panic!("{}/{}: {e}", case.feature, case.name));
    let mut out: Table = defaults_overlay(&repo_root(), version, node_type(ty))
        .into_iter()
        .collect();
    for (k, v) in table {
        // a vector the case spells whole (`segments = [4, 8, 4]`) replaces the overlay's split
        // components (`segments_x`, ...)
        if v.as_array().is_some_and(|a| a.len() == 3) {
            for axis in ["x", "y", "z"] {
                out.remove(&format!("{k}_{axis}"));
            }
        }
        out.insert(k.clone(), v.clone());
    }
    out
}

/// The node type (without `forge.`) a child of the scripted case form is: the form lists a
/// child's parameters by label and not its type, so the type is read off the labels it
/// carries (every type below has a label no other one has).
pub fn scripted_child_type(t: &Table) -> &'static str {
    let has = |k: &str| t.contains_key(k);
    let ty = t.get("type").and_then(|v| v.as_str());
    match ty {
        Some("Arc") => "arc_spline",
        Some("Linear") | Some("Bezier") | Some("Cubic") | Some("B-Spline") | Some("Akima") => {
            "spline"
        }
        _ if has("separate_surfaces") => "cube",
        _ if has("orientation") && has("size") => "pyramid",
        _ if has("radial_bias_percent") => "helix_spline",
        _ if has("petals") => "flower_spline",
        _ if has("outer_radius") && has("twist_percent") => "star_spline",
        _ if has("rounding") => "rectangle_spline",
        _ if has("ellipse") && has("ring") => "circle_spline",
        _ if has("start_angle_degrees") && has("radius") => "arc_spline",
        _ => panic!(
            "scripted child of unknown type: {:?}",
            t.keys().collect::<Vec<_>>()
        ),
    }
}

/// One scripted object (`position`, `rotation_degrees`, `scale`, `parameters`) as a legacy
/// child table: `type`, `pos`, a non-identity `rot` / `xscale`, and its parameters.
pub fn scripted_child(obj: &Table, ty: Option<&str>) -> Table {
    let params = obj
        .get("parameters")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    let mut t = Table::new();
    t.insert(
        "type".into(),
        toml::Value::String(
            ty.unwrap_or_else(|| scripted_child_type(&params))
                .to_string(),
        ),
    );
    let arr = |k: &str| obj.get(k).cloned();
    if let Some(p) = arr("position") {
        t.insert("pos".into(), p);
    }
    let zero = |v: &toml::Value| {
        v.as_array()
            .is_some_and(|a| a.iter().all(|x| x.as_float().unwrap_or(0.0).abs() < 1e-9))
    };
    let one = |v: &toml::Value| {
        v.as_array().is_some_and(|a| {
            a.iter()
                .all(|x| (x.as_float().unwrap_or(1.0) - 1.0).abs() < 1e-9)
        })
    };
    if let Some(r) = arr("rotation_degrees").filter(|r| !zero(r)) {
        t.insert("rot".into(), r);
    }
    if let Some(sc) = arr("scale").filter(|s| !one(s)) {
        t.insert("xscale".into(), sc);
    }
    for (k, v) in params {
        // the spline type label of an Arc / Linear child names the type, it is not a param
        if k == "type" {
            continue;
        }
        t.insert(k, v);
    }
    t
}

/// Case `[params]` in the legacy form. The scripted form (`frame` / `fps`, the generator's
/// own `[params.generator.parameters]`, children as `[params.input.<n>]` objects with
/// `position`, `rotation_degrees`, `scale` and `parameters`, an optional `[params.wrapper]`
/// Extrude around a Spline Mask) is turned into the flat form the readers know:
/// the generator's params at the top, `input`, `input2`, ... children, the mask's
/// `extrude_offset` and `caps`. Legacy tables pass through unchanged.
pub fn legacy_params(raw: Table) -> Table {
    if !raw.contains_key("generator") {
        return raw;
    }
    let mut out = Table::new();
    for k in ["frame", "fps"] {
        if let Some(v) = raw.get(k) {
            out.insert(k.into(), v.clone());
        }
    }
    let gen_obj = raw["generator"].as_table().unwrap();
    if let Some(p) = gen_obj.get("parameters").and_then(|v| v.as_table()) {
        for (k, v) in p {
            out.insert(k.clone(), v.clone());
        }
    }
    if let Some(inputs) = raw.get("input").and_then(|v| v.as_table()) {
        let mut kids: Vec<(u32, &Table)> = inputs
            .iter()
            .filter_map(|(k, v)| Some((k.parse().ok()?, v.as_table()?)))
            .collect();
        kids.sort_by_key(|(n, _)| *n);
        for (n, obj) in kids {
            let key = if n == 1 {
                "input".to_string()
            } else {
                format!("input{n}")
            };
            out.insert(key, toml::Value::Table(scripted_child(obj, None)));
        }
    }
    if let Some(r) = raw.get("reference").and_then(|v| v.as_table()) {
        // `reference_object` names the child (a Cube); the case's reference link is that child
        out.remove("reference_object");
        out.insert("input".into(), toml::Value::Table(scripted_child(r, None)));
    }
    if let Some(w) = raw.get("wrapper").and_then(|v| v.as_table()) {
        let p = w["parameters"].as_table().unwrap();
        out.insert("extrude_offset".into(), p["offset"].clone());
        out.insert(
            "caps".into(),
            toml::Value::Boolean(p["start_cap"].as_integer().unwrap_or(0) != 0),
        );
    }
    out
}

/// The case's `[params]` with the overlay applied to the case's own node (type = the
/// feature) and to every `[params.input*]` / `[params.generator]` table (type = its
/// `type` key).
pub fn resolved_params(case: &GoldenCase, manifest: &CaseManifest) -> Table {
    let raw: Table = legacy_params(manifest.params.clone().into_iter().collect());
    let mut out = with_defaults(case, manifest, &case.feature, &Table::new());
    for (k, v) in raw {
        match v {
            toml::Value::Table(t) => {
                let ty = t.get("type").and_then(|v| v.as_str()).map(str::to_string);
                let t = match ty {
                    Some(ty) => with_defaults(case, manifest, &ty, &t),
                    None => t,
                };
                out.insert(k, toml::Value::Table(t));
            }
            other => {
                out.insert(k, other);
            }
        }
    }
    out
}

/// Case files hold angles as typed in the app (degrees); angle params store radians.
fn angle(desc: &forge_core::ParamDesc, v: f64) -> f64 {
    if desc.ui.unit == forge_core::Unit::Angle {
        v.to_radians()
    } else {
        v
    }
}

/// A flat case table (scalars and enums) as validated values, keyed like our param ids.
pub fn typed_params(reg: &Registry, type_id: &str, raw: &Table) -> BTreeMap<String, Value> {
    let ty = reg.get(type_id).unwrap();
    normalize(ty, raw)
        .iter()
        .filter(|(k, _)| !is_display_only(k))
        .map(|(k, v)| {
            let desc = ty
                .find_param(k)
                .unwrap_or_else(|| panic!("{type_id}: case key {k:?} is not a param"));
            let value = match (&desc.kind, v) {
                (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
                (ParamKind::Bool, toml::Value::Integer(i)) => Value::Bool(*i != 0),
                (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
                (ParamKind::Float { .. }, toml::Value::Integer(i)) => {
                    Value::Float(angle(desc, *i as f64))
                }
                (ParamKind::Float { .. }, toml::Value::Float(f)) => Value::Float(angle(desc, *f)),
                (ParamKind::Enum { .. }, toml::Value::String(s)) => Value::Enum(enum_id(desc, s)),
                (ParamKind::String, toml::Value::String(s)) => Value::String(s.clone()),
                (ParamKind::Vec3 { .. }, toml::Value::Array(a)) if a.len() == 3 => {
                    let f = |i: usize| a[i].as_float().or(a[i].as_integer().map(|x| x as f64));
                    Value::Vec3([f(0).unwrap(), f(1).unwrap(), f(2).unwrap()])
                }
                other => panic!("{type_id}.{k}: unexpected value {other:?}"),
            };
            (k.clone(), desc.validate(value).expect("valid case value"))
        })
        .collect()
}

pub fn vec3(v: &toml::Value) -> Value {
    let a = v.as_array().expect("pos is an array");
    let f = |i: usize| {
        a[i].as_float()
            .or(a[i].as_integer().map(|x| x as f64))
            .unwrap()
    };
    Value::Vec3([f(0), f(1), f(2)])
}

/// Sets a node's params from a case table: `pos` is the position, `type` is skipped,
/// sub-tables are inputs, everything else a param of the node.
pub fn apply(
    tx: &mut forge_core::Tx,
    reg: &Registry,
    id: NodeId,
    type_id: &str,
    table: &toml::map::Map<String, toml::Value>,
) {
    let ty = reg.get(type_id).unwrap();
    for (k, v) in &normalize(ty, table) {
        // `type` names an input's node type, unless the node has a `type` param of its own and
        // the value is one of its options (the Sphere's Type)
        let names_a_param = k == "type"
            && ty.find_param("type").is_some_and(|d| match (&d.kind, v.as_str()) {
                (ParamKind::Enum { .. }, Some(s)) => {
                    let id = enum_id(d, s);
                    matches!(&d.kind, ParamKind::Enum { options } if options.iter().any(|o| o.id == id))
                }
                _ => false,
            });
        if (k == "type" && !names_a_param) || v.is_table() {
            continue;
        }
        if k == "pos" {
            tx.set_param(id, "transform.pos", vec3(v)).unwrap();
            continue;
        }
        // The scripted form's rotation (h, p, b in degrees) and scale of a child.
        if k == "rot" {
            let Value::Vec3(d) = vec3(v) else {
                unreachable!()
            };
            tx.set_param(id, "transform.rot", Value::Vec3(d.map(f64::to_radians)))
                .unwrap();
            continue;
        }
        if k == "xscale" {
            tx.set_param(id, "transform.scale", vec3(v)).unwrap();
            continue;
        }
        if is_display_only(k) {
            continue;
        }
        let desc = ty
            .find_param(k)
            .unwrap_or_else(|| panic!("{type_id}: {k:?} is not a param"));
        let value = match (&desc.kind, v) {
            (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
            (ParamKind::Bool, toml::Value::Integer(i)) => Value::Bool(*i != 0),
            (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
            (ParamKind::Float { .. }, toml::Value::Integer(i)) => {
                Value::Float(angle(desc, *i as f64))
            }
            (ParamKind::Float { .. }, toml::Value::Float(f)) => Value::Float(angle(desc, *f)),
            (ParamKind::Vec3 { .. }, v @ toml::Value::Array(_)) => vec3(v),
            (ParamKind::Enum { .. }, toml::Value::String(s)) => Value::Enum(enum_id(desc, s)),
            (ParamKind::String, toml::Value::String(s)) => Value::String(s.clone()),
            (ParamKind::Color, v @ toml::Value::Array(_)) => match vec3(v) {
                Value::Vec3(c) => Value::Color(c),
                _ => unreachable!(),
            },
            other => panic!("{type_id}.{k}: unexpected value {other:?}"),
        };
        tx.set_param(id, k, value).unwrap();
    }
}

/// Creates the spline a flat or scripted `input` table describes under `parent` and sets its
/// params: a spline primitive by its `type` (`circle_spline`, or the label `Arc`), or an editable
/// spline from `points` (+ `close_spline`, `type` = the interpolation label).
pub fn create_spline(
    tx: &mut forge_core::Tx,
    reg: &Registry,
    parent: Option<NodeId>,
    input: &Table,
) -> NodeId {
    use forge_generators::editable_spline::{self, SplineData};
    let label = input.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if let Some(points) = input.get("points").and_then(|v| v.as_array()) {
        let pts: Vec<glam::DVec3> = points
            .iter()
            .map(|p| match vec3(p) {
                Value::Vec3(v) => glam::DVec3::from_array(v),
                _ => unreachable!(),
            })
            .collect();
        let closed = input
            .get("close_spline")
            .is_some_and(|v| v.as_integer() == Some(1) || v.as_bool() == Some(true));
        let id = tx.create(editable_spline::TYPE, parent, None).unwrap();
        for (k, v) in SplineData::from_points(pts, closed).to_values() {
            tx.set_param(id, &k, v).unwrap();
        }
        let ty = reg.get(editable_spline::TYPE).unwrap();
        let own: Table = input
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "points" | "close_spline" | "type" | "pos"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        tx.set_param(
            id,
            "type",
            Value::Enum(enum_id(ty.find_param("type").unwrap(), label)),
        )
        .unwrap();
        apply(tx, reg, id, editable_spline::TYPE, &own);
        if let Some(p) = input.get("pos") {
            tx.set_param(id, "transform.pos", vec3(p)).unwrap();
        }
        return id;
    }
    let mut table = input.clone();
    let type_id = if label.ends_with("_spline") {
        label.to_string()
    } else {
        // the label is the Arc's own Type (Arc, Sector, ...): the node type is read off the labels
        let node = scripted_child_type(&table);
        if node == "arc_spline" {
            table.insert("arc_type".into(), toml::Value::String(label.to_string()));
        }
        table.remove("type");
        node.to_string()
    };
    let ty = format!("forge.{type_id}");
    let id = tx.create(&ty, parent, None).unwrap();
    apply(tx, reg, id, &ty, &table);
    id
}
