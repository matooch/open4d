//! Shader field: a shader sampled in space (docs/specs/FIELDS.md F7). The shaders it needs are
//! brief 125; until then the node exists (so lists, undo and files work) and building it fails.

use forge_core::{NodeType, ParamDesc};

use crate::node::{FIELD, field_type};
use crate::{Field, FieldError};

pub const SHADER: &str = "forge.field.shader";

pub fn shader_type() -> NodeType {
    field_type(
        SHADER,
        "Shader Field",
        vec![ParamDesc::string("shader", "Shader", "").group(FIELD)],
        0.0,
        false,
        false,
    )
}

/// Always `Unsupported`: the shader library is not built yet.
pub(crate) fn field() -> Result<Box<dyn Field>, FieldError> {
    Err(FieldError::Unsupported("shader field (needs brief 125)"))
}
