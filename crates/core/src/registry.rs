//! Node types and the plugin registry (docs/specs/CORE.md §2).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::{ParamDesc, ParamKind, transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    Object,
    Generator,
    Deformer,
    Effector,
    Field,
    Tag,
    Material,
    Shader,
    RenderEngine,
}

impl Category {
    /// True for categories that live in the object tree.
    pub fn is_hierarchy(self) -> bool {
        matches!(
            self,
            Category::Object
                | Category::Generator
                | Category::Deformer
                | Category::Effector
                | Category::Field
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeType {
    pub id: String,
    pub label: String,
    pub category: Category,
    pub params: Vec<ParamDesc>,
    /// Whether the UI's Create menu offers this type (docs/specs/CORE.md §2).
    pub in_create_menu: bool,
}

impl NodeType {
    pub fn new(id: &str, label: &str, category: Category) -> NodeType {
        NodeType {
            id: id.to_string(),
            label: label.to_string(),
            category,
            params: Vec::new(),
            in_create_menu: true,
        }
    }

    /// Keeps the type out of the Create menu.
    pub fn not_in_create_menu(mut self) -> NodeType {
        self.in_create_menu = false;
        self
    }

    pub fn param(mut self, desc: ParamDesc) -> NodeType {
        self.params.push(desc);
        self
    }

    pub fn find_param(&self, id: &str) -> Option<&ParamDesc> {
        self.params.iter().find(|p| p.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    #[error("invalid node type id {0:?} (expected namespace.name, [a-z0-9_.-])")]
    InvalidTypeId(String),
    #[error("node type {0:?} is already registered")]
    DuplicateType(String),
    #[error("{type_id}: invalid parameter id {param:?} (expected [a-z0-9_.])")]
    InvalidParamId { type_id: String, param: String },
    #[error("{type_id}: duplicate parameter id {param:?}")]
    DuplicateParam { type_id: String, param: String },
    #[error("{type_id}: default of {param:?} is not a valid value")]
    InvalidDefault { type_id: String, param: String },
    #[error("{type_id}: enum {param:?} needs at least one option and unique option ids")]
    InvalidEnum { type_id: String, param: String },
    #[error("{type_id}: parameter id {param:?} uses the reserved `transform.` prefix")]
    ReservedParam { type_id: String, param: String },
}

/// A bundle of node types registered together.
pub trait Plugin {
    fn id(&self) -> &str;
    fn register(&self, registry: &mut Registry) -> Result<(), RegistryError>;
}

#[derive(Debug, Clone, Default)]
pub struct Registry {
    types: BTreeMap<String, Arc<NodeType>>,
}

fn valid_type_id(id: &str) -> bool {
    id.contains('.')
        && id.split('.').all(|seg| {
            !seg.is_empty()
                && seg
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        })
}

fn valid_param_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    /// Registry with the core built-in types (`forge.null`).
    pub fn with_builtins() -> Registry {
        let mut r = Registry::new();
        r.register(NodeType::new("forge.null", "Null", crate::Category::Object))
            .expect("built-ins are valid");
        r.register(crate::smoothing::node_type())
            .expect("built-ins are valid");
        r
    }

    /// Registers a type; hierarchy types get the transform params prepended
    /// (docs/specs/TRANSFORM.md §2).
    pub fn register(&mut self, mut ty: NodeType) -> Result<(), RegistryError> {
        if !valid_type_id(&ty.id) {
            return Err(RegistryError::InvalidTypeId(ty.id));
        }
        if self.types.contains_key(&ty.id) {
            return Err(RegistryError::DuplicateType(ty.id));
        }
        if let Some(p) = ty
            .params
            .iter()
            .find(|p| p.id.starts_with(transform::PREFIX))
        {
            return Err(RegistryError::ReservedParam {
                type_id: ty.id.clone(),
                param: p.id.clone(),
            });
        }
        if ty.category.is_hierarchy() {
            ty.params.splice(0..0, transform::params());
        }
        let mut seen = BTreeSet::new();
        for p in &ty.params {
            let err_ctx = || (ty.id.clone(), p.id.clone());
            if !valid_param_id(&p.id) {
                let (type_id, param) = err_ctx();
                return Err(RegistryError::InvalidParamId { type_id, param });
            }
            if !seen.insert(p.id.as_str()) {
                let (type_id, param) = err_ctx();
                return Err(RegistryError::DuplicateParam { type_id, param });
            }
            if let ParamKind::Enum { options } = &p.kind {
                let ids: BTreeSet<&str> = options.iter().map(|o| o.id.as_str()).collect();
                if options.is_empty() || ids.len() != options.len() {
                    let (type_id, param) = err_ctx();
                    return Err(RegistryError::InvalidEnum { type_id, param });
                }
            }
            if p.validate(p.default.clone()).as_ref() != Ok(&p.default) {
                let (type_id, param) = err_ctx();
                return Err(RegistryError::InvalidDefault { type_id, param });
            }
        }
        self.types.insert(ty.id.clone(), Arc::new(ty));
        Ok(())
    }

    /// Registers all of a plugin's types, or none of them.
    pub fn install(&mut self, plugin: &dyn Plugin) -> Result<(), RegistryError> {
        let mut staged = self.clone();
        plugin.register(&mut staged)?;
        *self = staged;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Arc<NodeType>> {
        self.types.get(id)
    }

    /// All types, sorted by id.
    pub fn types(&self) -> impl Iterator<Item = &Arc<NodeType>> {
        self.types.values()
    }

    pub fn types_in(&self, category: Category) -> impl Iterator<Item = &Arc<NodeType>> {
        self.types().filter(move |t| t.category == category)
    }
}
