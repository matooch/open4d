//! The Matrix object: the cloner's distribution modes and Transform tab without any
//! prototype, so it makes clone data and no geometry (clone-toolset spec M1).

use std::collections::BTreeMap;

use forge_core::{NodeType, ParamDesc, ParamKind, Value};

use crate::node::{cloner_type, settings_of};
use crate::settings::Settings;

/// Type id of the Matrix object.
pub const MATRIX: &str = "forge.matrix";

/// Cloner parameters that only make sense with prototypes.
const PROTOTYPE_PARAMS: &[&str] = &[
    "clones_mode",
    "clones_seed",
    "reset_coordinates",
    "fix_texture",
    "instance_mode",
    "viewport_mode",
];

/// The Matrix node type: the cloner's parameters without the prototype options, with the
/// Matrix defaults (Grid mode, count (40, 1, 40), size 10.5) and the `generate` option.
pub fn matrix_type() -> NodeType {
    let cloner = cloner_type();
    let mut ty = NodeType::new(MATRIX, "Matrix", cloner.category);
    for mut desc in cloner.params {
        if PROTOTYPE_PARAMS.contains(&desc.id.as_str()) {
            continue;
        }
        match desc.id.as_str() {
            "grid_count" => desc.default = Value::Vec3([40.0, 1.0, 40.0]),
            "grid_size" => desc.default = Value::Vec3([10.5; 3]),
            // The Matrix calls the mode "Grid".
            "mode" => {
                if let ParamKind::Enum { options } = &mut desc.kind {
                    for o in options.iter_mut().filter(|o| o.id == "grid") {
                        o.label = "Grid".into();
                    }
                }
            }
            _ => {}
        }
        if desc.id == "mode" {
            ty = ty.param(
                ParamDesc::enumeration(
                    "generate",
                    "Generate",
                    "matrices_only",
                    &[
                        ("matrices_only", "Matrices Only"),
                        ("particles", "Particles"),
                    ],
                )
                .group("Object"),
            );
        }
        ty = ty.param(desc);
    }
    ty
}

/// Reads a Matrix object's parameters as cloner settings (spec M1).
pub fn matrix_settings(values: &BTreeMap<String, Value>) -> Settings {
    settings_of(&matrix_type(), values)
}
