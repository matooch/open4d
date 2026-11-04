//! The preset library and the recent file list (docs/specs/UI.md §26).
//!
//! No window and no global paths: a [`Browser`] only touches the settings file and the
//! library folder it was given, so tests and headless runs stay away from the user's data.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use forge_core::{
    EditError, NodeId, Registry, Scene, Track, Tx, Value, file, layers::LAYER as LAYER_PARAM,
};
use serde_json::{Value as Json, json};

/// Recent files kept (parity-pending).
pub const MAX_RECENT_FILES: usize = 10;

/// Where a preset comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Builtin,
    User,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Builtin => "builtin",
            Source::User => "user",
        }
    }
}

/// One entry of the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetInfo {
    /// `builtin:<slug>` or `user:<slug>`.
    pub id: String,
    pub name: String,
    pub source: Source,
}

/// Our own presets, compiled in: `(slug, native scene file)`.
const BUILTIN: [(&str, &str); 3] = [
    (
        "pedestal",
        include_str!("../presets/objects/pedestal.forge"),
    ),
    ("snowman", include_str!("../presets/objects/snowman.forge")),
    ("table", include_str!("../presets/objects/table.forge")),
];

/// Library and recent files of one application.
#[derive(Debug, Clone, Default)]
pub struct Browser {
    /// Newest first, at most [`MAX_RECENT_FILES`].
    pub recent: Vec<String>,
    settings: Option<PathBuf>,
    library: Option<PathBuf>,
}

impl Browser {
    /// Uses `settings` for the recent list (loading it now) and `library` for user presets.
    /// `None` keeps that part in memory only / absent.
    pub fn use_files(&mut self, settings: Option<PathBuf>, library: Option<PathBuf>) {
        self.recent = settings
            .as_deref()
            .map(|p| {
                read_settings(p)["recent"]["files"]
                    .as_array()
                    .map(|a| {
                        let mut seen = BTreeSet::new();
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .filter(|s| seen.insert(s.clone()))
                            .take(MAX_RECENT_FILES)
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        self.settings = settings;
        self.library = library;
    }

    /// The folder user presets live in (`<library>/objects`), if there is a library.
    pub fn user_dir(&self) -> Option<PathBuf> {
        self.library.as_ref().map(|l| l.join("objects"))
    }

    /// Moves `path` (made absolute) to the front of the recent list and stores the list.
    pub fn record_recent(&mut self, path: &str) {
        let path = absolute(path);
        self.recent.retain(|p| *p != path);
        self.recent.insert(0, path);
        self.recent.truncate(MAX_RECENT_FILES);
        self.store_recent();
    }

    /// Drops entry `index`; false when there is none.
    pub fn remove_recent(&mut self, index: usize) -> bool {
        if index >= self.recent.len() {
            return false;
        }
        self.recent.remove(index);
        self.store_recent();
        true
    }

    /// Empties the list; returns how many entries it held.
    pub fn clear_recent(&mut self) -> usize {
        let n = self.recent.len();
        self.recent.clear();
        self.store_recent();
        n
    }

    fn store_recent(&self) {
        let Some(p) = &self.settings else { return };
        let mut settings = read_settings(p);
        if !settings["recent"].is_object() {
            settings["recent"] = json!({});
        }
        settings["recent"]["files"] = json!(self.recent);
        let written = p
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(p, settings.to_string()));
        if let Err(e) = written {
            tracing::warn!("could not store the recent files: {e}");
        }
    }

    /// Built-in presets, then user presets, each group sorted by name; `query` keeps the
    /// presets whose name contains it (case-insensitive).
    pub fn list(&self, query: Option<&str>) -> Vec<PresetInfo> {
        let mut builtin: Vec<PresetInfo> = BUILTIN
            .iter()
            .map(|(slug, _)| info(Source::Builtin, slug))
            .collect();
        let mut user: Vec<PresetInfo> = self
            .user_dir()
            .and_then(|d| std::fs::read_dir(d).ok())
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|e| {
                let path = e.path();
                let ok = path.is_file() && path.extension().is_some_and(|x| x == "forge");
                let stem = path.file_stem()?.to_str()?.to_string();
                (ok && slugify(&stem) == stem).then(|| info(Source::User, &stem))
            })
            .collect();
        for group in [&mut builtin, &mut user] {
            group.sort_by_key(|p| (p.name.to_lowercase(), p.id.clone()));
        }
        let q = query.map(str::to_lowercase).filter(|q| !q.is_empty());
        builtin
            .into_iter()
            .chain(user)
            .filter(|p| q.as_ref().is_none_or(|q| p.name.to_lowercase().contains(q)))
            .collect()
    }

    /// The bytes of the preset `id`.
    pub fn read(&self, id: &str) -> Result<Vec<u8>, String> {
        match split_id(id)? {
            (Source::Builtin, slug) => BUILTIN
                .iter()
                .find(|(s, _)| *s == slug)
                .map(|(_, text)| text.as_bytes().to_vec())
                .ok_or_else(|| format!("no preset {id}")),
            (Source::User, slug) => {
                let dir = self.user_dir().ok_or("no user preset folder")?;
                std::fs::read(dir.join(format!("{slug}.forge")))
                    .map_err(|_| format!("no preset {id}"))
            }
        }
    }

    /// Writes `bytes` as the user preset `name`; returns its id and path.
    pub fn write_user(
        &self,
        name: &str,
        bytes: &[u8],
        overwrite: bool,
    ) -> Result<(String, PathBuf), String> {
        let dir = self.user_dir().ok_or("no user preset folder")?;
        let slug = slugify(name);
        if slug.is_empty() {
            return Err("the preset name needs letters or digits".into());
        }
        let path = dir.join(format!("{slug}.forge"));
        if path.exists() && !overwrite {
            return Err(format!("preset user:{slug} exists (use overwrite)"));
        }
        std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::write(&path, bytes))
            .map_err(|e| format!("could not write the preset: {e}"))?;
        Ok((format!("user:{slug}"), path))
    }

    /// Deletes the user preset `id`.
    pub fn delete_user(&self, id: &str) -> Result<(), String> {
        match split_id(id)? {
            (Source::Builtin, _) => Err("built-in presets cannot be deleted".into()),
            (Source::User, slug) => {
                let dir = self.user_dir().ok_or("no user preset folder")?;
                std::fs::remove_file(dir.join(format!("{slug}.forge")))
                    .map_err(|_| format!("no preset {id}"))
            }
        }
    }
}

fn info(source: Source, slug: &str) -> PresetInfo {
    PresetInfo {
        id: format!("{}:{slug}", source.name()),
        name: title_of(slug),
        source,
    }
}

/// `builtin:<slug>` / `user:<slug>`; the slug is checked so an id never leaves the folder.
fn split_id(id: &str) -> Result<(Source, &str), String> {
    let (source, slug) = match id.split_once(':') {
        Some(("builtin", s)) => (Source::Builtin, s),
        Some(("user", s)) => (Source::User, s),
        _ => {
            return Err(format!(
                "bad preset id {id:?} (builtin:<name> or user:<name>)"
            ));
        }
    };
    if slug.is_empty() || slugify(slug) != slug {
        return Err(format!("bad preset id {id:?}"));
    }
    Ok((source, slug))
}

/// Lower-case ASCII letters and digits; every other run becomes one `-`, trimmed.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// The display name of a slug: words separated by `-`, each starting upper case.
pub fn title_of(slug: &str) -> String {
    slug.split('-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `path` made absolute when possible.
pub fn absolute(path: &str) -> String {
    std::path::absolute(path).map_or_else(|_| path.to_string(), |p| p.to_string_lossy().into())
}

/// The file name of `path` for display.
pub fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into())
}

fn read_settings(path: &Path) -> Json {
    match std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Json>(&t).ok())
    {
        Some(v @ Json::Object(_)) => v,
        _ => json!({}),
    }
}

/// Parses a preset file with `registry`; its warnings come back as one string.
pub fn parse_preset(registry: Arc<Registry>, bytes: &[u8]) -> Result<(Scene, String), String> {
    let (scene, report) = file::load(registry, &String::from_utf8_lossy(bytes))
        .map_err(|e| format!("could not read the preset: {e}"))?;
    Ok((scene, report.warnings.join("; ")))
}

fn points_inside(v: &Value, inside: &BTreeSet<NodeId>) -> bool {
    match v {
        Value::Link(Some(t)) => inside.contains(t),
        Value::LinkList(l) => l.iter().any(|t| inside.contains(t)),
        _ => false,
    }
}

fn remap(v: &Value, map: &BTreeMap<NodeId, NodeId>) -> Value {
    match v {
        Value::Link(Some(t)) => Value::Link(map.get(t).copied()),
        Value::LinkList(l) => {
            Value::LinkList(l.iter().filter_map(|t| map.get(t).copied()).collect())
        }
        other => other.clone(),
    }
}

/// Copies the subtrees of `roots` of `src` (hierarchy nodes) under `parent` of the
/// transaction's scene, after the existing children. Links inside the copied nodes are
/// remapped, other links and the layer link are dropped. Returns the new roots.
pub fn graft(
    src: &Scene,
    roots: &[NodeId],
    tx: &mut Tx,
    parent: Option<NodeId>,
) -> Result<Vec<NodeId>, EditError> {
    let mut inside = BTreeSet::new();
    for r in roots {
        inside.insert(*r);
        inside.extend(src.descendants(*r));
    }
    // tags are nodes too: a link may point at one
    let tags: Vec<NodeId> = inside
        .iter()
        .filter_map(|i| src.node(*i))
        .flat_map(|n| n.tags.clone())
        .collect();
    inside.extend(tags);
    let mut g = Grafter {
        src,
        inside: &inside,
        map: BTreeMap::new(),
        pending: Vec::new(),
    };
    let mut created = Vec::new();
    for r in roots {
        created.push(g.copy(tx, *r, parent)?);
    }
    for (node, key, value) in std::mem::take(&mut g.pending) {
        tx.set_param(node, &key, remap(&value, &g.map))?;
    }
    Ok(created)
}

struct Grafter<'a> {
    src: &'a Scene,
    inside: &'a BTreeSet<NodeId>,
    map: BTreeMap<NodeId, NodeId>,
    pending: Vec<(NodeId, String, Value)>,
}

impl Grafter<'_> {
    fn fill(&mut self, tx: &mut Tx, from: NodeId, to: NodeId) -> Result<(), EditError> {
        let node = self.src.node(from).ok_or(EditError::UnknownNode(from))?;
        tx.rename(to, &node.name)?;
        for (key, value) in &node.params {
            if key == LAYER_PARAM {
                continue;
            }
            let has_link = matches!(value, Value::Link(Some(_)) | Value::LinkList(_));
            if points_inside(value, self.inside) {
                self.pending.push((to, key.clone(), value.clone()));
            } else if has_link {
                continue;
            } else {
                tx.set_param(to, key, value.clone())?;
            }
        }
        for (key, track) in &node.tracks {
            tx.set_track(to, key, Some(Track::clone(track)))?;
        }
        self.map.insert(from, to);
        Ok(())
    }

    fn copy(
        &mut self,
        tx: &mut Tx,
        from: NodeId,
        parent: Option<NodeId>,
    ) -> Result<NodeId, EditError> {
        let node = self.src.node(from).ok_or(EditError::UnknownNode(from))?;
        let new = tx.create(&node.type_id, parent, None)?;
        self.fill(tx, from, new)?;
        for tag in &node.tags {
            let t = self.src.node(*tag).ok_or(EditError::UnknownNode(*tag))?;
            let id = tx.attach_tag(&t.type_id, new, None)?;
            self.fill(tx, *tag, id)?;
        }
        for child in &node.children {
            self.copy(tx, *child, Some(new))?;
        }
        Ok(new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_and_titles() {
        assert_eq!(slugify("My  Table #2!"), "my-table-2");
        assert_eq!(slugify("  --  "), "");
        assert_eq!(slugify("../etc"), "etc");
        assert_eq!(title_of("my-table-2"), "My Table 2");
        assert_eq!(slugify(&title_of("snowman")), "snowman");
    }

    #[test]
    fn ids_are_checked() {
        assert!(split_id("builtin:table").is_ok());
        assert!(split_id("user:a-b").is_ok());
        for bad in [
            "table",
            "user:",
            "user:../x",
            "other:x",
            "user:A",
            "user:a/b",
        ] {
            assert!(split_id(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn builtin_list_is_sorted_and_searchable() {
        let b = Browser::default();
        let all = b.list(None);
        let names: Vec<_> = all.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Pedestal", "Snowman", "Table"]);
        assert_eq!(b.list(Some("SNOW")).len(), 1);
        assert!(b.list(Some("zzz")).is_empty());
        assert!(all.iter().all(|p| b.read(&p.id).is_ok()));
    }

    #[test]
    fn recent_list_order_and_limit_in_memory() {
        let mut b = Browser::default();
        for i in 0..12 {
            b.record_recent(&format!("/scenes/{i}.forge"));
        }
        b.record_recent("/scenes/5.forge");
        assert_eq!(b.recent.len(), MAX_RECENT_FILES);
        // stored absolute: on Windows "/scenes/5.forge" gains the current drive
        let five = absolute("/scenes/5.forge");
        assert_eq!(b.recent[0], five);
        assert_eq!(b.recent.iter().filter(|p| **p == five).count(), 1);
        assert!(b.remove_recent(0));
        assert!(!b.remove_recent(99));
        assert_eq!(b.clear_recent(), MAX_RECENT_FILES - 1);
    }
}
