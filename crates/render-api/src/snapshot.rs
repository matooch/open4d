//! The scene snapshot (RENDER.md R1).

use std::fmt;
use std::sync::Arc;

use forge_material::{ShadingGraph, TextureSampler};
use glam::DMat4;

use crate::camera::{Camera, Projection};
use crate::engine::RenderError;
use crate::light::{Environment, Light};

/// A triangle mesh in object space.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshData {
    pub positions: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    /// Shading normals, 3 per triangle (one per corner), object space. Empty = flat.
    pub normals: Vec<[f64; 3]>,
    /// UVW, 3 per triangle. Empty = zero.
    pub uvs: Vec<[f64; 3]>,
}

impl MeshData {
    pub fn new(positions: Vec<[f64; 3]>, triangles: Vec<[u32; 3]>) -> MeshData {
        MeshData {
            positions,
            triangles,
            normals: Vec::new(),
            uvs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub mesh: usize,
    /// Object to world, column-major.
    pub transform: [f64; 16],
    pub material: usize,
    pub object_id: u32,
}

impl Instance {
    pub fn new(mesh: usize, transform: DMat4, material: usize, object_id: u32) -> Instance {
        Instance {
            mesh,
            transform: transform.to_cols_array(),
            material,
            object_id,
        }
    }
}

#[derive(Clone)]
pub struct MaterialData {
    pub name: String,
    pub graph: ShadingGraph,
    pub sampler: Arc<dyn TextureSampler + Send + Sync>,
}

impl fmt::Debug for MaterialData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaterialData")
            .field("name", &self.name)
            .field("graph", &self.graph)
            .finish_non_exhaustive()
    }
}

impl MaterialData {
    /// A material from a compiled graph with a sampler that answers white for every texture.
    pub fn new(name: &str, graph: ShadingGraph) -> MaterialData {
        MaterialData {
            name: name.to_string(),
            graph,
            sampler: Arc::new(forge_material::ir::White),
        }
    }

    pub fn with_sampler(mut self, s: Arc<dyn TextureSampler + Send + Sync>) -> MaterialData {
        self.sampler = s;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub meshes: Vec<MeshData>,
    pub instances: Vec<Instance>,
    pub materials: Vec<MaterialData>,
    pub lights: Vec<Light>,
    pub camera: Camera,
    pub environment: Environment,
}

impl Default for Snapshot {
    fn default() -> Self {
        Snapshot {
            meshes: Vec::new(),
            instances: Vec::new(),
            materials: Vec::new(),
            lights: Vec::new(),
            camera: Camera::default_perspective(),
            environment: Environment::default(),
        }
    }
}

impl Snapshot {
    pub fn validate(&self) -> Result<(), RenderError> {
        let bad = |s: String| Err(RenderError::Snapshot(s));
        for (i, m) in self.meshes.iter().enumerate() {
            let n = m.positions.len() as u32;
            if m.positions.iter().flatten().any(|v| !v.is_finite()) {
                return bad(format!("mesh {i} has non-finite positions"));
            }
            if m.triangles.iter().flatten().any(|&v| v >= n) {
                return bad(format!("mesh {i} has a triangle index out of range"));
            }
            let c = m.triangles.len() * 3;
            if !m.normals.is_empty() && m.normals.len() != c {
                return bad(format!("mesh {i}: normals must be 3 per triangle"));
            }
            if !m.uvs.is_empty() && m.uvs.len() != c {
                return bad(format!("mesh {i}: uvs must be 3 per triangle"));
            }
        }
        for (i, inst) in self.instances.iter().enumerate() {
            if inst.mesh >= self.meshes.len() {
                return bad(format!("instance {i}: mesh {} out of range", inst.mesh));
            }
            if inst.material >= self.materials.len() {
                return bad(format!(
                    "instance {i}: material {} out of range",
                    inst.material
                ));
            }
            if inst.transform.iter().any(|v| !v.is_finite()) {
                return bad(format!("instance {i}: non-finite transform"));
            }
        }
        if self.camera.transform.iter().any(|v| !v.is_finite()) {
            return bad("camera transform is not finite".into());
        }
        match self.camera.projection {
            Projection::Perspective {
                focal_mm,
                sensor_mm,
                ..
            } if focal_mm <= 0.0 || sensor_mm <= 0.0 => {
                return bad("camera focal length and sensor must be positive".into());
            }
            Projection::Orthographic { width } if width <= 0.0 => {
                return bad("orthographic width must be positive".into());
            }
            _ => {}
        }
        Ok(())
    }
}
