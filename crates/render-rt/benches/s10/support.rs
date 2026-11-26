use forge_material::ir;
use forge_render_api::{
    Camera, HybridSettings, Instance, Light, LightKind, Lighting, MaterialData, MeshData,
    Projection, RenderSettings, ShadowMode, Snapshot,
};
use forge_render_rt::ray::BackendKind;
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const WARMUPS: u32 = 2;
pub const SAMPLES: u32 = 5;
const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    machine: String,
    adapter: String,
    api: String,
    backend: String,
    hybrid: bool,
    workload: String,
    size: [u32; 2],
    warmups: u32,
    samples: u32,
}

impl Identity {
    pub fn new(
        machine: &str,
        adapter: &str,
        api: &str,
        backend: Option<BackendKind>,
        hybrid: bool,
    ) -> Self {
        Self {
            machine: machine.into(),
            adapter: adapter.into(),
            api: api.into(),
            backend: backend.map_or_else(|| "none".into(), |kind| format!("{kind:?}")),
            hybrid,
            workload: "s10-rt-v1".into(),
            size: [WIDTH, HEIGHT],
            warmups: WARMUPS,
            samples: SAMPLES,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub identity: Identity,
    pub median_ns: BTreeMap<String, u64>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum BaselineError {
    #[error("incompatible baseline identity")]
    Incompatible,
    #[error("missing timing: {0}")]
    Missing(String),
    #[error("zero timing: {0}")]
    Zero(String),
    #[error("more than 10% slower: {0:?}")]
    Regressed(Vec<String>),
}

pub fn regression_check(baseline: &Run, current: &Run) -> Result<(), BaselineError> {
    if baseline.identity != current.identity {
        return Err(BaselineError::Incompatible);
    }
    if !baseline.median_ns.contains_key("frame") {
        return Err(BaselineError::Missing("frame".into()));
    }
    let mut regressed = Vec::new();
    for (name, &reference) in &baseline.median_ns {
        let actual = *current
            .median_ns
            .get(name)
            .ok_or_else(|| BaselineError::Missing(name.clone()))?;
        if actual == 0 || reference == 0 {
            return Err(BaselineError::Zero(name.clone()));
        }
        if u128::from(actual) * 10 > u128::from(reference) * 11 {
            regressed.push(name.clone());
        }
    }
    if regressed.is_empty() {
        Ok(())
    } else {
        Err(BaselineError::Regressed(regressed))
    }
}

pub fn settings(hybrid: bool) -> RenderSettings {
    RenderSettings {
        default_light: false,
        hybrid: HybridSettings {
            enabled: hybrid,
            shadows: hybrid,
            reflections: hybrid,
            ao: hybrid,
            gi: hybrid,
            ..HybridSettings::default()
        },
        ..RenderSettings::new(WIDTH, HEIGHT, 1)
    }
}

fn surface() -> MeshData {
    const N: u32 = 1000;
    let positions = (0..=N)
        .flat_map(|z| (0..=N).map(move |x| [x, z]))
        .map(|[x, z]| {
            [
                f64::from(x) * 0.44 - 220.0,
                0.0,
                f64::from(z) * 0.44 - 220.0,
            ]
        })
        .collect();
    let mut triangles = Vec::with_capacity((2 * N * N) as usize);
    for z in 0..N {
        for x in 0..N {
            let a = z * (N + 1) + x;
            let b = a + 1;
            let d = a + (N + 1);
            let c = d + 1;
            triangles.extend([[a, d, c], [a, c, b]]);
        }
    }
    MeshData::new(positions, triangles)
}

fn cube() -> MeshData {
    let positions = vec![
        [-1.25, -1.25, -1.25],
        [1.25, -1.25, -1.25],
        [1.25, 1.25, -1.25],
        [-1.25, 1.25, -1.25],
        [-1.25, -1.25, 1.25],
        [1.25, -1.25, 1.25],
        [1.25, 1.25, 1.25],
        [-1.25, 1.25, 1.25],
    ];
    let triangles = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 4, 7, 3],
        [1, 2, 6, 5],
        [3, 7, 6, 2],
        [0, 1, 5, 4],
    ]
    .into_iter()
    .flat_map(|[a, b, c, d]| [[a, b, c], [a, c, d]])
    .collect();
    MeshData::new(positions, triangles)
}

fn look_at(from: DVec3, to: DVec3, up: DVec3) -> DMat4 {
    let z = (to - from).normalize();
    let x = up.cross(z).normalize();
    let y = z.cross(x);
    DMat4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        from.extend(1.0),
    )
}

fn material(name: &str, color: [f64; 3]) -> MaterialData {
    MaterialData::new(
        name,
        ir::ShadingGraph {
            nodes: vec![ir::IrNode::Color(color), ir::IrNode::Color([1.0; 3])],
            outputs: ir::Outputs {
                diffuse: Some(ir::Diffuse {
                    color: 0,
                    level: 1.0,
                    falloff: 0.0,
                    model: ir::DiffuseModel::Lambertian,
                    roughness: 0.0,
                }),
                specular: Some(ir::Specular {
                    kind: "ggx".into(),
                    width: 0.25,
                    falloff: 0.0,
                    inner_width: 0.0,
                    strength: 0.4,
                    color: 1,
                    metal: false,
                    colored: false,
                }),
                ..ir::Outputs::default()
            },
        },
    )
}

pub fn fixture() -> Snapshot {
    let mut light = Light::new(
        LightKind::Infinite { angle: 0.03 },
        look_at(DVec3::new(-250.0, 450.0, 120.0), DVec3::ZERO, DVec3::Y),
    );
    light.shadow = ShadowMode::Soft;
    let mut scene = Snapshot {
        meshes: vec![surface(), cube()],
        materials: vec![
            material("Surface", [0.35, 0.36, 0.38]),
            material("Clones", [0.44, 0.24, 0.15]),
        ],
        lights: vec![light],
        camera: Camera::new(
            look_at(DVec3::new(0.0, 720.0, -170.0), DVec3::ZERO, DVec3::Y),
            Projection::Orthographic { width: 820.0 },
        ),
        ..Snapshot::default()
    };
    scene.environment.lighting = Lighting::Constant {
        color: [0.7, 0.8, 1.0],
        intensity: 0.25,
    };
    scene.instances.reserve_exact(10_001);
    scene
        .instances
        .push(Instance::new(0, DMat4::IDENTITY, 0, 1));
    for z in 0..100 {
        for x in 0..100 {
            let center = DVec3::new(f64::from(x) * 4.0 - 198.0, 1.25, f64::from(z) * 4.0 - 198.0);
            scene.instances.push(Instance::new(
                1,
                DMat4::from_translation(center),
                1,
                2 + z * 100 + x,
            ));
        }
    }
    scene
}
