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

/// `overlay(version of the case, node type) ∪ table`; the table's keys win.
pub fn with_defaults(case: &GoldenCase, manifest: &CaseManifest, ty: &str, table: &Table) -> Table {
    let version = app_version(&repo_root(), manifest)
        .unwrap_or_else(|e| panic!("{}/{}: {e}", case.feature, case.name));
    let mut out: Table = defaults_overlay(&repo_root(), version, node_type(ty))
        .into_iter()
        .collect();
    for (k, v) in table {
        out.insert(k.clone(), v.clone());
    }
    out
}

/// The case's `[params]` with the overlay applied to the case's own node (type = the
/// feature) and to every `[params.input*]` / `[params.generator]` table (type = its
/// `type` key).
pub fn resolved_params(case: &GoldenCase, manifest: &CaseManifest) -> Table {
    let raw: Table = manifest.params.clone().into_iter().collect();
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
    raw.iter()
        .map(|(k, v)| {
            let desc = ty
                .find_param(k)
                .unwrap_or_else(|| panic!("{type_id}: case key {k:?} is not a param"));
            let value = match (&desc.kind, v) {
                (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
                (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
                (ParamKind::Float { .. }, toml::Value::Integer(i)) => {
                    Value::Float(angle(desc, *i as f64))
                }
                (ParamKind::Float { .. }, toml::Value::Float(f)) => Value::Float(angle(desc, *f)),
                (ParamKind::Enum { .. }, toml::Value::String(s)) => Value::Enum(s.clone()),
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
    for (k, v) in table {
        if k == "type" || v.is_table() {
            continue;
        }
        if k == "pos" {
            tx.set_param(id, "transform.pos", vec3(v)).unwrap();
            continue;
        }
        let desc = ty
            .find_param(k)
            .unwrap_or_else(|| panic!("{type_id}: {k:?} is not a param"));
        let value = match (&desc.kind, v) {
            (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
            (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
            (ParamKind::Float { .. }, toml::Value::Integer(i)) => {
                Value::Float(angle(desc, *i as f64))
            }
            (ParamKind::Float { .. }, toml::Value::Float(f)) => Value::Float(angle(desc, *f)),
            (ParamKind::Vec3 { .. }, v @ toml::Value::Array(_)) => vec3(v),
            (ParamKind::Enum { .. }, toml::Value::String(s)) => Value::Enum(s.clone()),
            other => panic!("{type_id}.{k}: unexpected value {other:?}"),
        };
        tx.set_param(id, k, value).unwrap();
    }
}
