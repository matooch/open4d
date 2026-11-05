//! Fuzz entry points, mutation engine and soak driver (docs/specs/PLATFORM.md §9, brief 182).
//!
//! The nightly cargo-fuzz targets (`fuzz/`, a separate cargo workspace) and the stable
//! mini-fuzz tests of this crate call the same functions in [`targets`].

pub mod alloc;
pub mod rng;
pub mod schema_gen;
pub mod soak;
pub mod targets;

pub use rng::{XorShift64, mutate};
pub use targets::{
    FS_COMMANDS, IMAGE_LIMITS, MAX_INPUT, fuzz_command_args, fuzz_exr, fuzz_image, fuzz_obj,
    fuzz_rpc_line, fuzz_scene_file, fuzz_script,
};

use std::path::{Path, PathBuf};

/// The committed seed corpus of `target` (`fuzz/corpus/<target>`), file contents sorted by name.
pub fn seed_corpus(target: &str) -> Vec<Vec<u8>> {
    let dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/corpus")
        .join(target);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.sort();
    files.iter().filter_map(|p| std::fs::read(p).ok()).collect()
}
