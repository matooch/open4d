mod common;
use common::*;
use forge_image::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

fn write_png(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let path = dir.join(name);
    let bytes = encode(&gradient(w, h), Format::Png, &EncodeOptions::default()).unwrap();
    std::fs::write(&path, bytes).unwrap();
    path
}

fn no_mips() -> LoadOptions {
    LoadOptions {
        mips: false,
        ..LoadOptions::default()
    }
}

#[test]
fn miss_then_hit_returns_same_arc() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_png(dir.path(), "a.png", 8, 8);
    let cache = TextureCache::new(1 << 20);
    let a = cache.get(&p, &LoadOptions::default()).unwrap();
    let b = cache.get(&p, &LoadOptions::default()).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    let s = cache.stats();
    assert_eq!((s.hits, s.misses, s.entries), (1, 1, 1));
    assert_eq!(s.bytes, a.bytes());
    assert_eq!(a.levels.len(), 4); // 8,4,2,1
    // an unnormalised path to the same file hits the same entry
    let alt = dir.path().join("sub/../a.png");
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    let c = cache.get(&alt, &LoadOptions::default()).unwrap();
    assert!(Arc::ptr_eq(&a, &c));
}

#[test]
fn options_are_part_of_the_key_and_change_the_data() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_png(dir.path(), "a.png", 4, 4);
    let cache = TextureCache::new(1 << 20);
    let srgb = cache.get(&p, &no_mips()).unwrap();
    let lin = cache
        .get(
            &p,
            &LoadOptions {
                tag: ColorSpace::Linear,
                ..no_mips()
            },
        )
        .unwrap();
    let raw = cache
        .get(
            &p,
            &LoadOptions {
                tag: ColorSpace::Raw,
                ..no_mips()
            },
        )
        .unwrap();
    assert_eq!(cache.stats().misses, 3);
    assert_eq!(cache.stats().entries, 3);
    // sRGB tag decodes to darker linear values than the linear tag for mid values
    let (a, b) = (srgb.levels[0].pixel(2, 2), lin.levels[0].pixel(2, 2));
    assert!(a[0] < b[0]);
    assert_eq!(lin.levels[0].data, raw.levels[0].data);
    assert_eq!(srgb.tag, ColorSpace::Srgb);
    assert_eq!(srgb.levels.len(), 1);
}

#[test]
fn mips_are_built_in_linear_space() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("checker.png");
    let img = Image::from_fn(2, 2, AlphaMode::Straight, |x, y| {
        let v = ((x + y) % 2) as f32;
        [v, v, v, 1.0]
    });
    std::fs::write(
        &p,
        encode(&img, Format::Png, &EncodeOptions::default()).unwrap(),
    )
    .unwrap();
    let cache = TextureCache::new(1 << 20);
    let t = cache.get(&p, &LoadOptions::default()).unwrap();
    assert!((t.levels[1].pixel(0, 0)[0] - 0.5).abs() < 1e-6);
}

#[test]
fn invalidated_when_size_or_mtime_changes() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_png(dir.path(), "a.png", 4, 4);
    let cache = TextureCache::new(1 << 20);
    let a = cache.get(&p, &no_mips()).unwrap();
    assert_eq!(a.levels[0].width, 4);
    // different content and size
    write_png(dir.path(), "a.png", 6, 6);
    let b = cache.get(&p, &no_mips()).unwrap();
    assert_eq!(b.levels[0].width, 6);
    assert!(!Arc::ptr_eq(&a, &b));
    let s = cache.stats();
    assert_eq!((s.invalidations, s.misses, s.entries), (1, 2, 1));
    // same size, only the mtime moves
    let f = std::fs::OpenOptions::new().write(true).open(&p).unwrap();
    f.set_modified(SystemTime::now() + Duration::from_secs(3600))
        .unwrap();
    drop(f);
    let c = cache.get(&p, &no_mips()).unwrap();
    assert!(!Arc::ptr_eq(&b, &c));
    assert_eq!(cache.stats().invalidations, 2);
    assert_eq!(cache.stats().entries, 1);
    // unchanged now: hit
    let d = cache.get(&p, &no_mips()).unwrap();
    assert!(Arc::ptr_eq(&c, &d));
}

#[test]
fn deleted_file_is_an_error_and_drops_entries() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_png(dir.path(), "a.png", 4, 4);
    let cache = TextureCache::new(1 << 20);
    cache.get(&p, &no_mips()).unwrap();
    std::fs::remove_file(&p).unwrap();
    assert!(matches!(cache.get(&p, &no_mips()), Err(ImageError::Io(_))));
    assert_eq!(cache.stats().entries, 0);
    assert_eq!(cache.stats().bytes, 0);
}

#[test]
fn lru_eviction_under_budget() {
    let dir = tempfile::tempdir().unwrap();
    let names = ["a.png", "b.png", "c.png", "d.png"];
    let paths: Vec<_> = names
        .iter()
        .map(|n| write_png(dir.path(), n, 8, 8))
        .collect();
    // each entry costs 8*8*16 = 1024 bytes; the budget holds two
    let cache = TextureCache::new(2500);
    cache.get(&paths[0], &no_mips()).unwrap();
    cache.get(&paths[1], &no_mips()).unwrap();
    assert_eq!(cache.stats().evictions, 0);
    cache.get(&paths[2], &no_mips()).unwrap(); // evicts a (least recently used)
    assert_eq!(cache.stats().evictions, 1);
    assert_eq!(cache.stats().entries, 2);
    assert!(cache.stats().bytes <= 2500);
    let before = cache.stats().hits;
    cache.get(&paths[1], &no_mips()).unwrap(); // b is still there and now most recent
    assert_eq!(cache.stats().hits, before + 1);
    cache.get(&paths[3], &no_mips()).unwrap(); // evicts c, not b
    assert_eq!(cache.stats().evictions, 2);
    let h = cache.stats().hits;
    cache.get(&paths[1], &no_mips()).unwrap();
    assert_eq!(cache.stats().hits, h + 1);
    let m = cache.stats().misses;
    cache.get(&paths[2], &no_mips()).unwrap();
    assert_eq!(cache.stats().misses, m + 1);
}

#[test]
fn oversize_texture_is_returned_but_not_cached_and_budget_can_shrink() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_png(dir.path(), "a.png", 8, 8);
    let tiny = TextureCache::new(100);
    let t = tiny.get(&p, &no_mips()).unwrap();
    assert_eq!(t.levels[0].width, 8);
    assert_eq!(tiny.stats().entries, 0);
    assert_eq!(tiny.stats().bytes, 0);

    let cache = TextureCache::new(1 << 20);
    cache.get(&p, &no_mips()).unwrap();
    cache.set_budget(10);
    assert_eq!(cache.stats().entries, 0);
    assert!(cache.stats().evictions >= 1);
    cache.set_budget(1 << 20);
    cache.get(&p, &no_mips()).unwrap();
    assert_eq!(cache.stats().entries, 1);
    cache.clear();
    assert_eq!(cache.stats().entries, 0);
    assert_eq!(cache.stats().bytes, 0);
}

#[test]
fn get_ref_treats_empty_as_none() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_png(dir.path(), "a.png", 4, 4);
    let cache = TextureCache::new(1 << 20);
    assert!(cache.get_ref("", &no_mips()).unwrap().is_none());
    let t = cache
        .get_ref(p.to_str().unwrap(), &no_mips())
        .unwrap()
        .unwrap();
    assert_eq!(t.levels[0].height, 4);
    assert!(
        cache
            .get_ref("/definitely/not/here.png", &no_mips())
            .is_err()
    );
}

#[test]
fn corrupt_file_is_a_decode_error_and_not_cached() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("bad.png");
    std::fs::write(&p, b"\x89PNG\r\n\x1a\nbroken").unwrap();
    let cache = TextureCache::new(1 << 20);
    assert!(cache.get(&p, &no_mips()).is_err());
    assert_eq!(cache.stats().entries, 0);
}

#[test]
fn exr_texture_is_linear_and_straight_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("t.exr");
    let img = Image::from_fn(4, 4, AlphaMode::Straight, |_, _| [2.0, 0.5, 0.25, 0.5]);
    let opts = EncodeOptions {
        depth: BitDepth::F32,
        ..EncodeOptions::default()
    };
    std::fs::write(&p, encode(&img, Format::Exr, &opts).unwrap()).unwrap();
    let cache = TextureCache::new(1 << 20);
    let t = cache.get(&p, &LoadOptions::default()).unwrap();
    assert_eq!(t.alpha, AlphaMode::Straight);
    let px = t.levels[0].pixel(1, 1);
    assert!((px[0] - 2.0).abs() < 1e-5 && (px[3] - 0.5).abs() < 1e-6);
    assert!((t.levels.last().unwrap().pixel(0, 0)[0] - 2.0).abs() < 1e-5);
}

#[test]
fn threads_share_one_entry() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_png(dir.path(), "a.png", 16, 16);
    let cache = Arc::new(TextureCache::new(1 << 22));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let c = cache.clone();
            let p = p.clone();
            std::thread::spawn(move || {
                for _ in 0..5 {
                    c.get(&p, &LoadOptions::default()).unwrap();
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let s = cache.stats();
    assert_eq!(s.hits + s.misses, 40);
    assert_eq!(s.entries, 1);
    assert!(s.misses >= 1 && s.misses <= 8);
}

/// macOS temp dirs are symlinks (`/var` → `/private/var`); a deleted file reached through a
/// symlinked folder must still drop its entry. This reproduces that on every Unix host.
#[cfg(unix)]
#[test]
fn deleted_file_through_a_symlinked_folder_drops_entries() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let p = write_png(&link, "a.png", 4, 4);
    let cache = TextureCache::new(1 << 20);
    cache.get(&p, &no_mips()).unwrap();
    assert_eq!(cache.stats().entries, 1);
    std::fs::remove_file(&p).unwrap();
    assert!(matches!(cache.get(&p, &no_mips()), Err(ImageError::Io(_))));
    assert_eq!(cache.stats().entries, 0);
    assert_eq!(cache.stats().bytes, 0);
}
