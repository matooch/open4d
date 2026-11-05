//! One fuzz entry point per reader of untrusted bytes (docs/specs/PLATFORM.md §9.1).
//!
//! Each function must return normally for every input; a panic is a bug in the reader.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use forge_app::App;
use forge_command::schema;
use forge_core::{Registry, file};
use forge_image::Limits;
use forge_io::obj;
use forge_script::{ScriptOptions, run_script};
use serde_json::{Value, json};

/// Commands that read or write files; never run with fuzzed arguments (spec §9.1).
pub const FS_COMMANDS: &[&str] = &[
    "file.new",
    "file.open",
    "file.save",
    "file.save_as",
    "file.import_obj",
    "file.export_obj",
    "file.import",
    "file.export",
    "file.autosave_now",
    "browser.save_preset",
    "browser.delete_preset",
    "browser.open_recent",
    "browser.clear_recent",
];

/// Decode limits the image targets use: bounded memory for honest readers.
pub const IMAGE_LIMITS: Limits = Limits {
    max_dimension: 4096,
    max_pixels: 4 << 20,
};

/// Largest input a target is given by the mutation engine (cargo-fuzz uses `-max_len`).
pub const MAX_INPUT: usize = 64 << 10;

fn registry() -> &'static Arc<Registry> {
    static R: OnceLock<Arc<Registry>> = OnceLock::new();
    R.get_or_init(|| Arc::new(forge_app::default_registry().expect("default registry")))
}

/// The commands a target may run, in a stable order (sorted by id).
pub fn fuzzable_command_ids(app: &App) -> Vec<String> {
    let mut ids: Vec<String> = app
        .commands
        .iter()
        .map(|c| c.id().to_string())
        .filter(|id| !FS_COMMANDS.contains(&id.as_str()))
        .collect();
    ids.sort();
    ids
}

const CHUNK_MARKER: &[u8] = b"\n%%FORGE-CHUNKS 1\n";

/// Recomputes the chunk hashes of a chunked scene file, so mutated chunk payloads get past the
/// integrity check and reach the mesh/index decoders (a fuzzer cannot guess FNV-1a). Returns
/// `None` when `data` has no chunk section or its framing is already broken.
pub fn reseal_chunks(data: &[u8]) -> Option<Vec<u8>> {
    let at = data
        .windows(CHUNK_MARKER.len())
        .position(|w| w == CHUNK_MARKER)?;
    let head_end = at + CHUNK_MARKER.len();
    let mut out = data[..head_end].to_vec();
    let mut rest = &data[head_end..];
    let count = u32::from_le_bytes(rest.get(..4)?.try_into().ok()?) as usize;
    out.extend_from_slice(&rest[..4]);
    rest = &rest[4..];
    for _ in 0..count {
        let kind = *rest.first()?;
        let len = u64::from_le_bytes(rest.get(1..9)?.try_into().ok()?);
        let len = usize::try_from(len).ok()?;
        let payload = rest.get(17..17usize.checked_add(len)?)?;
        let hash = payload.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
        });
        out.push(kind);
        out.extend_from_slice(&(len as u64).to_le_bytes());
        out.extend_from_slice(&hash.to_le_bytes());
        out.extend_from_slice(payload);
        rest = &rest[17 + len..];
    }
    out.extend_from_slice(rest);
    Some(out)
}

fn load_and_resave(data: &[u8]) {
    if let Ok((scene, _report)) = file::load_bytes(registry().clone(), data) {
        let text = file::save(&scene);
        let again = file::load(registry().clone(), &text);
        assert!(
            again.is_ok(),
            "a loaded scene was saved in a form that does not load: {:?}",
            again.err()
        );
    }
}

/// Native scene file: load, and when it loads, save and load again. A chunked file is also
/// loaded with its chunk hashes repaired ([`reseal_chunks`]).
pub fn fuzz_scene_file(data: &[u8]) {
    load_and_resave(data);
    if let Some(sealed) = reseal_chunks(data)
        && sealed != data
    {
        load_and_resave(&sealed);
    }
}

/// OBJ reader (both winding options) and, for what it reads, the writer.
pub fn fuzz_obj(data: &[u8]) {
    let text = String::from_utf8_lossy(data);
    for flip_z in [false, true] {
        if let Ok(objects) = obj::read(&text, &obj::ReadOptions { flip_z }) {
            let _ = obj::write(&objects, &obj::WriteOptions::default());
        }
    }
}

/// Raster / HDR / TIFF decoding.
pub fn fuzz_image(data: &[u8]) {
    let _ = forge_image::detect_format(data);
    if let Ok(decoded) = forge_image::decode(data, &IMAGE_LIMITS) {
        let _ = decoded.info;
    }
}

/// Multi-layer EXR decoding.
pub fn fuzz_exr(data: &[u8]) {
    if let Ok(layers) = forge_image::decode_exr_layers(data, &IMAGE_LIMITS) {
        for l in &layers {
            let _ = l.to_image();
        }
    }
}

/// Command arguments: `data[0..2]` (little endian) picks a command, the rest is the JSON
/// argument (a plain string when it is not JSON). Validated against the command's schema and
/// run on a fresh application.
pub fn fuzz_command_args(data: &[u8]) {
    if data.len() < 2 {
        return;
    }
    let mut app = App::new_default();
    let ids = fuzzable_command_ids(&app);
    let id = &ids[(u16::from_le_bytes([data[0], data[1]]) as usize) % ids.len()];
    let body = &data[2..];
    let args: Value = serde_json::from_slice(body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(body).into_owned()));
    if let Some(handler) = app.commands.get(id) {
        let _ = schema::validate(&handler.args_schema(), &args);
    }
    // A selection and a few nodes give the commands something to act on.
    seed_scene(&mut app);
    let _ = app.run(id, args);
}

/// A small scene for commands that need nodes.
pub fn seed_scene(app: &mut App) {
    for (ty, _) in [("forge.cube", 0), ("forge.sphere", 1), ("forge.null", 2)] {
        let _ = app.run("object.create", json!({ "type": ty }));
    }
}

/// An RPC request line: the validation path, and the methods on a fresh application.
pub fn fuzz_rpc_line(data: &[u8]) {
    let _ = forge_rpc::handle_request_line(data, None);
    let _ = forge_rpc::handle_request_line(data, Some("token"));
    let Ok(Value::Object(request)) = serde_json::from_slice::<Value>(data) else {
        return;
    };
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return;
    };
    let params = match request.get("params") {
        Some(p @ Value::Object(_)) => p.clone(),
        _ => json!({}),
    };
    if method == "command.run"
        && let Some(id) = params.get("id").and_then(Value::as_str)
        && FS_COMMANDS.contains(&id)
    {
        return;
    }
    if method == "viewport.screenshot" {
        return;
    }
    let mut app = App::new_default();
    seed_scene(&mut app);
    let _ = forge_rpc::dispatch(&mut app, method, params);
}

/// Script compile and run, with tight limits.
pub fn fuzz_script(data: &[u8]) {
    let source = String::from_utf8_lossy(data);
    // `file.export_obj` is allowed to scripts by design (SCRIPTING.md §3); a fuzzer must not
    // write files, so sources that mention it are skipped.
    if source.contains("export") || source.contains("file.") {
        return;
    }
    let mut app = App::new_default();
    seed_scene(&mut app);
    let opts = ScriptOptions {
        max_operations: 20_000,
        max_commands: 50,
        timeout: Duration::from_secs(1),
        max_call_levels: 16,
    };
    let commands = app.commands.clone();
    let _ = run_script(&mut app, &commands, &source, &opts);
}
