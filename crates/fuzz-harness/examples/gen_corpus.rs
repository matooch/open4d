//! Regenerates the seed corpora under `fuzz/corpus/<target>/` from this repository's own
//! fixtures, encoders and samples (docs/specs/PLATFORM.md §9.4):
//! `cargo run -p forge-fuzz-harness --example gen_corpus`.
//!
//! Nothing here reads a reference-application file: scene seeds are our test fixtures, OBJ
//! seeds are written by our own writer from our own primitives, image seeds are written by our
//! own encoders.

use std::fs;
use std::path::{Path, PathBuf};

use forge_app::App;
use forge_fuzz_harness::targets::{FS_COMMANDS, fuzzable_command_ids};
use forge_image::{
    AlphaMode, BitDepth, EncodeOptions, ExrChannel, ExrLayer, ExrPrecision, Format, Image, encode,
    encode_exr_layers,
};
use serde_json::json;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn put(target: &str, name: &str, bytes: &[u8]) {
    assert!(bytes.len() < 64 << 10, "{target}/{name} is too large");
    let dir = root().join("fuzz/corpus").join(target);
    fs::create_dir_all(&dir).expect("create corpus dir");
    fs::write(dir.join(name), bytes).expect("write seed");
}

fn scenes() {
    let dir = root().join("crates/core/tests/fixtures");
    let mut names: Vec<_> = fs::read_dir(&dir)
        .expect("fixtures")
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|n| n.ends_with(".forge"))
        .collect();
    names.sort();
    for n in names {
        let bytes = fs::read(dir.join(&n)).expect("fixture");
        if bytes.len() < 32 << 10 {
            put("scene_file", &n, &bytes);
        }
    }
}

fn objs() {
    let mut app = App::new_default();
    for (i, ty) in [
        "forge.cube",
        "forge.sphere",
        "forge.cylinder",
        "forge.plane",
    ]
    .iter()
    .enumerate()
    {
        app.run("object.create", json!({ "type": ty }))
            .expect("create");
        let objects = forge_io::export_scene(app.doc.scene(), None);
        let text = forge_io::obj::write(&objects, &forge_io::obj::WriteOptions::default());
        if text.len() < 48 << 10 {
            put("obj", &format!("scene-{i}.obj"), text.as_bytes());
        }
    }
    put(
        "obj",
        "tiny-quad.obj",
        b"# a quad\no quad\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvn 0 0 1\nf 1/1/1 2/1/1 3/1/1 4/1/1\n",
    );
    put(
        "obj",
        "negative-indices.obj",
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\ng a\nusemtl m\ns off\nl 1 2\np 3\n",
    );
    put("obj", "empty.obj", b"");
}

fn images() {
    let img = Image::from_fn(8, 6, AlphaMode::Straight, |x, y| {
        [x as f32 / 7.0, y as f32 / 5.0, 0.25, 1.0]
    });
    for (name, format, depth) in [
        ("rgba8.png", Format::Png, BitDepth::U8),
        ("rgba16.png", Format::Png, BitDepth::U16),
        ("rgb.jpg", Format::Jpeg, BitDepth::U8),
        ("rgba8.tif", Format::Tiff, BitDepth::U8),
        ("rgba16.tif", Format::Tiff, BitDepth::U16),
        ("float.hdr", Format::Hdr, BitDepth::F32),
        ("float.exr", Format::Exr, BitDepth::F32),
    ] {
        let opts = EncodeOptions {
            depth,
            ..EncodeOptions::default()
        };
        if let Ok(bytes) = encode(&img, format, &opts) {
            put("image", name, &bytes);
        }
    }
}

fn exrs() {
    let layer = |name: Option<&str>, chans: &[&str], w: usize, h: usize| ExrLayer {
        name: name.map(str::to_string),
        width: w,
        height: h,
        channels: chans
            .iter()
            .enumerate()
            .map(|(k, c)| ExrChannel {
                name: c.to_string(),
                samples: (0..w * h).map(|i| (i + k) as f32 * 0.125).collect(),
            })
            .collect(),
    };
    let single = [layer(None, &["R", "G", "B", "A"], 4, 3)];
    let multi = [
        layer(Some("beauty"), &["R", "G", "B", "A"], 4, 4),
        layer(Some("depth"), &["Z"], 4, 4),
    ];
    for (name, layers, prec) in [
        ("single-f32.exr", &single[..], ExrPrecision::F32),
        ("multi-f16.exr", &multi[..], ExrPrecision::F16),
        ("multi-f32.exr", &multi[..], ExrPrecision::F32),
    ] {
        if let Ok(bytes) = encode_exr_layers(layers, prec) {
            put("exr", name, &bytes);
        }
    }
}

fn commands() {
    let app = App::new_default();
    for (i, id) in fuzzable_command_ids(&app).iter().enumerate() {
        let Some(handler) = app.commands.get(id) else {
            continue;
        };
        let mut data = (i as u16).to_le_bytes().to_vec();
        data.extend_from_slice(handler.sample_args().to_string().as_bytes());
        put("command_args", &format!("{id}.bin"), &data);
    }
    assert!(FS_COMMANDS.iter().all(|c| app.commands.get(c).is_some()));
}

fn rpc() {
    let lines = [
        (
            "version",
            json!({"jsonrpc":"2.0","id":1,"method":"app.version"}),
        ),
        (
            "list",
            json!({"jsonrpc":"2.0","id":2,"method":"command.list","params":{}}),
        ),
        (
            "run-create",
            json!({"jsonrpc":"2.0","id":3,"method":"command.run","params":{"id":"object.create","args":{"type":"forge.cube"}}}),
        ),
        (
            "scene",
            json!({"jsonrpc":"2.0","id":"a","method":"scene.get"}),
        ),
        (
            "node",
            json!({"jsonrpc":"2.0","id":4,"method":"node.get","params":{"id":1}}),
        ),
        (
            "mesh",
            json!({"jsonrpc":"2.0","id":5,"method":"eval.mesh","params":{"node":1,"frame":0}}),
        ),
        (
            "subscribe",
            json!({"jsonrpc":"2.0","id":6,"method":"events.subscribe","params":{"kinds":["scene","frame"]}}),
        ),
        (
            "token",
            json!({"jsonrpc":"2.0","id":7,"token":"token","method":"timeline.get"}),
        ),
    ];
    for (name, v) in lines {
        put(
            "rpc_line",
            &format!("{name}.json"),
            v.to_string().as_bytes(),
        );
    }
    put(
        "rpc_line",
        "bad-json.json",
        b"{\"jsonrpc\":\"2.0\",\"id\":1,",
    );
    put("rpc_line", "array.json", b"[1,2,3]");
}

fn scripts() {
    let dir = root().join("examples/scripts");
    for e in fs::read_dir(&dir).expect("examples/scripts").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.ends_with(".rhai") {
            put("script", &name, &fs::read(e.path()).expect("script"));
        }
    }
    put(
        "script",
        "arith.rhai",
        b"let x = 0; for i in 0..100 { x += i; } x\n",
    );
    put(
        "script",
        "recursion.rhai",
        b"fn f(n) { if n == 0 { 0 } else { 1 + f(n - 1) } } f(10)\n",
    );
}

fn main() {
    scenes();
    objs();
    images();
    exrs();
    commands();
    rpc();
    scripts();
}
