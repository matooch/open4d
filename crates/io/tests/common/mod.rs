//! Shared by the parity tests of this crate: the R20 defaults overlay
//! (docs/specs/INFRA.md §3.1). Our descriptors default to R25; a case observed in R20 is
//! built from `tests/defaults/r20.toml` plus its own params, the case winning.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

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
