//! Native scene file: deterministic RON (docs/specs/CORE.md §6).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{Category, Node, NodeId, Registry, Scene, Value};

pub const FORMAT: &str = "forge3d-scene";
/// Version 2 added `Value::Mesh`, version 3 `Value::IndexList`; older files still load.
pub const VERSION: u32 = 3;

#[derive(Serialize, Deserialize)]
struct SceneFile {
    format: String,
    version: u32,
    roots: Vec<NodeRecord>,
}

#[derive(Serialize, Deserialize)]
struct NodeRecord {
    id: NodeId,
    type_id: String,
    name: String,
    params: BTreeMap<String, Value>,
    tags: Vec<NodeRecord>,
    children: Vec<NodeRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    #[error("not a valid scene file: {0}")]
    Parse(String),
    #[error("unexpected format {0:?}")]
    WrongFormat(String),
    #[error("unsupported scene version {0}")]
    UnsupportedVersion(u32),
    #[error("node id {0} appears more than once")]
    DuplicateId(NodeId),
    #[error("invalid structure: {0}")]
    Structure(String),
}

/// Recoverable issues found while loading.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadReport {
    pub warnings: Vec<String>,
}

pub fn save(scene: &Scene) -> String {
    fn record(scene: &Scene, id: NodeId) -> NodeRecord {
        let n = scene.node(id).expect("listed node exists");
        NodeRecord {
            id,
            type_id: n.type_id.clone(),
            name: n.name.clone(),
            params: n.params.clone(),
            tags: n.tags.iter().map(|t| record(scene, *t)).collect(),
            children: n.children.iter().map(|c| record(scene, *c)).collect(),
        }
    }
    let file = SceneFile {
        format: FORMAT.to_string(),
        version: VERSION,
        roots: scene.roots().iter().map(|r| record(scene, *r)).collect(),
    };
    ron::ser::to_string_pretty(&file, ron::ser::PrettyConfig::default())
        .expect("scene data always serializes")
}

pub fn load(registry: Arc<Registry>, text: &str) -> Result<(Scene, LoadReport), LoadError> {
    let file: SceneFile = ron::from_str(text).map_err(|e| LoadError::Parse(e.to_string()))?;
    if file.format != FORMAT {
        return Err(LoadError::WrongFormat(file.format));
    }
    if file.version > VERSION {
        return Err(LoadError::UnsupportedVersion(file.version));
    }
    let mut loader = Loader {
        scene: Scene::new(registry),
        report: LoadReport::default(),
        seen: BTreeSet::new(),
    };
    for r in file.roots {
        loader.node(r, Place::Child(None))?;
    }
    let next = loader.seen.last().map_or(1, |id| id.0 + 1);
    loader.scene.set_next_id(next);
    Ok((loader.scene, loader.report))
}

#[derive(Clone, Copy)]
enum Place {
    Child(Option<NodeId>),
    Tag(NodeId),
}

struct Loader {
    scene: Scene,
    report: LoadReport,
    seen: BTreeSet<NodeId>,
}

impl Loader {
    fn warn(&mut self, id: NodeId, msg: String) {
        self.report.warnings.push(format!("node {id}: {msg}"));
    }

    fn node(&mut self, r: NodeRecord, place: Place) -> Result<(), LoadError> {
        let id = r.id;
        if !self.seen.insert(id) {
            return Err(LoadError::DuplicateId(id));
        }
        let ty = self.scene.registry().get(&r.type_id).cloned();
        let params = match &ty {
            None => {
                self.warn(id, format!("unknown type {:?} (kept as-is)", r.type_id));
                r.params
            }
            Some(ty) => {
                let ok = match place {
                    Place::Child(_) => ty.category.is_hierarchy(),
                    Place::Tag(_) => ty.category == Category::Tag,
                };
                if !ok {
                    return Err(LoadError::Structure(format!(
                        "node {id} of type {:?} ({:?}) cannot be placed here",
                        r.type_id, ty.category
                    )));
                }
                let mut stored = r.params;
                let mut params = BTreeMap::new();
                for desc in &ty.params {
                    let value = match stored.remove(&desc.id) {
                        None => desc.default.clone(),
                        Some(v) => match desc.validate(v.clone()) {
                            Ok(valid) => {
                                if valid != v {
                                    self.warn(id, format!("param {:?} clamped", desc.id));
                                }
                                valid
                            }
                            Err(e) => {
                                self.warn(
                                    id,
                                    format!("param {:?}: {e}; reset to default", desc.id),
                                );
                                desc.default.clone()
                            }
                        },
                    };
                    params.insert(desc.id.clone(), value);
                }
                for unknown in stored.keys() {
                    self.warn(id, format!("unknown param {unknown:?} dropped"));
                }
                params
            }
        };
        let (parent, host) = match place {
            Place::Child(p) => (p, None),
            Place::Tag(h) => (None, Some(h)),
        };
        self.scene.insert_loaded(Node {
            id,
            type_id: r.type_id,
            name: r.name,
            params,
            parent,
            children: r.children.iter().map(|c| c.id).collect(),
            tags: r.tags.iter().map(|t| t.id).collect(),
            host,
            rev: 0,
        });
        for t in r.tags {
            self.node(t, Place::Tag(id))?;
        }
        for c in r.children {
            self.node(c, Place::Child(Some(id)))?;
        }
        Ok(())
    }
}
