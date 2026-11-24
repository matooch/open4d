//! Scene-to-viewport material adapter (VIEWPORT.md §15).

use std::sync::Arc;

use forge_core::{Node, NodeId, Scene, Value};
use forge_geometry::{CornerUvs, PolyMesh};
use forge_material::node_material::{NODE_MATERIAL, compile_hosted};
use forge_material::{
    MATERIAL, Material, Projection, TEXTURE_TAG, TagSpace, UVW_TAG, generate_uvw,
};
use forge_shaders::ShaderSet;
use forge_viewport::material_preview::PreviewMaterial;
use glam::{DMat4, DVec3};

pub(crate) fn load_image(path: &str) -> Option<Arc<forge_image::Image>> {
    let bytes = std::fs::read(path).ok()?;
    let decoded = forge_image::decode(&bytes, &forge_image::Limits::default()).ok()?;
    Some(Arc::new(decoded.image))
}

pub(crate) struct Binding<'a> {
    pub tag: &'a Node,
    pub material: &'a Node,
    uvw: Option<&'a Node>,
}

impl<'a> Binding<'a> {
    pub fn resolve(scene: &'a Scene, id: NodeId) -> Option<Self> {
        let host = scene.node(id)?;
        let mut tags = host.tags.iter().filter_map(|id| scene.node(*id));
        let (tag, material) = tags.clone().rev().find_map(|tag| {
            if tag.type_id != TEXTURE_TAG
                || matches!(tag.params.get("selection"), Some(Value::String(s)) if !s.is_empty())
            {
                return None;
            }
            let Some(Value::Link(Some(id))) = tag.params.get("material") else {
                return None;
            };
            let material = scene
                .node(*id)
                .filter(|n| matches!(n.type_id.as_str(), MATERIAL | NODE_MATERIAL))?;
            Some((tag, material))
        })?;
        Some(Self {
            tag,
            material,
            uvw: tags.find(|n| n.type_id == UVW_TAG),
        })
    }

    pub fn material_revision(&self, scene: &Scene) -> u64 {
        scene
            .deep_revision(self.material.id)
            .unwrap_or(self.material.rev)
    }

    pub fn revision(&self, scene: &Scene) -> u64 {
        use forge_viewport::cache::combine;
        let mut rev = combine(self.tag.rev, self.material_revision(scene));
        rev = combine(rev, self.tag.id.0);
        if let Some(uvw) = self.uvw {
            rev = combine(rev, uvw.rev);
        }
        rev
    }

    pub fn bake(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
    ) -> Arc<PreviewMaterial> {
        let graph = compile_hosted(self.material)
            .unwrap_or_else(|_| forge_material::compile(&Material::new()));
        let (shaders, _) = ShaderSet::from_material(scene, self.material.id, &load_image);
        Arc::new(PreviewMaterial::bake(
            device,
            queue,
            &graph,
            &shaders,
            |name| shaders.shaders.contains_key(name),
        ))
    }

    pub fn coords(&self, mesh: &PolyMesh, world_to_local: DMat4, stored: bool) -> CornerUvs {
        let kind = match self.tag.params.get("projection") {
            Some(Value::Enum(s)) => s.as_str(),
            _ => "cubic",
        };
        if kind == "uvw"
            && stored
            && let Some(uv) = self.stored_coords(mesh)
        {
            return uv;
        }
        let local = PolyMesh {
            points: mesh
                .points
                .iter()
                .map(|p| world_to_local.transform_point3(*p))
                .collect(),
            polys: mesh.polys.clone(),
        };
        let vec = |key, fallback| match self.tag.params.get(key) {
            Some(Value::Vec3(v)) => *v,
            _ => fallback,
        };
        let space = TagSpace {
            position: DVec3::from(vec("position", [0.0; 3])),
            rotation: vec("rotation", [0.0; 3]),
            size: DVec3::from(vec("size", [100.0; 3])),
        };
        generate_uvw(
            &local,
            &space,
            Projection::from_id(kind, None).unwrap_or(Projection::Cubic),
            DMat4::IDENTITY,
        )
        .expect("preview projection is supported")
    }

    fn stored_coords(&self, mesh: &PolyMesh) -> Option<CornerUvs> {
        let Some(Value::FloatList(values)) = self.uvw?.params.get("uvw") else {
            return None;
        };
        let count = mesh.polys.iter().map(Vec::len).sum::<usize>();
        if values.len() != 2 * count {
            return None;
        }
        let mut pairs = values.as_chunks::<2>().0.iter().copied();
        Some(CornerUvs {
            corners: mesh
                .polys
                .iter()
                .map(|p| pairs.by_ref().take(p.len()).collect())
                .collect(),
        })
    }
}
