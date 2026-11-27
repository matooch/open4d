//! Screen-space handles that edit object parameters along local axes.

use forge_core::{NodeId, ParamKind, Scene, Value};
use forge_viewport::{camera::Camera, snap::line_param_closest_to_ray};
use glam::DVec3;

#[derive(Clone, Copy)]
pub(crate) struct Handle {
    pub param: &'static str,
    factor: f64,
    value: f64,
    origin: DVec3,
    axis: DVec3,
}

impl Handle {
    pub fn point(&self) -> DVec3 {
        self.origin + self.axis * (self.value * self.factor)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SizeDrag {
    pub node: NodeId,
    pub param: &'static str,
    pub gesture: u64,
    handle: Handle,
    start: f64,
}

impl SizeDrag {
    pub fn begin(
        node: NodeId,
        handle: Handle,
        camera: &Camera,
        [x, y]: [f64; 2],
        [w, h]: [f64; 2],
        gesture: u64,
    ) -> Option<Self> {
        let ray = camera.ray(x, y, w, h);
        let start = line_param_closest_to_ray(handle.origin, handle.axis, &ray)?;
        Some(Self {
            node,
            param: handle.param,
            gesture,
            handle,
            start,
        })
    }

    pub fn value(
        &self,
        scene: &Scene,
        camera: &Camera,
        [x, y]: [f64; 2],
        [w, h]: [f64; 2],
    ) -> Option<f64> {
        let ray = camera.ray(x, y, w, h);
        let distance = line_param_closest_to_ray(self.handle.origin, self.handle.axis, &ray)?;
        let value = self.handle.value + (distance - self.start) / self.handle.factor;
        let ParamKind::Float { min, max } = scene.param_desc(self.node, self.param)?.kind else {
            return None;
        };
        let value = value.clamp(min, max);
        (value.is_finite() && scene.param(self.node, self.param) != Some(&Value::Float(value)))
            .then_some(value)
    }
}

pub(crate) fn for_node(scene: &Scene, id: NodeId) -> Vec<Handle> {
    let Some(node) = scene.node(id) else {
        return Vec::new();
    };
    let Some(world) = scene.world_matrix(id) else {
        return Vec::new();
    };
    let number = |param| match scene.param(id, param) {
        Some(Value::Float(value)) => *value,
        _ => 0.0,
    };
    let option = |param| match scene.param(id, param) {
        Some(Value::Enum(value)) => value.as_str(),
        _ => "",
    };
    let make = |param, direction, factor, offset| Handle {
        param,
        value: number(param),
        factor,
        origin: world.transform_point3(offset),
        axis: world.transform_vector3(direction),
    };
    let centered = |param, direction, factor| make(param, direction, factor, DVec3::ZERO);
    let (up, radial, other) = match option("orientation") {
        "pos_x" => (DVec3::X, -DVec3::Y, DVec3::Z),
        "neg_x" => (-DVec3::X, DVec3::Y, DVec3::Z),
        "neg_y" => (-DVec3::Y, -DVec3::X, DVec3::Z),
        "pos_z" => (DVec3::Z, DVec3::X, -DVec3::Y),
        "neg_z" => (-DVec3::Z, DVec3::X, DVec3::Y),
        _ => (DVec3::Y, DVec3::X, DVec3::Z),
    };
    match node.type_id.as_str() {
        "forge.cube" => vec![
            centered("size_x", DVec3::X, 0.5),
            centered("size_y", DVec3::Y, 0.5),
            centered("size_z", DVec3::Z, 0.5),
        ],
        "forge.sphere" => vec![centered("radius", DVec3::X, 1.0)],
        "forge.plane" => vec![
            centered("width", radial, 0.5),
            centered("height", other, 0.5),
        ],
        "forge.cylinder" => vec![centered("radius", radial, 1.0), centered("height", up, 0.5)],
        "forge.cone" => {
            let cap = up * (number("height") * 0.5);
            vec![
                centered("height", up, 0.5),
                make("bottom_radius", radial, 1.0, -cap),
                make("top_radius", radial, 1.0, cap),
            ]
        }
        "forge.torus" => vec![
            centered("ring_radius", radial, 1.0),
            make("pipe_radius", up, 1.0, radial * number("ring_radius")),
        ],
        "forge.light" if option("type") == "area" => match option("area_shape") {
            "rectangle" => vec![
                centered("size_x", DVec3::X, 0.5),
                centered("size_y", DVec3::Y, 0.5),
            ],
            "disc" | "sphere" => vec![centered("size_x", DVec3::X, 0.5)],
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}
