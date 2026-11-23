//! Alembic Ogawa transforms and fixed-topology mesh caches (IO.md §7).

mod ogawa;
mod read;
mod write;

use forge_core::transform::{Transform, hpb_from_rotation};
use glam::{DMat4, DVec3};

pub use read::{ImportScene, read};
pub use write::write;

const MAX_FRAMES: usize = 10_000;
const MAX_DEPTH: usize = 64;
const MAX_OBJECTS: usize = 100_000;
const MAX_BYTES: usize = 128 * 1024 * 1024;
const POINT: &str = "geoScope=vtx;interpretation=point";
const UV: &str = "arrayExtent=1;geoScope=fvr;interpretation=vector;isGeomParam=true;podExtent=2;podName=float32_t";
const XFORM: &str = "schema=AbcGeom_Xform_v3";
const MESH: &str = "schema=AbcGeom_PolyMesh_v1;schemaBaseType=AbcGeom_GeomBase_v1";
const CAMERA: &str = "schema=AbcGeom_Camera_v1";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid Alembic: {0}")]
    Invalid(&'static str),
    #[error("unsupported Alembic: {0}")]
    Unsupported(&'static str),
}

fn invalid(reason: &'static str) -> Error {
    Error::Invalid(reason)
}

fn checked_scale(scale: f64) -> Result<(), Error> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(invalid("scale must be finite and positive"));
    }
    Ok(())
}

fn point(p: DVec3, scale: f64) -> DVec3 {
    DVec3::new(p.x, p.y, -p.z) * scale
}

fn flip(mut matrix: DMat4, scale: f64) -> DMat4 {
    let basis = DMat4::from_scale(DVec3::new(1.0, 1.0, -1.0));
    matrix = basis * matrix * basis;
    matrix.w_axis.x *= scale;
    matrix.w_axis.y *= scale;
    matrix.w_axis.z *= scale;
    matrix
}

fn trs(matrix: DMat4) -> Result<Transform, Error> {
    if !matrix.is_finite()
        || matrix.x_axis.w != 0.0
        || matrix.y_axis.w != 0.0
        || matrix.z_axis.w != 0.0
        || matrix.w_axis.w != 1.0
    {
        return Err(Error::Unsupported("non-affine transform"));
    }
    let xyz = [
        matrix.x_axis.truncate(),
        matrix.y_axis.truncate(),
        matrix.z_axis.truncate(),
    ];
    let mut scale = DVec3::new(xyz[0].length(), xyz[1].length(), xyz[2].length());
    if !scale.is_finite() || scale.min_element() <= 1e-15 {
        return Err(Error::Unsupported("singular transform"));
    }
    if matrix.determinant() < 0.0 {
        scale.x = -scale.x;
    }
    let basis = glam::DMat3::from_cols(xyz[0] / scale.x, xyz[1] / scale.y, xyz[2] / scale.z);
    if basis.x_axis.dot(basis.y_axis).abs() > 1e-8
        || basis.y_axis.dot(basis.z_axis).abs() > 1e-8
        || basis.z_axis.dot(basis.x_axis).abs() > 1e-8
    {
        return Err(Error::Unsupported("sheared transform"));
    }
    Ok(Transform {
        pos: matrix.w_axis.truncate(),
        rot: hpb_from_rotation(basis),
        scale,
    })
}
