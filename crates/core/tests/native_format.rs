//! docs/specs/CORE.md §6.1-6.5 (brief 163): migrations, binary chunks, external references,
//! the fuzz entry point.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use forge_core::file::{
    self, CHUNK_MIN_BYTES, LoadOptions, MIGRATIONS, SaveOptions, VERSION, external_refs,
};
use forge_core::{
    Category, Document, LoadError, MeshData, NodeId, NodeType, ParamDesc, Registry, Value,
};

fn registry() -> Arc<Registry> {
    let mut r = (*common::registry()).clone();
    r.register(
        NodeType::new("test.poly", "Poly", Category::Object)
            .param(ParamDesc::mesh("mesh", "Mesh"))
            .param(ParamDesc::index_list("selection.points", "Selected points")),
    )
    .unwrap();
    r.register(
        NodeType::new("test.tex", "Texture holder", Category::Object)
            .param(ParamDesc::file_path("file", "File", ""))
            .param(ParamDesc::string("note", "Note", "")),
    )
    .unwrap();
    Arc::new(r)
}

fn fixture(name: &str) -> Vec<u8> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn quad() -> MeshData {
    MeshData {
        points: vec![
            [0.0, 0.0, 0.0],
            [1.5, 0.0, 0.0],
            [1.5, 0.1, 1e-9],
            [0.0, 2.0, 0.0],
        ],
        polys: vec![vec![0, 1, 2, 3], vec![0, 1, 2]],
    }
}

/// Every fixture describes the same scene, limited to what its version could store.
fn check_logical_scene(scene: &forge_core::Scene, v: u32) {
    let (root, bx, poly, sel) = (NodeId(1), NodeId(2), NodeId(3), NodeId(4));
    assert_eq!(scene.node(root).unwrap().children, vec![bx, poly], "v{v}");
    assert_eq!(scene.node(bx).unwrap().tags, vec![sel], "v{v}");
    assert_eq!(scene.param(bx, "size"), Some(&Value::Float(3.0)), "v{v}");
    assert_eq!(scene.param(bx, "segments"), Some(&Value::Int(4)), "v{v}");
    assert_eq!(scene.param(sel, "set"), Some(&Value::String("a".into())));
    let mesh = if v >= 2 { quad() } else { MeshData::default() };
    assert_eq!(
        scene.param(poly, "mesh"),
        Some(&Value::Mesh(Arc::new(mesh))),
        "v{v}"
    );
    let idx = if v >= 3 { vec![1, 3] } else { vec![] };
    assert_eq!(
        scene.param(poly, "selection.points"),
        Some(&Value::IndexList(idx)),
        "v{v}"
    );
    assert_eq!(scene.node(bx).unwrap().tracks.len(), usize::from(v >= 4));
    let s = scene.settings();
    if v >= 5 {
        assert_eq!((s.fps, s.start, s.end), (24, 0, 48), "v{v}");
    } else {
        assert_eq!(s, forge_core::SceneSettings::default(), "v{v}");
    }
}

#[test]
fn the_migration_table_has_one_step_per_version() {
    assert_eq!(MIGRATIONS.len() as u32, VERSION - 1);
    assert_eq!(VERSION, 6);
}

#[test]
fn every_old_version_has_a_fixture_that_loads_and_migrates() {
    for v in 1..=5u32 {
        let bytes = fixture(&format!("scene-v{v}.forge"));
        assert!(
            String::from_utf8_lossy(&bytes).contains(&format!("version: {v},")),
            "fixture v{v} must say its own version"
        );
        let (scene, report) = file::load_bytes(registry(), &bytes).unwrap();
        assert!(report.warnings.is_empty(), "v{v}: {:?}", report.warnings);
        check_logical_scene(&scene, v);
        // after loading, the scene saves as the current version and reloads equal
        let text = file::save(&scene);
        assert!(text.contains(&format!("version: {VERSION}")), "{text}");
        let (again, report) = file::load(registry(), &text).unwrap();
        assert!(report.warnings.is_empty());
        assert_eq!(file::save(&again), text, "v{v}");
        check_logical_scene(&again, v);
    }
}

#[test]
fn version_6_fixtures_are_current_and_byte_stable() {
    // The current fixtures pin the on-disk format: change them only with a version bump.
    let (text_scene, bin_scene) = (fixture_scene(false), fixture_scene(true));
    let opts = SaveOptions::default();
    let text = file::save_with(&text_scene, &opts);
    let bin = file::save_with(&bin_scene, &opts);
    if std::env::var_os("FORGE_WRITE_FIXTURES").is_some() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        std::fs::write(dir.join("scene-v6.forge"), &text).unwrap();
        std::fs::write(dir.join("scene-v6-chunked.forge"), &bin).unwrap();
    }
    assert_eq!(text, fixture("scene-v6.forge"), "text fixture drifted");
    assert_eq!(
        bin,
        fixture("scene-v6-chunked.forge"),
        "chunk fixture drifted"
    );
    assert!(!contains(&text, b"%%FORGE-CHUNKS"));
    assert!(contains(&bin, b"\n%%FORGE-CHUNKS 1\n"));
    for (bytes, scene) in [(&text, &text_scene), (&bin, &bin_scene)] {
        let (loaded, report) = file::load_bytes(registry(), bytes).unwrap();
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_eq!(file::save(&loaded), file::save(scene));
    }
}

/// A scene with a small mesh (inline) or one large mesh and index list (chunks).
fn fixture_scene(big: bool) -> forge_core::Scene {
    let mut doc = Document::new(registry());
    doc.transact("build", |tx| {
        let p = tx.create("test.poly", None, None)?;
        tx.rename(p, "Poly")?;
        if big {
            tx.set_param(p, "mesh", Value::Mesh(Arc::new(grid_mesh(20))))?;
            tx.set_param(p, "selection.points", Value::IndexList((0..300).collect()))?;
        } else {
            tx.set_param(p, "mesh", Value::Mesh(Arc::new(quad())))?;
            tx.set_param(p, "selection.points", Value::IndexList(vec![1, 3]))?;
        }
        Ok(())
    })
    .unwrap();
    doc.scene().clone()
}

fn grid_mesh(n: u32) -> MeshData {
    let mut points = Vec::new();
    for j in 0..=n {
        for i in 0..=n {
            points.push([
                f64::from(i) * 0.1,
                f64::from(j) / 3.0,
                (f64::from(i * j)).sqrt(),
            ]);
        }
    }
    let mut polys = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            polys.push(vec![a, a + 1, a + n + 2, a + n + 1]);
        }
    }
    MeshData { points, polys }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn small_values_stay_text_and_large_ones_become_chunks() {
    assert_eq!(CHUNK_MIN_BYTES, 1024);
    let small = file::save_with(&fixture_scene(false), &SaveOptions::default());
    let text = String::from_utf8(small).unwrap();
    assert!(text.contains("Mesh(("), "{text}");
    assert!(text.contains("IndexList("), "{text}");

    let big = file::save_with(&fixture_scene(true), &SaveOptions::default());
    let marker = b"\n%%FORGE-CHUNKS 1\n";
    let at = big.windows(marker.len()).position(|w| w == marker).unwrap();
    let head = std::str::from_utf8(&big[..at]).unwrap();
    assert!(head.contains("version: 6"), "{head}");
    assert!(head.contains("\"mesh\": MeshChunk(0)"), "{head}");
    assert!(
        head.contains("\"selection.points\": IndexListChunk(1)"),
        "{head}"
    );
    assert!(!head.contains("points: ["), "no inline arrays: {head}");
    // 441 points x 24 bytes + 400 quads x 20 bytes + 300 x 4: far below the text form
    let all_text = file::save(&fixture_scene(true));
    assert!(
        big.len() * 2 < all_text.len(),
        "{} vs {}",
        big.len(),
        all_text.len()
    );
    // a zero threshold chunks everything chunkable; usize::MAX nothing
    let all = file::save_with(
        &fixture_scene(false),
        &SaveOptions {
            chunk_min_bytes: 0,
            ..SaveOptions::default()
        },
    );
    assert!(contains(&all, b"MeshChunk(0)") && contains(&all, b"IndexListChunk(1)"));
    let none = file::save_with(
        &fixture_scene(true),
        &SaveOptions {
            chunk_min_bytes: usize::MAX,
            ..SaveOptions::default()
        },
    );
    assert_eq!(none, all_text.into_bytes());
}

#[test]
fn chunked_round_trip_is_bit_exact_and_deterministic() {
    let tricky = [
        -0.0,
        0.1 + 0.2,
        f64::MIN_POSITIVE,
        5e-324,
        1.7976931348623157e308,
        std::f64::consts::PI,
        1.0 / 3.0,
        -123_456_789.123_456_79,
    ];
    let mut mesh = grid_mesh(12);
    for (i, v) in tricky.iter().enumerate() {
        mesh.points[i] = [*v, -*v, *v * 0.5];
    }
    let mut doc = Document::new(registry());
    let id = doc
        .transact("m", |tx| {
            let p = tx.create("test.poly", None, None)?;
            tx.set_param(p, "mesh", Value::Mesh(Arc::new(mesh.clone())))?;
            Ok(p)
        })
        .unwrap();
    for opts in [
        SaveOptions::default(),
        SaveOptions {
            chunk_min_bytes: usize::MAX,
            ..SaveOptions::default()
        },
    ] {
        let a = file::save_with(doc.scene(), &opts);
        assert_eq!(a, file::save_with(doc.scene(), &opts));
        let (scene, report) = file::load_bytes(registry(), &a).unwrap();
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        let Some(Value::Mesh(back)) = scene.param(id, "mesh") else {
            panic!("mesh")
        };
        assert_eq!(back.polys, mesh.polys);
        for (x, y) in back
            .points
            .iter()
            .flatten()
            .zip(mesh.points.iter().flatten())
        {
            assert_eq!(x.to_bits(), y.to_bits(), "{x} vs {y}");
        }
        assert_eq!(file::save_with(&scene, &opts), a);
    }
}

#[test]
fn text_loader_refuses_chunk_references_without_a_section() {
    let bin = file::save_with(&fixture_scene(true), &SaveOptions::default());
    let marker = b"\n%%FORGE-CHUNKS 1\n";
    let at = bin.windows(marker.len()).position(|w| w == marker).unwrap();
    let head = std::str::from_utf8(&bin[..at]).unwrap();
    assert!(matches!(
        file::load(registry(), head),
        Err(LoadError::Chunk(_))
    ));
    assert!(matches!(
        file::load_bytes(registry(), head.as_bytes()),
        Err(LoadError::Chunk(_))
    ));
}

#[test]
fn damaged_chunk_sections_are_errors_not_panics() {
    let bin = file::save_with(&fixture_scene(true), &SaveOptions::default());
    let marker = b"\n%%FORGE-CHUNKS 1\n";
    let at = bin.windows(marker.len()).position(|w| w == marker).unwrap() + marker.len();
    let chunk_err = |bytes: &[u8]| match file::load_bytes(registry(), bytes) {
        Err(LoadError::Chunk(_)) => {}
        other => panic!("expected a chunk error, got {:?}", other.map(|_| ())),
    };
    // truncated at every length inside the section
    for cut in (at..bin.len()).step_by(37).chain([bin.len() - 1]) {
        chunk_err(&bin[..cut]);
    }
    // flipped payload byte: hash mismatch
    let mut bad = bin.clone();
    let last = bad.len() - 5;
    bad[last] ^= 0x40;
    chunk_err(&bad);
    // trailing byte
    let mut bad = bin.clone();
    bad.push(0);
    chunk_err(&bad);
    // absurd chunk count
    let mut bad = bin.clone();
    bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    chunk_err(&bad);
    // unknown chunk kind
    let mut bad = bin.clone();
    bad[at + 4] = 99;
    chunk_err(&bad);
    // a reference to a chunk that does not exist
    let text =
        String::from_utf8_lossy(&bin[..at - marker.len()]).replace("MeshChunk(0)", "MeshChunk(9)");
    let mut bad = text.into_bytes();
    bad.extend_from_slice(&bin[at - marker.len()..]);
    chunk_err(&bad);
    // a chunk of the wrong kind for its reference
    let text = String::from_utf8_lossy(&bin[..at - marker.len()])
        .replace("MeshChunk(0)", "IndexListChunk(0)");
    let mut bad = text.into_bytes();
    bad.extend_from_slice(&bin[at - marker.len()..]);
    chunk_err(&bad);
}

#[test]
fn the_reader_is_total_on_mutated_input() {
    // Deterministic stand-in for the fuzz target of brief 182: flip, cut and splice bytes of
    // real files; every result must be Ok or Err, never a panic or a huge allocation.
    let seeds = [
        file::save_with(&fixture_scene(true), &SaveOptions::default()),
        file::save_with(&fixture_scene(false), &SaveOptions::default()),
        fixture("scene-v3.forge"),
        fixture("scene-v5.forge"),
    ];
    let reg = registry();
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for seed in &seeds {
        for _ in 0..300 {
            let mut b = seed.clone();
            for _ in 0..=(next() % 4) {
                let i = (next() as usize) % b.len();
                match next() % 3 {
                    0 => b[i] ^= 1 << (next() % 8),
                    1 => b.truncate(i.max(1)),
                    _ => b.insert(i, next() as u8),
                }
            }
            let _ = file::load_bytes(reg.clone(), &b);
        }
    }
    for junk in [&b""[..], b"\n%%FORGE-CHUNKS 1\n", b"\xff\xfe", b"(", b"()"] {
        assert!(file::load_bytes(reg.clone(), junk).is_err());
    }
}

#[test]
fn versions_zero_and_future_are_refused() {
    let t = |v: u32| format!(r#"(format: "forge3d-scene", version: {v}, roots: [])"#);
    assert_eq!(
        file::load(registry(), &t(0)).err(),
        Some(LoadError::UnsupportedVersion(0))
    );
    assert_eq!(
        file::load(registry(), &t(VERSION + 1)).err(),
        Some(LoadError::UnsupportedVersion(VERSION + 1))
    );
    assert!(file::load(registry(), &t(VERSION)).is_ok());
}

#[test]
fn version_5_files_with_link_lists_load() {
    // Written by builds between briefs 068 and 163 (the version was not bumped then).
    let mut r = (*registry()).clone();
    r.register(
        NodeType::new("test.cloner", "Cloner", Category::Generator).param(ParamDesc::link_list(
            "effectors",
            "Effectors",
            &[Category::Object, Category::Generator],
        )),
    )
    .unwrap();
    let r = Arc::new(r);
    let text = r#"(format: "forge3d-scene", version: 5, settings: (fps: 30, start: 0, end: 90), roots: [
        (id: 1, type_id: "forge.null", name: "a", params: {}, tracks: {}, tags: [], children: []),
        (id: 2, type_id: "test.cloner", name: "c", params: {"effectors": LinkList([1])}, tracks: {}, tags: [], children: []),
    ])"#;
    let (scene, report) = file::load(r.clone(), text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(
        scene.param(NodeId(2), "effectors"),
        Some(&Value::LinkList(vec![NodeId(1)]))
    );
    let saved = file::save(&scene);
    assert!(saved.contains(&format!("version: {VERSION}")));
    assert_eq!(file::save(&file::load(r, &saved).unwrap().0), saved);
}

#[test]
fn hidden_object_params_of_brief_149_are_filled_for_old_files() {
    let (scene, report) = file::load_bytes(registry(), &fixture("scene-v5.forge")).unwrap();
    assert!(report.warnings.is_empty());
    assert!(scene.param(NodeId(1), "object.enabled").is_some());
    assert!(scene.param(NodeId(1), "object.editor_vis").is_some());
}

// --- external references ---

/// An absolute path on this OS: Windows needs a drive prefix.
fn abs(p: &str) -> String {
    if cfg!(windows) {
        format!("C:{p}")
    } else {
        p.to_string()
    }
}

fn tex_scene(path: &str) -> (forge_core::Scene, NodeId) {
    let mut doc = Document::new(registry());
    let id = doc
        .transact("t", |tx| {
            let id = tx.create("test.tex", None, None)?;
            tx.set_param(id, "file", Value::String(path.into()))?;
            tx.set_param(id, "note", Value::String(abs("/proj/scenes/keep.png")))?;
            Ok(id)
        })
        .unwrap();
    (doc.scene().clone(), id)
}

fn saved_text(scene: &forge_core::Scene, base: &str) -> String {
    let opts = SaveOptions {
        base_dir: Some(PathBuf::from(abs(base))),
        ..SaveOptions::default()
    };
    String::from_utf8(file::save_with(scene, &opts)).unwrap()
}

#[test]
fn absolute_paths_are_saved_relative_to_the_scene_folder() {
    for (abs_path, rel) in [
        ("/proj/scenes/tex/a.png", "tex/a.png"),
        ("/proj/scenes/a.png", "a.png"),
        ("/proj/shared/b.png", "../shared/b.png"),
        ("/other/deep/c.png", "../../other/deep/c.png"),
        ("/proj/scenes/./x/../y.png", "y.png"),
        ("", ""),
        ("relative/already.png", "relative/already.png"),
    ] {
        let path = if abs_path.starts_with('/') {
            abs(abs_path)
        } else {
            abs_path.to_string()
        };
        let (scene, _) = tex_scene(&path);
        let text = saved_text(&scene, "/proj/scenes");
        assert!(
            text.contains(&format!("\"file\": String(\"{rel}\")")),
            "{path}: {text}"
        );
        // only file reference params are rewritten
        let keep = format!("\"note\": String({:?})", abs("/proj/scenes/keep.png"));
        assert!(text.contains(&keep), "{text}");
    }
}

#[test]
fn relative_paths_resolve_against_the_new_folder_so_projects_can_move() {
    let (scene, id) = tex_scene(&abs("/proj/scenes/tex/a.png"));
    let bytes = saved_text(&scene, "/proj/scenes").into_bytes();
    for (base, expect) in [
        ("/proj/scenes", "/proj/scenes/tex/a.png"),
        ("/moved/here", "/moved/here/tex/a.png"),
    ] {
        let (base, expect) = (abs(base), abs(expect));
        let expect = expect.as_str();
        let opts = LoadOptions {
            base_dir: Some(PathBuf::from(base)),
            ..LoadOptions::default()
        };
        let (loaded, report) = file::load_with(registry(), &bytes, &opts).unwrap();
        assert!(report.warnings.is_empty());
        assert_eq!(
            loaded.param(id, "file"),
            Some(&Value::String(expect.into()))
        );
        assert_eq!(
            external_refs(&loaded)
                .iter()
                .map(|r| (r.node, r.param.as_str(), r.path.as_str()))
                .collect::<Vec<_>>(),
            vec![(id, "file", expect)]
        );
    }
    // ".." segments
    let (scene, id) = tex_scene(&abs("/proj/shared/b.png"));
    let bytes = saved_text(&scene, "/proj/scenes").into_bytes();
    let opts = LoadOptions {
        base_dir: Some(PathBuf::from(abs("/x/y"))),
        ..LoadOptions::default()
    };
    let (loaded, _) = file::load_with(registry(), &bytes, &opts).unwrap();
    assert_eq!(
        loaded.param(id, "file"),
        Some(&Value::String(abs("/x/shared/b.png")))
    );
    // without a base dir nothing is touched
    let (loaded, _) = file::load_bytes(registry(), &bytes).unwrap();
    assert_eq!(
        loaded.param(id, "file"),
        Some(&Value::String("../shared/b.png".into()))
    );
}

#[test]
fn missing_external_files_warn_only_when_asked_and_the_link_is_kept() {
    let dir = std::env::temp_dir().join(format!("forge-ext-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("tex")).unwrap();
    std::fs::write(dir.join("tex/there.png"), b"x").unwrap();
    let (scene, id) = tex_scene("tex/there.png");
    let bytes = file::save_with(&scene, &SaveOptions::default());
    let check = |bytes: &[u8], on: bool| {
        let opts = LoadOptions {
            base_dir: Some(dir.clone()),
            check_external: on,
        };
        file::load_with(registry(), bytes, &opts).unwrap()
    };
    let (_, report) = check(&bytes, true);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);

    let (scene, id2) = tex_scene("tex/gone.png");
    let bytes = file::save_with(&scene, &SaveOptions::default());
    let (loaded, report) = check(&bytes, true);
    assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
    assert!(report.warnings[0].contains("not found") && report.warnings[0].contains("gone.png"));
    assert!(
        matches!(loaded.param(id2, "file"), Some(Value::String(s)) if s.ends_with("tex/gone.png"))
    );
    assert!(check(&bytes, false).1.warnings.is_empty());
    let _ = id;
    std::fs::remove_dir_all(&dir).unwrap();
}
