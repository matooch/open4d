//! Shaders hosted in the document (SHADERS.md S11, MATERIAL.md M6).

use std::sync::Arc;

use forge_core::{Category, NodeId, Scene, Value};
use forge_image::Image;
use forge_material::shader_name;

use crate::build::{Inputs, ShaderError, Values, build};
use crate::shader::{Shader, ShaderSet};

/// Deepest shader tree [`build_node`] follows.
pub const MAX_DEPTH: usize = 32;

/// Decodes the image of a Bitmap shader from its `file` param.
pub type ImageLoader<'a> = &'a dyn Fn(&str) -> Option<Arc<Image>>;

/// A loader that never finds an image (a Bitmap then fails to build).
pub fn no_images(_: &str) -> Option<Arc<Image>> {
    None
}

/// Builds the shader of node `id`: its params are the values, its children (in order) the
/// input shaders.
pub fn build_node(scene: &Scene, id: NodeId, images: ImageLoader) -> Result<Shader, ShaderError> {
    build_at(scene, id, images, 0)
}

fn build_at(
    scene: &Scene,
    id: NodeId,
    images: ImageLoader,
    depth: usize,
) -> Result<Shader, ShaderError> {
    if depth > MAX_DEPTH {
        return Err(ShaderError::Inputs("shader tree too deep".into()));
    }
    let node = scene
        .node(id)
        .ok_or_else(|| ShaderError::Inputs(format!("node {id} does not exist")))?;
    let is_shader = scene
        .node_type(id)
        .is_some_and(|t| t.category == Category::Shader);
    if !is_shader {
        return Err(ShaderError::UnknownType(node.type_id.clone()));
    }
    let values: Values = node.params.clone();
    let mut inputs = Inputs::default();
    for c in &node.children {
        inputs.shaders.push(build_at(scene, *c, images, depth + 1)?);
    }
    if let Some(Value::String(path)) = node.params.get("file")
        && !path.is_empty()
    {
        inputs.image = images(path);
    }
    build(&node.type_id, &values, &inputs)
}

impl ShaderSet {
    /// Every shader the texture slots of `material` link to, keyed by
    /// [`forge_material::shader_name`]. A slot whose tree does not build is left out (it samples
    /// white) and returned with its error.
    pub fn from_material(
        scene: &Scene,
        material: NodeId,
        images: ImageLoader,
    ) -> (ShaderSet, Vec<(NodeId, ShaderError)>) {
        let mut set = ShaderSet::default();
        let mut failed = Vec::new();
        let Some(node) = scene.node(material) else {
            return (set, failed);
        };
        for v in node.params.values() {
            if let Value::Link(Some(root)) = v {
                match build_node(scene, *root, images) {
                    Ok(s) => {
                        set.shaders.insert(shader_name(*root), s);
                    }
                    Err(e) => failed.push((*root, e)),
                }
            }
        }
        (set, failed)
    }
}
