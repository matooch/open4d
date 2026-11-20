//! Reading and writing OpenVDB float grids (docs/specs/VOLUME.md section 10).
//!
//! The layout follows the public description of the file format (version 224: header, grid
//! descriptors, the 5-4-3 tree as topology then buffers, node masks, per-node value compression
//! with a metadata byte). It is tested by round trips and hand-built streams, not against a
//! reference file (none could be exported).

use std::collections::BTreeMap;

use glam::{DVec3, IVec3};

use crate::grid::SparseGrid;

const MAGIC: i64 = 0x5644_4220;
const WRITE_VERSION: u32 = 224;
const FLOAT_TREE: &str = "Tree_float_5_4_3";
const ZIP: u32 = 1;
const ACTIVE_MASK: u32 = 2;
const BLOSC: u32 = 4;
const MAX_NODES: u32 = 1 << 16;

/// What the values of a grid are used for (the `class` metadata).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridClass {
    LevelSet,
    FogVolume,
    Unknown,
}

impl GridClass {
    fn name(self) -> &'static str {
        match self {
            GridClass::LevelSet => "level set",
            GridClass::FogVolume => "fog volume",
            GridClass::Unknown => "unknown",
        }
    }
    fn parse(s: &str) -> GridClass {
        match s {
            "level set" => GridClass::LevelSet,
            "fog volume" => GridClass::FogVolume,
            _ => GridClass::Unknown,
        }
    }
}

/// One float grid of a file.
#[derive(Debug, Clone, PartialEq)]
pub struct VdbGrid {
    pub name: String,
    pub class: GridClass,
    pub grid: SparseGrid<f32>,
    /// Inactive tiles with a value other than the background that the reader dropped.
    pub skipped_tiles: usize,
}

impl VdbGrid {
    pub fn new(name: &str, class: GridClass, grid: SparseGrid<f32>) -> VdbGrid {
        VdbGrid {
            name: name.to_string(),
            class,
            grid,
            skipped_tiles: 0,
        }
    }
}

/// The result of [`read_vdb`]: the float grids and the number of grids of other types skipped.
#[derive(Debug, Clone, PartialEq)]
pub struct VdbFile {
    pub grids: Vec<VdbGrid>,
    pub skipped_grids: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VdbError {
    #[error("not a VDB file (bad magic)")]
    BadMagic,
    #[error("unexpected end of data at byte {0}")]
    Truncated(usize),
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    #[error("corrupt: {0}")]
    Corrupt(&'static str),
}

// ---------------------------------------------------------------------------------------------
// Writing

/// Options of [`write_vdb_with`].
#[derive(Debug, Clone, Copy, Default)]
pub struct WriteOptions {
    /// Wrap every value block in a zlib stream of stored blocks (exercises the zip path).
    pub zip: bool,
}

/// Writes the grids as a version 224 file.
pub fn write_vdb(grids: &[VdbGrid]) -> Vec<u8> {
    write_vdb_with(grids, WriteOptions::default())
}

pub fn write_vdb_with(grids: &[VdbGrid], opts: WriteOptions) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&WRITE_VERSION.to_le_bytes());
    out.extend_from_slice(&9u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.push(1);
    out.extend_from_slice(b"00000000-0000-4000-8000-000000000000");
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(grids.len() as u32).to_le_bytes());
    for g in grids {
        write_grid(&mut out, g, opts);
    }
    out
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn put_meta(out: &mut Vec<u8>, name: &str, ty: &str, value: &[u8]) {
    put_str(out, name);
    put_str(out, ty);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

fn write_grid(out: &mut Vec<u8>, g: &VdbGrid, opts: WriteOptions) {
    put_str(out, &g.name);
    put_str(out, FLOAT_TREE);
    put_str(out, "");
    let offsets_at = out.len();
    out.extend_from_slice(&[0u8; 24]);
    let grid_pos = out.len() as i64;
    let flags = ACTIVE_MASK | if opts.zip { ZIP } else { 0 };
    out.extend_from_slice(&flags.to_le_bytes());
    // grid metadata
    let bounds = g.grid.bounds().unwrap_or((IVec3::ZERO, IVec3::ZERO));
    let vec3i = |v: IVec3| {
        let mut b = Vec::new();
        for c in v.to_array() {
            b.extend_from_slice(&c.to_le_bytes());
        }
        b
    };
    out.extend_from_slice(&5u32.to_le_bytes());
    put_meta(out, "class", "string", g.class.name().as_bytes());
    put_meta(out, "name", "string", g.name.as_bytes());
    put_meta(
        out,
        "file_voxel_count",
        "int64",
        &(g.grid.active_count() as i64).to_le_bytes(),
    );
    put_meta(out, "file_bbox_min", "vec3i", &vec3i(bounds.0));
    put_meta(out, "file_bbox_max", "vec3i", &vec3i(bounds.1));
    // transform: a row-vector affine map
    put_str(out, "AffineMap");
    let h = g.grid.voxel_size();
    let o = g.grid.origin();
    let m = [
        h, 0.0, 0.0, 0.0, 0.0, h, 0.0, 0.0, 0.0, 0.0, h, 0.0, o.x, o.y, o.z, 1.0,
    ];
    for v in m {
        out.extend_from_slice(&v.to_le_bytes());
    }
    // tree
    let (topology, buffers) = write_tree(&g.grid, opts);
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&topology);
    let block_pos = out.len() as i64;
    out.extend_from_slice(&buffers);
    let end_pos = out.len() as i64;
    out[offsets_at..offsets_at + 8].copy_from_slice(&grid_pos.to_le_bytes());
    out[offsets_at + 8..offsets_at + 16].copy_from_slice(&block_pos.to_le_bytes());
    out[offsets_at + 16..offsets_at + 24].copy_from_slice(&end_pos.to_le_bytes());
}

/// A mask of `words.len() * 64` bits.
struct Mask(Vec<u64>);

impl Mask {
    fn new(bits: usize) -> Mask {
        Mask(vec![0; bits.div_ceil(64)])
    }
    fn set(&mut self, i: usize) {
        self.0[i / 64] |= 1 << (i % 64);
    }
    fn get(&self, i: usize) -> bool {
        self.0[i / 64] >> (i % 64) & 1 == 1
    }
    fn count(&self) -> usize {
        self.0.iter().map(|w| w.count_ones() as usize).sum()
    }
    fn save(&self, out: &mut Vec<u8>) {
        for w in &self.0 {
            out.extend_from_slice(&w.to_le_bytes());
        }
    }
}

/// A leaf of the written tree, in file (x-major) voxel order.
struct Leaf {
    key: IVec3,
    values: Vec<f32>,
    active: Mask,
}

fn vdb_offset(o: usize) -> usize {
    // our offset is x + 8 y + 64 z; the file's is 64 x + 8 y + z
    let (x, y, z) = (o % 8, (o / 8) % 8, o / 64);
    x * 64 + y * 8 + z
}

fn floor_div(v: IVec3, d: i32) -> IVec3 {
    IVec3::new(v.x.div_euclid(d), v.y.div_euclid(d), v.z.div_euclid(d))
}

fn key3(v: IVec3) -> (i32, i32, i32) {
    (v.x, v.y, v.z)
}

fn write_tree(grid: &SparseGrid<f32>, opts: WriteOptions) -> (Vec<u8>, Vec<u8>) {
    let bg = grid.background();
    let mut leaves: Vec<Leaf> = Vec::new();
    for (key, values, words) in grid.blocks() {
        let mut v = vec![0.0f32; 512];
        let mut active = Mask::new(512);
        let mut any = false;
        for (o, val) in values.iter().enumerate() {
            let on = words[o / 64] >> (o % 64) & 1 == 1;
            v[vdb_offset(o)] = *val;
            if on {
                active.set(vdb_offset(o));
            }
            any |= on || *val != bg;
        }
        if any {
            leaves.push(Leaf {
                key,
                values: v,
                active,
            });
        }
    }
    // group: level 2 (512 leaves = 4096 voxels per axis) > level 1 (16 leaves) > leaf
    type L1 = BTreeMap<(i32, i32, i32), Vec<usize>>;
    let mut l2: BTreeMap<(i32, i32, i32), L1> = BTreeMap::new();
    for (i, l) in leaves.iter().enumerate() {
        let k2 = key3(floor_div(l.key, 512));
        let k1 = key3(floor_div(l.key, 16));
        l2.entry(k2).or_default().entry(k1).or_default().push(i);
    }
    let mut topo = Vec::new();
    let mut bufs = Vec::new();
    topo.extend_from_slice(&bg.to_le_bytes());
    topo.extend_from_slice(&0u32.to_le_bytes());
    topo.extend_from_slice(&(l2.len() as u32).to_le_bytes());
    for (k2, children) in &l2 {
        for c in [k2.0, k2.1, k2.2] {
            topo.extend_from_slice(&(c * 4096).to_le_bytes());
        }
        // level 2 node: 32^3 slots, each a level 1 node covering 128 voxels (16 leaves)
        let mut child_mask = Mask::new(32 * 32 * 32);
        for k1 in children.keys() {
            let (lx, ly, lz) = (
                k1.0.rem_euclid(32) as usize,
                k1.1.rem_euclid(32) as usize,
                k1.2.rem_euclid(32) as usize,
            );
            child_mask.set(lx * 1024 + ly * 32 + lz);
        }
        child_mask.save(&mut topo);
        Mask::new(32 * 32 * 32).save(&mut topo);
        write_tile_values(&mut topo, bg, 32 * 32 * 32);
        for members in children.values() {
            let mut child_mask = Mask::new(16 * 16 * 16);
            let mut by_slot: BTreeMap<usize, usize> = BTreeMap::new();
            for &i in members {
                let k = leaves[i].key;
                let slot = (k.x.rem_euclid(16) * 256 + k.y.rem_euclid(16) * 16 + k.z.rem_euclid(16))
                    as usize;
                child_mask.set(slot);
                by_slot.insert(slot, i);
            }
            child_mask.save(&mut topo);
            Mask::new(16 * 16 * 16).save(&mut topo);
            write_tile_values(&mut topo, bg, 16 * 16 * 16);
            for i in by_slot.values() {
                leaves[*i].active.save(&mut topo);
                write_leaf_buffer(&mut bufs, &leaves[*i], bg, opts);
            }
        }
    }
    (topo, bufs)
}

/// Tile values of an internal node without tiles: every inactive slot is the background.
fn write_tile_values(out: &mut Vec<u8>, _bg: f32, _slots: usize) {
    out.push(0); // metadata: inactive values are all the background
}

fn put_block(out: &mut Vec<u8>, bytes: &[u8], opts: WriteOptions) {
    // an empty block writes nothing, zipped or not
    if !opts.zip || bytes.is_empty() {
        out.extend_from_slice(bytes);
        return;
    }
    let z = zlib_stored(bytes);
    out.extend_from_slice(&(z.len() as i64).to_le_bytes());
    out.extend_from_slice(&z);
}

fn write_leaf_buffer(out: &mut Vec<u8>, leaf: &Leaf, bg: f32, opts: WriteOptions) {
    leaf.active.save(out);
    // inactive values of the leaf
    let mut distinct: Vec<f32> = Vec::new();
    for (i, v) in leaf.values.iter().enumerate() {
        if !leaf.active.get(i) && !distinct.iter().any(|d| d.to_bits() == v.to_bits()) {
            distinct.push(*v);
        }
    }
    let mut selection = Mask::new(512);
    let (meta, head): (u8, Vec<f32>) = match distinct.len() {
        0 => (0, vec![]),
        1 if distinct[0].to_bits() == bg.to_bits() => (0, vec![]),
        1 if distinct[0].to_bits() == (-bg).to_bits() => (1, vec![]),
        1 => (2, vec![distinct[0]]),
        2 => {
            for (i, v) in leaf.values.iter().enumerate() {
                if !leaf.active.get(i) && v.to_bits() == distinct[1].to_bits() {
                    selection.set(i);
                }
            }
            (5, vec![distinct[0], distinct[1]])
        }
        _ => (6, vec![]),
    };
    out.push(meta);
    for v in &head {
        out.extend_from_slice(&v.to_le_bytes());
    }
    if meta == 5 {
        selection.save(out);
    }
    let mut data = Vec::new();
    for (i, v) in leaf.values.iter().enumerate() {
        if meta == 6 || leaf.active.get(i) {
            data.extend_from_slice(&v.to_le_bytes());
        }
    }
    put_block(out, &data, opts);
}

// ---------------------------------------------------------------------------------------------
// zlib

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &d in data {
        a = (a + u32::from(d)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// A zlib stream of stored (uncompressed) blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut chunks = data.chunks(65535).peekable();
    if chunks.peek().is_none() {
        out.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    while let Some(c) = chunks.next() {
        out.push(u8::from(chunks.peek().is_none()));
        out.extend_from_slice(&(c.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
        out.extend_from_slice(c);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    bit: u32,
    buf: u32,
}

impl Bits<'_> {
    fn need(&mut self, n: u32) -> Result<(), VdbError> {
        while self.bit < n {
            let b = *self
                .data
                .get(self.pos)
                .ok_or(VdbError::Truncated(self.pos))?;
            self.pos += 1;
            self.buf |= u32::from(b) << self.bit;
            self.bit += 8;
        }
        Ok(())
    }
    fn take(&mut self, n: u32) -> Result<u32, VdbError> {
        if n == 0 {
            return Ok(0);
        }
        self.need(n)?;
        let v = self.buf & ((1u32 << n) - 1);
        self.buf >>= n;
        self.bit -= n;
        Ok(v)
    }
}

struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Huffman {
        let mut counts = [0u16; 16];
        for &l in lengths {
            counts[l as usize] += 1;
        }
        counts[0] = 0;
        let mut offs = [0u16; 16];
        for i in 1..16 {
            offs[i] = offs[i - 1] + counts[i - 1];
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (s, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbols[offs[l as usize] as usize] = s as u16;
                offs[l as usize] += 1;
            }
        }
        Huffman { counts, symbols }
    }
    fn decode(&self, b: &mut Bits) -> Result<u16, VdbError> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= b.take(1)? as i32;
            let count = i32::from(self.counts[len]);
            if code - count < first {
                return self
                    .symbols
                    .get((index + (code - first)) as usize)
                    .copied()
                    .ok_or(VdbError::Corrupt("bad Huffman code"));
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(VdbError::Corrupt("bad Huffman code"))
    }
}

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// Inflates a zlib stream (RFC 1950 / 1951) into exactly `expected` bytes.
pub fn inflate_zlib(data: &[u8], expected: usize) -> Result<Vec<u8>, VdbError> {
    if data.len() < 2
        || data[0] & 0x0f != 8
        || (u16::from(data[0]) * 256 + u16::from(data[1])) % 31 != 0
    {
        return Err(VdbError::Corrupt("bad zlib header"));
    }
    let mut b = Bits {
        data,
        pos: 2,
        bit: 0,
        buf: 0,
    };
    let mut out: Vec<u8> = Vec::with_capacity(expected.min(1 << 24));
    loop {
        let last = b.take(1)? == 1;
        match b.take(2)? {
            0 => {
                b.bit = 0;
                b.buf = 0;
                let p = b.pos;
                let hdr = data.get(p..p + 4).ok_or(VdbError::Truncated(p))?;
                let len = usize::from(u16::from_le_bytes([hdr[0], hdr[1]]));
                if hdr[2] != !hdr[0] || hdr[3] != !hdr[1] {
                    return Err(VdbError::Corrupt("bad stored block"));
                }
                let body = data.get(p + 4..p + 4 + len).ok_or(VdbError::Truncated(p))?;
                out.extend_from_slice(body);
                b.pos = p + 4 + len;
            }
            t @ (1 | 2) => {
                let (lit, dist) = if t == 1 {
                    let mut l = [8u8; 288];
                    l[144..256].fill(9);
                    l[256..280].fill(7);
                    (Huffman::new(&l), Huffman::new(&[5u8; 30]))
                } else {
                    let hlit = b.take(5)? as usize + 257;
                    let hdist = b.take(5)? as usize + 1;
                    let hclen = b.take(4)? as usize + 4;
                    const ORDER: [usize; 19] = [
                        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
                    ];
                    let mut cl = [0u8; 19];
                    for &o in ORDER.iter().take(hclen) {
                        cl[o] = b.take(3)? as u8;
                    }
                    let clh = Huffman::new(&cl);
                    let mut lengths = vec![0u8; hlit + hdist];
                    let mut i = 0;
                    while i < hlit + hdist {
                        let sym = clh.decode(&mut b)?;
                        let (val, rep) = match sym {
                            0..=15 => (sym as u8, 1),
                            16 => {
                                if i == 0 {
                                    return Err(VdbError::Corrupt("repeat without length"));
                                }
                                (lengths[i - 1], 3 + b.take(2)? as usize)
                            }
                            17 => (0, 3 + b.take(3)? as usize),
                            _ => (0, 11 + b.take(7)? as usize),
                        };
                        if i + rep > hlit + hdist {
                            return Err(VdbError::Corrupt("too many code lengths"));
                        }
                        lengths[i..i + rep].fill(val);
                        i += rep;
                    }
                    (
                        Huffman::new(&lengths[..hlit]),
                        Huffman::new(&lengths[hlit..]),
                    )
                };
                loop {
                    let sym = lit.decode(&mut b)?;
                    match sym {
                        0..=255 => out.push(sym as u8),
                        256 => break,
                        257..=285 => {
                            let i = usize::from(sym - 257);
                            let len = usize::from(LEN_BASE[i])
                                + b.take(u32::from(LEN_EXTRA[i]))? as usize;
                            let d = dist.decode(&mut b)? as usize;
                            if d >= 30 {
                                return Err(VdbError::Corrupt("bad distance code"));
                            }
                            let dd = usize::from(DIST_BASE[d])
                                + b.take(u32::from(DIST_EXTRA[d]))? as usize;
                            if dd > out.len() {
                                return Err(VdbError::Corrupt("distance too far back"));
                            }
                            for _ in 0..len {
                                let v = out[out.len() - dd];
                                out.push(v);
                            }
                        }
                        _ => return Err(VdbError::Corrupt("bad length code")),
                    }
                    if out.len() > expected {
                        return Err(VdbError::Corrupt("inflated data too long"));
                    }
                }
            }
            _ => return Err(VdbError::Corrupt("bad block type")),
        }
        if out.len() > expected {
            return Err(VdbError::Corrupt("inflated data too long"));
        }
        if last {
            break;
        }
    }
    if out.len() != expected {
        return Err(VdbError::Corrupt("inflated size differs"));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Reading

struct Cur<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], VdbError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(VdbError::Truncated(self.pos))?;
        let s = self
            .data
            .get(self.pos..end)
            .ok_or(VdbError::Truncated(self.pos))?;
        self.pos = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, VdbError> {
        Ok(self.bytes(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, VdbError> {
        Ok(u32::from_le_bytes(
            self.bytes(4)?.try_into().expect("4 bytes"),
        ))
    }
    fn i32(&mut self) -> Result<i32, VdbError> {
        Ok(self.u32()? as i32)
    }
    fn i64(&mut self) -> Result<i64, VdbError> {
        Ok(i64::from_le_bytes(
            self.bytes(8)?.try_into().expect("8 bytes"),
        ))
    }
    fn f32(&mut self) -> Result<f32, VdbError> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn f64(&mut self) -> Result<f64, VdbError> {
        Ok(f64::from_bits(self.i64()? as u64))
    }
    fn string(&mut self) -> Result<String, VdbError> {
        let n = self.u32()? as usize;
        Ok(String::from_utf8_lossy(self.bytes(n)?).into_owned())
    }
    fn mask(&mut self, bits: usize) -> Result<Mask, VdbError> {
        let mut m = Mask::new(bits);
        for w in &mut m.0 {
            *w = u64::from_le_bytes(self.bytes(8)?.try_into().expect("8 bytes"));
        }
        Ok(m)
    }
}

/// Metadata block: returns the `(name, type, bytes)` entries.
fn read_meta(c: &mut Cur) -> Result<Vec<(String, String, Vec<u8>)>, VdbError> {
    let n = c.u32()?;
    if n > 4096 {
        return Err(VdbError::Corrupt("too many metadata entries"));
    }
    let mut out = Vec::new();
    for _ in 0..n {
        let name = c.string()?;
        let ty = c.string()?;
        let len = c.u32()? as usize;
        out.push((name, ty, c.bytes(len)?.to_vec()));
    }
    Ok(out)
}

fn read_transform(c: &mut Cur) -> Result<(f64, DVec3), VdbError> {
    let ty = c.string()?;
    let mut d = |n: usize| -> Result<Vec<f64>, VdbError> { (0..n).map(|_| c.f64()).collect() };
    match ty.as_str() {
        "AffineMap" => {
            let m = d(16)?;
            Ok((m[0], DVec3::new(m[12], m[13], m[14])))
        }
        "UniformScaleMap" | "ScaleMap" => {
            let v = d(15)?;
            Ok((v[0], DVec3::ZERO))
        }
        "UniformScaleTranslateMap" | "ScaleTranslateMap" => {
            let v = d(18)?;
            Ok((v[3], DVec3::new(v[0], v[1], v[2])))
        }
        _ => Err(VdbError::Unsupported("transform type")),
    }
}

/// The flags and background that value blocks need.
struct Ctx {
    version: u32,
    flags: u32,
    background: f32,
}

/// Reads the `count` values of a node (`readCompressedValues`).
fn read_values(
    c: &mut Cur,
    ctx: &Ctx,
    count: usize,
    value_mask: &Mask,
) -> Result<Vec<f32>, VdbError> {
    let mask_compressed = ctx.flags & ACTIVE_MASK != 0;
    let mut meta = 6u8;
    if ctx.version >= 222 {
        meta = c.u8()?;
        if meta > 6 {
            return Err(VdbError::Corrupt("bad value metadata"));
        }
    }
    let bg = ctx.background;
    let mut inactive0 = if meta == 0 { bg } else { -bg };
    let mut inactive1 = bg;
    if matches!(meta, 2 | 4 | 5) {
        inactive0 = c.f32()?;
        if meta == 5 {
            inactive1 = c.f32()?;
        }
    }
    let selection = if matches!(meta, 3..=5) {
        Some(c.mask(count)?)
    } else {
        None
    };
    let temp_count = if mask_compressed && meta != 6 && ctx.version >= 222 {
        value_mask.count()
    } else {
        count
    };
    let bytes = temp_count
        .checked_mul(4)
        .ok_or(VdbError::Corrupt("size overflow"))?;
    let raw: Vec<u8> = if bytes == 0 {
        Vec::new()
    } else if ctx.flags & BLOSC != 0 {
        return Err(VdbError::Unsupported("blosc compression"));
    } else if ctx.flags & ZIP != 0 {
        let n = c.i64()?;
        if n > 0 {
            let z = c.bytes(usize::try_from(n).map_err(|_| VdbError::Corrupt("size"))?)?;
            inflate_zlib(z, bytes)?
        } else {
            let len = usize::try_from(n.unsigned_abs()).map_err(|_| VdbError::Corrupt("size"))?;
            if len != bytes {
                return Err(VdbError::Corrupt("raw block size differs"));
            }
            c.bytes(len)?.to_vec()
        }
    } else {
        c.bytes(bytes)?.to_vec()
    };
    let temp: Vec<f32> = raw
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect();
    if mask_compressed && temp_count != count {
        let mut out = Vec::with_capacity(count);
        let mut t = 0;
        for i in 0..count {
            if value_mask.get(i) {
                out.push(
                    *temp
                        .get(t)
                        .ok_or(VdbError::Corrupt("missing active value"))?,
                );
                t += 1;
            } else if selection.as_ref().is_some_and(|s| s.get(i)) {
                out.push(inactive1);
            } else {
                out.push(inactive0);
            }
        }
        Ok(out)
    } else {
        Ok(temp)
    }
}

/// Topology of one internal node, with the leaves' topology below it.
struct Node {
    /// `(slot, node)` children of an internal node, ascending; empty for leaves.
    children: Vec<(usize, Node)>,
    /// Leaf: the value mask.
    mask: Option<Mask>,
}

struct Tree {
    /// Level 2 children with their origin.
    roots: Vec<(IVec3, Node)>,
    skipped_tiles: usize,
}

fn read_internal(c: &mut Cur, ctx: &Ctx, log2: u32, skipped: &mut usize) -> Result<Node, VdbError> {
    let slots = 1usize << (3 * log2);
    let child_mask = c.mask(slots)?;
    let value_mask = c.mask(slots)?;
    let values = read_values(c, ctx, slots, &value_mask)?;
    for (i, value) in values.iter().enumerate() {
        if child_mask.get(i) {
            continue;
        }
        if value_mask.get(i) {
            return Err(VdbError::Unsupported("active tiles"));
        }
        if value.to_bits() != ctx.background.to_bits() {
            *skipped += 1;
        }
    }
    let mut children = Vec::new();
    for i in 0..slots {
        if child_mask.get(i) {
            let node = if log2 == 4 {
                Node {
                    children: Vec::new(),
                    mask: Some(c.mask(512)?),
                }
            } else {
                read_internal(c, ctx, 4, skipped)?
            };
            children.push((i, node));
        }
    }
    Ok(Node {
        children,
        mask: None,
    })
}

fn read_topology(c: &mut Cur, ctx: &Ctx) -> Result<Tree, VdbError> {
    let tiles = c.u32()?;
    let nchildren = c.u32()?;
    if tiles > MAX_NODES || nchildren > MAX_NODES {
        return Err(VdbError::Corrupt("too many root entries"));
    }
    let mut skipped = 0;
    for _ in 0..tiles {
        let _origin = [c.i32()?, c.i32()?, c.i32()?];
        let value = c.f32()?;
        let active = c.u8()?;
        if active != 0 {
            return Err(VdbError::Unsupported("active tiles"));
        }
        if value.to_bits() != ctx.background.to_bits() {
            skipped += 1;
        }
    }
    let mut roots = Vec::new();
    for _ in 0..nchildren {
        let origin = IVec3::new(c.i32()?, c.i32()?, c.i32()?);
        roots.push((origin, read_internal(c, ctx, 5, &mut skipped)?));
    }
    Ok(Tree {
        roots,
        skipped_tiles: skipped,
    })
}

/// Reads the buffers of the leaves in topology order and fills the grid.
fn read_buffers(
    c: &mut Cur,
    ctx: &Ctx,
    tree: &Tree,
    grid: &mut SparseGrid<f32>,
) -> Result<(), VdbError> {
    for (origin, l2) in &tree.roots {
        for (slot2, l1) in &l2.children {
            let o2 = *origin
                + IVec3::new(
                    (*slot2 / 1024) as i32,
                    ((*slot2 / 32) % 32) as i32,
                    (*slot2 % 32) as i32,
                ) * 128;
            for (slot1, leaf) in &l1.children {
                let key = floor_div(
                    o2 + IVec3::new(
                        (*slot1 / 256) as i32,
                        ((*slot1 / 16) % 16) as i32,
                        (*slot1 % 16) as i32,
                    ) * 8,
                    8,
                );
                let topology_mask = leaf
                    .mask
                    .as_ref()
                    .ok_or(VdbError::Corrupt("leaf without mask"))?;
                let mask = c.mask(512)?;
                if mask.0 != topology_mask.0 {
                    return Err(VdbError::Corrupt("leaf masks differ"));
                }
                let values = read_values(c, ctx, 512, &mask)?;
                let mut ours = vec![ctx.background; 512];
                let mut words = [0u64; 8];
                for (o, slot) in ours.iter_mut().enumerate() {
                    let f = vdb_offset(o);
                    *slot = values[f];
                    if mask.get(f) {
                        words[o / 64] |= 1 << (o % 64);
                    }
                }
                if words.iter().any(|w| *w != 0)
                    || ours.iter().any(|v| v.to_bits() != ctx.background.to_bits())
                {
                    grid.set_block(key, &ours, words);
                }
            }
        }
    }
    Ok(())
}

/// Reads the float grids of a VDB file.
pub fn read_vdb(data: &[u8]) -> Result<VdbFile, VdbError> {
    let mut c = Cur { data, pos: 0 };
    if c.i64()? != MAGIC {
        return Err(VdbError::BadMagic);
    }
    let version = c.u32()?;
    if !(222..=228).contains(&version) {
        return Err(VdbError::Unsupported("file version"));
    }
    let _major = c.u32()?;
    let _minor = c.u32()?;
    let offsets = c.u8()? != 0;
    c.bytes(36)?;
    read_meta(&mut c)?;
    let ngrids = c.u32()?;
    if ngrids > 4096 {
        return Err(VdbError::Corrupt("too many grids"));
    }
    let mut grids = Vec::new();
    let mut skipped_grids = 0;
    for _ in 0..ngrids {
        let name = c.string()?;
        let ty = c.string()?;
        let _instance = c.string()?;
        let (grid_pos, _block_pos, end_pos) = if offsets {
            (c.i64()?, c.i64()?, c.i64()?)
        } else {
            (0, 0, 0)
        };
        if ty != FLOAT_TREE {
            if !offsets {
                return Err(VdbError::Unsupported("grid type without offsets"));
            }
            c.pos = usize::try_from(end_pos).map_err(|_| VdbError::Corrupt("grid end"))?;
            if c.pos > data.len() {
                return Err(VdbError::Truncated(c.pos));
            }
            skipped_grids += 1;
            continue;
        }
        if offsets && usize::try_from(grid_pos).ok() != Some(c.pos) {
            return Err(VdbError::Corrupt("grid offset"));
        }
        let flags = c.u32()?;
        let meta = read_meta(&mut c)?;
        let class = meta
            .iter()
            .find(|(n, _, _)| n == "class")
            .map_or(GridClass::Unknown, |(_, _, v)| {
                GridClass::parse(&String::from_utf8_lossy(v))
            });
        let (voxel, origin) = read_transform(&mut c)?;
        if !(voxel.is_finite() && voxel > 0.0 && origin.is_finite()) {
            return Err(VdbError::Corrupt("transform"));
        }
        let buffer_count = c.u32()?;
        if buffer_count != 1 {
            return Err(VdbError::Unsupported("multiple buffers"));
        }
        let background = c.f32()?;
        let ctx = Ctx {
            version,
            flags,
            background,
        };
        let tree = read_topology(&mut c, &ctx)?;
        let mut grid = SparseGrid::new(background, voxel, origin);
        read_buffers(&mut c, &ctx, &tree, &mut grid)?;
        grids.push(VdbGrid {
            name,
            class,
            grid,
            skipped_tiles: tree.skipped_tiles,
        });
        if offsets {
            // trust the end offset when it is where we stopped, otherwise resynchronise
            let end = usize::try_from(end_pos).map_err(|_| VdbError::Corrupt("grid end"))?;
            if end < c.pos || end > data.len() {
                return Err(VdbError::Corrupt("grid end"));
            }
            c.pos = end;
        }
    }
    Ok(VdbFile {
        grids,
        skipped_grids,
    })
}
