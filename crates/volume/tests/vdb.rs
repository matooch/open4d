//! Test vectors made with a reference zlib: fixed and dynamic Huffman blocks.
const FIXED_A10: &str = "78da4b4c84010014e103cb";
const DYNAMIC_PATTERN: &str = "78daedd2439220010000b0b16ddbb66ddbb66ddbb66ddbb66ddbb64efb8eadeabc2120d0485884140cec7c62b22a3ac6564e9e0111f16979a5352ddd43538b1b07e7776fbf908818f8e474acbc22d2ca5a86960eeefe61b1a939c5d54d9d8313f3eb7ba7b72fdf10f06878a4342cdc42528a1a06e6766ebe2131c95945950d1dfd6373ab3b27d74f5f60b0a838c4d44c9c8212f2ea7aa6b62edec151899905e5f56dbda333cbdb47978f1f2030c85844940c1cfc6272aa3a26d64e5e811109e97965b52d3dc3534b9b0717f76f7f50889804e4f46cbca232cada46968e1efee171a9b925d5cd5d83930bebfb67b7af3f1008e87864b42c3cc2524a9a0616f66e7ea13129d945558d1d03e3736bbb2737cf5fe070a8b824d4cc5c82920aeafa66b6ae3ec1d149998515f5ed7da3b32bdbc7578f9fa03028d844548c1c02e2726aba2636ce5e41910919f96575ad3d23d34b5b87170fef7fd048988414f4ec7ca2b22adac6568e9e01e1f169b9a535cddd43938b1bfbe777afbf900818f86474ac3c22d24a5a86160eee7e61b12939c5554d9d0313f36b7ba7372fdfe0f068b8a434ccdc42928a1afae676aebe21d1c95985950dedfd63b3ab3bc7d74f9f60b02838c4544c9c0212f26a7aa6362ede4151891905e5756dbd2333cb5b47970f1f20d0c85884940cecfc62b2aa3ac6d64e9e8111f1e979a5b52dddc3538b9b07e7f76fbf50881804e4746cbc2232ca5a46960e1efe6171a93925d54d5d83130beb7b67b72f3f10f0e878a4b42cdcc2528a9a06e6f66ebea131c9d945958d1dfde373abbb27d7cf5f6070a83824d44c5c82120aea7a66b62e3ec15149990515f56d7da3332bdb47578f1fa030c8d844948c1cfce272aaba26d6ce5e819109e9f965b5ad3dc3d34b9b8717f7ef7f5048980414f46c7ca2322ada46568e1e01e17169b92535cd5d43930b1bfb6777af3f9008e8f864b4ac3cc2d24a9a8616f6ee7ea1b129d9c5558d9d03e3f36bbba737cfdfe07068b82434cc5c42920a1afa6676ae3e21d1495985150ded7d63b32b3bc7574f9fa0b028d8c4548c9c02e2f26abaa636cede41918919f9e575adbd23d3cb5b87970fef204078203c101e080f8407c203e181f0ff67f87faf22b57c";
const FIXED_HELLO: &str = "78dacb48cdc9c95728cf2fca4951c8c0ce0600ef930d55";
const DYNAMIC_NOISE: &str = "78da01f4010bfe0000000102030406080a0c0f1215181c2024282d32373c42484e545b6269707880889099a2abb4bec8d2dce7f2fd0814202c3845525f6c7a8896a4b3c2d1e0f00010203142536476889aacbfd2e5f80c2034485d72879cb2c8def40b223950688098b0c9e2fb142e48627c97b2cde804203c587592afccea0826446382a1c0e00020406182a3c4e6082a4c6f92b5d8fc2044688db2d7fc22486e94bbe209305880a8d0f9224b749ec8f21c47729dc8f4204c78a5d2ff2c5a88b6e4134271a0d000306091c2f3245688baec1f5285b8ec205488bdf2275c92c8fe346ba2d9104880b8f029629bd40e4882bcf7326da8e4205c98d5124f8cca084684c3024180c0004080c1024384c6084a8ccf125598dc2064a8ed3277bc02488ed41b62a9f03880c81059a2eb347ec8125ca7f23d88d4206cb805529fec3a88d62473c21160b00050a0f14293e43688da2c7fd22578cc2074c81d72c71c72c81e74cb2279d02880d83089e23b94ee48a2fc57b20d68c4207cd83592ef4caa0866c42382e140a00060c02182e344a6086acc2f92f558bc2084e84db2177ce248ae147be249b01880e850b9228bf45ec8329c0772dd48b4208cf865d23fac1a88f664d342b120900070e051c233a41688fa6cdf52c538ac2094087df267dc52c83eb42ba219900880f870e962db54ce48c23cb732ad28a4209c18959cceecb2";

mod common;

use common::{cube, sphere};
use forge_volume::vdb::inflate_zlib;
use forge_volume::{
    GridClass, SdfParams, SparseGrid, VdbError, VdbGrid, WriteOptions, mesh_to_sdf, read_vdb,
    sdf_to_fog, write_vdb, write_vdb_with,
};
use glam::{DVec3, IVec3};

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn pattern() -> Vec<u8> {
    (0..3000usize)
        .map(|i| ((i * 7 + i / 3) % 251) as u8)
        .collect()
}

#[test]
fn inflate_reads_fixed_dynamic_and_stored_blocks() {
    assert_eq!(inflate_zlib(&unhex(FIXED_A10), 10).unwrap(), vec![b'a'; 10]);
    assert_eq!(
        inflate_zlib(&unhex(FIXED_HELLO), 35).unwrap(),
        b"hello world hello world hello world"
    );
    assert_eq!(
        inflate_zlib(&unhex(DYNAMIC_PATTERN), 3000).unwrap(),
        pattern()
    );
    let noise: Vec<u8> = (0..500usize)
        .map(|i| (((i * i) >> 3) % 256) as u8)
        .collect();
    assert_eq!(inflate_zlib(&unhex(DYNAMIC_NOISE), 500).unwrap(), noise);
    // wrong expected size, truncation and a bad header are errors, not panics
    assert!(inflate_zlib(&unhex(FIXED_A10), 11).is_err());
    assert!(inflate_zlib(&unhex(DYNAMIC_PATTERN)[..100], 3000).is_err());
    assert!(inflate_zlib(&[0x00, 0x00, 0x00], 1).is_err());
}

fn assert_same(a: &SparseGrid<f32>, b: &SparseGrid<f32>) {
    assert_eq!(a.background(), b.background());
    assert_eq!(a.voxel_size(), b.voxel_size());
    assert_eq!(a.origin(), b.origin());
    assert_eq!(a.active_count(), b.active_count());
    // every voxel either grid stores reads the same value and activity on both
    for (i, _, _) in a.stored_voxels().iter().chain(&b.stored_voxels()) {
        assert_eq!(a.get(*i).to_bits(), b.get(*i).to_bits(), "{i}");
        assert_eq!(a.is_active(*i), b.is_active(*i), "{i}");
    }
}

fn sdf_sphere() -> SparseGrid<f32> {
    mesh_to_sdf(&sphere(100.0, 24, 32, DVec3::ZERO), &SdfParams::default())
}

#[test]
fn a_written_grid_reads_back_exactly() {
    let g = sdf_sphere();
    let bytes = write_vdb(&[VdbGrid::new("sphere", GridClass::LevelSet, g.clone())]);
    assert_eq!(&bytes[..8], &[0x20, 0x42, 0x44, 0x56, 0, 0, 0, 0]);
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 224);
    let f = read_vdb(&bytes).unwrap();
    assert_eq!(f.skipped_grids, 0);
    assert_eq!(f.grids.len(), 1);
    let r = &f.grids[0];
    assert_eq!(r.name, "sphere");
    assert_eq!(r.class, GridClass::LevelSet);
    assert_eq!(r.skipped_tiles, 0);
    assert_same(&g, &r.grid);
    // the filled interior (a second inactive value) survives
    assert_eq!(r.grid.get(IVec3::ZERO), -20.0);
    assert!(!r.grid.is_active(IVec3::ZERO));
}

#[test]
fn zipped_blocks_and_every_node_level() {
    let g = sdf_sphere();
    let plain = write_vdb(&[VdbGrid::new("s", GridClass::LevelSet, g.clone())]);
    let zipped = write_vdb_with(
        &[VdbGrid::new("s", GridClass::LevelSet, g.clone())],
        WriteOptions { zip: true },
    );
    assert_ne!(plain, zipped);
    assert_same(&g, &read_vdb(&zipped).unwrap().grids[0].grid);

    // far apart voxels in different level 1 and level 2 nodes, negative coordinates, an origin
    // and a voxel size that are not the default
    let mut h = SparseGrid::new(0.5f32, 0.25, DVec3::new(1.5, -2.0, 0.125));
    let far = [
        IVec3::new(-5000, 3, 4097),
        IVec3::new(-5001, 3, 4097),
        IVec3::new(0, 0, 0),
        IVec3::new(127, -128, 129),
        IVec3::new(128, -129, 130),
        IVec3::new(4096, 4096, -4096),
    ];
    for (n, i) in far.iter().enumerate() {
        h.set(*i, n as f32 * 0.1 - 0.2);
    }
    h.set_inactive(IVec3::new(5, 5, 5), -0.5);
    h.set_inactive(IVec3::new(6, 5, 5), 0.5); // the background
    h.set_inactive(IVec3::new(7, 5, 5), 0.123); // a third inactive value in the leaf
    for z in [false, true] {
        let b = write_vdb_with(
            &[VdbGrid::new("far", GridClass::Unknown, h.clone())],
            WriteOptions { zip: z },
        );
        let r = read_vdb(&b).unwrap();
        assert_same(&h, &r.grids[0].grid);
        assert_eq!(r.grids[0].grid.get(IVec3::new(7, 5, 5)), 0.123);
        assert_eq!(r.grids[0].class, GridClass::Unknown);
    }
}

#[test]
fn several_grids_fog_and_an_empty_grid() {
    let sdf = sdf_sphere();
    let fog = sdf_to_fog(&sdf, 20.0);
    let empty = SparseGrid::new(3.0f32, 2.0, DVec3::ZERO);
    let c = mesh_to_sdf(
        &cube(DVec3::splat(50.0), DVec3::ZERO),
        &SdfParams::default(),
    );
    let bytes = write_vdb(&[
        VdbGrid::new("a", GridClass::LevelSet, sdf.clone()),
        VdbGrid::new("fog", GridClass::FogVolume, fog.clone()),
        VdbGrid::new("empty", GridClass::Unknown, empty.clone()),
        VdbGrid::new("cube", GridClass::LevelSet, c.clone()),
    ]);
    let f = read_vdb(&bytes).unwrap();
    let names: Vec<_> = f.grids.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["a", "fog", "empty", "cube"]);
    assert_same(&sdf, &f.grids[0].grid);
    assert_same(&fog, &f.grids[1].grid);
    assert_eq!(f.grids[1].class, GridClass::FogVolume);
    assert_eq!(f.grids[2].grid.active_count(), 0);
    assert_eq!(f.grids[2].grid.background(), 3.0);
    assert_same(&c, &f.grids[3].grid);
}

#[test]
fn other_grid_types_are_skipped_through_the_end_offset() {
    let g = sdf_sphere();
    let mut bytes = write_vdb(&[
        VdbGrid::new("vec", GridClass::Unknown, g.clone()),
        VdbGrid::new("keep", GridClass::LevelSet, g.clone()),
    ]);
    // rename the first grid's type to another one of the same length
    let at = bytes
        .windows(16)
        .position(|w| w == b"Tree_float_5_4_3")
        .unwrap();
    bytes[at..at + 16].copy_from_slice(b"Tree_vec3f_5_4_3");
    let f = read_vdb(&bytes).unwrap();
    assert_eq!(f.skipped_grids, 1);
    assert_eq!(f.grids.len(), 1);
    assert_eq!(f.grids[0].name, "keep");
    assert_same(&g, &f.grids[0].grid);
}

/// A minimal file with one grid whose root has the given topology bytes and no buffers.
fn file_with_root(flags: u32, root: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0x5644_4220i64.to_le_bytes());
    for v in [224u32, 9, 1] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.push(1);
    out.extend_from_slice(&[b'0'; 36]);
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    let s = |o: &mut Vec<u8>, t: &str| {
        o.extend_from_slice(&(t.len() as u32).to_le_bytes());
        o.extend_from_slice(t.as_bytes());
    };
    s(&mut out, "g");
    s(&mut out, "Tree_float_5_4_3");
    s(&mut out, "");
    let at = out.len();
    out.extend_from_slice(&[0u8; 24]);
    let start = out.len() as i64;
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    s(&mut out, "UniformScaleMap");
    for v in [
        10.0f64, 10.0, 10.0, 0.1, 0.1, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(root);
    let end = out.len() as i64;
    out[at..at + 8].copy_from_slice(&start.to_le_bytes());
    out[at + 8..at + 16].copy_from_slice(&end.to_le_bytes());
    out[at + 16..at + 24].copy_from_slice(&end.to_le_bytes());
    out
}

fn root(background: f32, tiles: &[([i32; 3], f32, u8)], children: u32) -> Vec<u8> {
    let mut r = background.to_le_bytes().to_vec();
    r.extend_from_slice(&(tiles.len() as u32).to_le_bytes());
    r.extend_from_slice(&children.to_le_bytes());
    for (o, v, a) in tiles {
        for c in o {
            r.extend_from_slice(&c.to_le_bytes());
        }
        r.extend_from_slice(&v.to_le_bytes());
        r.push(*a);
    }
    r
}

#[test]
fn tiles_blosc_and_other_foreign_features() {
    // an empty root with an older style transform reads: voxel size from the scale map
    let f = read_vdb(&file_with_root(2, &root(20.0, &[], 0))).unwrap();
    assert_eq!(f.grids[0].grid.voxel_size(), 10.0);
    assert_eq!(f.grids[0].grid.background(), 20.0);
    // an inactive tile with the background is nothing; with another value it is counted
    let f = read_vdb(&file_with_root(2, &root(20.0, &[([0, 0, 0], 20.0, 0)], 0))).unwrap();
    assert_eq!(f.grids[0].skipped_tiles, 0);
    let f = read_vdb(&file_with_root(2, &root(20.0, &[([0, 0, 0], -20.0, 0)], 0))).unwrap();
    assert_eq!(f.grids[0].skipped_tiles, 1);
    // an active tile and blosc are refused
    assert_eq!(
        read_vdb(&file_with_root(2, &root(20.0, &[([0, 0, 0], 1.0, 1)], 0))).unwrap_err(),
        VdbError::Unsupported("active tiles")
    );
    // a child with blosc values: the first value block of the child fails cleanly
    let mut child = root(20.0, &[], 1);
    child.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let r = read_vdb(&file_with_root(2 | 4, &child));
    assert!(r.is_err());
    // wrong magic, wrong version, tiny inputs
    assert_eq!(
        read_vdb(b"nope nope nope nope").unwrap_err(),
        VdbError::BadMagic
    );
    assert!(read_vdb(&[]).is_err());
    let mut bad = file_with_root(2, &root(20.0, &[], 0));
    bad[8] = 99;
    assert_eq!(
        read_vdb(&bad).unwrap_err(),
        VdbError::Unsupported("file version")
    );
    // absurd counts are refused before any allocation
    assert!(read_vdb(&file_with_root(2, &root(20.0, &[], u32::MAX))).is_err());
}

#[test]
fn the_reader_survives_truncation_and_corruption() {
    let g = sdf_sphere();
    let bytes = write_vdb(&[VdbGrid::new("s", GridClass::LevelSet, g)]);
    // every prefix is an error (the grid offsets say how long the file must be)
    let step = (bytes.len() / 400).max(1);
    for n in (0..bytes.len()).step_by(step) {
        assert!(read_vdb(&bytes[..n]).is_err(), "prefix {n}");
    }
    // seeded corruption: flip bytes, any outcome but a panic is fine
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..300 {
        let mut b = bytes.clone();
        for _ in 0..1 + next() % 8 {
            let i = (next() % b.len() as u64) as usize;
            b[i] = next() as u8;
        }
        let _ = read_vdb(&b);
    }
    // and a header-heavy corruption: random bytes after a valid magic
    for _ in 0..100 {
        let mut b = bytes[..200].to_vec();
        for byte in b.iter_mut().skip(12) {
            *byte = next() as u8;
        }
        let _ = read_vdb(&b);
    }
}

/// O33 `vdb-export` needs a human (the reference's VDB export wedges the app), so the golden
/// file is absent: UNSUPPORTED until it exists. When it appears it must read as a float grid
/// with voxel size 10 (the case's builder), a background and active voxels.
#[test]
fn the_reference_vdb_is_read_when_it_exists() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/volumes/vdb-export/volume.vdb");
    if !path.exists() {
        // UNSUPPORTED: docs/parity/volumes-2.md, `vdb-export` not observed
        return;
    }
    let f = read_vdb(&std::fs::read(&path).unwrap()).expect("the reference file reads");
    let g = &f.grids[0].grid;
    assert!((g.voxel_size() - 10.0).abs() < 1e-9, "{}", g.voxel_size());
    assert!(g.active_count() > 100);
    assert!(g.background() > 0.0);
}
