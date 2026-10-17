//! Object transforms (docs/specs/TRANSFORM.md). Conventions come from
//! docs/parity/transform.md: rotation fields (h, p, b) turn about Y, X, Z by the
//! negated angle (the reference world is left-handed), applied b, then p, then h.

use std::collections::BTreeMap;

use glam::{DMat3, DMat4, DVec3, EulerRot};

use crate::{ParamDesc, Unit, Value};

pub const POS: &str = "transform.pos";
pub const ROT: &str = "transform.rot";
pub const SCALE: &str = "transform.scale";
/// Param ids with this prefix are reserved for the transform.
pub const PREFIX: &str = "transform.";

const GROUP: &str = "Coordinates";

/// The parameters every hierarchy type gets, in order.
pub(crate) fn params() -> [ParamDesc; 3] {
    [
        ParamDesc::vec3(POS, "Position", [0.0; 3])
            .unit(Unit::Distance)
            .group(GROUP),
        ParamDesc::vec3(ROT, "Rotation", [0.0; 3])
            .unit(Unit::Angle)
            .group(GROUP),
        ParamDesc::vec3(SCALE, "Scale", [1.0; 3]).group(GROUP),
    ]
}

/// Position, rotation `(h, p, b)` in radians, and scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub pos: DVec3,
    pub rot: DVec3,
    pub scale: DVec3,
}

impl Transform {
    pub const IDENTITY: Transform = Transform {
        pos: DVec3::ZERO,
        rot: DVec3::ZERO,
        scale: DVec3::ONE,
    };

    /// `T(pos) · Ry(−h) · Rx(−p) · Rz(−b) · S(scale)`.
    pub fn matrix(&self) -> DMat4 {
        let (h, p, b) = (self.rot.x, self.rot.y, self.rot.z);
        DMat4::from_translation(self.pos)
            * DMat4::from_rotation_y(-h)
            * DMat4::from_rotation_x(-p)
            * DMat4::from_rotation_z(-b)
            * DMat4::from_scale(self.scale)
    }

    /// Reads the transform params; missing or mistyped components stay identity.
    pub fn from_params(params: &BTreeMap<String, Value>) -> Transform {
        let get = |id: &str, fallback: DVec3| match params.get(id) {
            Some(Value::Vec3(v)) => DVec3::from_array(*v),
            _ => fallback,
        };
        Transform {
            pos: get(POS, DVec3::ZERO),
            rot: get(ROT, DVec3::ZERO),
            scale: get(SCALE, DVec3::ONE),
        }
    }
}

impl Transform {
    /// The rotation part of [`Transform::matrix`]: `Ry(−h) · Rx(−p) · Rz(−b)`.
    pub fn rotation(&self) -> DMat3 {
        DMat3::from_rotation_y(-self.rot.x)
            * DMat3::from_rotation_x(-self.rot.y)
            * DMat3::from_rotation_z(-self.rot.z)
    }
}

/// `(h, p, b)` with `Ry(−h)·Rx(−p)·Rz(−b) = m` (docs/specs/MANIPULATE.md §1).
pub fn hpb_from_rotation(m: DMat3) -> DVec3 {
    let (a, b, c) = m.to_euler(EulerRot::YXZ);
    DVec3::new(-a, -b, -c)
}

/// Upper 3×3 of `m` with normalized columns.
pub fn rotation_part(m: &DMat4) -> DMat3 {
    let m3 = DMat3::from_mat4(*m);
    DMat3::from_cols(
        m3.x_axis.normalize_or_zero(),
        m3.y_axis.normalize_or_zero(),
        m3.z_axis.normalize_or_zero(),
    )
}
