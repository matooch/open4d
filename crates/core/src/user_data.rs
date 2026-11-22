//! Per-node user data (custom parameters; brief 110).

use crate::ParamDesc;

/// Reserved prefix for user-data parameter ids. Type parameters may not use it.
pub const PREFIX: &str = "user_data.";

/// Generate an id for the next user-data entry on a node that already has `len` entries.
pub fn id_for_index(index: usize) -> String {
    format!("{PREFIX}{index}")
}

/// Build a user-data descriptor with the standard id for the given index and a
/// "User Data" UI group. The descriptor is validated by the caller.
pub fn descriptor(index: usize, desc: ParamDesc) -> ParamDesc {
    ParamDesc {
        id: id_for_index(index),
        ui: crate::UiHint {
            group: "User Data".to_string(),
            ..desc.ui
        },
        ..desc
    }
}
